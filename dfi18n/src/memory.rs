// TODO: consider move this to DFHack Lua implementation

use std::ffi;
use std::sync::{OnceLock, RwLock};

use anyhow::{Context, Result, anyhow};
use indexmap::IndexMap;

use lua53_sys as lua;

// A file offset entry
type FileOffset = (String, usize); // (path, offset)

// A memory region entry
type MemoryRegion = (String, usize, usize, usize); // (path, base, start, end)

// Memory regions added from Lua
static MEMORY_REGIONS: OnceLock<RwLock<Vec<MemoryRegion>>> = OnceLock::new();

// Get the memory regions
fn memory_regions() -> &'static RwLock<Vec<MemoryRegion>> {
  MEMORY_REGIONS.get_or_init(|| RwLock::new(Vec::new()))
}

// Get file offset by memory address
pub fn get_file_offset_by_memory_address(memory_address: usize) -> Option<FileOffset> {
  let regions = memory_regions().read().unwrap();
  for (path, base, start, end) in regions.iter() {
    if *start <= memory_address && memory_address < *end {
      return Some((path.to_owned(), memory_address - base));
    }
  }
  None
}

// Add a memory region entry from Lua
#[unsafe(no_mangle)]
extern "C" fn add_memory_region(lua_state: *mut ffi::c_void) {
  let path = lua::check_string(lua_state, 1);
  #[cfg(target_os = "linux")]
  let base = lua::check_integer(lua_state, 2) as usize;
  let mut start = lua::check_integer(lua_state, 3) as usize;
  let mut end = lua::check_integer(lua_state, 4) as usize;

  // get module base for Windows
  #[cfg(target_os = "windows")]
  let base = unsafe { winapi::um::libloaderapi::GetModuleHandleW(std::ptr::null()) as usize };

  // lock regions for writing
  let mut regions = memory_regions().write().unwrap();

  // merge adjacent regions on the left
  if let Some(lindex) =
    regions.iter().position(|(lpath, lbase, _lstart, lend)| *lpath == path && *lbase == base && *lend == start)
  {
    let lremoved = regions.remove(lindex);
    start = lremoved.2; // update start to lstart
  }

  // merge adjacent regions on the right
  if let Some(rindex) =
    regions.iter().position(|(rpath, rbase, rstart, _rend)| *rpath == path && *rbase == base && *rstart == end)
  {
    let rremoved = regions.remove(rindex);
    end = rremoved.3; // update end to rend
  }

  // add merged region
  regions.push((path, base, start, end));
}

// A memory search entry
type MemorySearch = (String, String, String, String); // (key, module, method, value)

// Memory searches added from Lua
static MEMORY_SEARCHES: OnceLock<RwLock<Vec<MemorySearch>>> = OnceLock::new();

// Get the memory searches
fn memory_searches() -> &'static RwLock<Vec<MemorySearch>> {
  MEMORY_SEARCHES.get_or_init(|| RwLock::new(Vec::new()))
}

// Add a memory search entry from Lua
#[unsafe(no_mangle)]
extern "C" fn add_memory_search(lua_state: *mut ffi::c_void) {
  let key = lua::check_string(lua_state, 1);
  let module = lua::check_string(lua_state, 2);
  let method = lua::check_string(lua_state, 3);
  let value = lua::check_string(lua_state, 4);

  // lock searches for writing
  let mut searches = memory_searches().write().unwrap();

  // add search entry
  searches.push((method, module, key, value));
}

// A pointer entry
type Pointer = (FileOffset, usize); // (file_offset, memory_address)

// Search key to memory address mapping
static POINTERS: OnceLock<IndexMap<String, Pointer>> = OnceLock::new();

// Get all pointers
pub fn pointers() -> &'static IndexMap<String, Pointer> {
  POINTERS.get().expect("memory searches have not been run")
}

// Get pointer by key
pub fn get_pointer(key: &str) -> Option<&'static Pointer> {
  pointers().get(key)
}

// Get memory address by key
pub fn get_memory_address_by_key(key: &str) -> Option<usize> {
  get_pointer(key).and_then(|p| Some(p.1))
}

// Get raw pointer by key
pub fn get_raw_pointer_by_key<T>(key: &str) -> Result<*const T> {
  let address = get_memory_address_by_key(key).ok_or(anyhow!("pointer not found for key: {}", key))?;
  Ok(address as *const T)
}

