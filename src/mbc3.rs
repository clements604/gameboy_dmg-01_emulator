use log::{debug, error, info};
use crate::mbc::{MBC, get_ram_size_in_bytes, get_ram_banks};
use crate::rom::ROMBanks;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

const MBC3_MAX_ROM_BANKS: usize = 128; // 2MB
const RTC_REG_COUNT: usize = 5;

// RTC Register indices
const RTC_SECONDS: usize = 0;    // 0-59
const RTC_MINUTES: usize = 1;    // 0-59
const RTC_HOURS: usize = 2;      // 0-23
const RTC_DAYS_LOW: usize = 3;   // Lower 8 bits of day counter (0-255)
const RTC_DAYS_HIGH: usize = 4;  // Upper 1 bit of day counter + flags
const RTC_HALT_BIT: u8 = 0x40;   // Bit 6, RTC halt flag
const RTC_DAY_CARRY_BIT: u8 = 0x80; // Bit 7, day counter carry flag

pub struct MBC3 {
    rom_banks: ROMBanks,
    ram: Vec<u8>,
    rtc_registers: [u8; RTC_REG_COUNT],
    rtc_latch_registers: [u8; RTC_REG_COUNT],
    rtc_latch_state: bool,
    rtc_base_time: SystemTime,

    rom_bank: usize,
    ram_bank: usize,
    ram_enabled: bool,
    has_ram: bool,
    has_battery: bool,
    has_rtc: bool,
    rom_bank_count: usize,
    ram_bank_count: usize,
}

impl MBC3 {
    pub fn new(rom_banks: ROMBanks, ram_size: u8, has_battery: bool, has_rtc: bool) -> Self {
        let ram_size_bytes = get_ram_size_in_bytes(ram_size);
        let has_ram = ram_size > 0;
        let rom_bank_count = std::cmp::min(MBC3_MAX_ROM_BANKS, rom_banks.data.len());
        let ram_bank_count = get_ram_banks(ram_size);

        MBC3 {
            rom_banks,
            ram: vec![0; ram_size_bytes],
            rtc_registers: [0; RTC_REG_COUNT],
            rtc_latch_registers: [0; RTC_REG_COUNT],
            rtc_latch_state: false,
            rtc_base_time: SystemTime::now(),

            rom_bank: 1,  // Default to bank 1
            ram_bank: 0,
            ram_enabled: false,
            has_ram,
            has_battery,
            has_rtc,
            rom_bank_count,
            ram_bank_count,
        }
    }

    // Helper function to get the effective ROM bank number
    fn get_active_rom_bank(&self) -> usize {
        // In MBC3, ROM bank 0 can be mapped to 0x4000-0x7FFF by selecting bank 0
        // But this is generally avoided in commercial games
        let bank = if self.rom_bank == 0 { 1 } else { self.rom_bank };

        // Mask to the number of banks we actually have
        bank & (self.rom_bank_count - 1)
    }

    // Helper function to determine if we are accessing RAM or RTC registers
    fn is_rtc_register(&self, bank: usize) -> bool {
        self.has_rtc && bank >= 0x08 && bank <= 0x0C
    }

    // Maps RAM bank index to RTC register index
    fn ram_bank_to_rtc_register(&self, bank: usize) -> usize {
        match bank {
            0x08 => RTC_SECONDS,
            0x09 => RTC_MINUTES,
            0x0A => RTC_HOURS,
            0x0B => RTC_DAYS_LOW,
            0x0C => RTC_DAYS_HIGH,
            _ => 0 // Should never happen if is_rtc_register check is done first
        }
    }

