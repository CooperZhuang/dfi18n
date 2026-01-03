use std::ffi;
use std::sync::{Mutex, MutexGuard, OnceLock};

use lua53_sys as lua;

// Current language tag
static LANG_TAG: OnceLock<Mutex<Option<String>>> = OnceLock::new();

// Get mutable access to the current language tag
pub fn get_lang_tag_mut() -> MutexGuard<'static, Option<String>> {
  LANG_TAG.get_or_init(|| Mutex::new(None)).lock().unwrap()
}

// Get the current language tag
pub fn current_lang_tag() -> String {
  get_lang_tag_mut().as_deref().unwrap_or("en").to_owned()
}

// Set the current language tag from Lua
#[unsafe(no_mangle)]
fn set_lang_tag(lua_state: *mut ffi::c_void) {
  let lang_tag = lua::check_string(lua_state, 1);
  let mut lang_tag_mut = get_lang_tag_mut();
  // TODO: validate the language tag
  *lang_tag_mut = Some(lang_tag);
}
