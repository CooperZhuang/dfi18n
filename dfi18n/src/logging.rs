use std::collections::HashSet;
use std::sync::OnceLock;

use parking_lot::RwLock;

use crate::{tasks, translation, translator};

static VISITED: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();

fn context_kind(context: &translation::TranslationContext) -> &'static str {
  match context {
    translation::TranslationContext::addst { .. } => "addst",
    translation::TranslationContext::addst_flag { .. } => "addst_flag",
    translation::TranslationContext::addcoloredst { .. } => "addcoloredst",
    translation::TranslationContext::top_addst { .. } => "top_addst",
    translation::TranslationContext::markup_text_box { .. } => "mtb_process_string_to_lines",
    translation::TranslationContext::dfhack { .. } => "dfhack",
  }
}

pub fn log_text(request: &translation::TranslationRequest, backtrace: &str, ptr: *const std::ffi::c_void) {
  let context = request.context().clone();
  let content = context.original();
  if translator::should_skip_translation(content) {
    return;
  }

  let function = context_kind(&context);
  // request.key() includes rendering context that can change between frames.
  // Diagnostics and realtime queueing care about the translatable content, so
  // deduplicate by rendering type + original text. One write lock avoids the
  // previous read-then-write race and prevents per-frame task/log churn.
  let visit_key = format!("{function}\0{content}");
  let visited = VISITED.get_or_init(|| RwLock::new(HashSet::new()));
  if !visited.write().insert(visit_key) {
    return;
  }

  let request = request.clone();
  let key = request.key().to_owned();
  let content = content.to_owned();
  let backtrace = backtrace.to_owned();
  let ptr = ptr as usize;
  tasks::spawn(async move {
    let started = std::time::Instant::now();
    // Ensure existing dictionaries/rulesets are checked and the cache is filled.
    translator::translate_task(request.clone()).await;
    let response = translator::translate(&request);
    let elapsed_ms = started.elapsed().as_millis();

    if let Some(response) = response {
      log::debug!(
        "translation_trace status=covered function={} elapsed_ms={} original={:?} translated={:?}",
        function,
        elapsed_ms,
        content,
        response.translated
      );
      return;
    }

    crate::realtime_translate::push(content.clone());
    let mut lines = vec![
      format!("========== {key}"),
      format!("translation_trace status=queued function={function} elapsed_ms={elapsed_ms}"),
      format!("[{function}] {backtrace}"),
      format!("viewscreen: {}", request.view_screen()),
      format!("coordinate: {:?}", request.coordinate()),
    ];

    if let Some(color_pair) = request.color_pair() {
      lines.push(format!("color_pair: {color_pair:?}"));
    }
    if let translation::TranslationContext::addst_flag { flag, .. } = context {
      lines.push(format!("flag: {flag:#010b}"));
    }

    lines.push(format!(
      "---- {} ({ptr:#x}) ----",
      if request.is_markup() { "MarkupText" } else { "PlainText" }
    ));
    lines.push(content);
    log::debug!("{}", lines.join("\n"));
  });
}
