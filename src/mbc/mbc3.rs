use std::time::SystemTime;
use std::path::{Path, PathBuf};
use std::fs;
use std::io;

use log::{debug, info};

use crate::mbc::mbc::{
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

const MBC3_MAX_ROM_BANKS: usize = 128;
const RTC_REG_COUNT: usize = 5;
const RTC_SECONDS_INDEX: usize = 0;
const RTC_MINUTES_INDEX: usize = 1;
const RTC_HOURS_INDEX: usize = 2;
const RTC_DAYS_LOW: usize = 3;   // https://gbdev.io/pandocs/MBC3.html#the-day-counter
const RTC_DAYS_HIGH: usize = 4;  // https://gbdev.io/pandocs/MBC3.html#the-day-counter
const RTC_HALT_BIT: u8 = 0x40;   // https://gbdev.io/pandocs/MBC3.html#clock-counter-registers
const RTC_DAY_CARRY_BIT: u8 = 0x80; // https://gbdev.io/pandocs/MBC3.html#clock-counter-registers
const RAM_ENABLE_MASK: u8 = 0x0F;
const RAM_ENABLE_VALUE: u8 = 0x0A;
const ROM_BANK_MASK: u8 = 0x7F;
const BANK_SIZE: usize = 0x2000;
const RAM_BANK_SELECT_END: u16 = 0x5FFF;
const RTC_LATCH_START: u16 = 0x6000;
const RTC_REG_START: usize = 0x08;
const RTC_REG_END: usize = 0x0C;
const RTC_REG_SECONDS: usize = 0x08;
const RTC_REG_MINUTES: usize = 0x09;
const RTC_REG_HOURS: usize = 0x0A;
const RTC_REG_DAYS_LOW: usize = 0x0B;
const RTC_REG_DAYS_HIGH: usize = 0x0C;

pub struct RtcData {
    registers: [u8; RTC_REG_COUNT],
    latch_registers: [u8; RTC_REG_COUNT],
    base_time: SystemTime,
    latch_state: bool,
    dirty: bool,
    save_path: PathBuf,
}

impl RtcData {
    pub fn new(rom_path: &Path) -> Self {
        let mut rtc = RtcData {
            registers: [0; RTC_REG_COUNT],
            latch_registers: [0; RTC_REG_COUNT],
            base_time: SystemTime::now(),
            latch_state: false,
            dirty: false,
            save_path: rom_path.with_extension("rtc"),
        };

        // Try to load existing RTC data
        rtc.load().unwrap_or_else(|e| {
            info!("Could not load RTC data: {}", e);
        });

        rtc
    }

    /// Load RTC data from a saved file
    pub fn load(&mut self) -> Result<(), io::Error> {
        if self.save_path.exists() {
            let data = fs::read(&self.save_path)?;

            if data.len() != RTC_REG_COUNT {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid RTC file size"));
            }

            // Load the registers
            for i in 0..RTC_REG_COUNT {
                self.registers[i] = data[i];
            }

            // Copy to latch registers
            self.latch_registers.copy_from_slice(&self.registers);

            // Reset base time to now
            self.base_time = SystemTime::now();
            self.dirty = false;
        }

        Ok(())
    }

    /// Save RTC data to file
    pub fn save(&mut self) -> Result<(), io::Error> {
        if self.dirty {
            self.update();
            debug!("Saved RTC data to {:?}", self.save_path);
            self.dirty = false;
            return fs::write(&self.save_path, &self.registers);
        }
        Ok(())
    }

    /// Update the RTC registers based on elapsed time
    pub fn update(&mut self) {
        // Only update if RTC is not halted
        if (self.registers[RTC_DAYS_HIGH] & RTC_HALT_BIT) == 0 {
            // Calculate elapsed seconds since last update
            if let Ok(elapsed) = SystemTime::now().duration_since(self.base_time) {
                let seconds = elapsed.as_secs();
                if seconds > 0 {
                    // Reset base time
                    self.base_time = SystemTime::now();

                    // Extract current RTC values
                    let mut secs = self.registers[RTC_SECONDS_INDEX] as u64;
                    let mut mins = self.registers[RTC_MINUTES_INDEX] as u64;
                    let mut hours = self.registers[RTC_HOURS_INDEX] as u64;
                    let mut days = ((self.registers[RTC_DAYS_HIGH] & 0x01) as u64) << 8 |
                        self.registers[RTC_DAYS_LOW] as u64;

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
                        self.registers[RTC_DAYS_HIGH] |= RTC_DAY_CARRY_BIT;
                        days %= 512;
                    }

                    // Update registers
                    self.registers[RTC_SECONDS_INDEX] = secs as u8;
                    self.registers[RTC_MINUTES_INDEX] = mins as u8;
                    self.registers[RTC_HOURS_INDEX] = hours as u8;
                    self.registers[RTC_DAYS_LOW] = (days & 0xFF) as u8;
                    self.registers[RTC_DAYS_HIGH] = (self.registers[RTC_DAYS_HIGH] & 0xFE) |
                        (((days >> 8) & 0x01) as u8);

                    self.dirty = true;
                }
            }
        }
    }

    /// Latch RTC values for reading
    pub fn latch(&mut self) {
        self.update();
        self.latch_registers.copy_from_slice(&self.registers);
    }

    /// Get a latched RTC register value
    pub fn read_latched(&self, reg_index: usize) -> u8 {
        if reg_index < RTC_REG_COUNT {
            self.latch_registers[reg_index]
        } else {
            0xFF
        }
    }

    /// Write to an RTC register
    pub fn write_register(&mut self, reg_index: usize, value: u8) {
        if reg_index < RTC_REG_COUNT {
            // First, ensure RTC is up-to-date
            self.update();

            // Apply value to the register with validation
            match reg_index {
                RTC_SECONDS_INDEX => self.registers[reg_index] = value & 0x3F,  // 0-59
                RTC_MINUTES_INDEX => self.registers[reg_index] = value & 0x3F,  // 0-59
                RTC_HOURS_INDEX => self.registers[reg_index] = value & 0x1F,    // 0-23
                RTC_DAYS_LOW => self.registers[reg_index] = value,        // 0-255
                RTC_DAYS_HIGH => {
                    // Preserve day carry flag if set
                    let day_carry = self.registers[reg_index] & RTC_DAY_CARRY_BIT;
                    self.registers[reg_index] = (value & 0xC1) | day_carry;

                    // If halt bit changes, reset base time
                    if (value & RTC_HALT_BIT) != (self.registers[reg_index] & RTC_HALT_BIT) {
                        self.base_time = SystemTime::now();
                    }
                }
                _ => {} // Should never happen
            }

            // Update latched registers too if RTC is halted
            if (self.registers[RTC_DAYS_HIGH] & RTC_HALT_BIT) != 0 {
                self.latch_registers[reg_index] = self.registers[reg_index];
            }

            self.dirty = true;
        }
    }

    /// Update latch state and latch RTC data if needed
    pub fn update_latch_state(&mut self, value: u8) {
        let new_latch = value & 0x01 != 0;

        if self.latch_state && !new_latch {
            // Latch the RTC data when transitioning from 1 to 0
            self.latch();
        }

        self.latch_state = new_latch;
    }

}

