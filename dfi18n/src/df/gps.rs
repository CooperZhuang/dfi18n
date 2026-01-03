use std::ffi;
use std::sync::OnceLock;

use lua53_sys as lua;

use crate::types;

// Get current coordinate
pub fn get_coordinate() -> &'static types::Coordinate {
  unsafe { (*GPS_SCREENX.get().unwrap() as *const types::Coordinate).as_ref_unchecked() }
}

// Get current dimensions
pub fn get_dimensions() -> &'static types::Dimensions {
  unsafe { (*GPS_DIMX.get().unwrap() as *const types::Dimensions).as_ref_unchecked() }
}

// Get current color info
pub fn get_color_info() -> &'static mut types::ColorInfo {
  unsafe { (*GPS_SCREENF.get().unwrap() as *mut types::ColorInfo).as_mut_unchecked() }
}

// Get the current color pair based on the translation function
pub fn get_color_pair(is_top: bool) -> types::ColorPair {
  let mut color_pair = types::ColorPair::from(get_color_info());
  let screen_info = super::renderer::get_screen_info();
  let screen = if is_top {
    screen_info.screen_top()
  } else {
    screen_info.screen()
  };

  // From g_src/enabler.cpp: renderer::screen_to_texid()
  let coordinate = get_coordinate();
  let dimentions = get_dimensions();
  let tile = (coordinate.column * dimentions.height + coordinate.row) as usize;
  let texpos = unsafe { (screen.texpos as *const ffi::c_long).add(tile).read() };
  let stp_flag = unsafe { (screen.texpos_flag as *const u32).add(tile).read() };
  if texpos != 0 {
    const SCREENTEXPOS_FLAG_GRAYSCALE: u32 = 0x1;
    const SCREENTEXPOS_FLAG_ADDCOLOR: u32 = 0x2;
    if stp_flag & SCREENTEXPOS_FLAG_GRAYSCALE != 0 {
      log::warn!("Unhandled grayscale tile in translation");
    } else if stp_flag & SCREENTEXPOS_FLAG_ADDCOLOR != 0 {
      // proceed normally
    } else {
      color_pair.foreground = types::Color { r: 255, g: 255, b: 255 };
      color_pair.background = types::Color { r: 0, g: 0, b: 0 };
    }
  }

  color_pair
}

// Get display title flag
pub fn get_display_title() -> &'static mut bool {
  unsafe { (*GPS_DISPLAY_TITLE.get().unwrap() as *mut bool).as_mut_unchecked() }
}

// Get texture size
pub fn get_texture_size(n: usize) -> (i32, i32) {
  const TEX_SIZE: usize = 24;
  unsafe { ((*GPS_TEX.get().unwrap() + n * TEX_SIZE) as *const (i32, i32)).read() }
}

// Get texture blits vector
pub fn get_texture_blits() -> &'static mut SVector<TextureBlits> {
  unsafe { (*GPS_TEXBLITS.get().unwrap() as *mut SVector<TextureBlits>).as_mut_unchecked() }
}

// gps.screenx
static GPS_SCREENX: OnceLock<usize> = OnceLock::new();
// gps.dimx
static GPS_DIMX: OnceLock<usize> = OnceLock::new();
// gps.screenf
static GPS_SCREENF: OnceLock<usize> = OnceLock::new();
// gps.tex
static GPS_TEX: OnceLock<usize> = OnceLock::new();
// gps.texblits
static GPS_TEXBLITS: OnceLock<usize> = OnceLock::new();
// gps.display_title
static GPS_DISPLAY_TITLE: OnceLock<usize> = OnceLock::new();

// Set gps.screenx from Lua
#[unsafe(no_mangle)]
extern "C" fn set_gps_screenx(lua_state: *mut ffi::c_void) {
  let addr = lua::check_integer(lua_state, 1) as usize;
  GPS_SCREENX.set(addr).unwrap();
}

// Set gps.dimx from Lua
#[unsafe(no_mangle)]
extern "C" fn set_gps_dimx(lua_state: *mut ffi::c_void) {
  let addr = lua::check_integer(lua_state, 1) as usize;
  GPS_DIMX.set(addr).unwrap();
}

// Set gps.screenf from Lua
#[unsafe(no_mangle)]
extern "C" fn set_gps_screenf(lua_state: *mut ffi::c_void) {
  let addr = lua::check_integer(lua_state, 1) as usize;
  GPS_SCREENF.set(addr).unwrap();
}

// Set gps.tex from Lua
#[unsafe(no_mangle)]
extern "C" fn set_gps_tex(lua_state: *mut ffi::c_void) {
  let addr = lua::check_integer(lua_state, 1) as usize;
  GPS_TEX.set(addr).unwrap();
}

// Set gps.texblits from Lua
#[unsafe(no_mangle)]
extern "C" fn set_gps_texblits(lua_state: *mut ffi::c_void) {
  let addr = lua::check_integer(lua_state, 1) as usize;
  GPS_TEXBLITS.set(addr).unwrap();
}

// Set gps.display_title from Lua
#[unsafe(no_mangle)]
extern "C" fn set_gps_display_title(lua_state: *mut ffi::c_void) {
  let addr = lua::check_integer(lua_state, 1) as usize;
  GPS_DISPLAY_TITLE.set(addr).unwrap();
}

#[repr(C)]
pub struct SVector<T> {
  pub begin: *const T,
  pub end: *const T,
  pub limit: *const T,
}

impl<T: Sized> SVector<T> {
  // Get the size of the SVector
  pub fn size(&self) -> usize {
    (self.end as usize - self.begin as usize) / std::mem::size_of::<T>()
  }

  // Get an element from the SVector by index
  pub fn get(&self, index: usize) -> &T {
    unsafe { self.begin.add(index).as_ref_unchecked() }
  }
}

// TextureBlits structure
#[repr(C)]
#[derive(Clone)]
pub struct TextureBlits {
  pub x: i32,
  pub y: i32,
  pub tex: i8,
}
