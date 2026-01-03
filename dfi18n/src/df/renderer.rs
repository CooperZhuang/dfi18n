use std::ffi;
use std::sync::OnceLock;

use lua53_sys as lua;

use crate::types;

// Get current screen info
pub fn get_screen_info() -> &'static types::ScreenInfo {
  unsafe { (*(RENDERER_SCREEN.get().unwrap()) as *const types::ScreenInfo).as_ref_unchecked() }
}

// Get a mutable reference to a screen cell at the given coordinate and layer
pub fn get_cell(coord: &types::Coordinate, is_top: bool) -> &'static mut [u8; 8] {
  let screen_info = get_screen_info();
  let screen = match is_top {
    false => screen_info.screen(),
    true => screen_info.screen_top(),
  };
  screen.cell(coord)
}

// Get the texture cell at the given coordinate and layer
pub fn get_tex_cell(coord: &types::Coordinate, is_top: bool) -> ffi::c_long {
  let screen_info = get_screen_info();
  let screen = match is_top {
    false => screen_info.screen(),
    true => screen_info.screen_top(),
  };
  screen.get_tex(coord)
}

// Get current SDL info
pub fn get_sdl_info() -> &'static types::SDLInfo {
  unsafe { (*(RENDERER_WINDOW.get().unwrap()) as *const types::SDLInfo).as_ref_unchecked() }
}

// Get current renderer info
pub fn get_renderer_info() -> &'static types::RendererInfo {
  unsafe { (*(RENDERER_DISPX.get().unwrap()) as *const types::RendererInfo).as_ref_unchecked() }
}

// renderer.screen
static RENDERER_SCREEN: OnceLock<usize> = OnceLock::new();
// renderer.window
static RENDERER_WINDOW: OnceLock<usize> = OnceLock::new();
// renderer.dispx
static RENDERER_DISPX: OnceLock<usize> = OnceLock::new();

// Set renderer.screen from Lua
#[unsafe(no_mangle)]
extern "C" fn set_enabler_renderer_screen(lua_state: *mut ffi::c_void) {
  let addr = lua::check_integer(lua_state, 1) as usize;
  RENDERER_SCREEN.set(addr).unwrap();
}

// Set renderer.window from Lua
#[unsafe(no_mangle)]
extern "C" fn set_enabler_renderer_window(lua_state: *mut ffi::c_void) {
  let addr = lua::check_integer(lua_state, 1) as usize;
  RENDERER_WINDOW.set(addr).unwrap();
}

// Set renderer.dispx from Lua
#[unsafe(no_mangle)]
extern "C" fn set_enabler_renderer_dispx(lua_state: *mut ffi::c_void) {
  let addr = lua::check_integer(lua_state, 1) as usize;
  RENDERER_DISPX.set(addr).unwrap();
}
