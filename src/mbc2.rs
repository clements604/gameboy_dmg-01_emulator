use std::io;
use std::path::Path;
use log::{debug, error};
use crate::mbc::{MBC, SRAM};
use crate::rom::ROMBanks;

/*
    https://gbdev.io/pandocs/MBC2.html
*/

const MBC2_RAM_SIZE: usize = 512;
const MBC2_RAM_ADDR_MASK: u16 = 0x01FF; // Only 9 bits are used for addressing

pub struct MBC2 {
    rom_banks: ROMBanks,
    sram: Option<SRAM>,
    rom_bank: usize,
    ram_enabled: bool,
    has_battery: bool,
}

impl MBC2 {
    pub fn new(rom_banks: ROMBanks, has_battery: bool, rom_path: &Path) -> Self {
        let sram = if has_battery {
            Some(SRAM::new(MBC2_RAM_SIZE, rom_path))
        } else {
            None
        };
        MBC2 {
            rom_banks,
            sram,
            rom_bank: 1,
            ram_enabled: false,
            has_battery,
        }
    }

    // Helper function to get the effective ROM bank number
    fn get_selected_rom_bank(&self) -> usize {
        // Only lowest 4 bits are used in MBC2, allowing 16 ROM banks
        self.rom_bank & 0x0F
    }
}

impl MBC for MBC2 {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                self.get_value_from_bank(0, address, &self.rom_banks.data)
            },
            0x4000..=0x7FFF => {
                let bank = self.get_selected_rom_bank();
                let bank_addr = address - 0x4000;
                self.get_value_from_bank(bank, bank_addr, &self.rom_banks.data)
            },
            0xA000..=0xBFFF => {
                if self.is_ram_enabled() {
                    let ram_addr = (address & MBC2_RAM_ADDR_MASK) as usize;
                    
                    if ram_addr < MBC2_RAM_SIZE {
                        if let Some(sram) = &self.sram {
                            sram.read(ram_addr) & 0x0F
                        } else {
                            0xFF
                        }
                    } else {
                        error!("Attempted to read from non-existent MBC2 RAM at {:04X}", address);
                        0xFF
                    }
                } else {
                    0xFF
                }
            },
            _ => {
                error!("Invalid MBC2 address for read: {:04X}", address);
                0xFF
            }
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x3FFF => {
                if (address & 0x0100) == 0 {
                    // RAM Enable (0x0A enables, anything else disables)
                    self.ram_enabled = (value & 0x0F) == 0x0A;
                } else {
                    // ROM Bank Select (lower 4 bits)
                    // If 0 is written, it's treated as 1
                    let bank_num = (value & 0x0F) as usize;
                    self.rom_bank = if bank_num == 0 { 1 } else { bank_num };
                    debug!("MBC2 ROM bank set to: {:02X}", self.rom_bank);
                }
            },
            0x4000..=0x7FFF => {
                debug!("Write to ROM area 4000-7FFF ignored in MBC2: {:04X} = {:02X}", address, value);
            },
            0xA000..=0xBFFF => {
                if self.is_ram_enabled() {
                    let ram_addr = (address & MBC2_RAM_ADDR_MASK) as usize;

                    if ram_addr < MBC2_RAM_SIZE {
                        if let Some(sram) = &mut self.sram {
                            sram.write(ram_addr, value & 0x0F);
                            debug!("Battery-backed MBC2 RAM write at {:04X} = {:02X}", address, value & 0x0F);
                        }
                    } else {
                        error!("Attempted to write to non-existent MBC2 RAM at {:04X}", address);
                    }
                } else {
                    debug!("Attempted to write to disabled MBC2 RAM: {:04X} = {:02X}", address, value);
                }
            },
            _ => {
                error!("Invalid MBC2 address for write: {:04X} = {:02X}", address, value);
            }
        }
    }

    fn get_rom_bank(&self) -> usize {
        self.get_selected_rom_bank()
    }

    fn get_ram_bank(&self) -> usize {
        0 // MBC2 doesn't use ram banking
    }

    fn is_ram_enabled(&self) -> bool {
        self.ram_enabled
    }

    fn save_ram(&mut self) -> Result<(), io::Error> {
        if self.has_battery {
            if let Some(sram) = &mut self.sram {
                debug!("MBC2 RAM saved successfully");
                return sram.save();
            }   
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