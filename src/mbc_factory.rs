use std::path::Path;
use log::debug;
use crate::mbc::{MBC, MBCType};
use crate::mbc0::MBC0;
use crate::mbc1::MBC1;
use crate::mbc2::MBC2;
use crate::mbc3::MBC3;
use crate::mbc5::MBC5;
use crate::rom::ROM;

pub fn create_mbc(rom: &ROM) -> Box<dyn MBC> {
    // Determine MBC type from cartridge type
    let mbc_type = MBCType::from_byte(rom.cartridge_type);

    // Check if the cartridge has battery-backed RAM
    let has_battery = match rom.cartridge_type {
        0x03 | 0x06 | 0x09 | 0x0D | 0x0F | 0x10 | 0x13 | 0x1B | 0x1E | 0xFF => true,
        _ => false
    };

    // Check if the cartridge has RTC (Real Time Clock)
    let has_rtc = match rom.cartridge_type {
        0x0F | 0x10 => true,
        _ => false
    };

    let has_rumble = match rom.cartridge_type {
        0x1C | 0x1D | 0x1E => true,
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
        },
        _ => {
            panic!("Unsupported MBC type: {:?}", mbc_type);
        }
    }
}