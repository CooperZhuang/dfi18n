use std::{collections::HashMap, error::Error};

use goblin::elf::Elf;
use indexmap::IndexMap;

// Struct of a ELF file
pub struct ElfFile {
  // ELF symbols offsets
  symbols: HashMap<String, usize>,
}

impl ElfFile {
  // Create a new ElfFile from raw ELF data
  pub fn new(data: &[u8]) -> Result<ElfFile, Box<dyn Error>> {
    let mut symbols = HashMap::new();
    let elf = Elf::parse(&data)?;
    for dynsym in elf.dynsyms.iter() {
      if let Some(name) = elf.dynstrtab.get_at(dynsym.st_name) {
        symbols.insert(name.to_owned(), dynsym.st_value as usize);
      }
    }
    Ok(ElfFile { symbols })
  }

  // Get function offsets for the provided symbols map
  pub fn function_offsets(&self, symbols_map: IndexMap<&str, &str>) -> Result<IndexMap<String, usize>, Box<dyn Error>> {
    let mut offsets = IndexMap::new();

    for (symbol, name) in symbols_map.into_iter() {
      if let Some(&offset) = self.symbols.get(symbol) {
        offsets.insert(name.to_owned(), offset);
      } else {
        return Err(format!("Symbol not found: {}", symbol).into());
      }
    }

    Ok(offsets)
  }
}
