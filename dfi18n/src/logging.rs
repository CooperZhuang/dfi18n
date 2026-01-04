use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};

use crate::{translation, translator};

static VISITED: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();
static TRANSLATED: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();

pub fn log_text(request: &translation::TranslationRequest, backtrace: &str, ptr: *const std::ffi::c_void) {
  // // XXX: debug
  // return;

  let key = request.key();
  let context = request.context();
  let content = context.original();

  if translator::should_skip_translation(content) {
    return;
  }

  // only log new translations requests once, and translated texts once
  let response = translator::translate(request);
  let visited = VISITED.get_or_init(|| RwLock::new(HashSet::new()));
  let translated = TRANSLATED.get_or_init(|| RwLock::new(HashSet::new()));
  if (response.is_none() && visited.read().unwrap().contains(key))
    || (response.is_some() && translated.read().unwrap().contains(key))
  {
    return;
  }

  if response.is_none() {
    // translating or untranslatable
    visited.write().unwrap().insert(key.to_owned());
  } else {
    // translated
    translated.write().unwrap().insert(key.to_owned());
  }

  let function = match context {
    translation::TranslationContext::addst { .. } => "addst",
    translation::TranslationContext::addst_flag { .. } => "addst_flag",
    translation::TranslationContext::addcoloredst { .. } => "addcoloredst",
    translation::TranslationContext::top_addst { .. } => "top_addst",
    translation::TranslationContext::markup_text_box { .. } => "mtb_process_string_to_lines",
    translation::TranslationContext::dfhack { .. } => "dfhack",
  };
  let mut lines = vec![format!("========== {key}"), format!("[{function}] {backtrace}")];

  let viewscreen = request.view_screen();
  lines.push(format!("viewscreen: {viewscreen}"));

  let coordinate = request.coordinate();
  lines.push(format!("coordinate: {coordinate:?}"));

  let color_pair = request.color_pair();
  if let Some(color_pair) = color_pair {
    lines.push(format!("color_pair: {color_pair:?}"));
  }

  if let Some(flag) = context.flag() {
    lines.push(format!("flag: {flag:#010b}"));
  }

  let is_markup = request.is_markup();
  lines.push(format!(
    "---- {} ({ptr:p}) ----",
    if is_markup { "MarkupText" } else { "PlainText" }
  ));

  lines.push(content.to_owned());
  if let Some(response) = response {
    lines.push("---- Translated ----".to_string());
    lines.push(response.translated);
  }

  let debug_string = lines.join("\n");

  // // XXX: debug
  // if function != "addcoloredst" {
  //   return;
  // }

  log::debug!("{debug_string}");
}
