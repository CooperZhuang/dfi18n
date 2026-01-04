use std::collections::{BTreeMap, HashMap};
use std::ffi;
use std::io::Read as _;
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

use lua53_sys as lua;
use sdl2_sys as sdl;

use crate::df;

// TODO: make this configurable
// Font scale factor when rendering glyphs, to make them look sharper
pub const FONT_SCALE_FACTOR: f32 = 2.0;

// Reset all loaded fonts along with cached glyph surfaces and textures
pub fn reset() {
  get_font_glyph_textures_mut().clear();
  get_font_glyph_surfaces_mut().clear();
  get_fonts_mut().clear();
}

// Curses glyph textures maps a renderer along with a character code to its SDL texture
type CursesGlyphTextures = BTreeMap<(usize, u8), sdl::Texture<'static>>;

// All curses glyph textures
static CURSES_GLYPH_TEXTURES: OnceLock<RwLock<CursesGlyphTextures>> = OnceLock::new();

// Getting mutable access to the curses glyph textures
pub fn get_curses_glyph_textures_mut() -> RwLockWriteGuard<'static, CursesGlyphTextures> {
  CURSES_GLYPH_TEXTURES.get_or_init(|| RwLock::new(BTreeMap::new())).write().unwrap()
}

// Gets the SDL texture for a given CP437 codepoint using the curses font
pub fn get_curses_glyph_texture(renderer: &sdl::Renderer<'static>, code: u8) -> sdl::Texture<'static> {
  let renderer_id = renderer.raw() as usize;
  let key = (renderer_id, code);
  let mut curses_glyph_textures = get_curses_glyph_textures_mut();

  curses_glyph_textures
    .entry(key)
    .or_insert_with(|| {
      let surface = df::enabler::get_curses_surface(code);
      sdl::Texture::from_surface(renderer, &surface)
    })
    .clone()
}

// Glyph textures maps a renderer along with a character to its SDL texture
type GlyphTexture = BTreeMap<(usize, char), sdl::Texture<'static>>;

// A collection of glyph textures grouped by language tag
type FontGlyphTextures = HashMap<String, GlyphTexture>;

// All font glyph textures
static FONT_GLYPH_TEXTURES: OnceLock<RwLock<FontGlyphTextures>> = OnceLock::new();

// Getting mutable access to the font glyph textures
pub fn get_font_glyph_textures_mut() -> RwLockWriteGuard<'static, FontGlyphTextures> {
  FONT_GLYPH_TEXTURES.get_or_init(|| RwLock::new(HashMap::new())).write().unwrap()
}

// Glyph surfaces maps a character to its SDL surface
type GlyphSurfaces = BTreeMap<char, sdl::Surface<'static>>;

// A collection of glyph surfaces grouped by language tag
type FontGlyphSurfaces = HashMap<String, GlyphSurfaces>;

// All font glyph surfaces
static FONT_GLYPH_SURFACES: OnceLock<RwLock<FontGlyphSurfaces>> = OnceLock::new();

// Getting mutable access to the font glyph surfaces
pub fn get_font_glyph_surfaces_mut() -> RwLockWriteGuard<'static, FontGlyphSurfaces> {
  FONT_GLYPH_SURFACES.get_or_init(|| RwLock::new(HashMap::new())).write().unwrap()
}

// All loaded fonts mapped by language tag
static FONTS: OnceLock<RwLock<HashMap<String, fontdue::Font>>> = OnceLock::new();

// Getting access to the loaded fonts
pub fn get_fonts() -> RwLockReadGuard<'static, HashMap<String, fontdue::Font>> {
  FONTS.get_or_init(|| RwLock::new(HashMap::new())).read().unwrap()
}

// Getting mutable access to the loaded fonts
pub fn get_fonts_mut() -> RwLockWriteGuard<'static, HashMap<String, fontdue::Font>> {
  FONTS.get_or_init(|| RwLock::new(HashMap::new())).write().unwrap()
}

