use std::fs;
use std::sync::{OnceLock, RwLock, RwLockWriteGuard};
use std::{collections::HashMap, ffi};

use lua53_sys as lua;
use sdl2_sys as sdl;

use crate::df;

const LOGO_PNG: &[u8] = include_bytes!("logo.png");

static LOGO_SURFACE: OnceLock<sdl::Surface<'static>> = OnceLock::new();

static LOGO_TEXTURE: OnceLock<sdl::Texture<'static>> = OnceLock::new();

fn get_logo_surface() -> &'static sdl::Surface<'static> {
  LOGO_SURFACE.get_or_init(|| sdl::Surface::load_image(LOGO_PNG))
}

pub fn get_logo_texture() -> &'static sdl::Texture<'static> {
  LOGO_TEXTURE.get_or_init(|| {
    let sdl_renderer = df::renderer::get_sdl_info().renderer();
    let surface = get_logo_surface();
    sdl::Texture::from_raw_surface(&sdl_renderer, surface.raw_mut())
  })
}

// Logo paths grouped by language tag
static TITLE_LOGO_PATHS: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();

// Getting mutable access to the title logo paths
pub fn get_title_logo_paths_mut() -> RwLockWriteGuard<'static, HashMap<String, String>> {
  TITLE_LOGO_PATHS.get_or_init(|| RwLock::new(HashMap::new())).write().unwrap()
}

// Logo textures cache grouped by language tag
static TITLE_LOGO: OnceLock<RwLock<HashMap<String, sdl::Texture<'static>>>> = OnceLock::new();

// Getting mutable access to the title logo textures cache
pub fn get_title_logo_mut() -> RwLockWriteGuard<'static, HashMap<String, sdl::Texture<'static>>> {
  TITLE_LOGO.get_or_init(|| RwLock::new(HashMap::new())).write().unwrap()
}

// Get title logo texture by language tag
pub fn get_title_logo_by_lang_tag(lang_tag: &str) -> Option<sdl::Texture<'static>> {
  let mut title_logo = get_title_logo_mut();
  if title_logo.contains_key(lang_tag) {
    return title_logo.get(lang_tag).cloned();
  }

  let title_logo_paths = get_title_logo_paths_mut();
  if let Some(path_str) = title_logo_paths.get(lang_tag) {
    let sdl_renderer = df::renderer::get_sdl_info().renderer();
    if let Ok(data) = fs::read(path_str) {
      if let Ok(surface) = sdl::Surface::try_load_image(&data) {
        let texture = sdl::Texture::from_surface(&sdl_renderer, &surface);
        return Some(title_logo.entry(lang_tag.to_owned()).or_insert(texture).to_owned());
      }
    }
  }

  None
}

// Load title logo texture from Lua
#[unsafe(no_mangle)]
extern "C" fn load_title_logo(lua_state: *mut ffi::c_void) {
  let lang_tag = lua::check_string(lua_state, 1);
  let path_str = lua::check_string(lua_state, 2);
  let mut title_logo_paths = get_title_logo_paths_mut();
  title_logo_paths.insert(lang_tag.to_owned(), path_str.to_owned());
}
