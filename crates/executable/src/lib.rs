mod globals;

#[cfg(target_os = "linux")]
mod elf;
#[cfg(target_os = "windows")]
mod pe;

// Export parse_globals function
pub use globals::parse_globals;

// Export ElfFile for Linux
#[cfg(target_os = "linux")]
pub use elf::ElfFile;
// Export PeFile for Windows
#[cfg(target_os = "windows")]
pub use pe::PeFile;
