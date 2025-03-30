use log::{debug, error, info};
use crate::mbc::{MBC, get_ram_size_in_bytes, get_ram_banks, get_rom_banks};
use crate::rom::ROMBanks;

/*
    https://gbdev.io/pandocs/MBC2.html
*/

const MBC2_RAM_SIZE: usize = 512; // 512 x 4 bits
const MBC2_RAM_ADDR_MASK: u16 = 0x01FF; // Only 9 bits are used for addressing

pub struct MBC2 {
    rom_banks: ROMBanks,
    ram: [u8; MBC2_RAM_SIZE],
    rom_bank: usize,
    ram_enabled: bool,
    has_battery: bool,
    rom_bank_count: usize,
}
impl MBC2 {
    pub fn new(rom_banks: ROMBanks, has_battery: bool) -> Self {
        // MBC2 can have at most 16 banks (256KB)
        let rom_bank_count = std::cmp::min(16, rom_banks.data.len());

        MBC2 {
            rom_banks,
            ram: [0; MBC2_RAM_SIZE],
            rom_bank: 1,  // Default to bank 1
            ram_enabled: false,
            has_battery,
            rom_bank_count,
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
                    // Fixed ROM bank 0
                    if let Some(bank) = self.rom_banks.data.get(0) {
                        if let Some(&value) = bank.get(address as usize) {
                            value
                        } else {
                            error!("Attempted to read beyond ROM bank 0 boundaries at {:04X}", address);
                            0xFF
                        }
                    } else {
                        error!("ROM bank 0 not available");
                        0xFF
                    }
                },
                0x4000..=0x7FFF => {
                    // Switchable ROM bank
                    let bank = self.get_selected_rom_bank();
                    let bank_addr = (address - 0x4000) as usize;

                    if let Some(bank_data) = self.rom_banks.data.get(bank) {
                        if let Some(&value) = bank_data.get(bank_addr) {
                            value
                        } else {
                            error!("Attempted to read beyond ROM bank boundaries at bank {} addr {:04X}", bank, address);
                            0xFF
                        }
                    } else {
                        error!("ROM bank {} not available (total banks: {})", bank, self.rom_banks.data.len());
                        0xFF
                    }
                },
                0xA000..=0xA1FF => {
                    // MBC2 has 512x4 bits of RAM built into the chip
                    if self.ram_enabled {
                        // Mask address to the valid RAM range and clear upper 4 bits as per spec
                        let ram_addr = (address & MBC2_RAM_ADDR_MASK) as usize;
                        if ram_addr < MBC2_RAM_SIZE {
                            // Return the RAM value with upper 4 bits masked to 0
                            self.ram[ram_addr] & 0x0F
                        } else {
                            debug!("Attempted to read from non-existent MBC2 RAM at {:04X}", address);
                            0xFF
                        }
                    } else {
                        // RAM disabled, return 0xFF
                        0xFF
                    }
                },
                0xA200..=0xBFFF => {
                    // Mirror of A000-A1FF, repeating every 512 bytes
                    if self.ram_enabled {
                        // Get the base address by applying the mask
                        let ram_addr = (address & MBC2_RAM_ADDR_MASK) as usize;
                        if ram_addr < MBC2_RAM_SIZE {
                            // Return the RAM value with upper 4 bits masked to 0
                            self.ram[ram_addr] & 0x0F
                        } else {
                            debug!("Attempted to read from non-existent MBC2 RAM at {:04X}", address);
                            0xFF
                        }
                    } else {
                        // RAM disabled, return 0xFF
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
                    // For MBC2, the 8th bit of the address determines the command:
                    // - If bit 8 is clear (address & 0x0100 == 0), it's a RAM enable command
                    // - If bit 8 is set (address & 0x0100 != 0), it's a ROM bank select command

                    if (address & 0x0100) == 0 {
                        // RAM Enable (0x0A enables, anything else disables)
                        self.ram_enabled = (value & 0x0F) == 0x0A;
                        debug!("MBC2 RAM enable set to: {}", self.ram_enabled);
                    } else {
                        // ROM Bank Select (lower 4 bits)
                        // If 0 is written, it's treated as 1
                        let bank_num = (value & 0x0F) as usize;
                        self.rom_bank = if bank_num == 0 { 1 } else { bank_num };
                        debug!("MBC2 ROM bank set to: {:02X}", self.rom_bank);
                    }
                },
                0x4000..=0x7FFF => {
                    // Ignored in MBC2
                    debug!("Write to ROM area 4000-7FFF ignored in MBC2: {:04X} = {:02X}", address, value);
                },
                0xA000..=0xA1FF => {
                    // Direct access to the built-in RAM
                    if self.ram_enabled {
                        let ram_addr = (address & MBC2_RAM_ADDR_MASK) as usize;
                        if ram_addr < MBC2_RAM_SIZE {
                            // Only lower 4 bits can be stored, upper bits are ignored
                            self.ram[ram_addr] = value & 0x0F;

                            // If this is battery-backed RAM, mark it for saving
                            if self.has_battery {
                                debug!("Battery-backed MBC2 RAM write at {:04X} = {:02X}", address, value & 0x0F);
                            }
                        } else {
                            debug!("Attempted to write to non-existent MBC2 RAM at {:04X}", address);
                        }
                    } else {
                        debug!("Attempted to write to disabled MBC2 RAM: {:04X} = {:02X}", address, value);
                    }
                },
                0xA200..=0xBFFF => {
                    // Mirror of A000-A1FF, repeating every 512 bytes
                    if self.ram_enabled {
                        let ram_addr = (address & MBC2_RAM_ADDR_MASK) as usize;
                        if ram_addr < MBC2_RAM_SIZE {
                            // Only lower 4 bits can be stored, upper bits are ignored
                            self.ram[ram_addr] = value & 0x0F;

                            // If this is battery-backed RAM, mark it for saving
                            if self.has_battery {
                                debug!("Battery-backed MBC2 RAM write at {:04X} = {:02X}", address, value & 0x0F);
                            }
                        } else {
                            debug!("Attempted to write to non-existent MBC2 RAM at {:04X}", address);
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
        0
    } // MBC2 doesn't use ram banking

    fn is_ram_enabled(&self) -> bool {
        self.ram_enabled
    }
}