    // Update the RTC registers based on elapsed time
    fn update_rtc(&mut self) {
        // Only update if RTC is not halted
        if self.has_rtc && (self.rtc_registers[RTC_DAYS_HIGH] & RTC_HALT_BIT) == 0 {
            // Calculate elapsed seconds since last update
            if let Ok(elapsed) = SystemTime::now().duration_since(self.rtc_base_time) {
                let seconds = elapsed.as_secs();
                if seconds > 0 {
                    // Reset base time
                    self.rtc_base_time = SystemTime::now();

                    // Extract current RTC values
                    let mut secs = self.rtc_registers[RTC_SECONDS] as u64;
                    let mut mins = self.rtc_registers[RTC_MINUTES] as u64;
                    let mut hours = self.rtc_registers[RTC_HOURS] as u64;
                    let mut days = ((self.rtc_registers[RTC_DAYS_HIGH] & 0x01) as u64) << 8 |
                        self.rtc_registers[RTC_DAYS_LOW] as u64;

                    // Add elapsed seconds
                    secs += seconds;

                    // Propagate carries
                    if secs >= 60 {
                        mins += secs / 60;
                        secs %= 60;
                    }

                    if mins >= 60 {
                        hours += mins / 60;
                        mins %= 60;
                    }

                    if hours >= 24 {
                        days += hours / 24;
                        hours %= 24;
                    }

                    // Check for day counter overflow (over 511 days)
                    if days > 511 {
                        // Set day counter carry flag
                        self.rtc_registers[RTC_DAYS_HIGH] |= RTC_DAY_CARRY_BIT;
                        days %= 512;
                    }

                    // Update registers
                    self.rtc_registers[RTC_SECONDS] = secs as u8;
                    self.rtc_registers[RTC_MINUTES] = mins as u8;
                    self.rtc_registers[RTC_HOURS] = hours as u8;
                    self.rtc_registers[RTC_DAYS_LOW] = (days & 0xFF) as u8;
                    self.rtc_registers[RTC_DAYS_HIGH] = (self.rtc_registers[RTC_DAYS_HIGH] & 0xFE) |
                        (((days >> 8) & 0x01) as u8);
                }
            }
        }
    }

    // Latch RTC values for reading
    fn latch_rtc(&mut self) {
        self.update_rtc();
        self.rtc_latch_registers.copy_from_slice(&self.rtc_registers);
    }
}

