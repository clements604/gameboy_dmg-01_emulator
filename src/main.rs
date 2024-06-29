mod CPU;
mod constants;
mod display;
mod rom;
mod memory_bus;
mod ppu;
mod rom_debug;
mod dmg_io;

use std::io::Write;
use std::sync::Mutex;

use log::{debug, error, info};
use crate::rom::ROM;

use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Read};

fn main() {
    // Open the log file
    let file = File::create("output.log").unwrap();
    let _ = env_logger::builder()
        .target(env_logger::Target::Stdout)
        //.target(env_logger::Target::Pipe(Box::new(file)))
        .filter_level(log::LevelFilter::Info)
        .is_test(false)
        .try_init();

    let mut cycle_count = 0;

    /*let mut display = display::Display::new(
        &String::from("RustGB"),
        display::SCREEN_WIDTH as u32,
        display::SCREEN_HEIGHT as u32,
    );*/


    //cpu.load_boot_rom(String::from("roms/boot/dmg0_boot.bin"));

    //let rom = load_rom(String::from("roms/Tetris.gb"));
    /*let rom = cpu.load_rom(String::from(
        "roms/Pokemon - Red Version (USA, Europe) (SGB Enhanced).gb",
    ));*/

    //let rom = load_rom(String::from("roms/test/cpu_instrs.gb"));
    let rom = load_rom(String::from("roms/test/cpu/06-ld r,r.gb"));

    let mut memory_bus = memory_bus::MemoryBus::new(&rom.rom);
    let mut cpu = CPU::CPU::new(&mut memory_bus);

    loop {
        cycle_count += 1;
        debug!("Cycle: {}", cycle_count);
        cpu.cycle();
    }

}
pub fn load_rom(file_path: String) -> ROM {
    debug!("Loading ROM: {}", file_path);
    let mut file = File::open(file_path).expect("ROM file not found");
    let mut buffer: Vec<u8> = Vec::new();

    // Read the file into a buffer
    file.read_to_end(&mut buffer).expect("Error reading file");
    debug!(
            "ROM file size: {} bytes / {} kilobytes",
            buffer.len(),
            buffer.len() / 1024
        );

    let rom = ROM::new(buffer);
    debug!("{}", rom);
    rom.validate_header_checksum().unwrap(); // Panics if the header checksum is invalid

    rom
}