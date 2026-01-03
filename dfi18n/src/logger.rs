use std::{path::Path, sync::OnceLock};

use flexi_logger::{FileSpec, LevelFilter, LogSpecBuilder, Logger, LoggerHandle, WriteMode, json_format};

use crate::{DATA_DIRECTORY, MOD_NAME};

// The singleton Logger instance
static LOGGER: OnceLock<LoggerHandle> = OnceLock::new();

// Setup the logger
pub fn setup() {
  get();
}

// Get the singleton Logger instance
pub fn get() -> &'static LoggerHandle {
  LOGGER.get_or_init(|| {
    // Only configure dfi18n module, and keep other modules at default level
    // TODO: make dfi18n module configurable
    let log_spec = LogSpecBuilder::new().default(LevelFilter::Info).module(MOD_NAME, LevelFilter::Debug).build();
    let logger = Logger::with(log_spec)
      .log_to_file(
        FileSpec::default().directory(Path::new(DATA_DIRECTORY).join("logs")).basename(MOD_NAME).suppress_timestamp(),
      )
      .write_mode(WriteMode::Async)
      .format(json_format)
      .start()
      .unwrap();

    logger
  })
}

// Get the log level for dfi18n module
pub fn level() -> LevelFilter {
  if let Ok(log_spec) = get().current_log_spec() {
    if let Some(level) = log_spec.level_for_module(Some(crate::MOD_NAME)) {
      return level;
    }
  }

  return LevelFilter::Info;
}
