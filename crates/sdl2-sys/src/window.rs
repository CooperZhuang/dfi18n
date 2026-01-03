use std::{ffi, marker::PhantomData, ops::Deref, sync::Arc};

use crate::*;

const SDL_WINDOWPOS_UNDEFINED_MASK: i32 = 0x1FFF0000;

// SDL Window wrapper
#[derive(Clone)]
pub struct Window<'a> {
  inner: Arc<WindowInner<'a>>,
}

impl<'a> Deref for Window<'a> {
  type Target = WindowInner<'a>;

  // Deref implementation to access the inner WindowInner
  fn deref(&self) -> &Self::Target {
    unsafe { Arc::as_ptr(&self.inner).as_ref_unchecked() }
  }
}

impl<'a> PartialEq for Window<'_> {
  // PartialEq implementation to compare two Window instances
  fn eq(&self, other: &Self) -> bool {
    self.raw() == other.raw()
  }
}

impl<'a> Window<'a> {
  // Create a new SDL Window with the given title, width, and height
  pub fn new(title: &str, width: ffi::c_int, height: ffi::c_int) -> Self {
    let c_title = ffi::CString::new(title).unwrap();
    let ptr = unsafe {
      SDL_CreateWindow(
        c_title.as_ptr(),
        SDL_WINDOWPOS_UNDEFINED_MASK,
        SDL_WINDOWPOS_UNDEFINED_MASK,
        width,
        height,
        0,
      )
    };

    if ptr.is_null() {
      log::error!("Failed to create window");
    }

    let inner = WindowInner {
      ptr,
      _marker: PhantomData,
    };

    Window { inner: Arc::new(inner) }
  }

  // Create a Window from a raw SDL_Window pointer
  pub fn from_raw(ptr: *mut SDL_Window) -> Self {
    let inner = WindowInner {
      ptr,
      _marker: PhantomData,
    };

    let boxed = Box::new(Window { inner: Arc::new(inner) });
    let leaked = Box::leak(boxed);
    leaked.clone()
  }

  // Create a renderer for the window
  pub fn create_renderer(&'_ self) -> Renderer<'_> {
    Renderer::new(self)
  }
}

// Inner structure for Window reference counting
#[derive(Debug)]
pub struct WindowInner<'a> {
  ptr: *mut SDL_Window,
  _marker: PhantomData<&'a ()>,
}

unsafe impl Send for WindowInner<'static> {}
unsafe impl Sync for WindowInner<'static> {}
impl Drop for WindowInner<'_> {
  // Destroy the SDL Window when the WindowInner is dropped
  fn drop(&mut self) {
    unsafe { SDL_DestroyWindow(self.ptr) };
  }
}

impl WindowInner<'_> {
  // Get the raw SDL_Window pointer
  pub fn raw(&self) -> *const SDL_Window {
    self.ptr
  }

  // Get a mutable raw SDL_Window pointer
  pub fn raw_mut(&self) -> *mut SDL_Window {
    self.ptr
  }

  // Get the size of the window
  pub fn get_size(&self) -> (ffi::c_int, ffi::c_int) {
    let mut w: ffi::c_int = 0;
    let mut h: ffi::c_int = 0;
    let pw = &mut w as *mut ffi::c_int;
    let ph = &mut h as *mut ffi::c_int;

    unsafe { SDL_GetWindowSize(self.ptr, pw, ph) };

    (w, h)
  }

  // Set the size of the window
  pub fn set_size(&self, width: ffi::c_int, height: ffi::c_int) {
    unsafe { SDL_SetWindowSize(self.ptr, width, height) };
  }

  // Get the position of the window
  pub fn get_position(&self) -> (ffi::c_int, ffi::c_int) {
    let mut x: ffi::c_int = 0;
    let mut y: ffi::c_int = 0;
    let px = &mut x as *mut ffi::c_int;
    let py = &mut y as *mut ffi::c_int;

    unsafe { SDL_GetWindowPosition(self.ptr, px, py) };

    (x, y)
  }

  // Set the position of the window
  pub fn set_position(&self, x: ffi::c_int, y: ffi::c_int) {
    unsafe { SDL_SetWindowPosition(self.ptr, x, y) };
  }
}

// Dummy SDL Window
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub struct SDL_Window {
  _unused: [u8; 0],
}
