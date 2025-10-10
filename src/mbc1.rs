use std::io;
use std::path::Path;
use log::{debug, error, info};
use crate::mbc::{MBC, get_ram_size_in_bytes, get_ram_banks, get_rom_banks, SRAM};
use crate::rom::ROMBanks;

/*
    https://gbdev.io/pandocs/MBC1.html
*/

#[derive(Debug, Clone, Copy, PartialEq)]
enum BankingMode {
    ROM, // Default mode: RAM bank 0 fixed, ROM banks switchable
    RAM, // RAM banking mode: ROM bank can be switched in high bits, RAM banks switchable
}

pub struct MBC1 {
    rom_banks: ROMBanks,
    sram: Option<SRAM>,
    rom_bank: usize,
    ram_bank: usize,
    ram_enabled: bool,
    has_ram: bool,
    has_battery: bool,
    banking_mode: BankingMode,
    rom_bank_count: usize,
    ram_bank_count: usize,
}

impl MBC1 {
    pub fn new(rom_banks: ROMBanks, rom_size: u8, ram_size: u8, has_battery: bool, rom_path: &Path) -> Self {
        let ram_size_bytes = get_ram_size_in_bytes(ram_size);
        let has_ram = ram_size > 0;
        info!("MBC1 has_ram: {}, ram_size_bytes: {}, has_battery: {}", has_ram, ram_size_bytes, has_battery);
        let rom_bank_count = get_rom_banks(rom_size);
        let ram_bank_count = get_ram_banks(ram_size);

        // Create SRAM only if there is RAM
        let sram = if has_ram {
            Some(SRAM::new(ram_size_bytes, rom_path))
        } else {
            None
        };

        MBC1 {
            rom_banks,
            sram,
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
            has_ram,
            has_battery,
            banking_mode: BankingMode::ROM,
            rom_bank_count,
            ram_bank_count,
        }
    }

    fn get_selected_rom_bank(&self) -> usize {
        let mut bank = self.rom_bank;

        if bank % 0x20 == 0 {
            bank += 1;
        }

        bank & (self.rom_bank_count - 1)
    }

    fn get_active_ram_bank(&self) -> usize {
        if self.banking_mode == BankingMode::RAM {
            self.ram_bank & (self.ram_bank_count - 1)
        } else {
            0
        }
    }

    fn get_bank_0(&self) -> usize {
        // In Mode 1 (RAM/Advanced), the upper 2 bits affect bank 0 for ROMs > 512KB
        if self.banking_mode == BankingMode::RAM {
            // Extract bits 5-6 from rom_bank and shift them to the correct position
            let upper_bits = (self.rom_bank & 0x60) >> 5;
            (upper_bits << 5) & (self.rom_bank_count - 1)
        } else {
            0
        }
    }
}

impl MBC for MBC1 {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                let bank = self.get_bank_0();
                /*if let Some(rom_bank) = self.rom_banks.data.get(bank) {
                    if let Some(&value) = rom_bank.get(address as usize) {
                        value
                    }
                    else {
                        error!("Error reading for ROM bank {} at address 0x{:X}", bank, address);
                        0xFF
                    }
                }
                else {
                    panic!("ROM bank {} not available", bank)
                }*/
                self.get_value_from_bank(bank, address, &self.rom_banks.data)
            },
            0x4000..=0x7FFF => {
                let bank_addr = address - 0x4000;

                /*if let Some(rom_bank) = self.rom_banks.data.get(self.get_selected_rom_bank()) {
                    if let Some(&value) = rom_bank.get(bank_addr) {
                        value
                    }
                    else {
                        error!("Error reading for ROM bank 1 at address 0x{:X}", address);
                        0xFF
                    }
                }
                else {
                    panic!("ROM bank 1 not available")
                }*/
                self.get_value_from_bank(self.get_selected_rom_bank(), bank_addr, &self.rom_banks.data)
            },
            0xA000..=0xBFFF => { // External RAM
                if self.is_ram_enabled() {
                    let bank = self.get_active_ram_bank();
                    let ram_address = bank * 0x2000 + (address - 0xA000) as usize;

                    if let Some(sram) = &self.sram {
                        sram.read(ram_address)
                    } else {
                        error!("SRAM is None when trying to read from it");
                        0xFF
                    }
                }
                else {
                    debug!("Attempt to read from ROM RAM when ram is not enabled or does not exist");
                    0xFF
                }
            },
            _ => {
                error!("Invalid MBC1 address for read: {:04X}", address);
                0xFF
            }
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => {
                self.ram_enabled = (value & 0x0F) == 0x0A;
            },
            0x2000..=0x3FFF => {
                let lower_bits = (value & 0x1F) as usize;
                let bank_num = if lower_bits == 0 { 1 } else { lower_bits };
                self.rom_bank = (self.rom_bank & 0x60) | bank_num;
            },
            0x4000..=0x5FFF => {
                let upper_bits = ((value & 0x03) as usize) << 5;

                // These bits always affect ROM bank bits 5-6
                self.rom_bank = (self.rom_bank & 0x1F) | upper_bits;

                // In RAM banking mode, they also select the RAM bank
                if self.banking_mode == BankingMode::RAM {
                    self.ram_bank = (value & 0x03) as usize;
                    debug!("RAM bank set to: {:02X}", self.ram_bank);
                }
            },
            0x6000..=0x7FFF => {
                self.banking_mode = if value & 0x01 == 0 {
                    BankingMode::ROM
                } else {
                    BankingMode::RAM
                };
                debug!("Banking mode set to: {:?}", self.banking_mode);
            },
            0xA000..=0xBFFF => {
                if self.is_ram_enabled() && self.has_ram {
                    let bank = self.get_active_ram_bank();
                    let addr = bank * 0x2000 + (address - 0xA000) as usize;

                    if let Some(sram) = &mut self.sram {
                        sram.write(addr, value);
                        
                        if self.has_battery {
                            debug!("Battery-backed MBC1 RAM write at bank {} addr {:04X} = {:02X}", 
                                  bank, address, value);
                        }
                    } else {
                        debug!("SRAM is None when trying to write to it");
                    }
                } else {
                    debug!("Attempted to write to disabled RAM: {:04X} = {:02X}", address, value);
                }
            },
            _ => {
                error!("Invalid MBC1 address for write: {:04X}", address);
            }
        }
    }

    fn get_rom_bank(&self) -> usize {
        self.get_selected_rom_bank()
    }

    fn get_ram_bank(&self) -> usize {
        self.get_active_ram_bank()
    }

    fn is_ram_enabled(&self) -> bool {
        self.ram_enabled && self.has_ram
    }

    fn save_ram(&mut self) -> Result<(), io::Error> {
        // Only save if this cartridge has battery-backed RAM
        if self.has_battery && self.has_ram {
            if let Some(sram) = &mut self.sram {
                debug!("MBC1 RAM saved successfully");
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