use std::ffi;
use std::sync::OnceLock;

use lua53_sys as lua;
use sdl2_sys as sdl;

use crate::memory;

// Get the SDL_Surface pointer of Dwarf Fortress curses font for a given CP437 codepoint
pub fn get_curses_surface(codepoint: u8) -> sdl::Surface<'static> {
  let textures_base = unsafe { *(*(ENABLER_TEXTURES.get().unwrap()) as *const usize) };
  let surface_raw = unsafe { *((textures_base + (codepoint as usize * 8)) as *const *mut sdl::SDL_Surface) };
  sdl::Surface::from_raw(surface_raw)
}

// Call the Dwarf Fortress function to get the key display
pub fn get_key_display(string_ptr: *const ffi::c_void, binding: i32) {
  let func_ptr: *const ffi::c_void = memory::get_raw_pointer_by_key("get_key_display").unwrap();
  let func: fn(*const ffi::c_void, *const ffi::c_void, i32) = unsafe { std::mem::transmute(func_ptr) };
  let enabler = *ENABLER.get().unwrap() as *const ffi::c_void;
  #[cfg(target_os = "windows")]
  func(enabler, string_ptr, binding);
  #[cfg(target_os = "linux")]
  func(string_ptr, enabler, binding);
}

// enabler
static ENABLER: OnceLock<usize> = OnceLock::new();
// enabler.textures
static ENABLER_TEXTURES: OnceLock<usize> = OnceLock::new();

// Set enabler from Lua
#[unsafe(no_mangle)]
extern "C" fn set_enabler(lua_state: *mut ffi::c_void) {
  let addr = lua::check_integer(lua_state, 1) as usize;
  ENABLER.set(addr).unwrap();
}

// Set enabler.textures from Lua
#[unsafe(no_mangle)]
extern "C" fn set_enabler_textures(lua_state: *mut ffi::c_void) {
  let addr = lua::check_integer(lua_state, 1) as usize;
  ENABLER_TEXTURES.set(addr).unwrap();
}
