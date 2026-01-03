use std::ffi;

// Native SDL Rectangle
#[repr(C)]
#[derive(Debug, Copy, Clone)]
#[allow(non_camel_case_types)]
pub struct SDL_Rect {
  pub x: ffi::c_int,
  pub y: ffi::c_int,
  pub w: ffi::c_int,
  pub h: ffi::c_int,
}
