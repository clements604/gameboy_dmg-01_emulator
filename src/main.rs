mod CPU;
mod constants;
mod display;
mod rom;
mod memory_bus;
mod ppu;
mod rom_debug;
mod dmg_io;
mod interupts;

use std::io::Write;
use std::sync::Mutex;

use log::{debug, error, info};
use crate::rom::ROM;

use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Read};
use crate::CPU::Flag;
use std::rc::Rc;
use std::cell::RefCell;

fn main() {
    // Open the log file
    //let file = File::create("output.log").unwrap();
    let _ = env_logger::builder()
        .target(env_logger::Target::Stdout)
        //.target(env_logger::Target::Pipe(Box::new(file)))
        .filter_level(log::LevelFilter::Info)
        .is_test(false)
        .try_init();

    let mut cycle_count = 0;
    let mut ppu_cycles: u16 = 0;
    
    //let boot_rom = load_boot_rom(String::from("roms/boot/dmg0_boot.bin"));
    let boot_rom = Option::None;

    //let rom = load_rom(String::from("roms/Tetris.gb"));
    /*let rom = load_rom(String::from(
        "roms/Pokemon - Red Version (USA, Europe) (SGB Enhanced).gb",
    ));*/
    
    //let rom = load_rom(String::from("roms/test/cpu/individual/01-special.gb")); //TODO FAILED
    //let rom = load_rom(String::from("roms/test/cpu/individual/02-interrupts.gb")); //TODO FAILED
    //let rom = load_rom(String::from("roms/test/cpu/individual/03-op sp,hl.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/04-op r,imm.gb")); // TODO implement interupts and timers. C229, C22D, C2231 are setting interupt and timers.
    //let rom = load_rom(String::from("roms/test/cpu/individual/05-op rp.gb")); // TODO no test rom output
    //let rom = load_rom(String::from("roms/test/cpu/individual/06-ld r,r.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/07-jr,jp,call,ret,rst.gb")); // TODO never finishes CURRENT INSTR DEBUG WIP
    //let rom = load_rom(String::from("roms/test/cpu/individual/08-misc instrs.gb")); // TODO no test rom output
    //let rom = load_rom(String::from("roms/test/cpu/individual/09-op r,r.gb")); // TODO never finishes
    //let rom = load_rom(String::from("roms/test/cpu/individual/10-bit ops.gb")); // TODO never finishes 211211211211211211211211211211211211211211211211211
    //let rom = load_rom(String::from("roms/test/cpu/individual/11-op a,(hl).gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/cpu_instrs.gb"));

    let rom = load_rom(String::from("roms/test/ppu/dmg-acid2.gb")); //TODO PPU

    let memory_bus = Rc::new(RefCell::new(memory_bus::MemoryBus::new(boot_rom, &rom)));
    let mut cpu = CPU::CPU::new(Rc::clone(&memory_bus));
    let mut ppu = ppu::Ppu::new();

    let mut display = display::Display::new(
        &String::from("RustGB"),
        Rc::clone(&memory_bus),
        display::SCREEN_WIDTH as u32,
        display::SCREEN_HEIGHT as u32,
    );
    let mut event_pump = display.sdl_context.event_pump().unwrap();
    
    //TODO make the program counter start dependant on boot rom presence
    //cpu.registers.pc = 0x0000;
    //cpu.registers.a = 0x0;
    //cpu.registers.b = 0x0;
    //cpu.registers.c = 0x0;
    //cpu.registers.d = 0x0;
    //cpu.registers.e = 0x0;
    //cpu.registers.f.set_flag(Flag::N, false);
    //cpu.registers.f.set_flag(Flag::Z, false);
    //cpu.registers.f.set_flag(Flag::H, false);
    //cpu.registers.f.set_flag(Flag::C, false);
    //cpu.registers.h = 0x0;
    //cpu.registers.l = 0x0;
    //cpu.registers.sp = 0xFFFE;
    
    /*for i in 0..0x100 {
        debug!("{:#X}: {:#X}", i, memory_bus.read_byte(i));
    }*/

    
    loop {
        cycle_count += 1;
        debug!("Cycle: {}", cycle_count);

        if cpu.registers.pc == 0xC73A { 
            //error!("{}", cpu.registers);
            //break;
        }
        //error!("{}", cpu.registers);

        cpu.cycle();
        ppu.step(&mut cpu, ppu_cycles);
        display.ui_update();
        //sleep for 2 seconds
        //std::thread::sleep(std::time::Duration::from_secs(2));
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

fn load_boot_rom(file_path: String) -> Vec<u8> {
    debug!("Loading boot ROM: {}", file_path);
    let mut file = File::open(file_path).expect("Boot ROM file not found");
    let mut buffer: Vec<u8> = Vec::new();

    // Read the file into a buffer
    file.read_to_end(&mut buffer).expect("Error reading file");
    debug!(
        "Boot ROM file size: {} bytes / {} kilobytes",
        buffer.len(),
        buffer.len() / 1024
    );

    buffer
}
