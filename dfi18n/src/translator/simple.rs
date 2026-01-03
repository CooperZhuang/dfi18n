use std::collections::HashMap;
use std::fs::File;
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

use anyhow::Result;

use lua53_sys as lua;

use crate::translation;

// Reset the simple translators
pub fn reset() {
  get_dicts_mut().clear();
}

// Simple dictionary maps original text to translated text along with tags
type SimpleDictionary = HashMap<String, (String, HashMap<String, String>)>;

// A collection of simple dictionaries grouped by language tag
type SimpleDictionaries = HashMap<String, SimpleDictionary>;

// Global storage for simple dictionaries
static DICTS: OnceLock<RwLock<SimpleDictionaries>> = OnceLock::new();

// Getting access to the dictionaries
fn get_dicts() -> RwLockReadGuard<'static, SimpleDictionaries> {
  DICTS.get_or_init(|| RwLock::new(SimpleDictionaries::new())).read().unwrap()
}

// Getting mutable access to the dictionaries
fn get_dicts_mut() -> RwLockWriteGuard<'static, SimpleDictionaries> {
  DICTS.get_or_init(|| RwLock::new(SimpleDictionaries::new())).write().unwrap()
}

// Translate text based on the provided language tag and context
pub fn translate(
  lang_tag: &str,
  context: &translation::TranslationContext,
) -> Option<translation::TranslationResponse> {
  // Extract the text to be translated from the context, colored text is not handled here
  let text = context.original();

  let dicts = get_dicts();
  dicts.get(lang_tag).and_then(|dict| {
    dict.get(text).and_then(|(translated, tags)| {
      Some(translation::TranslationResponse {
        translated: translated.to_owned(),
        alignment: match tags.get("ALIGNMENT").map(|s| s.as_str()) {
          Some("LEFT") => translation::TextAlignment::Left,
          Some("RIGHT") => translation::TextAlignment::Right,
          Some("CENTER") => translation::TextAlignment::Center,
          _ => translation::TextAlignment::Left,
        },
      })
    })
  })
}

// Load a simple dictionary from a CSV file into the global storage
#[unsafe(no_mangle)]
extern "C" fn load_simple_dict(lua_state: *mut std::ffi::c_void) {
  let lang_tag = lua::check_string(lua_state, 1);
  let path_str = lua::check_string(lua_state, 2);

  let mut dicts = get_dicts_mut();
  let dict = dicts.entry(lang_tag.to_string()).or_insert_with(SimpleDictionary::new);
  if let Err(err) = load_csv(
    &path_str,
    |Entry {
       text,
       translation,
       tags,
     }| {
      dict.insert(text, (translation, parse_tags(&tags)));
    },
  ) {
    log::warn!("Failed to load simple translator data from {path_str:?}: {err}");
    // TODO: consider returning error message to Lua
    return;
  };

  log::info!("Loaded Simple translator data for language {lang_tag:?} from {path_str:?}");
}

// CSV entry
#[derive(Debug, serde::Deserialize)]
struct Entry {
  // Original text
  text: String,
  // Translated text
  translation: String,
  // Tags
  tags: String,
}

// Load CSV file and process each entry with the provided function
fn load_csv<T: serde::de::DeserializeOwned, P: AsRef<std::path::Path>, F>(path: P, mut f: F) -> Result<()>
where
  F: FnMut(T),
{
  for entry in csv::Reader::from_reader(File::open(path)?).deserialize::<T>() {
    f(entry?);
  }

  Ok(())
}

static DF_TAG_REGEX: OnceLock<regex::Regex> = OnceLock::new();

// Get the regex for DF txt tags
// TODO: move to `utils` module
fn get_df_tag_regex() -> &'static regex::Regex {
  DF_TAG_REGEX.get_or_init(|| regex::Regex::new(r"\[([^\[:]+):([^:\]]+)\]").unwrap())
}

// Parse tags in DF txt format [KEY:VALUE]
fn parse_tags(tags_str: &str) -> HashMap<String, String> {
  let mut tags = HashMap::new();
  let regex = get_df_tag_regex();
  for cap in regex.captures_iter(tags_str) {
    if let (Some(key), Some(value)) = (cap.get(1), cap.get(2)) {
      tags.insert(key.as_str().to_owned(), value.as_str().to_owned());
    }
  }
  tags
}
