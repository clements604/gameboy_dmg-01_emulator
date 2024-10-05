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
mod joypad;
mod tile_map_display;
mod main_display;
mod background_display;

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
//use sdl2::EventPump;
use crate::timer::{Timer, TimerFrequency};

use minifb::{Key, Scale, Window, WindowOptions};
use crate::main_display::MainDisplay;
use crate::tile_map_display::DebugDisplay;

struct Emulator {
    ticks: u64,
    cpu: Rc<RefCell<CPU::CPU>>,

    ppu: Rc<RefCell<ppu::Ppu>>,
    //ppu_experiment: Rc<RefCell<ppu_experiment::Ppu>>,

    memory_bus: Rc<RefCell<memory_bus::MemoryBus>>,
    dma: Rc<RefCell<dma::Dma>>,
    //display: Rc<RefCell<display::Display>>,
    debug_window: DebugDisplay,
    background_display: background_display::BackgroundDisplay,
    main_display: MainDisplay,

    //event_pump: EventPump,
    previous_frame: u32,
    previous_ly: u8,
}

impl Emulator {
    pub fn new(boot_rom: Option<Vec<u8>>, rom: &ROM) -> Emulator {

        let boot_rom_enabled = match boot_rom {
            Some(_) => {
                true
            },
            None => {
                false
            },
        };

        let memory_bus = Rc::new(RefCell::new(memory_bus::MemoryBus::new(boot_rom, &rom, None, None, None)));

        let mut debug_window = DebugDisplay::new();
        let mut main_display= MainDisplay::new();
        let mut background_display = background_display::BackgroundDisplay::new();

        let cpu = Rc::new(RefCell::new(CPU::CPU::new(memory_bus.clone())));

        let dma = Rc::new(RefCell::new(dma::Dma::new(memory_bus.clone())));
        let lcd = Rc::new(RefCell::new(lcd::LCD::new(dma.clone())));
        
        let ppu = Rc::new(RefCell::new(ppu::Ppu::new(cpu.clone(), lcd.clone()/*, display.clone()*/)));
        
        //let ppu_experiment = Rc::new(RefCell::new(ppu_experiment::Ppu::new(cpu.clone(), lcd.clone(), display.clone())));

        let io = Rc::new(RefCell::new(dmg_io::IO::new(cpu.clone(), lcd.clone(), ppu.clone())));



        memory_bus.borrow_mut().dmg_io = Some(io.clone());
        memory_bus.borrow_mut().dma = Some(dma.clone());

        memory_bus.borrow_mut().ppu = Some(ppu.clone());
        //memory_bus.borrow_mut().ppu_experiment = Some(ppu_experiment.clone());

        memory_bus.borrow_mut().cpu = Some(cpu.clone());

        match boot_rom_enabled {
            true => {
                cpu.borrow_mut().registers.pc = 0x0000
            },
            false => {
                cpu.borrow_mut().registers.pc = 0x0100
            },
        }



        Emulator {
            ticks: 0,
            cpu,

            ppu,
            
            //ppu_experiment,

            memory_bus,
            //display,

            //event_pump,
            dma,

            debug_window,
            background_display,
            main_display,
            
            previous_frame: 0,
            previous_ly: 0,
        }
    }