// Run memory searches and populate key to address mapping
pub fn run_searches() -> Result<()> {
  // run only once
  if POINTERS.get().is_some() {
    return Ok(());
  }

  // collected pointers
  let mut pointers = IndexMap::new();

  // searcher maps by module
  let mut searcher_maps = IndexMap::new();
  // symbol maps by module
  let mut symbol_maps = IndexMap::new();

  // get memory regions and searches
  let regions = memory_regions().read().unwrap();
  let searches = memory_searches().read().unwrap();

  // collect searches by method and module
  for (method, module, key, value) in searches.iter() {
    match method.as_str() {
      "pattern" => {
        let searcher_map = searcher_maps.entry(module).or_insert_with(IndexMap::new);
        searcher_map.insert(key, Searcher::new(value)?);
      }
      "symbol" => {
        let symbol_map = symbol_maps.entry(module).or_insert_with(IndexMap::new);
        symbol_map.insert(value, key);
      }
      _ => {
        log::warn!("Unknown memory search method: {}, ignored", method);
      }
    }
  }

  // module to path mapping
  let mut module_map = IndexMap::new();
  for (path, _base, _start, _end) in regions.iter() {
    let module = path.rsplit_once(std::path::MAIN_SEPARATOR).ok_or(anyhow!("invalid module path: {path}"))?.1;
    module_map.insert(module, path);
  }

  // use searchers to search memory regions
  for (&module, searcher_map) in searcher_maps.iter() {
    for (path, base, start, end) in regions.iter().filter(|(path, _, _, _)| path.ends_with(module)) {
      let data = unsafe { std::slice::from_raw_parts(*start as *const u8, end - start) };
      for (&key, searcher) in searcher_map.iter() {
        // TODO: run searches in parallel to improve performance
        let matches = searcher.matches(data);
        let n_matches = matches.len();
        if n_matches != 1 {
          log::warn!("Memory search pattern {key} in {module} expected 1 match but found {n_matches} matches, ignored",);
          continue;
        }

        // insert the found pointer
        let memory_address = start + matches[0];
        pointers.insert(
          key.to_owned(),
          ((path.to_owned(), memory_address - base), memory_address),
        );
      }
    }
  }

  // use symbol tables to resolve symbols
  for (&module, symbol_map) in symbol_maps.iter() {
    let &path = module_map.get(module.as_str()).ok_or(anyhow!("no module path found for module: {module}"))?;
    let lib = unsafe { libloading::Library::new(path).context(format!("failed to load module {module}"))? };
    for (&symbol, &key) in symbol_map.iter() {
      let memory_address =
        *unsafe { lib.get::<usize>(symbol) }.context(format!("failed to get symbol {symbol} from module {module}"))?;

      // insert the found pointer
      let file_offset = get_file_offset_by_memory_address(memory_address).ok_or(anyhow!(
        "failed to get file offset for symbol {symbol} in module {module}"
      ))?;
      pointers.insert(key.to_owned(), (file_offset, memory_address));
    }
  }

  POINTERS.get_or_init(|| pointers);

  Ok(())
}

// Token types for pattern matching
enum Token {
  // matches a known byte - 00 to FF
  Value(u8),
  // matches any ASCII digit - ##
  Digit,
  // matches any byte - ??
  Wildcard,
}

// Memory searcher based on a pattern
struct Searcher {
  // Parsed pattern tokens
  tokens: Vec<Token>,
}

impl Searcher {
  // Create a new searcher from a pattern string
  fn new(pattern: &str) -> Result<Self> {
    let mut tokens = Vec::new();
    for byte in pattern.split(' ') {
      let token = if byte == "??" {
        Token::Wildcard
      } else if byte == "##" {
        Token::Digit
      } else {
        Token::Value(u8::from_str_radix(byte, 16)?)
      };
      tokens.push(token);
    }

    Ok(Self { tokens })
  }

  // Find all offsets that match the pattern in the given data
  fn matches(&self, data: &[u8]) -> Vec<usize> {
    let mut result = Vec::new();

    data.windows(self.tokens.len()).enumerate().for_each(|(offset, window)| {
      for (token, byte) in self.tokens.iter().zip(window.iter()) {
        match token {
          Token::Wildcard => continue,
          Token::Digit if byte.is_ascii_digit() => continue,
          Token::Value(expected) if *expected == *byte => continue,
          _ => return,
        }
      }
      result.push(offset);
    });

    result
  }
}
