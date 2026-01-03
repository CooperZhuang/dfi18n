use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use lua53_sys as lua;

use crate::{game, lang, tasks, translation};

mod rulesets;
mod simple;

// Reset the translators and translation caches
pub fn reset() {
  rulesets::reset();
  simple::reset();
  get_caches_mut().clear();
}

// Translation cache maps TranslationRequest key to optional TranslationResponse
type TranslationCache = HashMap<String, Option<translation::TranslationResponse>>;

// A collection of translation caches grouped by language tag
type TranslationCaches = HashMap<String, TranslationCache>;

// Caches for translation requests and their responses
static CACHES: OnceLock<RwLock<TranslationCaches>> = OnceLock::new();

// Getting the write lock for the caches
fn get_caches_mut() -> std::sync::RwLockWriteGuard<'static, TranslationCaches> {
  CACHES.get_or_init(|| RwLock::new(TranslationCaches::new())).write().unwrap()
}

// Check if the content should skip translation
pub fn should_skip_translation(original: &str) -> bool {
  // don't skip game version strings
  if original == game::version() {
    return false;
  }

  original.len() < 2
    || original.starts_with("FPS: ")
    || original.chars().all(|c| c.is_ascii_digit() || c.is_ascii_punctuation() || c.is_ascii_whitespace())
}

// Translate the given TranslationRequest
pub fn translate(request: &translation::TranslationRequest) -> Option<translation::TranslationResponse> {
  let lang_tag = lang::current_lang_tag();

  let mut caches = get_caches_mut();
  let cache = caches.entry(lang_tag.clone()).or_insert_with(TranslationCache::new);
  let key = request.key();
  if let Some(cached) = cache.get(key) {
    // return cached response
    return cached.clone();
  } else {
    // insert a placeholder to indicate this request is being processed
    cache.insert(key.to_owned(), None);
  }

  // spawn a task to perform the translation
  tasks::spawn(translate_task(request.clone()));

  // return no translation for now
  None
}

// The translation task that performs the actual translation
pub async fn translate_task(request: translation::TranslationRequest) {
  let lang_tag = lang::current_lang_tag();

  let response = do_translate(&request);
  let mut caches = get_caches_mut();
  let cache = caches.entry(lang_tag).or_insert_with(TranslationCache::new);
  cache.insert(request.key().to_owned(), response);
}

// Perform the actual translation using different methods
pub fn do_translate(request: &translation::TranslationRequest) -> Option<translation::TranslationResponse> {
  // add MOD info to game version strings
  if request.original() == game::version() {
    let translated = format!(
      "{} + {}-{} v{}",
      game::version(),
      crate::MOD_NAME,
      game::os_platform(),
      game::mod_version()
    );

    return Some(translation::TranslationResponse {
      translated,
      alignment: translation::TextAlignment::default(),
    });
  }

  let lang_tag = lang::current_lang_tag();

  // chain translation methods
  None
    .or_else(|| simple::translate(&lang_tag, request.context()))
    .or_else(|| rulesets::translate(&lang_tag, request.context()))
}

// Synchronous translation function called from Lua (will not use cache)
#[unsafe(no_mangle)]
extern "C" fn sync_translate(lua_state: *mut std::ffi::c_void) -> i32 {
  let content = lua::check_string(lua_state, 1);
  let request = translation::TranslationRequest::new(translation::TranslationInput::addst { content });
  let response = do_translate(&request);
  if let Some(response) = response {
    lua::push_string(lua_state, &response.translated.as_str());
  } else {
    lua::push_nil(lua_state);
  }
  return 1;
}

// Asynchronous translation function called from Lua
#[unsafe(no_mangle)]
extern "C" fn async_translate(lua_state: *mut std::ffi::c_void) -> i32 {
  let content = lua::check_string(lua_state, 1);
  let request = translation::TranslationRequest::new(translation::TranslationInput::addst { content });
  let response = translate(&request);
  if let Some(response) = response {
    lua::push_string(lua_state, &response.translated.as_str());
  } else {
    lua::push_nil(lua_state);
  }
  return 1;
}
