use std::error::Error;

use goblin::pe::{PE, section_table::SectionTable};

// Struct of a PE file
pub struct PeFile {
  // PE sections
  sections: Vec<PeSection>,
}

impl PeFile {
  // Creates a new PeFile from raw PE data
  pub fn new(data: &[u8]) -> Result<PeFile, Box<dyn Error>> {
    let pe = PE::parse(&data)?;
    let mut sections = Vec::new();
    for section in pe.sections.iter() {
      sections.push(PeSection::from(section));
    }
    Ok(PeFile { sections })
  }

  // Converts a file offset to a virtual address
  pub fn offset_to_address(&self, offset: u32) -> Option<u32> {
    for section in &self.sections {
      if offset >= section.start && offset < section.start + section.size {
        let offset_in_section = offset - section.start;
        return Some(section.address + offset_in_section);
      }
    }

    None
  }
}

// Struct of a PE section
struct PeSection {
  // Section start offset in file
  start: u32,
  // Section size in file
  size: u32,
  // Section virtual address in memory
  address: u32,
}

impl From<&SectionTable> for PeSection {
  // Creates a PeSection from a SectionTable
  fn from(section: &SectionTable) -> Self {
    PeSection {
      start: section.pointer_to_raw_data,
      size: section.size_of_raw_data,
      address: section.virtual_address,
    }
  }
}
