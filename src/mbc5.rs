use std::io;
use std::path::Path;

use log::{debug, error};

use crate::mbc::{MBC, get_ram_size_in_bytes, SRAM};
use crate::rom::ROMBanks;

const MBC5_MAX_ROM_BANKS: usize = 512; // 8MB

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
            0x0000..=0x3FFF => {
                // Fixed ROM bank 0
                /*if let Some(bank) = self.rom_banks.data.get(0) {
                    if let Some(&value) = bank.get(address as usize) {
                        value
                    } else {
                        error!("Attempted to read beyond ROM bank 0 boundaries at {:04X}", address);
                        0xFF
                    }
                } else {
                    panic!("ROM bank 0 not available");
                }*/
                self.get_value_from_bank(0, address, &self.rom_banks.data)
            },
            0x4000..=0x7FFF => {
                // Switchable ROM bank
                /*let bank = self.get_active_rom_bank();
                let bank_addr = address - 0x4000;

                if let Some(bank_data) = self.rom_banks.data.get(bank) {
                    if let Some(&value) = bank_data.get(bank_addr) {
                        value
                    } else {
                        error!("Attempted to read beyond ROM bank boundaries at bank {} addr {:04X}", bank, address);
                        0xFF
                    }
                } else {
                    panic!("ROM bank {} not available (total banks: {})", bank, self.rom_banks.data.len());
                }*/
                self.get_value_from_bank(self.get_active_rom_bank(), address - 0x4000, &self.rom_banks.data)
            },
            0xA000..=0xBFFF => {
                if self.is_ram_enabled() && self.has_ram {
                    // Read from RAM
                    if let Some(sram) = &self.sram {
                        let ram_addr = self.ram_bank * 0x2000 + (address - 0xA000) as usize;
                        sram.read(ram_addr)
                    } else {
                        error!("SRAM is None when trying to read from it");
                        0xFF
                    }
                } else {
                    // RAM disabled or doesn't exist, return 0xFF
                    0xFF
                }
            },
            _ => {
                error!("Invalid MBC5 address for read: {:04X}", address);
                0xFF
            }
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => {
                // RAM Enable (any value with bit 0 set enables, other values disable)
                self.ram_enabled = (value & 0x0F) == 0x0A;
            },
            0x2000..=0x2FFF => {
                // ROM Bank Number (lower 8 bits)
                // MBC5 uses a 9-bit bank number for up to 512 ROM banks
                let low_bits = value as usize;
                self.rom_bank = (self.rom_bank & 0x100) | low_bits;
            },
            0x3000..=0x3FFF => {
                // ROM Bank Number (9th bit)
                let high_bit = ((value & 0x01) as usize) << 8;
                self.rom_bank = (self.rom_bank & 0xFF) | high_bit;
            },
            0x4000..=0x5FFF => {
                // RAM Bank Number (4 bits)
                // For rumble carts, bit 3 controls rumble motor (0=off, 1=on)
                if self.has_rumble {
                    let rumble_on = (value & 0x08) != 0;
                    debug!("MBC5 Rumble state: {}", rumble_on);
                    // Use only bits 0-2 for RAM bank selection
                    self.ram_bank = (value & 0x07) as usize;
                } else {
                    // Use bits 0-3 for RAM bank selection (up to 16 banks)
                    self.ram_bank = (value & 0x0F) as usize;
                }
            },
            0xA000..=0xBFFF => {
                if self.is_ram_enabled() && self.has_ram {
                    // Write to RAM
                    if let Some(sram) = &mut self.sram {
                        let ram_addr = self.ram_bank * 0x2000 + (address - 0xA000) as usize;
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
                error!("SRAM is None when trying to save it");
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