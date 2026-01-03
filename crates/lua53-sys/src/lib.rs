use std::ffi::{self, CString};

// DFHack's lua53 library bindings
#[cfg_attr(target_os = "linux", link(name = "lua53"))]
#[cfg_attr(target_os = "windows", link(name = "lua53", kind = "raw-dylib"))]
unsafe extern "C-unwind" {
  #[cfg_attr(target_os = "linux", link_name = "_Z17luaL_checkintegerP9lua_Statei")]
  #[cfg_attr(target_os = "windows", link_name = "?luaL_checkinteger@@YA_JPEAUlua_State@@H@Z")]
  fn luaL_checkinteger(L: *mut ffi::c_void, arg: ffi::c_int) -> isize;

  #[cfg_attr(target_os = "linux", link_name = "_Z17luaL_checklstringP9lua_StateiPm")]
  #[cfg_attr(
    target_os = "windows",
    link_name = "?luaL_checklstring@@YAPEBDPEAUlua_State@@HPEA_K@Z"
  )]
  fn luaL_checklstring(L: *mut ffi::c_void, arg: ffi::c_int, l: *mut usize) -> *const ffi::c_char;

  #[cfg_attr(target_os = "linux", link_name = "_Z11lua_pushnilP9lua_State")]
  #[cfg_attr(target_os = "windows", link_name = "?lua_pushnil@@YAXPEAUlua_State@@@Z")]
  fn lua_pushnil(L: *mut ffi::c_void);

  #[cfg_attr(target_os = "linux", link_name = "_Z15lua_pushintegerP9lua_Statex")]
  #[cfg_attr(target_os = "windows", link_name = "?lua_pushinteger@@YAXPEAUlua_State@@_J@Z")]
  fn lua_pushinteger(L: *mut ffi::c_void, n: isize);

  #[cfg_attr(target_os = "linux", link_name = "_Z14lua_pushstringP9lua_StatePKc")]
  #[cfg_attr(target_os = "windows", link_name = "?lua_pushstring@@YAPEBDPEAUlua_State@@PEBD@Z")]
  fn lua_pushstring(L: *mut ffi::c_void, s: *const ffi::c_char);

  #[cfg_attr(target_os = "linux", link_name = "_Z15lua_pushbooleanP9lua_Statei")]
  #[cfg_attr(target_os = "windows", link_name = "?lua_pushboolean@@YAXPEAUlua_State@@H@Z")]
  fn lua_pushboolean(L: *mut ffi::c_void, b: ffi::c_int);
}

// Check and return an integer from the Lua stack
pub fn check_integer(state: *mut ffi::c_void, index: ffi::c_int) -> isize {
  unsafe { luaL_checkinteger(state, index) }
}

// Check and return a string from the Lua stack
pub fn check_string(state: *mut ffi::c_void, index: ffi::c_int) -> String {
  let mut size: usize = 0;
  let str_ptr = unsafe { luaL_checklstring(state, index, &mut size as *mut usize) };
  let slice = unsafe { std::slice::from_raw_parts(str_ptr as *const u8, size) };
  String::from_utf8_lossy(slice).into_owned()
}

// Check and return a string (converted from CP437 code page) from the Lua stack
pub fn check_cp437_string(state: *mut ffi::c_void, index: ffi::c_int) -> String {
  let mut size: usize = 0;
  let str_ptr = unsafe { luaL_checklstring(state, index, &mut size as *mut usize) };
  let slice = unsafe { std::slice::from_raw_parts(str_ptr as *const u8, size) };
  let cstring = CString::new(slice).unwrap();
  cp437_string::c_string_to_string(cstring.as_ptr())
}

// Push nil onto the Lua stack
pub fn push_nil(state: *mut ffi::c_void) {
  unsafe { lua_pushnil(state) }
}

// Push an integer onto the Lua stack
pub fn push_integer(state: *mut ffi::c_void, value: isize) {
  unsafe { lua_pushinteger(state, value) }
}

// Push a string onto the Lua stack
pub fn push_string(state: *mut ffi::c_void, value: &str) {
  unsafe { lua_pushstring(state, ffi::CString::new(value).unwrap().as_ptr()) }
}

// Push a boolean onto the Lua stack
pub fn push_boolean(state: *mut ffi::c_void, value: bool) {
  unsafe { lua_pushboolean(state, value as ffi::c_int) }
}
