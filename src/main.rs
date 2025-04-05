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
mod joypad;
mod tile_map_display;
mod main_display;
mod background_display;
mod window_display;
mod mbc;
mod mbc0;
mod mbc1;
mod mbc_factory;
mod mbc2;
mod mbc3;
mod mbc5;

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
use std::time::Instant;
//use sdl2::EventPump;
use crate::timer::{Timer, TimerFrequency};

use minifb::{Key, Scale, Window, WindowOptions};
use crate::display::LIGHTEST_GREEN;
use crate::interupts::Interrupt::JOYPAD;
use crate::joypad::Button;
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
    window_display: window_display::WindowDisplay,

    //event_pump: EventPump,
    previous_frame: u32,
    previous_ly: u8,
    
    last_time: Instant, // Used for FPS calculation
    frame_count: u32, // Used for FPS calculation

    previous_keys: Vec<Key>,
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
        let mut window_display = window_display::WindowDisplay::new();

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
            memory_bus,
            dma,
            debug_window,
            background_display,
            main_display,
            window_display,
            previous_frame: 0,
            previous_ly: 0,
            last_time: Instant::now(),
            frame_count: 0,
            
            previous_keys: Vec::new(),
        }
    }

    fn cycle(&mut self) {
        let current_keys: Vec<Key> = self.main_display.window.get_keys();
        for key in &self.previous_keys {
            if !current_keys.contains(key) {
                match key {
                    Key::Up => self.memory_bus.borrow_mut().dmg_io.as_ref().unwrap().borrow_mut().joypad.button_released(Button::Up),
                    Key::Left => self.memory_bus.borrow_mut().dmg_io.as_ref().unwrap().borrow_mut().joypad.button_released(Button::Left),
                    Key::Down => self.memory_bus.borrow_mut().dmg_io.as_ref().unwrap().borrow_mut().joypad.button_released(Button::Down),
                    Key::Right => self.memory_bus.borrow_mut().dmg_io.as_ref().unwrap().borrow_mut().joypad.button_released(Button::Right),
                    Key::A => self.memory_bus.borrow_mut().dmg_io.as_ref().unwrap().borrow_mut().joypad.button_released(Button::A),
                    Key::B => self.memory_bus.borrow_mut().dmg_io.as_ref().unwrap().borrow_mut().joypad.button_released(Button::B),
                    Key::Enter => self.memory_bus.borrow_mut().dmg_io.as_ref().unwrap().borrow_mut().joypad.button_released(Button::Start),
                    Key::Backspace => self.memory_bus.borrow_mut().dmg_io.as_ref().unwrap().borrow_mut().joypad.button_released(Button::Select),
                    _ => (),
                }
                
            }
        }

        let previous_window_enabled = self.memory_bus.borrow().ppu.as_ref().unwrap().borrow().lcdc & 0x20 != 0;
        
        let cpu_cycles = self.cpu.borrow_mut().cycle();

        let current_window_enabled = self.memory_bus.borrow().ppu.as_ref().unwrap().borrow().lcdc & 0x20 != 0;
        if previous_window_enabled != current_window_enabled {
            info!("Window enable changed mid-frame: LY={}, now={}", 
           self.memory_bus.borrow().ppu.as_ref().unwrap().borrow().ly, current_window_enabled);
        }

        if self.memory_bus.borrow().enabling_ime {
            self.memory_bus.borrow_mut().interrupt_master_enable = true;
            self.memory_bus.borrow_mut().enabling_ime = false;
        }

        if self.cpu.borrow().registers.pc == 0x0B7D {//0x0B7D
            info!("{}", self.cpu.borrow().registers);
            info!("STAT: {:#X}", self.memory_bus.borrow().dmg_io.as_ref().unwrap().borrow().ppu.borrow().stat);
            //self.ppu.borrow_mut().stat = 0x80;
            //info!("{}", self.memory_bus.borrow().dmg_io.as_ref().unwrap().borrow().ppu.borrow());
            debug!("hello");
        }

        // Update the timer with the number of CPU cycles
        if self.memory_bus.borrow().dmg_io.as_ref().unwrap().borrow_mut().timer.cycle(cpu_cycles) {
            // If timer overflows, trigger a Timer interrupt
            self.cpu.borrow_mut().trigger_interrupt(interupts::Interrupt::TIMER);
        }

        self.ppu.borrow_mut().tick(cpu_cycles);

        for _ in 0..cpu_cycles {
            self.dma.borrow_mut().dma_tick();
        }

        self.cpu.borrow_mut().check_interrupts();

        if self.previous_frame != self.ppu.borrow().current_frame {

            //self.display.borrow_mut().ui_update();
            //if self.ppu.borrow().lcd_ppu_enabled() {
                self.main_display.update(self.ppu.borrow().framebuffer.clone());
                //self.background_display.update(&self.ppu.borrow().get_debug_background_tile_map());
                //self.debug_window.update(&self.ppu.borrow().get_tile_map());
            /*self.window_display.update_with_fixed_size_tiles(
                self.ppu.borrow().lcd.as_ref().borrow().window_x,
                self.ppu.borrow().lcd.as_ref().borrow().window_y,
                &self.ppu.borrow().get_window_tiles()
            );*/

            self.frame_count += 1;  // Increment frame count each frame

            let now = Instant::now();
            let elapsed = now.duration_since(self.last_time);

            if elapsed.as_secs() >= 1 {
                info!("FPS: {}", self.frame_count);
                self.frame_count = 0;  // Reset frame count
                self.last_time = now;
            }

            self.previous_frame = self.ppu.borrow().current_frame;
        }
        
    }
    
    fn handle_input(&mut self) {
        let mut joypad = self.memory_bus.borrow_mut().dmg_io.as_mut().unwrap().borrow_mut().joypad;
        let keys = &self.main_display.window.get_keys();
        if keys.len() > 0 {
            info!("Keys pressed: {:?}", keys);
        }
        for key in keys {
            match key {
                Key::Up => joypad.button_pressed(Button::Up),
                Key::Left => joypad.button_pressed(Button::Left),
                Key::Down => joypad.button_pressed(Button::Down),
                Key::Right => joypad.button_pressed(Button::Right),
                Key::A => joypad.button_pressed(Button::A),
                Key::B => joypad.button_pressed(Button::B),
                Key::Enter => joypad.button_pressed(Button::Start),
                Key::Backspace => joypad.button_pressed(Button::Select),
                _ => (),
            }
        }

        self.memory_bus.borrow_mut().dmg_io.as_mut().unwrap().borrow_mut().joypad = joypad;

        if u8::from(joypad) != 0xFF {
            //self.cpu.borrow_mut().trigger_interrupt(JOYPAD);
            //self.memory_bus.borrow_mut().interrupt_master_enable = true;
            //info!("{:8b}", u8::from(joypad));
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
    //let rom = load_rom(String::from("roms/Alleyway.gb"));
    let rom = load_rom(String::from("roms/Legend of Zelda - Links Awakening.gb"));
    /*let rom = load_rom(String::from(
        "roms/Pokemon - Red Version (USA, Europe) (SGB Enhanced).gb",
    ));*/

    /*
     * CPU instructions
    */
    //let rom = load_rom(String::from("roms/test/cpu/individual/01-special.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/02-interrupts.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/03-op sp,hl.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/04-op r,imm.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/05-op rp.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/06-ld r,r.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/07-jr,jp,call,ret,rst.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/08-misc instrs.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/09-op r,r.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/10-bit ops.gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/individual/11-op a,(hl).gb")); // PASSED
    //let rom = load_rom(String::from("roms/test/cpu/cpu_instrs.gb"));//TODO infinate loop due to no MBC implementation
    
    /*
    * CPU timing
     */
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/cpu/timing/instr_timing.gb"));// TODO FAILED
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/add_sp_e_timing.gb"));// TODO FAILED
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/boot_div2-S.gb"));// TODO FAILED
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/call_timing.gb"));// TODO FAILED

    /*
     * Graphics
    */
   //let rom = load_rom(String::from("roms/test/ppu/dmg-acid2.gb")); //TODO PPU
   //let rom = load_rom(String::from("/home/josh/Downloads/lyc.gb")); // PASSED
   //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/ppu/lcdon_timing-GS.gb")); //TODO LYC
   //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/ppu/hblank_ly_scx_timing-GS.gb")); //TODO FAILED

    /*
     * Memory timing
    */
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/memory/mem_timing.gb")); // TODO no debug output
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/memory/01-read_timing.gb")); // TODO
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/memory/02-write_timing.gb")); // TODO no debug output
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/memory/03-modify_timing.gb")); // TODO no debug output

    /*
    * Interrupt timing
    */
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/interrupts/interrupt_time.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/ei_sequence.gb"));

    /*let memory_bus = Rc::new(RefCell::new(memory_bus::MemoryBus::new(boot_rom, &rom)));
    let mut cpu = CPU::CPU::new(Rc::clone(&memory_bus));
    let mut ppu = ppu::Ppu::new();*/

let mut emulator = Emulator::new(boot_rom, &rom);

    

    while emulator.main_display.window.is_open() && !emulator.main_display.window.is_key_down(Key::Escape) {
        emulator.handle_input();
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
