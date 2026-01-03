pub mod main_interface;

#[repr(C)]
#[derive(Clone)]
pub struct MarkupTextWord {
  pub str: [u8; 32], // C++ std::string
  pub red: u8,
  pub green: u8,
  pub blue: u8,
  pub link_index: i32,
  pub px: i32,
  pub py: i32,
  pub flags: u32,
}

#[repr(C)]
pub struct MarkupTextBox {
  pub word: cpp::RawCppVector,
  pub link: cpp::RawCppVector,
  pub current_width: i32,
  pub max_y: i32,
  pub environment: usize,
}
