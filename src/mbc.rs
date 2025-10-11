use std::io;
use std::path::{Path, PathBuf};
use log::{debug, error};

// Memory sizes (in bytes)
const KB_16: usize = 16 * 1024;
const KB_32: usize = 32 * 1024;
const KB_64: usize = 64 * 1024;
const KB_128: usize = 128 * 1024;
const KB_256: usize = 256 * 1024;
const KB_512: usize = 512 * 1024;
const MB_1: usize = 1024 * 1024;
const MB_2: usize = 2 * MB_1;
const MB_4: usize = 4 * MB_1;
const MB_8: usize = 8 * MB_1;

// RAM sizes
const KB_2: usize = 2 * 1024;
const KB_8: usize = 8 * 1024;

// MBC type codes
const MBC_NONE: u8 = 0x00;
const MBC1_ROM: u8 = 0x01;
const MBC1_ROM_RAM: u8 = 0x02;
const MBC1_ROM_RAM_BATTERY: u8 = 0x03;
const MBC2_ROM: u8 = 0x05;
const MBC2_ROM_BATTERY: u8 = 0x06;
const MBC3_ROM: u8 = 0x0F;
const MBC3_ROM_RAM: u8 = 0x10;
const MBC3_ROM_RAM_BATTERY: u8 = 0x11;
const MBC3_ROM_TIMER: u8 = 0x12;
const MBC3_ROM_TIMER_BATTERY: u8 = 0x13;
const MBC5_ROM: u8 = 0x19;
const MBC5_ROM_RAM: u8 = 0x1A;
const MBC5_ROM_RAM_BATTERY: u8 = 0x1B;
const MBC5_RUMBLE: u8 = 0x1C;
const MBC5_RUMBLE_RAM: u8 = 0x1D;
const MBC5_RUMBLE_RAM_BATTERY: u8 = 0x1E;

// Memory region addresses
pub const ROM_BANK0_START: u16 = 0x0000;
pub const ROM_BANK0_END: u16 = 0x3FFF;
pub const ROM_BANK_N_START: u16 = 0x4000;
pub const ROM_BANK_N_END: u16 = 0x7FFF;
pub const RAM_START: u16 = 0xA000;
pub const RAM_END: u16 = 0xBFFF;
pub const INVALID_READ_VALUE: u8 = 0xFF;
pub const ROM_BANK_SELECT_START: u16 = 0x2000;


pub trait MBC {
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, value: u8);
    fn get_rom_bank(&self) -> usize;
    fn get_ram_bank(&self) -> usize;
    fn is_ram_enabled(&self) -> bool;
    fn save_ram(&mut self) -> Result<(), io::Error>;
    fn dirty_sram(&self) -> bool;

    fn get_value_from_bank(&self, bank: usize, address: u16, data: &Vec<Vec<u8>>) -> u8 {
        if let Some(bank) = data.get(bank) {
            if let Some(&value) = bank.get(address as usize) {
                value
            } else {
                error!("Attempted to read beyond ROM bank 0 boundaries at {:04X}", address);
                0xFF
            }
        } else {
            error!("ROM bank {} not available", bank);
            0xFF
        }
    }
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
            MBC_NONE => MBCType::None,
            MBC1_ROM | MBC1_ROM_RAM | MBC1_ROM_RAM_BATTERY => MBCType::MBC1,
            MBC2_ROM | MBC2_ROM_BATTERY => MBCType::MBC2,
            MBC3_ROM | MBC3_ROM_RAM | MBC3_ROM_RAM_BATTERY | MBC3_ROM_TIMER | MBC3_ROM_TIMER_BATTERY => MBCType::MBC3,
            MBC5_ROM | MBC5_ROM_RAM | MBC5_ROM_RAM_BATTERY | MBC5_RUMBLE | MBC5_RUMBLE_RAM | MBC5_RUMBLE_RAM_BATTERY => MBCType::MBC5,
            _ => panic!("Unsupported MBC type: {:#X}", value)
        }
    }
}

pub fn get_rom_size_in_bytes(rom_size: u8) -> usize {
    match rom_size {
        0x00 => KB_32,    // 32KB (2 banks)
        0x01 => KB_64,    // 64KB (4 banks)
        0x02 => KB_128,   // 128KB (8 banks)
        0x03 => KB_256,   // 256KB (16 banks)
        0x04 => KB_512,   // 512KB (32 banks)
        0x05 => MB_1,     // 1MB (64 banks)
        0x06 => MB_2,     // 2MB (128 banks)
        0x07 => MB_4,     // 4MB (256 banks)
        0x08 => MB_8,     // 8MB (512 banks)
        _ => KB_32,       // Default to 32KB
    }
}

pub fn get_rom_banks(rom_size: u8) -> usize {
    get_rom_size_in_bytes(rom_size) / KB_16
}

pub fn get_ram_size_in_bytes(ram_size: u8) -> usize {
    match ram_size {
        0x00 => 0,        // No RAM
        0x01 => KB_2,     // 2KB
        0x02 => KB_8,     // 8KB
        0x03 => KB_32,    // 32KB (4 banks of 8KB)
        0x04 => KB_128,   // 128KB (16 banks of 8KB)
        0x05 => KB_64,    // 64KB (8 banks of 8KB)
        _ => 0,           // Default to no RAM
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
    
    pub fn save(&mut self) -> Result<(), io::Error> {
        if self.dirty {
            self.dirty = false;
            debug!("Saved RAM data to {:?}", &self.save_file_path);
            return std::fs::write(&self.save_file_path, &self.data);
        }
        Ok(())
    }

}