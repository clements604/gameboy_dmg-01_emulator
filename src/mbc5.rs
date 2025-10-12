use std::io;
use std::path::Path;

use log::{debug, info};

use crate::mbc::{
    MBC,
    get_ram_size_in_bytes,
    SRAM,
    ROM_BANK0_START,
    ROM_BANK0_END,
    ROM_BANK_N_START,
    ROM_BANK_N_END,
    RAM_START,
    RAM_END,
    INVALID_READ_VALUE,
    ROM_BANK_SELECT_START
};
use crate::rom::ROMBanks;

const MBC5_MAX_ROM_BANKS: usize = 512; // 8MB
const RAM_ENABLE_AREA_END: u16 = 0x1FFF;
const RAM_ENABLE_MASK: u8 = 0x0F;
const RAM_ENABLE_VALUE: u8 = 0x0A;
const ROM_BANK_BITS_MASK: usize = 0x100;
const ROM_BANK_HIGH_BIT_MASK: u8 = 0x01;
const RAM_BANK_MASK: u8 = 0x0F;
const RUMBLE_BIT_MASK: u8 = 0x08;
const RAM_BANK_RUMBLE_MASK: u8 = 0x07;
const BANK_SIZE: usize = 0x2000;
const ROM_BANK_HIGH_BIT_AREA_START: u16 = 0x3000;
const ROM_BANK_HIGH_BIT_AREA_END: u16 = 0x3FFF;
const RAM_BANK_SELECT_END: u16 = 0x5FFF;
// ROM bank control bitmasks
const ROM_BANK_LOW_BITS_MASK: usize = 0xFF;
const ROM_BANK_HIGH_BIT_SHIFT: usize = 8;

// ROM bank selection areas
const ROM_BANK_LOW_BITS_END: u16 = 0x2FFF;

pub struct MBC5 {
    rom_banks: ROMBanks,
    sram: Option<SRAM>,
    rom_bank: usize,
    ram_bank: usize,
    ram_enabled: bool,
    has_ram: bool,
    has_battery: bool,
    has_rumble: bool,
    rom_bank_count: usize,
}

impl MBC5 {
    pub fn new(rom_banks: ROMBanks, ram_size: u8, has_battery: bool, has_rumble: bool, rom_path: &Path) -> Self {
        let ram_size_bytes = get_ram_size_in_bytes(ram_size);
        let has_ram = ram_size > 0;
        let rom_bank_count = std::cmp::min(MBC5_MAX_ROM_BANKS, rom_banks.data.len());

        // Create SRAM only if there is RAM
        let sram = if has_ram {
            Some(SRAM::new(ram_size_bytes, rom_path))
        } else {
            None
        };

        MBC5 {
            rom_banks,
            sram,
            rom_bank: 1,  // Default to bank 1
            ram_bank: 0,
            ram_enabled: false,
            has_ram,
            has_battery,
            has_rumble,
            rom_bank_count,
        }
    }

    fn get_active_rom_bank(&self) -> usize {
        self.rom_bank % self.rom_bank_count
    }
    
}

impl MBC for MBC5 {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            ROM_BANK0_START..=ROM_BANK_HIGH_BIT_AREA_END => {
                self.get_value_from_bank(0, address, &self.rom_banks.data)
            },
            ROM_BANK_N_START..=ROM_BANK_N_END => {
                self.get_value_from_bank(self.get_active_rom_bank(), address - ROM_BANK_N_START, &self.rom_banks.data)
            },
            RAM_START..=RAM_END => {
                if self.is_ram_enabled() && self.has_ram {
                    // Read from RAM
                    if let Some(sram) = &self.sram {
                        let ram_addr = self.ram_bank * BANK_SIZE + (address - RAM_START) as usize;
                        sram.read(ram_addr)
                    } else {
                        info!("SRAM is None when trying to read from it");
                        INVALID_READ_VALUE
                    }
                } else {
                    INVALID_READ_VALUE
                }
            },
            _ => {
                info!("Invalid MBC5 address for read: {:04X}", address);
                INVALID_READ_VALUE
            }
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            ROM_BANK0_START..=RAM_ENABLE_AREA_END => {
                // RAM Enable (any value with bit 0 set enables, other values disable)
                self.ram_enabled = (value & RAM_ENABLE_MASK) == RAM_ENABLE_VALUE;
            },
            ROM_BANK_SELECT_START..=ROM_BANK_LOW_BITS_END => {
                // ROM Bank Number (lower 8 bits)
                // MBC5 uses a 9-bit bank number for up to 512 ROM banks
                let low_bits = value as usize;
                self.rom_bank = (self.rom_bank & ROM_BANK_BITS_MASK) | low_bits;
            },
            ROM_BANK_HIGH_BIT_AREA_START..=ROM_BANK0_END => {
                // ROM Bank Number (9th bit)
                let high_bit = ((value & ROM_BANK_HIGH_BIT_MASK) as usize) << ROM_BANK_HIGH_BIT_SHIFT;
                self.rom_bank = (self.rom_bank & ROM_BANK_LOW_BITS_MASK) | high_bit;
            },
            ROM_BANK_N_START..=RAM_BANK_SELECT_END => {
                // RAM Bank Number (4 bits)
                // For rumble carts, bit 3 controls rumble motor (0=off, 1=on)
                if self.has_rumble {
                    let rumble_on = (value & RUMBLE_BIT_MASK) != 0;
                    debug!("MBC5 Rumble state: {}", rumble_on);
                    // Use only bits 0-2 for RAM bank selection
                    self.ram_bank = (value & RAM_BANK_RUMBLE_MASK) as usize;
                } else {
                    // Use bits 0-3 for RAM bank selection (up to 16 banks)
                    self.ram_bank = (value & RAM_BANK_MASK) as usize;
                }
            },
            RAM_START..=RAM_END => {
                if self.is_ram_enabled() && self.has_ram {
                    // Write to RAM
                    if let Some(sram) = &mut self.sram {
                        let ram_addr = self.ram_bank * BANK_SIZE + (address - RAM_START) as usize;
                        sram.write(ram_addr, value);

                        // Log if this is battery-backed RAM
                        if self.has_battery {
                            debug!("Battery-backed MBC5 RAM write at bank {} addr {:04X} = {:02X}", 
                                  self.ram_bank, address, value);
                        }
                    }
                }
            },
            _ => {
                // Ignore writes to unsupported areas
                debug!("Ignored write to address: {:04X} = {:02X}", address, value);
            }
        }
    }

    fn get_rom_bank(&self) -> usize {
        self.get_active_rom_bank()
    }

    fn get_ram_bank(&self) -> usize {
        self.ram_bank
    }

    fn is_ram_enabled(&self) -> bool {
        self.ram_enabled && self.has_ram
    }

    fn save_ram(&mut self) -> Result<(), io::Error> {
        // Only save if this cartridge has battery-backed RAM
        if self.has_battery && self.has_ram {
            if let Some(sram) = &mut self.sram {
                return sram.save();
            } else {
                info!("SRAM is None when trying to save it");
            }
        } else {
            debug!("Not saving MBC5 RAM - no battery or no RAM");
        }
        Ok(())
    }

    fn dirty_sram(&self) -> bool {
        if let Some(sram) = &self.sram {
            sram.dirty
        } else {
            false
        }
    }
    
}