pub struct MBC3 {
    rom_banks: ROMBanks,
    sram: Option<SRAM>,
    rtc: Option<RtcData>,
    rom_bank: usize,
    ram_bank: usize,
    ram_enabled: bool,
    has_ram: bool,
    has_battery: bool,
    has_rtc: bool,
    rom_bank_count: usize,
}

impl MBC3 {
    pub fn new(rom_banks: ROMBanks, ram_size: u8, has_battery: bool, has_rtc: bool, rom_path: &Path) -> Self {
        let ram_size_bytes = get_ram_size_in_bytes(ram_size);
        let has_ram = ram_size > 0;
        let rom_bank_count = std::cmp::min(MBC3_MAX_ROM_BANKS, rom_banks.data.len());

        // Create SRAM only if there is RAM
        let sram = if has_ram {
            Some(SRAM::new(ram_size_bytes, rom_path))
        } else {
            None
        };

        // Create RTC only if needed
        let rtc = if has_rtc {
            Some(RtcData::new(rom_path))
        } else {
            None
        };

        MBC3 {
            rom_banks,
            sram,
            rtc,
            rom_bank: 1,  // Default to bank 1
            ram_bank: 0,
            ram_enabled: false,
            has_ram,
            has_battery,
            has_rtc,
            rom_bank_count,
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
        self.has_rtc && bank >= RTC_REG_START && bank <= RTC_REG_END
    }
    
    fn ram_bank_to_rtc_register(&self, bank: &usize) -> usize {
        match bank {
            &RTC_REG_SECONDS => RTC_SECONDS_INDEX,
            &RTC_REG_MINUTES => RTC_MINUTES_INDEX,
            &RTC_REG_HOURS => RTC_HOURS_INDEX,
            &RTC_REG_DAYS_LOW => RTC_DAYS_LOW,
            &RTC_REG_DAYS_HIGH => RTC_DAYS_HIGH,
            _ => 0 // Should never happen if is_rtc_register check is done first
        }
    }

}

impl MBC for MBC3 {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            ROM_BANK0_START..=ROM_BANK0_END => {
                self.get_value_from_bank(0, address, &self.rom_banks.data)
            },
            ROM_BANK_N_START..=ROM_BANK_N_END => {
                // Switchable ROM bank
                let bank = self.get_active_rom_bank();
                let bank_addr = address - ROM_BANK_N_START;
                self.get_value_from_bank(bank, bank_addr, &self.rom_banks.data)
            },
            RAM_START..=RAM_END => {
                if self.is_ram_enabled() {
                    let bank = self.get_ram_bank();
                    if self.is_rtc_register(bank) {
                        // Read from RTC register
                        if let Some(rtc) = &self.rtc {
                            let rtc_reg = self.ram_bank_to_rtc_register(&bank);
                            rtc.read_latched(rtc_reg)
                        } else {
                            info!("Attempted to read from RTC register but RTC is not available");
                            INVALID_READ_VALUE
                        }
                    } else if self.has_ram {
                        // Read from RAM if it exists
                        if let Some(sram) = &self.sram {
                            let ram_addr = self.get_ram_bank() * BANK_SIZE + (address - RAM_START) as usize;
                            sram.read(ram_addr)
                        } else {
                            info!("Attempted to read from RAM but RAM is not available");
                            INVALID_READ_VALUE
                        }
                    } else {
                        info!("Attempted to read from RAM/RTC but neither is available");
                        INVALID_READ_VALUE
                    }
                } else {
                    // RAM/RTC disabled, return 0xFF
                    INVALID_READ_VALUE
                }
            },
            _ => {
                info!("Invalid MBC3 address for read: {:04X}", address);
                INVALID_READ_VALUE
            }
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            ROM_BANK0_START..=0x1FFF => {
                // RAM and Timer Enable (0x0A enables, anything else disables)
                self.ram_enabled = (value & RAM_ENABLE_MASK) == RAM_ENABLE_VALUE;
            },
            ROM_BANK_SELECT_START..=ROM_BANK0_END => {
                // ROM Bank Number (7 bits)
                let bank_num = (value & ROM_BANK_MASK) as usize;
                // For MBC3, bank 0 can be selected (unlike MBC1)
                self.rom_bank = bank_num;
            },
            ROM_BANK_N_START..=RAM_BANK_SELECT_END => {
                // RAM Bank Number or RTC Register Select
                self.ram_bank = value as usize;
            },
            RTC_LATCH_START..=ROM_BANK_N_END => {
                // Latch Clock Data
                // When write changes from non-zero to zero, latch the RTC data
                if let Some(rtc) = &mut self.rtc {
                    rtc.update_latch_state(value);
                }
            },
            RAM_START..=RAM_END => {
                if self.is_ram_enabled() {
                    // Extract ram_bank first to avoid borrowing issues
                    let bank = self.get_ram_bank();
                    if self.is_rtc_register(bank) {
                        // Write to RTC register
                        let rtc_reg = self.ram_bank_to_rtc_register(&bank);
                        if let Some(rtc) = &mut self.rtc {
                            rtc.write_register(rtc_reg, value);
                        }
                    } else if self.has_ram {
                        // Write to RAM
                        if let Some(sram) = &mut self.sram {
                            let ram_addr = bank * BANK_SIZE + (address - RAM_START) as usize;
                            sram.write(ram_addr, value);
                        }
                    }
                }
            },
            _ => {
                info!("Invalid MBC3 address for write: {:04X} = {:02X}", address, value);
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

    fn save_ram(&mut self) -> Result<(), io::Error> {
        // Save RAM if it exists and has battery
        if self.has_battery {
            if let Some(sram) = &mut self.sram {
                match sram.save() {
                    Ok(_) => {},
                    Err(e) => {
                        info!("Error saving SRAM: {}", e);
                        return Err(e);
                    }
                }
            }
        }

        // Save RTC if it exists
        if self.has_rtc {
            if let Some(rtc) = &mut self.rtc {
                match rtc.save() {
                    Ok(_) => {},
                    Err(e) => {
                        info!("Error saving RTC data: {}", e);
                        return Err(e);
                    }
                }
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