use std::collections::HashMap;
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

use rule_based_translator::Translator;

use crate::translation;

// Reset the rulesets translators
pub fn reset() {
  get_translators_mut().clear();
}

// Translate text based on the provided language tag and context
pub fn translate(
  lang_tag: &str,
  context: &translation::TranslationContext,
) -> Option<translation::TranslationResponse> {
  let translators = get_translators();
  let translator = translators.get(lang_tag)?;
  let text = context.original();

  translator.translate(text).map(|translated| translation::TranslationResponse {
    translated,
    alignment: translation::TextAlignment::default(),
  })
}

// A global registry of translators categorized by type and language tag
static TRANSLATORS: OnceLock<RwLock<HashMap<String, Translator>>> = OnceLock::new();

// Getting access to the translators registry
fn get_translators() -> RwLockReadGuard<'static, HashMap<String, Translator>> {
  TRANSLATORS.get_or_init(|| RwLock::new(HashMap::new())).read().unwrap()
}

// Getting mutable access to the translators registry
fn get_translators_mut() -> RwLockWriteGuard<'static, HashMap<String, Translator>> {
  TRANSLATORS.get_or_init(|| RwLock::new(HashMap::new())).write().unwrap()
}

// Load rulesets from a directory for a specific language
#[unsafe(no_mangle)]
extern "C" fn load_translation_rulesets(lua_state: *mut std::ffi::c_void) {
  let lang_tag = lua53_sys::check_string(lua_state, 1);
  let path = lua53_sys::check_string(lua_state, 2);

  let mut translators = get_translators_mut();
  let translator = translators.entry(lang_tag.to_owned()).or_insert_with(Translator::default);

  let _ = translator.load_from_dir(&path);

  // TODO: return load result to Lua
  log::info!("Loaded Rulesets translator for language {lang_tag:?} from {path:?}");
}
