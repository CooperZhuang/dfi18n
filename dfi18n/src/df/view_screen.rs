use std::ffi;
use std::sync::{OnceLock, RwLock};

use lua53_sys as lua;

// FIXME: this should return Vec<String> for all layers of view screens

// Get the current view screen
pub fn get_view_screen() -> String {
  let vs = VIEW_SCREEN.get().unwrap();
  vs.read().unwrap().to_owned()
}

// Current view screen
static VIEW_SCREEN: OnceLock<RwLock<String>> = OnceLock::new();

// Set the current view screen from Lua
#[unsafe(no_mangle)]
extern "C" fn set_view_screen(lua_state: *mut ffi::c_void) {
  let name = lua::check_string(lua_state, 1);
  let vs = VIEW_SCREEN.get_or_init(|| RwLock::new(String::new()));
  *vs.write().unwrap() = name;
}