// Adds a new font from Lua by providing the language tag and the font file path
#[unsafe(no_mangle)]
extern "C" fn add_font(lua_state: *mut ffi::c_void) {
  let lang_tag = lua::check_string(lua_state, 1);
  let path_str = lua::check_string(lua_state, 2);

  // load the font file data
  let mut data = Vec::new();
  if let Err(err) = std::fs::File::open(&path_str).and_then(|mut f| f.read_to_end(&mut data)) {
    log::warn!(r#"Failed to load font file "{path_str}": {err}"#);
    return;
  }

  // parse the font data
  if let Err(err) = fontdue::Font::from_bytes(data, fontdue::FontSettings::default()).map(|font| {
    // store the loaded font
    let mut fonts = get_fonts_mut();
    fonts.insert(lang_tag.to_string(), font);
    // initialize glyph surfaces and textures for the new font
    let mut font_glyph_surfaces = get_font_glyph_surfaces_mut();
    font_glyph_surfaces.insert(lang_tag.to_string(), BTreeMap::new());
    let mut font_glyph_textures = get_font_glyph_textures_mut();
    font_glyph_textures.insert(lang_tag.to_string(), BTreeMap::new());
    log::debug!(r#"Loaded font for language tag "{lang_tag}" from "{path_str}""#);
  }) {
    log::warn!(r#"Failed to parse font file "{path_str}": {err}"#);
    return;
  }
}

// Gets the SDL surface for a given character glyph
pub fn get_glyph_surface(ch: char) -> sdl::Surface<'static> {
  // always use the built-in curses surface for CP437 characters
  let cp437_codepoint = cp437_string::char_to_cp437_codepoint(ch);
  if cp437_codepoint != 0 {
    return df::enabler::get_curses_surface(cp437_codepoint);
  }

  // rasterize the glyph using the loaded font if not already cached
  let lang_tag = crate::lang::current_lang_tag();
  let mut font_glyph_surfaces = get_font_glyph_surfaces_mut();
  if let Some(glyph_surfaces) = font_glyph_surfaces.get_mut(&lang_tag) {
    // only rasterize and cache the glyph if not already done
    if !glyph_surfaces.contains_key(&ch) {
      // ensure the font for the current language tag is loaded
      if let Some(font) = get_fonts().get(&lang_tag) {
        // calculate font size based on original curses font size and scale factor
        let orig_size = df::renderer::get_renderer_info().orig_size();
        let font_scale = FONT_SCALE_FACTOR;
        let font_size = if orig_size.height > orig_size.width {
          orig_size.height as f32
        } else {
          orig_size.width as f32
        } * font_scale;

        // rasterize the glyph, and calculate the surface size and the pixels buffer boundary
        let (metrics, bitmap) = font.rasterize(ch, font_size);
        let mut buff_width = metrics.advance_width.ceil() as i32;
        if (font_size - buff_width as f32).abs() / font_size < 0.1 {
          buff_width = font_size.round() as i32;
        }
        let mut buff_height = metrics.advance_height.ceil() as i32;
        if (font_size - buff_height as f32).abs() / font_size < 0.1 {
          buff_height = font_size.round() as i32;
        }
        let buff_size = (buff_width * buff_height) as isize;

        // create the surface and fill in the pixel data
        let surface = sdl::Surface::new(buff_width, buff_height);
        surface.with_lock_mut(|buffer| {
          // adjustment made to y-offset to better align glyphs vertically
          // TODO: make this configurable
          let y_offset = (font_size / 8.0).round() as i32;

          // horizontal offset of the left-most edge of the glyph bitmap (xmin is the left-most edge)
          let dx = metrics.xmin;
          // vertical offset of the top-most edge of the glyph bitmap (ymin is the bottom-most edge)
          let dy = (buff_height - metrics.height as i32) - (metrics.ymin + y_offset);

          // for each pixel in the glyph bitmap, set the corresponding pixel in the surface buffer
          for y in 0..metrics.height {
            for x in 0..metrics.width {
              // only set the alpha channel based on the glyph bitmap value
              let alpha = bitmap[y * metrics.width + x];

              // calculate the offset in the surface pixels buffer, ensure it's within bounds
              let offset = ((y as i32 + dy) * buff_width + x as i32 + dx) as isize;
              if offset < 0 || offset >= buff_size {
                continue;
              }

              // set the pixel RGBA values (white color with the glyph alpha)
              let offset = offset as usize;
              buffer[offset * 4 + 0] = 255;
              buffer[offset * 4 + 1] = 255;
              buffer[offset * 4 + 2] = 255;
              buffer[offset * 4 + 3] = alpha;
            }
          }
        });

        // cache the rasterized glyph surface
        glyph_surfaces.insert(ch, surface);
      }
    }

    // return the cached glyph surface if available
    if let Some(g) = glyph_surfaces.get(&ch) {
      return g.clone();
    }
  }

  // fallback to the '?' built-in curses surface if failed to rasterize the requested character
  df::enabler::get_curses_surface('?' as u8)
}

// Gets the SDL texture for a given character glyph
pub fn get_glyph_texture(renderer: &sdl::Renderer<'static>, ch: char) -> sdl::Texture<'static> {
  // always use the built-in curses surface for CP437 characters
  let cp437_codepoint = cp437_string::char_to_cp437_codepoint(ch);
  if cp437_codepoint != 0 {
    return get_curses_glyph_texture(renderer, cp437_codepoint);
  }

  // rasterize the glyph into a surface if not already cached, then create and cache the texture
  let lang_tag = crate::lang::current_lang_tag();
  let mut font_glyph_textures = get_font_glyph_textures_mut();
  if let Some(glyph_textures) = font_glyph_textures.get_mut(&lang_tag) {
    // only create and cache the texture if not already done
    let renderer_id = renderer.raw() as usize;
    if !glyph_textures.contains_key(&(renderer_id, ch)) {
      let surface = get_glyph_surface(ch);
      let texture = sdl::Texture::from_surface(renderer, &surface);
      glyph_textures.insert((renderer_id, ch), texture);
    }

    // return the cached glyph texture if available
    if let Some(t) = glyph_textures.get(&(renderer_id, ch)) {
      return t.clone();
    }
  }

  // fallback to the '?' built-in curses texture if failed to create the requested character texture
  get_glyph_texture(renderer, '?')
}

// Gets the original size of a given character glyph
pub fn get_glyph_orig_size(ch: char) -> (i32, i32) {
  if cp437_string::char_to_cp437_codepoint(ch) != 0 {
    // use the built-in curses glyph size for CP437 characters
    let orig_size = df::renderer::get_renderer_info().orig_size();
    (orig_size.width, orig_size.height)
  } else {
    // use the rasterized glyph surface size divided by the font scale factor for other characters
    let surface = get_glyph_surface(ch);
    let (w, h) = surface.get_size();
    (
      (w as f32 / FONT_SCALE_FACTOR).round() as i32,
      (h as f32 / FONT_SCALE_FACTOR).round() as i32,
    )
  }
}
