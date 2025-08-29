use std::io;
use log::{debug, error, info};
use crate::mbc::{MBC, get_ram_size_in_bytes, get_ram_banks};
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
        info!("ram_size_bytes {}", ram_size_bytes);

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
        debug!("read_byte called for address {}", address);
        match address {
            0x0000..=0x3FFF => { // ROM bank 0
                if let Some(rom_bank) = self.rom_banks.data.get(0) {
                    if let Some(&value) = rom_bank.get(address as usize) {
                        value
                    }
                    else {
                        error!("Error reading for ROM bank 0 at address 0x{:X}", address);
                        0xFF
                    }
                }
                else {
                    panic!("ROM bank 0 not available")
                    //0xFF
                }
            },
            0x4000..=0x7FFF => { // ROM bank 1–N (in the case of MBC0 this is always 1)
                let bank_addr = (address - 0x4000) as usize;
                if let Some(rom_bank) = self.rom_banks.data.get(1) {
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
                    //0xFF
                }
            },
            0xA000..=0xBFFF => { // External RAM
                let implicit_ram_enabled = if self.has_battery {
                    true
                }
                else {
                    self.ram_enabled
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
        info!("write_byte called for address 0x{:X} with value 0x{:02X}", address, value);
        match address {
            0x0000..=0x7FFF => {
                // ROM writes might be attempting to toggle RAM, try to detect this
                if address <= 0x1FFF && (value & 0x0F) == 0x0A {
                    self.ram_enabled = true;
                    info!("RAM enabled via write to ROM area");
                } else {
                    // ROM writes are ignored in MBC0
                    error!("Attempted write to ROM area: {:04X} = {:02X}", address, value);
                }
            },
            0xA000..=0xBFFF => { // Optionally up to 8 KiB of RAM could be connected at $A000-BFFF, using a discrete logic decoder in place of a full MBC chip
                let implicit_ram_enabled = if self.has_battery {
                    true
                }
                else {
                    self.ram_enabled
                };

                if implicit_ram_enabled && self.has_ram {
                    let ram_addr = (address - 0xA000) as usize;
                    if ram_addr < self.ram.len() {
                        self.ram[ram_addr] = value;

                        if self.has_battery {
                            error!("battery-backed RAM write not implemented");
                            info!("Battery-backed RAM write at {:04X} = value 0x{:02X}", address, value);
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
        0 // Constant for MBC0
    }

    fn get_ram_bank(&self) -> usize {
        1 // Constant for MBC0
    }

    fn is_ram_enabled(&self) -> bool {
        self.ram_enabled && self.has_ram
    }

    fn save_ram(&mut self) -> Result<(), io::Error> {
        //unimplemented!("Saving is not implemented for MBC0!")
        Ok(())
    }
    fn dirty_sram(&self) -> bool {
        false
    }

}