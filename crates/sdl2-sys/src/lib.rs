use std::ffi;

mod rect;
mod renderer;
mod surface;
mod texture;
mod window;

pub use rect::*;
pub use renderer::*;
pub use surface::*;
pub use texture::*;
pub use window::*;

// Dwarf Fortress's SDL2 library bindings
#[cfg_attr(target_os = "windows", link(name = "SDL2", kind = "raw-dylib"))]
unsafe extern "C-unwind" {
  fn SDL_CreateRGBSurfaceWithFormat(
    flags: u32,
    width: ffi::c_int,
    height: ffi::c_int,
    depth: ffi::c_int,
    format: u32,
  ) -> *mut SDL_Surface;
  fn SDL_LockSurface(surface: *mut SDL_Surface) -> ffi::c_int;
  fn SDL_UnlockSurface(surface: *mut SDL_Surface);
  fn SDL_FreeSurface(surface: *mut SDL_Surface);
  fn SDL_CreateWindow(
    title: *const ffi::c_char,
    x: ffi::c_int,
    y: ffi::c_int,
    w: ffi::c_int,
    h: ffi::c_int,
    flags: u32,
  ) -> *mut SDL_Window;
  fn SDL_GetWindowSize(window: *mut SDL_Window, w: *mut ffi::c_int, h: *mut ffi::c_int);
  fn SDL_SetWindowSize(window: *mut SDL_Window, w: ffi::c_int, h: ffi::c_int);
  fn SDL_GetWindowPosition(window: *mut SDL_Window, x: *mut ffi::c_int, y: *mut ffi::c_int);
  fn SDL_SetWindowPosition(window: *mut SDL_Window, x: ffi::c_int, y: ffi::c_int);
  fn SDL_DestroyWindow(window: *mut SDL_Window);
  fn SDL_CreateRenderer(window: *mut SDL_Window, index: ffi::c_int, flags: u32) -> *mut SDL_Renderer;
  fn SDL_SetRenderDrawColor(renderer: *mut SDL_Renderer, r: u8, g: u8, b: u8, a: u8) -> ffi::c_int;
  fn SDL_GetRendererOutputSize(renderer: *mut SDL_Renderer, w: *mut ffi::c_int, h: *mut ffi::c_int) -> ffi::c_int;
  fn SDL_RenderSetLogicalSize(renderer: *mut SDL_Renderer, w: ffi::c_int, h: ffi::c_int) -> ffi::c_int;
  fn SDL_RenderClear(renderer: *mut SDL_Renderer) -> ffi::c_int;
  fn SDL_RenderFillRect(renderer: *mut SDL_Renderer, rect: *const SDL_Rect) -> ffi::c_int;
  fn SDL_RenderDrawRect(renderer: *mut SDL_Renderer, rect: *const SDL_Rect) -> ffi::c_int;
  fn SDL_RenderPresent(renderer: *mut SDL_Renderer);
  fn SDL_DestroyRenderer(renderer: *mut SDL_Renderer);
  fn SDL_CreateTextureFromSurface(renderer: *mut SDL_Renderer, surface: *mut SDL_Surface) -> *mut SDL_Texture;
  fn SDL_QueryTexture(
    texture: *mut SDL_Texture,
    format: *mut u32,
    access: *mut ffi::c_int,
    w: *mut ffi::c_int,
    h: *mut ffi::c_int,
  ) -> ffi::c_int;
  fn SDL_SetTextureScaleMode(texture: *mut SDL_Texture, scaleMode: u32) -> ffi::c_int;
  fn SDL_SetTextureBlendMode(texture: *mut SDL_Texture, blendMode: u32) -> ffi::c_int;
  fn SDL_SetTextureColorMod(texture: *mut SDL_Texture, r: u8, g: u8, b: u8) -> ffi::c_int;
  fn SDL_RenderCopy(
    renderer: *mut SDL_Renderer,
    texture: *mut SDL_Texture,
    srcrect: *const SDL_Rect,
    dstrect: *const SDL_Rect,
  ) -> ffi::c_int;
  fn SDL_DestroyTexture(texture: *mut SDL_Texture);
  fn SDL_GetTicks() -> u32;
  fn SDL_ThreadID() -> ffi::c_ulong;
  fn SDL_RWFromConstMem(mem: *const ffi::c_void, size: ffi::c_int) -> *mut SDL_RWops;
}

#[cfg_attr(target_os = "windows", link(name = "SDL2_image", kind = "raw-dylib"))]
unsafe extern "C-unwind" {
  fn IMG_Load_RW(src: *mut SDL_RWops, freesrc: ffi::c_int) -> *mut SDL_Surface;
}

// Get the number of milliseconds since the SDL library initialization
pub fn get_ticks() -> u32 {
  unsafe { SDL_GetTicks() }
}

// Get the current thread ID
pub fn thread_id() -> u64 {
  unsafe { SDL_ThreadID().into() }
}
