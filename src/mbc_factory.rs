use log::{debug, error, info};
use crate::mbc::{MBC, MBCType};
use crate::mbc0::MBC0;
use crate::mbc1::MBC1;
use crate::rom::ROM;

pub fn create_mbc(rom: &ROM) -> Box<dyn MBC> {
    // Determine MBC type from cartridge type
    let mbc_type = MBCType::from_byte(rom.cartridge_type);
    let has_battery = match mbc_type {
        MBCType::None => false,
        _ => mbc_type.has_battery(rom.cartridge_type)
    };

    // Create ROM banks from the ROM
    let rom_banks = rom.load_rom_to_banks();

    match mbc_type {
        MBCType::None => {
            info!("Creating MBC0 (No MBC) controller");
            Box::new(MBC0::new(rom_banks, rom.ram_size, has_battery))
        },
        MBCType::MBC1 => {
            info!("Creating MBC1 controller");
            Box::new(MBC1::new(rom_banks, rom.rom_size, rom.ram_size, has_battery))
        },
        _ => {
            panic!("Unsupported MBC type: {:?}. Falling back to MBC0", mbc_type);
            Box::new(MBC0::new(rom_banks, rom.ram_size, has_battery))
        }
    }
}