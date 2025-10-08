use std::io;
use std::path::{Path, PathBuf};
use log::debug;

pub trait MBC {
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, value: u8);
    fn get_rom_bank(&self) -> usize;
    fn get_ram_bank(&self) -> usize;
    fn is_ram_enabled(&self) -> bool;
    fn save_ram(&mut self) -> Result<(), io::Error>;
    fn dirty_sram(&self) -> bool;
}
#[derive(Debug, Clone, Copy)]
pub enum MBCType {
    None,
    MBC1,
    MBC2,
    MBC3,
    MBC5,
}

impl MBCType {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0x00 => MBCType::None,
            0x01 | 0x02 | 0x03 => MBCType::MBC1,
            0x05 | 0x06 => MBCType::MBC2,
            0x0F | 0x10 | 0x11 | 0x12 | 0x13 => MBCType::MBC3,
            0x19 | 0x1A | 0x1B | 0x1C | 0x1D | 0x1E => MBCType::MBC5,
            _ => { panic!("Unsupported MBC type: {:#X}", value) }
        }
    }
}

pub fn get_rom_size_in_bytes(rom_size: u8) -> usize {
    match rom_size {
        0x00 => 32 * 1024,     // 32KB (2 banks)
        0x01 => 64 * 1024,     // 64KB (4 banks)
        0x02 => 128 * 1024,    // 128KB (8 banks)
        0x03 => 256 * 1024,    // 256KB (16 banks)
        0x04 => 512 * 1024,    // 512KB (32 banks)
        0x05 => 1024 * 1024,   // 1MB (64 banks)
        0x06 => 2 * 1024 * 1024, // 2MB (128 banks)
        0x07 => 4 * 1024 * 1024, // 4MB (256 banks)
        0x08 => 8 * 1024 * 1024, // 8MB (512 banks)
        _ => 32 * 1024,        // Default to 32KB
    }
}

pub fn get_rom_banks(rom_size: u8) -> usize {
    get_rom_size_in_bytes(rom_size) / (16 * 1024)
}

pub fn get_ram_size_in_bytes(ram_size: u8) -> usize {
    match ram_size {
        0x00 => 0,             // No RAM
        0x01 => 2 * 1024,      // 2KB
        0x02 => 8 * 1024,      // 8KB
        0x03 => 32 * 1024,     // 32KB (4 banks of 8KB)
        0x04 => 128 * 1024,    // 128KB (16 banks of 8KB)
        0x05 => 64 * 1024,     // 64KB (8 banks of 8KB)
        _ => 0,                // Default to no RAM
    }
}

pub fn get_ram_banks(ram_size: u8) -> usize {
    match ram_size {
        0x00 => 0,             // No RAM
        0x01 => 1,             // 1 bank
        0x02 => 1,             // 1 bank
        0x03 => 4,             // 4 banks
        0x04 => 16,            // 16 banks
        0x05 => 8,             // 8 banks
        _ => 0,                // Default to no banks
    }
}

pub struct SRAM {
    data: Vec<u8>,
    pub(crate) dirty: bool,
    save_file_path: PathBuf,
}

impl SRAM {
    pub fn new(size: usize, rom_file_path: &Path) -> Self{
        let mut data = vec![0; size];
        let save_path = rom_file_path.with_extension("sav");

        if save_path.exists() {
            if let Ok(saved_data) = std::fs::read(&save_path) {
                if saved_data.len() == size {
                    data.copy_from_slice(&saved_data);
                }
            }
        }

        Self {
            data,
            dirty: false,
            save_file_path: save_path,
        }
    }
    
    pub fn read(&self, address: usize) -> u8 {
        self.data[address]
    }
    
    pub fn write(&mut self, address: usize, value: u8) {
        self.data[address] = value;
        self.dirty = true;
    }
    
    pub fn save(&mut self) {
        if self.dirty {
            std::fs::write(&self.save_file_path, &self.data).expect("Failed to save SRAM data");
            self.dirty = false;
            debug!("Saved RAM data to {:?}", &self.save_file_path);
        }
    }
    
}