impl MBC for MBC3 {
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
                    panic!("ROM bank 0 not available");
                    //0xFF
                }
            },
            0x4000..=0x7FFF => {
                // Switchable ROM bank
                let bank = self.get_active_rom_bank();
                let bank_addr = (address - 0x4000) as usize;

                if let Some(bank_data) = self.rom_banks.data.get(bank) {
                    if let Some(&value) = bank_data.get(bank_addr) {
                        value
                    } else {
                        error!("Attempted to read beyond ROM bank boundaries at bank {} addr {:04X}", bank, address);
                        0xFF
                    }
                } else {
                    panic!("ROM bank {} not available (total banks: {})", bank, self.rom_banks.data.len());
                    //0xFF
                }
            },
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    if self.is_rtc_register(self.ram_bank) {
                        // Read from RTC register
                        let rtc_reg = self.ram_bank_to_rtc_register(self.ram_bank);
                        self.rtc_latch_registers[rtc_reg]
                    } else if self.has_ram && self.ram_bank < self.ram_bank_count {
                        // Read from RAM
                        let ram_addr = self.ram_bank as usize * 0x2000 + (address - 0xA000) as usize;
                        if ram_addr < self.ram.len() {
                            self.ram[ram_addr]
                        } else {
                            error!("Attempted to read from non-existent RAM at bank {} addr {:04X}", self.ram_bank, address);
                            0xFF
                        }
                    } else {
                        error!("Attempted to read from invalid RAM/RTC bank: {}", self.ram_bank);
                        0xFF
                    }
                } else {
                    // RAM/RTC disabled, return 0xFF
                    0xFF
                }
            },
            _ => {
                error!("Invalid MBC3 address for read: {:04X}", address);
                0xFF
            }
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => {
                // RAM and Timer Enable (0x0A enables, anything else disables)
                self.ram_enabled = (value & 0x0F) == 0x0A;
                debug!("MBC3 RAM/Timer enable set to: {}", self.ram_enabled);
            },
            0x2000..=0x3FFF => {
                // ROM Bank Number (7 bits)
                let bank_num = (value & 0x7F) as usize;
                // For MBC3, bank 0 can be selected (unlike MBC1)
                self.rom_bank = bank_num;
                debug!("MBC3 ROM bank set to: {:02X}", self.rom_bank);
            },
            0x4000..=0x5FFF => {
                // RAM Bank Number or RTC Register Select
                self.ram_bank = value as usize;
                if self.is_rtc_register(self.ram_bank) {
                    debug!("MBC3 RTC register selected: {:02X}", self.ram_bank);
                } else {
                    debug!("MBC3 RAM bank selected: {:02X}", self.ram_bank);
                }
            },
            0x6000..=0x7FFF => {
                // Latch Clock Data
                // When write changes from non-zero to zero, latch the RTC data
                let old_latch = self.rtc_latch_state;
                let new_latch = value & 0x01 != 0;

                if old_latch && !new_latch {
                    // Latch the RTC data
                    debug!("MBC3 latching RTC data");
                    let mut mbc = unsafe { &mut *(self as *const MBC3 as *mut MBC3) };
                    mbc.latch_rtc();
                }

                self.rtc_latch_state = new_latch;
            },
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    if self.is_rtc_register(self.ram_bank) {
                        // Write to RTC register
                        let rtc_reg = self.ram_bank_to_rtc_register(self.ram_bank);

                        // Special handling for RTC registers
                        let mut mbc = unsafe { &mut *(self as *const MBC3 as *mut MBC3) };

                        // First, ensure RTC is up-to-date
                        mbc.update_rtc();

                        // Apply value to the actual register with validation
                        match rtc_reg {
                            RTC_SECONDS => mbc.rtc_registers[rtc_reg] = value & 0x3F,  // 0-59
                            RTC_MINUTES => mbc.rtc_registers[rtc_reg] = value & 0x3F,  // 0-59
                            RTC_HOURS => mbc.rtc_registers[rtc_reg] = value & 0x1F,    // 0-23
                            RTC_DAYS_LOW => mbc.rtc_registers[rtc_reg] = value,        // 0-255
                            RTC_DAYS_HIGH => {
                                // Preserve day carry flag if set
                                let day_carry = mbc.rtc_registers[rtc_reg] & RTC_DAY_CARRY_BIT;
                                mbc.rtc_registers[rtc_reg] = (value & 0xC1) | day_carry;

                                // If halt bit changes, reset base time
                                if (value & RTC_HALT_BIT) != (mbc.rtc_registers[rtc_reg] & RTC_HALT_BIT) {
                                    mbc.rtc_base_time = SystemTime::now();
                                }
                            }
                            _ => {} // Should never happen
                        }

                        // Update latched registers too if RTC is halted
                        if (mbc.rtc_registers[RTC_DAYS_HIGH] & RTC_HALT_BIT) != 0 {
                            mbc.rtc_latch_registers[rtc_reg] = mbc.rtc_registers[rtc_reg];
                        }

                        debug!("MBC3 RTC register {:02X} write: {:02X}", self.ram_bank, value);
                    } else if self.has_ram && self.ram_bank < self.ram_bank_count {
                        // Write to RAM
                        let ram_addr = self.ram_bank as usize * 0x2000 + (address - 0xA000) as usize;
                        if ram_addr < self.ram.len() {
                            self.ram[ram_addr] = value;

                            // If this is battery-backed RAM, mark it for saving
                            if self.has_battery {
                                debug!("Battery-backed MBC3 RAM write at bank {} addr {:04X} = {:02X}", 
                                      self.ram_bank, address, value);
                            }
                        } else {
                            debug!("Attempted to write to non-existent RAM at bank {} addr {:04X}", self.ram_bank, address);
                        }
                    } else {
                        debug!("Attempted to write to invalid RAM/RTC bank: {}", self.ram_bank);
                    }
                } else {
                    debug!("Attempted to write to disabled RAM/RTC: {:04X} = {:02X}", address, value);
                }
            },
            _ => {
                error!("Invalid MBC3 address for write: {:04X} = {:02X}", address, value);
            }
        }
    }

    fn get_rom_bank(&self) -> usize {
        self.get_active_rom_bank()
    }

    fn get_ram_bank(&self) -> usize {
        if self.is_rtc_register(self.ram_bank) {
            // When RTC is selected, indicate with a special value
            0xFF
        } else {
            self.ram_bank
        }
    }

    fn is_ram_enabled(&self) -> bool {
        self.ram_enabled
    }
}
