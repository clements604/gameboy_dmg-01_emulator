mod cpu;
mod rom;
mod memory_bus;
mod ppu;
mod rom_debug;
mod io;
mod interrupts;
mod dma;
mod lcd;
mod timer;
mod joypad;
mod display;
mod mbc;
mod emulator_config;
mod emulator;

use std::fs::File;
use std::io::Read;

use log::{debug, error, info};
use rfd::FileDialog;

use crate::rom::ROM;
use crate::emulator_config::EmulatorConfig;
use crate::emulator::Emulator;

const KILOBYTES_CONVERSION: usize = 1024;
const BOOT_ROM_SIZE: usize = 256;

fn main() {
    let emulator_config = EmulatorConfig::new();
    
    let logging_level = match emulator_config.logging_level.as_str() {
        "DEBUG" => log::LevelFilter::Debug,
        "ERROR" => log::LevelFilter::Error,
        "INFO" => log::LevelFilter::Info,
        _ => log::LevelFilter::Error,
    };
    
    let _ = env_logger::builder()
        .target(env_logger::Target::Stdout)
        .filter_level(logging_level)
        .is_test(false)
        .try_init();

    
    let boot_rom = match emulator_config.boot_rom {
        Some(ref path) => {
            Some(load_boot_rom(path.to_string()))
        },
        None => {
            None
        },
    };

    let rom_file = FileDialog::new()
        .add_filter(".gb", &["gb"])
        .pick_file();
    info!("ROM file selected: {:?}", rom_file);

    let rom = match rom_file {
        Some(path) => {
            load_rom(path.to_str().unwrap().to_string())
        },
        None => {
            error!("No ROM file selected, exiting...");
            std::process::exit(1);
        },
    };

    let mut emulator = Emulator::new(emulator_config, boot_rom, &rom);

    while emulator.running {
        emulator.cycle();
    }
}

fn load_rom(file_path: String) -> ROM {
    debug!("Loading ROM: {}", &file_path);
    let mut file = File::open(&file_path).expect("ROM file not found");
    let mut buffer: Vec<u8> = Vec::new();

    file.read_to_end(&mut buffer).expect("Error reading file");
    debug!(
            "ROM file size: {} bytes / {} kilobytes",
            buffer.len(),
            buffer.len() / KILOBYTES_CONVERSION
        );

    let rom = ROM::new(file_path, buffer);
    rom.validate_header_checksum().unwrap(); // Panics if the header checksum is invalid

    rom
}

fn load_boot_rom(file_path: String) -> Vec<u8> {
    let mut file = File::open(file_path).unwrap_or_else(|_| {
        error!("Boot ROM file not found");
        std::process::exit(1);
    });
    let mut buffer: Vec<u8> = Vec::new();
    // Read the file into a buffer
    file.read_to_end(&mut buffer).expect("Error reading file");


    if buffer.len() != BOOT_ROM_SIZE {
        error!("Invalid boot ROM size: expected {} bytes, got {} bytes", 
            BOOT_ROM_SIZE, buffer.len());
        std::process::exit(1);
    }
    
    buffer
}
