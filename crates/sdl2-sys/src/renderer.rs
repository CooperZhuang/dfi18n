use std::{marker::PhantomData, ops::Deref, sync::Arc};

use crate::*;

const SDL_RENDERER_TARGETTEXTURE: u32 = 0x00000008;

// SDL Renderer wrapper
#[derive(Clone)]
pub struct Renderer<'a> {
  inner: Arc<RendererInner<'a>>,
}

impl<'a> Deref for Renderer<'a> {
  type Target = RendererInner<'a>;

  // Deref implementation to access the inner RendererInner
  fn deref(&self) -> &Self::Target {
    unsafe { Arc::as_ptr(&self.inner).as_ref_unchecked() }
  }
}

impl<'a> PartialEq for Renderer<'_> {
  // PartialEq implementation to compare two Renderer instances
  fn eq(&self, other: &Self) -> bool {
    self.raw() == other.raw()
  }
}

impl<'a> Renderer<'a> {
  // Create a new SDL Renderer for the given window
  pub fn new(window: &Window) -> Self {
    let ptr = unsafe { SDL_CreateRenderer(window.raw_mut(), -1, SDL_RENDERER_TARGETTEXTURE) };

    if ptr.is_null() {
      log::error!("Failed to create renderer");
    }

    let inner = RendererInner {
      ptr,
      _marker: PhantomData,
    };

    Renderer { inner: Arc::new(inner) }
  }

  // Create a Renderer from a raw SDL_Renderer pointer
  pub fn from_raw(ptr: *mut SDL_Renderer) -> Self {
    let inner = RendererInner {
      ptr,
      _marker: PhantomData,
    };

    let boxed = Box::new(Renderer { inner: Arc::new(inner) });
    let leaked = Box::leak(boxed);
    leaked.clone()
  }
}

// Inner structure for Renderer reference counting
#[derive(Debug)]
pub struct RendererInner<'a> {
  ptr: *mut SDL_Renderer,
  _marker: PhantomData<&'a ()>,
}

unsafe impl Send for RendererInner<'static> {}
unsafe impl Sync for RendererInner<'static> {}
impl Drop for RendererInner<'_> {
  // Destroy the SDL Renderer when the RendererInner is dropped
  fn drop(&mut self) {
    unsafe { SDL_DestroyRenderer(self.ptr) };
  }
}

impl RendererInner<'_> {
  // Get the raw SDL_Renderer pointer
  pub fn raw(&self) -> *const SDL_Renderer {
    self.ptr
  }

  // Get a mutable raw SDL_Renderer pointer
  pub fn raw_mut(&self) -> *mut SDL_Renderer {
    self.ptr
  }

  // Set the logical size of the renderer
  pub fn set_logical_size(&self, width: ffi::c_int, height: ffi::c_int) {
    if unsafe { SDL_RenderSetLogicalSize(self.ptr, width, height) != 0 } {
      log::error!("Failed to set logical size of renderer");
    }
  }

  // Get the output size of the renderer
  pub fn get_output_size(&self) -> (ffi::c_int, ffi::c_int) {
    let mut w: ffi::c_int = 0;
    let mut h: ffi::c_int = 0;
    let pw = &mut w as *mut ffi::c_int;
    let ph = &mut h as *mut ffi::c_int;

    if unsafe { SDL_GetRendererOutputSize(self.ptr, pw, ph) != 0 } {
      log::error!("Failed to get output size of renderer");
      return (0, 0);
    }

    (w, h)
  }

  // Clear the renderer with the given RGBA color
  pub fn clear_color(&self, r: u8, g: u8, b: u8, a: u8) {
    self.set_draw_color(r, g, b, a);
    self.clear();
  }

  // Fill a rectangle in the renderer with the given RGBA color
  pub fn fill_rect(&self, rect: &SDL_Rect, r: u8, g: u8, b: u8, a: u8) {
    self.set_draw_color(r, g, b, a);
    if unsafe { SDL_RenderFillRect(self.ptr, rect as *const SDL_Rect) != 0 } {
      log::error!("Failed to fill rectangle in renderer");
    }
  }

  // Draw a rectangle outline in the renderer with the given RGBA color
  pub fn draw_rect(&self, rect: &SDL_Rect, r: u8, g: u8, b: u8, a: u8) {
    self.set_draw_color(r, g, b, a);
    if unsafe { SDL_RenderDrawRect(self.ptr, rect as *const SDL_Rect) != 0 } {
      log::error!("Failed to draw rectangle in renderer");
    }
  }

  // Copy a texture to the renderer at the specified source and destination rectangles
  pub fn copy(&self, texture: &Texture, src_rect: Option<&SDL_Rect>, dst_rect: Option<&SDL_Rect>) {
    let src_ptr = match src_rect {
      Some(rect) => rect as *const SDL_Rect,
      None => std::ptr::null(),
    };
    let dst_ptr = match dst_rect {
      Some(rect) => rect as *const SDL_Rect,
      None => std::ptr::null(),
    };

    if unsafe { SDL_RenderCopy(self.ptr, texture.raw_mut(), src_ptr, dst_ptr) != 0 } {
      log::error!("Failed to copy texture to renderer");
    }
  }

  // Present the rendered content to the window
  pub fn present(&self) {
    unsafe { SDL_RenderPresent(self.ptr) };
  }

  // Set the drawing color for the renderer
  fn set_draw_color(&self, r: u8, g: u8, b: u8, a: u8) {
    if unsafe { SDL_SetRenderDrawColor(self.ptr, r, g, b, a) != 0 } {
      log::error!("Failed to set draw color");
    }
  }

  // Clear the renderer
  fn clear(&self) {
    if unsafe { SDL_RenderClear(self.ptr) != 0 } {
      log::error!("Failed to clear renderer");
    }
  }
}

// Dummy SDL Renderer
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub struct SDL_Renderer {
  _unused: [u8; 0],
}
