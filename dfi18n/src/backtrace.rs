use crate::{logger, memory};

// FIXME: incorrect offset for Windows builds

// collect backtrace information
pub fn backtrace() -> String {
  if logger::level() < flexi_logger::LevelFilter::Debug {
    // Log level is less than DEBUG, skip backtrace
    return String::new();
  }

  let mut addresses = Vec::<String>::default();
  backtrace::trace(|frame| {
    let ip = frame.ip() as usize;
    if ip == 0 {
      return false;
    }
    let fp = frame.symbol_address() as usize;

    for (i, address) in vec![ip, fp].into_iter().enumerate() {
      let prefix = if i == 0 { "^" } else { "" };
      if let Some((path, offset)) = memory::get_file_offset_by_memory_address(address) {
        if path.ends_with("/libg_src_lib.so") {
          addresses.push(format!("{prefix}@{offset:x}"));
        } else {
          addresses.push(format!("{prefix}{offset:x}"));
        }
      }
    }

    true
  });
  addresses.join("/")
}
