use std::path::Path;
use log::debug;
use crate::mbc::{MBC, MBCType};
use crate::mbc0::MBC0;
use crate::mbc1::MBC1;
use crate::mbc2::MBC2;
use crate::mbc3::MBC3;
use crate::mbc5::MBC5;
use crate::rom::ROM;

const CART_TYPE_ROM_RAM_BATT: u8 = 0x03;
const CART_TYPE_MBC1_RAM_BATT: u8 = 0x06;
const CART_TYPE_MMM01_RAM_BATT: u8 = 0x09;
const CART_TYPE_MBC3_RAM_BATT: u8 = 0x0D;
const CART_TYPE_MBC3_TIMER_BATT: u8 = 0x0F;
const CART_TYPE_MBC3_TIMER_RAM_BATT: u8 = 0x10;
const CART_TYPE_MBC4_RAM_BATT: u8 = 0x13;
const CART_TYPE_MBC5_RAM_BATT: u8 = 0x1B;
const CART_TYPE_MBC5_RUMBLE_RAM_BATT: u8 = 0x1E;
const CART_TYPE_HUC1_RAM_BATT: u8 = 0xFF;
const CART_TYPE_MBC5_RUMBLE: u8 = 0x1C;
const CART_TYPE_MBC5_RUMBLE_RAM: u8 = 0x1D;

pub fn create_mbc(rom: &ROM) -> Box<dyn MBC> {
    // Determine MBC type from cartridge type
    let mbc_type = MBCType::from_byte(rom.cartridge_type);

    // Check if the cartridge has battery-backed RAM
    let has_battery = match rom.cartridge_type {
        CART_TYPE_ROM_RAM_BATT | CART_TYPE_MBC1_RAM_BATT | CART_TYPE_MMM01_RAM_BATT |
        CART_TYPE_MBC3_RAM_BATT | CART_TYPE_MBC3_TIMER_BATT | CART_TYPE_MBC3_TIMER_RAM_BATT |
        CART_TYPE_MBC4_RAM_BATT | CART_TYPE_MBC5_RAM_BATT | CART_TYPE_MBC5_RUMBLE_RAM_BATT |
        CART_TYPE_HUC1_RAM_BATT => true,
        _ => false
    };

    // Check if the cartridge has RTC (Real Time Clock)
    let has_rtc = match rom.cartridge_type {
        CART_TYPE_MBC3_TIMER_BATT | CART_TYPE_MBC3_TIMER_RAM_BATT => true,
        _ => false
    };

    let has_rumble = match rom.cartridge_type {
        CART_TYPE_MBC5_RUMBLE | CART_TYPE_MBC5_RUMBLE_RAM | CART_TYPE_MBC5_RUMBLE_RAM_BATT => true,
        _ => false
    };
    
    // Create ROM banks from the ROM
    let rom_banks = rom.load_rom_to_banks();

    match mbc_type {
        MBCType::None => {
            debug!("Creating MBC0 (No MBC) controller");
            Box::new(MBC0::new(rom_banks, rom.ram_size, has_battery))
        },
        MBCType::MBC1 => {
            debug!("Creating MBC1 controller");
            Box::new(MBC1::new(rom_banks, rom.rom_size, rom.ram_size, has_battery, Path::new(&rom.rom_file_path)))
        },
        MBCType::MBC2 => {
            debug!("Creating MBC2 controller");
            Box::new(MBC2::new(rom_banks, has_battery, Path::new(&rom.rom_file_path)))
        },
        MBCType::MBC3 => {
            if has_rtc {
                debug!("Creating MBC3 controller with Real Time Clock");
            } else {
                debug!("Creating MBC3 controller");
            }
            Box::new(MBC3::new(rom_banks, rom.ram_size, has_battery, has_rtc, Path::new(&rom.rom_file_path)))
        },
        MBCType::MBC5 => {
            if has_rumble {
                debug!("Creating MBC5 controller with Rumble");
            } else {
                debug!("Creating MBC5 controller");
            }
            Box::new(MBC5::new(rom_banks, rom.ram_size, has_battery, has_rumble, Path::new(&rom.rom_file_path)))
        }
    }
}