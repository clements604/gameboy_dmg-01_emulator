mod CPU;
mod constants;
mod display;
mod rom;
mod memory_bus;
mod ppu;
mod rom_debug;
mod dmg_io;
mod interupts;
mod dma;
mod lcd;
mod timer;
mod ppu_experiment;
mod ppu_pipeline;

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
use sdl2::EventPump;
use crate::timer::Timer;

struct Emulator {
    ticks: u64,
    cpu: Rc<RefCell<CPU::CPU>>,

    ppu: Rc<RefCell<ppu::Ppu>>,
    //ppu_experiment: Rc<RefCell<ppu_experiment::Ppu>>,

    memory_bus: Rc<RefCell<memory_bus::MemoryBus>>,
    dma: Rc<RefCell<dma::Dma>>,
    display: Rc<RefCell<display::Display>>,
    timer: Timer,
    event_pump: EventPump,
    previous_frame: u32,
}

impl Emulator {
    pub fn new(boot_rom: Option<Vec<u8>>, rom: &ROM) -> Emulator {

        let memory_bus = Rc::new(RefCell::new(memory_bus::MemoryBus::new(boot_rom, &rom, None, None, None)));

        let display = Rc::new(RefCell::new(display::Display::new(
            &String::from("RustGB"),
            Rc::clone(&memory_bus),
            display::SCREEN_WIDTH as u32,
            display::SCREEN_HEIGHT as u32,
        )));
        let event_pump = display.borrow_mut().sdl_context.event_pump().unwrap();
        
        let cpu = Rc::new(RefCell::new(CPU::CPU::new(memory_bus.clone())));

        let timer = Timer::new(Rc::clone(&cpu));
        let dma = Rc::new(RefCell::new(dma::Dma::new(memory_bus.clone())));
        let lcd = Rc::new(RefCell::new(lcd::LCD::new(dma.clone())));

        let ppu = Rc::new(RefCell::new(ppu::Ppu::new(cpu.clone(), lcd.clone(), display.clone())));
        //let ppu_experiment = Rc::new(RefCell::new(ppu_experiment::Ppu::new(cpu.clone(), lcd.clone(), display.clone())));

        let io = Rc::new(RefCell::new(dmg_io::IO::new(cpu.clone(), lcd.clone(), ppu.clone())));
        memory_bus.borrow_mut().dmg_io = Some(io.clone());
        memory_bus.borrow_mut().dma = Some(dma.clone());

        memory_bus.borrow_mut().ppu = Some(ppu.clone());
        //memory_bus.borrow_mut().ppu_experiment = Some(ppu_experiment.clone());

        memory_bus.borrow_mut().cpu = Some(cpu.clone());
        


        Emulator {
            ticks: 0,
            cpu,

            ppu,
            //ppu_experiment,

            memory_bus,
            display,
            timer,
            event_pump,
            dma,
            previous_frame: 0,
        }
    }
    
    fn cycle(&mut self) {
        for event in self.event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit { .. } => break,
                _ => {}
            }
        }
        let cpu_cycles = self.cpu.borrow_mut().cycle();
        if self.memory_bus.borrow().interrupt_master_enable {
            self.cpu.borrow_mut().handle_interrupts();
            self.memory_bus.borrow_mut().enabling_ime = false;
        }
        if self.memory_bus.borrow().enabling_ime {
            self.memory_bus.borrow_mut().interrupt_master_enable = true;
        }

        for cycles in 0..cpu_cycles {
            for _ in 0..4 {
                self.ticks += 1;
                self.timer.tick();

                self.ppu.borrow_mut().tick(cycles);
                //self.ppu_experiment.borrow_mut().cycle();
            }
        }
        self.dma.borrow_mut().dma_tick();

        if self.previous_frame != self.ppu.borrow().current_frame {
            self.display.borrow_mut().ui_update();
            self.previous_frame = self.ppu.borrow().current_frame;
        }
    }
    
}

fn main() {
    // Open the log file
    //let file = File::create("output.log").unwrap();
    let _ = env_logger::builder()
        .target(env_logger::Target::Stdout)
        //.target(env_logger::Target::Pipe(Box::new(file)))
        .filter_level(log::LevelFilter::Info)
        .is_test(false)
        .try_init();

    //let boot_rom = Some(load_boot_rom(String::from("roms/boot/dmg0_boot.bin")));
    let boot_rom = Option::None;

    //let rom = load_rom(String::from("roms/Tetris.gb"));
    //let rom = load_rom(String::from("roms/Dr. Mario.gb"));
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
    //let rom = load_rom(String::from("roms/test/cpu/individual/11-op a,(hl).gb")); // TODO was passed, now never finishes
    //let rom = load_rom(String::from("roms/test/cpu/cpu_instrs.gb"));

    let rom = load_rom(String::from("roms/test/ppu/dmg-acid2.gb")); //TODO PPU

    /*let memory_bus = Rc::new(RefCell::new(memory_bus::MemoryBus::new(boot_rom, &rom)));
    let mut cpu = CPU::CPU::new(Rc::clone(&memory_bus));
    let mut ppu = ppu::Ppu::new();*/

let mut emulator = Emulator::new(boot_rom, &rom);
    
    loop {
        emulator.cycle();
        //emulator.display.ui_update();
    }

}



fn load_rom(file_path: String) -> ROM {
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
