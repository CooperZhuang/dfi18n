use std::ffi;

unsafe extern "C-unwind" {
  fn string_size(ptr: *const ffi::c_void) -> usize;
  fn cp437_cstr_ptr_to_utf8_slice(ptr: *const ffi::c_char, utf8_slice: *mut u8) -> usize;
  fn cp437_string_ptr_to_utf8_slice(ptr: *const ffi::c_void, utf8_slice: *mut u8) -> usize;
  fn utf8_slice_to_cp437_codepoint(ptr: *const u8, len: usize) -> u8;
}

// Decode a C string pointer to a Rust UTF-8 String
pub fn c_string_to_string(ptr: *const ffi::c_char) -> String {
  if ptr.is_null() {
    return String::new();
  }

  let c_str = unsafe { ffi::CStr::from_ptr(ptr) };
  let size = c_str.count_bytes();
  let mut utf8_bytes: Vec<u8> = Vec::with_capacity(size * 4); // max 4 bytes per char in UTF-8
  let utf8_size = unsafe { cp437_cstr_ptr_to_utf8_slice(ptr, utf8_bytes.as_mut_ptr()) };
  unsafe { utf8_bytes.set_len(utf8_size) };
  String::from_utf8(utf8_bytes).unwrap_or_default()
}

// Decode a C++ string pointer to a Rust UTF-8 String
pub fn cxx_string_to_string(ptr: *const ffi::c_void) -> String {
  let size = unsafe { string_size(ptr) };
  let mut utf8_bytes: Vec<u8> = Vec::with_capacity(size * 4); // max 4 bytes per char in UTF-8
  let utf8_size = unsafe { cp437_string_ptr_to_utf8_slice(ptr, utf8_bytes.as_mut_ptr()) };
  unsafe { utf8_bytes.set_len(utf8_size) };
  String::from_utf8(utf8_bytes).unwrap_or_default()
}

// Convert a single CP437 codepoint to its character, '\u{0}' for NUL
pub fn cp437_codepoint_to_char(codepoint: u8) -> char {
  let c_string = if codepoint == 0 {
    ffi::CString::new("").unwrap()
  } else {
    ffi::CString::from_vec_with_nul(vec![codepoint, 0]).unwrap()
  };
  let ptr = c_string.as_c_str().as_ptr();
  let mut utf8_bytes: Vec<u8> = Vec::with_capacity(4); // max 4 bytes per char in UTF-8
  let utf8_size = unsafe { cp437_cstr_ptr_to_utf8_slice(ptr, utf8_bytes.as_mut_ptr()) };
  unsafe { utf8_bytes.set_len(utf8_size) };
  String::from_utf8(utf8_bytes).unwrap_or_default().chars().next().unwrap_or(' ')
}

// Convert a Rust string to a vector of CP437 codepoints
pub fn string_to_cp437_codepoints(s: &str) -> Vec<u8> {
  s.chars().map(|ch| char_to_cp437_codepoint(ch)).collect()
}

// Convert a character to its CP437 codepoint, 0 for not found
pub fn char_to_cp437_codepoint(ch: char) -> u8 {
  let ch = ch.to_string();
  unsafe { utf8_slice_to_cp437_codepoint(ch.as_ptr(), ch.len()) }
}

// Check if a character is representable in CP437
pub fn char_is_cp437(ch: char) -> bool {
  char_to_cp437_codepoint(ch) != 0
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_string_to_cp437_codepoint() {
    assert_eq!(char_to_cp437_codepoint('\u{0}'), 0x0); // 0x00 mapped to space
    assert_eq!(char_to_cp437_codepoint('☺'), 0x01); // first CP437 character (except NULL)
    assert_eq!(char_to_cp437_codepoint(' '), 0x20); // space only maps back to 0x20, not 0x00
    assert_eq!(char_to_cp437_codepoint('A'), 0x41); // ASCII character - same codepoint
    assert_eq!(char_to_cp437_codepoint('é'), 0x82); // Latin character
    assert_eq!(char_to_cp437_codepoint('¥'), 0x9d); // Symbol character
    assert_eq!(char_to_cp437_codepoint('■'), 0xfe); // Last CP437 character (except 0xff)
    assert_eq!(char_to_cp437_codepoint('\u{a0}'), 0xff); // 0xff mapped to NBSP
    assert_eq!(char_to_cp437_codepoint('😀'), 0x00); // not found
    assert_eq!(char_to_cp437_codepoint('汉'), 0x00); // not found
  }
}
