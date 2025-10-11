use std::io;
use log::{debug, error};
use crate::mbc::{MBC, get_ram_size_in_bytes};
use crate::rom::ROMBanks;

pub struct MBC0 {
    rom_banks: ROMBanks,
    ram: Vec<u8>,
    ram_enabled: bool,
    has_ram: bool,
    has_battery: bool,
}

impl MBC0 {
    pub fn new(rom_banks: ROMBanks, ram_size: u8, has_battery: bool) -> MBC0 {
        let ram_size_bytes = get_ram_size_in_bytes(ram_size);
        debug!("ram_size_bytes {}", ram_size_bytes);
        let has_ram = ram_size > 0;
        MBC0 {
            rom_banks,
            ram: vec![0; ram_size_bytes],
            ram_enabled: false,
            has_ram,
            has_battery,
        }
    }
}

impl MBC for MBC0 {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => { // ROM bank 0
                self.get_value_from_bank(0, address, &self.rom_banks.data)
            },
            0x4000..=0x7FFF => { // ROM bank 1–N (in the case of MBC0 this is always 1)
                let bank_addr = address - 0x4000;
                self.get_value_from_bank(1, bank_addr, &self.rom_banks.data)
            },
            0xA000..=0xBFFF => { // External RAM
                let implicit_ram_enabled = if self.has_battery {
                    true
                }
                else {
                    self.is_ram_enabled()
                };

                if implicit_ram_enabled && self.has_ram {
                    let ram_address = (address - 0xA000) as usize;
                    if ram_address < self.ram.len() {
                        self.ram[ram_address]
                    }
                    else {
                        error!("Error reading for ram with capacity of {} for address 0x{:X}", self.ram.len(), address);
                        0xFF
                    }
                }
                else {
                    error!("Attempt to read from ROM RAM when ram is not enabled or does not exist");
                    0xFF
                }
            },
            _ => {
                error!("Invalid MBC0 address for read: {:04X}", address);
                0xFF
            }
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x7FFF => {
                if address <= 0x1FFF && (value & 0x0F) == 0x0A {
                    self.ram_enabled = true;
                } else {
                    error!("Attempted write to ROM area: {:04X} = {:02X}", address, value);
                }
            },
            0xA000..=0xBFFF => {
                let implicit_ram_enabled = if self.has_battery {
                    true
                }
                else {
                    self.is_ram_enabled()
                };
                if implicit_ram_enabled && self.has_ram {
                    let ram_addr = (address - 0xA000) as usize;
                    if ram_addr < self.ram.len() {
                        self.ram[ram_addr] = value;

                        if self.has_battery {
                            error!("battery-backed RAM write not implemented");
                        }
                    }
                }
                else {
                    error!("Attempted write to RAM area: {:04X} value 0x{:02X}", address, value);
                }
            },
            _ => {
                error!("Invalid MBC0 address for write: {:04X}", address);
            }
        }
    }

    fn get_rom_bank(&self) -> usize {
        0
    }

    fn get_ram_bank(&self) -> usize {
        1
    }

    fn is_ram_enabled(&self) -> bool {
        self.ram_enabled && self.has_ram
    }

    fn save_ram(&mut self) -> Result<(), io::Error> {
        Ok(())
    }
    fn dirty_sram(&self) -> bool {
        false
    }

}