    fn cycle(&mut self) {

        let cpu_cycles = self.cpu.borrow_mut().cycle();

        if self.cpu.borrow().registers.pc == 0xC2C0 {//0xCB89
            info!("{}", self.cpu.borrow().registers);
            //info!("{}", self.memory_bus.borrow().dmg_io.as_ref().unwrap().borrow().ppu.borrow());
            debug!("hello");
        }

        if self.memory_bus.borrow().dmg_io.as_ref().unwrap().borrow_mut().timer.cycle(1) {
            self.cpu.borrow_mut().trigger_interrupt(interupts::Interrupt::TIMER);
        }

        self.ppu.borrow_mut().tick(cpu_cycles);

        self.dma.borrow_mut().dma_tick();

        if self.memory_bus.borrow().interrupt_master_enable {
            self.cpu.borrow_mut().handle_interrupts();
            self.memory_bus.borrow_mut().enabling_ime = false;
        }
        if self.memory_bus.borrow().enabling_ime {
            self.memory_bus.borrow_mut().interrupt_master_enable = true;
        }

        if self.previous_frame != self.ppu.borrow().current_frame {
            //self.display.borrow_mut().ui_update();
            if self.ppu.borrow().lcd_ppu_enabled() {
                //self.debug_window.update(&self.ppu.borrow().get_tile_map());
                //self.background_display.update(&self.ppu.borrow().get_debug_background_tile_map());

                //self.main_display.update(&Vec::from(self.ppu.borrow().get_window_tiles()));
                //info!("{:?}", self.ppu.borrow().get_background_tile_map().len());
                //self.ppu.borrow().get_window_tiles();

                //info!("{:?}", self.ppu.borrow().populate_background_tiles());
                //self.ppu.borrow_mut().gpt_get_viewport();
                self.main_display.update(self.ppu.borrow_mut().gpt_render_viewport());
                //self.ppu.borrow_mut().get_viewport_pixels();
                
                info!("Scroll X: {}", self.ppu.borrow().scroll_x);
                info!("Scroll Y: {}", self.ppu.borrow().scroll_y);
                debug!("");

            }
            self.previous_frame = self.ppu.borrow().current_frame;
        }
        
    }
    
    fn debug_ly(&mut self) {
        let new_ly = self.ppu.borrow().ly;
        if new_ly != self.previous_ly {
            debug!("LY: {}", new_ly);
            self.previous_ly = new_ly;
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

    let boot_rom = Some(load_boot_rom(String::from("roms/boot/dmg0_boot.bin")));
    //let boot_rom = Option::None;

    //let rom = load_rom(String::from("roms/Tetris.gb"));
    //let rom = load_rom(String::from("roms/Dr. Mario.gb"));
    /*let rom = load_rom(String::from(
        "roms/Pokemon - Red Version (USA, Europe) (SGB Enhanced).gb",
    ));*/

    /*
     * CPU instructions
    */
    //let rom = load_rom(String::from("roms/test/cpu/individual/01-special.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/02-interrupts.gb")); //TODO infinate loop due to joypad interrupt?
    //let rom = load_rom(String::from("roms/test/cpu/individual/03-op sp,hl.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/04-op r,imm.gb")); // TODO never finishes
    //let rom = load_rom(String::from("roms/test/cpu/individual/05-op rp.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/06-ld r,r.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/07-jr,jp,call,ret,rst.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/08-misc instrs.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/09-op r,r.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/10-bit ops.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/11-op a,(hl).gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/cpu_instrs.gb"));//TODO infinate loop due to joypad interrupt?
    
    /*
    * CPU timing
     */
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/cpu/timing/instr_timing.gb"));// TODO FAILED

    /*
     * Graphics
    */
   let rom = load_rom(String::from("roms/test/ppu/dmg-acid2.gb")); //TODO PPU
    
    /*
     * Memory timing
    */
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/memory/mem_timing.gb")); // TODO no debug output
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/memory/01-read_timing.gb")); // TODO no debug output
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/memory/02-write_timing.gb")); // TODO no debug output
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/memory/03-modify_timing.gb")); // TODO no debug output

    /*
    * Interrupt timing
    */
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/interrupts/interrupt_time.gb"));

    /*let memory_bus = Rc::new(RefCell::new(memory_bus::MemoryBus::new(boot_rom, &rom)));
    let mut cpu = CPU::CPU::new(Rc::clone(&memory_bus));
    let mut ppu = ppu::Ppu::new();*/

let mut emulator = Emulator::new(boot_rom, &rom);

    

    while emulator.main_display.window.is_open() && !emulator.main_display.window.is_key_down(Key::Escape) {
        emulator.cycle();
    }

    /*loop {
        emulator.cycle();
        //emulator.display.ui_update();
    }*/

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
