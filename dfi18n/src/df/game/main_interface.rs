use std::sync::OnceLock;

// Get mutable help struct
pub fn get_help_mut() -> &'static mut Help {
  unsafe { (*GAME_MAIN_INTERFACE_HELP.get().unwrap() as *mut Help).as_mut_unchecked() }
}

// game.main_interface.help
static GAME_MAIN_INTERFACE_HELP: OnceLock<usize> = OnceLock::new();

// Set game.main_interface.help from Lua
#[unsafe(no_mangle)]
extern "C" fn set_game_main_interface_help(lua_state: *mut std::ffi::c_void) {
  let addr = lua53_sys::check_integer(lua_state, 1) as usize;
  GAME_MAIN_INTERFACE_HELP.set(addr).unwrap();
}

#[repr(C)]
pub struct Help {
  pub open: bool,
  pub flag: u32,
  pub context_flag: u32,
  pub context: u32,
  pub header: [u8; 32],
  pub text: [super::MarkupTextBox; 20],
}
