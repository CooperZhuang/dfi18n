use std::sync::OnceLock;

// Information about the game and MOD versions
#[derive(Debug)]
struct GameInfo {
  // os: String,
  // platform: String,
  os_platform: String,
  version: String,
  mod_version: String,
}

// Global storage for game information
static GAME_INFO: OnceLock<GameInfo> = OnceLock::new();

// Set the game information
pub fn set_game_info(os: String, platform: String, version: String, mod_version: String) {
  let os_platform = format!("{os}-{platform}");
  let info = GameInfo {
    // os,
    // platform,
    os_platform,
    version,
    mod_version,
  };
  GAME_INFO.set(info).unwrap();
}

// Getters for game version
pub fn version() -> &'static str {
  &GAME_INFO.get().unwrap().version
}

// Getters for game platform with OS
pub fn os_platform() -> &'static str {
  &GAME_INFO.get().unwrap().os_platform
}

// Getters for MOD version
pub fn mod_version() -> &'static str {
  &GAME_INFO.get().unwrap().mod_version
}
