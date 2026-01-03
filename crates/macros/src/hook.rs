#[macro_export]
macro_rules! hook {
  // Empty: do nothing
  () => {};

  // Single function: define hook for a function
  (fn $name:ident($($param:ident : $argument_type:ty),*) -> $return_type:ty;) => {
    with_builtin_macros::with_eager_expansions! {
      // Setup static detour for the hook, with variable name "hook_{name}"
      retour::static_detour! {
        static ${concat(hook_, $name)}: fn($($argument_type),*) -> $return_type;
      }

      // Setup the attach function for initializing the hook, with function name "attach_{name}"
      fn ${concat(attach_, $name)}(ptr: *const std::ffi::c_void) -> retour::Result<()> {
        unsafe {
          let target = std::mem::transmute(ptr);
          ${concat(hook_, $name)}.initialize(target, $name)?;
          ${concat(hook_, $name)}.enable()?;
        }

        Ok(())
      }

      // Setup the call function for calling the original function, with function name "call_{name}", this function may not be called
      #[allow(dead_code)]
      pub fn ${concat(call_, $name)}($($param : $argument_type),*) -> $return_type {
        ${concat(hook_, $name)}.call($($param),*)
      }
    }
  };

  // Single function without return type: expand to a hook of a single function returning ()
  (fn $name:ident($($param:ident : $argument_type:ty),*);) => {
    hook!{ fn $name($($param : $argument_type),*) -> (); }
  };

  // Multiple functions with the first function without return type: expand the first function to a hook of the function returning (), then process the rest
  (fn $name:ident($($param:ident : $argument_type:ty),*);
   $($rest:tt)*) => {
    hook!{ fn $name($($param : $argument_type),*) -> (); }
    hook!{ $($rest)* }
  };

  // Multiple functions with the first function having a return type: expand the first function to a hook of the function, then process the rest
  (fn $name:ident($($param:ident : $argument_type:ty),*) -> $return_type:ty;
   $($rest:tt)*) => {
    hook!{ fn $name($($param : $argument_type),*) -> $return_type; }
    hook!{ $($rest)* }
  };
}
