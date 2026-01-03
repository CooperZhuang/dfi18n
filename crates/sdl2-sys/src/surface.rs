use std::{ffi, marker::PhantomData, ops::Deref, slice, sync::Arc};

use anyhow::{Result, anyhow};

use crate::*;

const SDL_PIXELFORMAT_ABGR8888: u32 = 0x16762004;

// SDL Surface wrapper
#[derive(Clone)]
pub struct Surface<'a> {
  inner: Arc<SurfaceInner<'a>>,
}

impl<'a> Deref for Surface<'a> {
  type Target = SurfaceInner<'a>;

  // Deref implementation to access the inner SurfaceInner
  fn deref(&self) -> &Self::Target {
    unsafe { Arc::as_ptr(&self.inner).as_ref_unchecked() }
  }
}

impl<'a> PartialEq for Surface<'_> {
  // PartialEq implementation to compare two Surface instances
  fn eq(&self, other: &Self) -> bool {
    self.raw() == other.raw()
  }
}

impl<'a> Surface<'a> {
  // Create a new SDL Surface with the given width and height
  pub fn new(width: ffi::c_int, height: ffi::c_int) -> Self {
    let ptr = unsafe { SDL_CreateRGBSurfaceWithFormat(0, width, height, 32, SDL_PIXELFORMAT_ABGR8888) };

    if ptr.is_null() {
      log::error!("Failed to create surface");
    }

    let inner = SurfaceInner {
      ptr,
      _marker: PhantomData,
    };

    Surface { inner: Arc::new(inner) }
  }

  // Create a Surface from a raw SDL_Surface pointer
  pub fn from_raw(ptr: *mut SDL_Surface) -> Self {
    let inner = SurfaceInner {
      ptr,
      _marker: PhantomData,
    };

    let boxed = Box::new(Surface { inner: Arc::new(inner) });
    let leaked = Box::leak(boxed);
    leaked.clone()
  }

  // Load an image from a byte slice and create a Surface
  pub fn load_image(data: &[u8]) -> Self {
    let rwops = unsafe { SDL_RWFromConstMem(data.as_ptr() as *const ffi::c_void, data.len() as ffi::c_int) };
    if rwops.is_null() {
      log::error!("Failed to create RWops from memory");
    }

    let surface_ptr = unsafe { IMG_Load_RW(rwops, 1) };
    if surface_ptr.is_null() {
      log::error!("Failed to load image from RWops");
    }

    let inner = SurfaceInner {
      ptr: surface_ptr,
      _marker: PhantomData,
    };

    Surface { inner: Arc::new(inner) }
  }

  // Try to load an image from a byte slice and create a Surface
  pub fn try_load_image(data: &[u8]) -> Result<Self> {
    let rwops = unsafe { SDL_RWFromConstMem(data.as_ptr() as *const ffi::c_void, data.len() as ffi::c_int) };
    if rwops.is_null() {
      return Err(anyhow!("Failed to create RWops from memory"));
    }

    let surface_ptr = unsafe { IMG_Load_RW(rwops, 1) };
    if surface_ptr.is_null() {
      return Err(anyhow!("Failed to load image from RWops"));
    }

    let inner = SurfaceInner {
      ptr: surface_ptr,
      _marker: PhantomData,
    };

    Ok(Surface { inner: Arc::new(inner) })
  }
}

// Inner structure for Surface reference counting
pub struct SurfaceInner<'a> {
  ptr: *mut SDL_Surface,
  _marker: PhantomData<&'a ()>,
}

unsafe impl Send for SurfaceInner<'static> {}
unsafe impl Sync for SurfaceInner<'static> {}
impl<'a> Drop for SurfaceInner<'a> {
  // Free the SDL Surface when the SurfaceInner is dropped
  fn drop(&mut self) {
    unsafe { SDL_FreeSurface(self.ptr) };
  }
}

impl SurfaceInner<'_> {
  // Get the raw SDL_Surface pointer
  pub fn raw(&self) -> *const SDL_Surface {
    self.ptr
  }

  // Get the raw SDL_Surface pointer
  pub fn raw_mut(&self) -> *mut SDL_Surface {
    self.ptr
  }

  // Get the size of the surface
  pub fn get_size(&self) -> (ffi::c_int, ffi::c_int) {
    let raw_ref = self.raw_ref();

    (raw_ref.get_width(), raw_ref.get_height())
  }

  // Execute a closure with a locked mutable reference to the pixel data
  pub fn with_lock_mut<R, F: FnOnce(&mut [u8]) -> R>(&self, f: F) -> R {
    // Lock the surface for pixel access
    if unsafe { SDL_LockSurface(self.ptr) != 0 } {
      log::error!("could not lock surface");
    }

    // Execute the closure with the pixel data
    let rv = f(self.raw_ref().get_pixels_mut());

    // Unlock the surface after pixel access
    unsafe { SDL_UnlockSurface(self.ptr) };

    rv
  }

  // Get a reference to the raw SDL_Surface
  fn raw_ref(&self) -> &SDL_Surface {
    unsafe { self.ptr.as_ref().unwrap_unchecked() }
  }
}

// Native SDL Surface
#[repr(C)]
#[derive(Debug, Copy, Clone)]
#[allow(non_camel_case_types)]
pub struct SDL_Surface {
  pub flags: u32,
  pub format: *mut ffi::c_void, // SDL_PixelFormat
  pub w: ffi::c_int,
  pub h: ffi::c_int,
  pub pitch: ffi::c_int,
  pub pixels: *mut u8,
  pub userdata: *mut ffi::c_void,
  pub locked: ffi::c_int,
  pub list_blitmap: *mut ffi::c_void,
  pub clip_rect: SDL_Rect,
  pub map: *mut ffi::c_void, // SDL_BlitMap
  pub refcount: ffi::c_int,
}

impl SDL_Surface {
  // Get the width of the surface
  pub fn get_width(&self) -> ffi::c_int {
    self.w
  }

  // Get the height of the surface
  pub fn get_height(&self) -> ffi::c_int {
    self.h
  }

  // Get a mutable slice to the pixel data
  pub fn get_pixels_mut(&self) -> &mut [u8] {
    let len = self.pitch * self.h;
    unsafe { slice::from_raw_parts_mut(self.pixels, len as usize) }
  }
}

// Dummy SDL RWops
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub struct SDL_RWops {
  _unused: [u8; 0],
}
