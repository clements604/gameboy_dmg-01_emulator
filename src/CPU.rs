
use std::{error, fmt};
use std::fs::File;
use std::io::prelude::*;
use log::{debug, error};

use crate::constants;
use constants::*;
use crate::rom;
use rom::*;


struct Registers {
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    pc: u16, // Program counter
    sp: u16, // Stack pointer
}
pub struct CPU {

    registers: Registers,
    work_ram: [u8; 0xFFFF],
    video_ram: [u16; 8192],
    rom_bank_0: [u8; 0x3FFF], //16KiB

}

impl Registers {
    pub fn new() -> Self {
        Registers {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            pc: 0,
            sp: 0,
        }
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            registers: Registers::new(),
            work_ram: [0; 0xFFFF],
            video_ram: [0; 8192],
            rom_bank_0: [0; 0x3FFF],
        }
    }

    /*
    *   Load the ROM into memory
    */
    pub fn load_rom(&mut self, file_path: String) {
        debug!("Loading ROM: {}", file_path);
        let mut file = File::open(file_path).expect("ROM file not found");
        let mut buffer: Vec<u8> = Vec::new();

        // Read the file into a buffer
        file.read_to_end(&mut buffer).expect("Error reading file");
        debug!("ROM file size: {} bytes / {} kilobytes", buffer.len(), buffer.len() / 1024);

        let rom = ROM::new(buffer);
        debug!("{}", rom);

        rom.validate_header_checksum().unwrap(); // Panics if the header checksum is invalid

        // TODO load rom to memory
        // Check cartridge type and load the ROM into memory based on the type
        for byte in 0x00..0x3FFF {
            self.rom_bank_0[byte] = rom.rom[byte];
        }
        debug!("ROM Bank 0 loaded into memory, size: {} bytes", self.rom_bank_0.len());
        if rom.cartridge_type == 0x00 {
            // ROM ONLY
            debug!("ROM ONLY");
        }
        else {
            debug!("ROM with MBC");
            let rom_banks    = rom.load_rom_to_banks();
            self.work_ram[0x4000..=0x7FFF].copy_from_slice(&rom_banks.data[0]); // Load the first bank of the ROM into memory
            debug!("ROM Bank 1 loaded into memory, size: {} bytes", rom_banks.data[0].len());
        }
        //debug!("ROM loaded into memory");
    }

    /*
    *   Write data to TODO
    */
    pub fn write_rom(&mut self, address: u16, data: u8) {
        unimplemented!("write_rom");
    }

    /*
    * nop - Do nothing
    */
    pub fn nop(&mut self) {
        debug!("NOP");
    }
    
    fn calculate_global_checksum(&mut self, rom_data: &[u8]) -> u16 {
        let mut global_checksum: u16 = 0;
        for (i, &byte) in rom_data.iter().enumerate() {
            if i == 0x014E || i == 0x014F {
                continue;
            }
            global_checksum = global_checksum.wrapping_add(byte as u16);
        }
        global_checksum
    }

}
