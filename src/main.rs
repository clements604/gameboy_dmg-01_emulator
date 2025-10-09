mod cpu;
mod constants;
mod rom;
mod memory_bus;
mod ppu;
mod rom_debug;
mod dmg_io;
mod interrupts;
mod dma;
mod lcd;
mod timer;
mod joypad;
mod main_display;
mod mbc;
mod mbc0;
mod mbc1;
mod mbc_factory;
mod mbc2;
mod mbc3;
mod mbc5;
mod emulaor_config;

use log::{debug, error, info};
use crate::rom::ROM;

use std::fs::File;
use std::io::Read;
use std::time::{Duration, Instant};
use sdl2::keyboard::Keycode;
use rfd::FileDialog;
use crate::emulaor_config::EmulatorConfig;
use crate::interrupts::Interrupt::JOYPAD;
use crate::joypad::Button;
use crate::main_display::MainDisplay;
use crate::memory_bus::MemoryBus;

struct Emulator {
    cpu: cpu::CPU,

    memory_bus: MemoryBus,

    main_display: MainDisplay,

    previous_frame: u32,

    last_time: Instant, // Used for FPS calculation
    frame_count: u32, // Used for FPS calculation
    input_check_counter: u32, // Counter for input checking

    // Frame rate cap variables
    target_frame_time: Duration,
    last_frame_time: Instant,
    running: bool,
    last_save_time: Instant,
}

impl Emulator {
    pub fn new(emulator_config: EmulatorConfig, boot_rom: Option<Vec<u8>>, rom: &ROM) -> Emulator {

        let mut cpu = cpu::CPU::new();

        match boot_rom {
            Some(_) => {
                cpu.pc = 0x0000;
            },
            None => {
                cpu.pc = 0x0100;
            },
        };
        
        let main_display = MainDisplay::new(emulator_config.scale_factor);

        Emulator {
            cpu,
            memory_bus: MemoryBus::new(boot_rom, &rom),
            main_display,
            previous_frame: 0,
            last_time: Instant::now(),
            frame_count: 0,
            input_check_counter: 0,
            // Set target frame time to ~16.67ms (60 FPS)
            target_frame_time: Duration::from_micros(16742),
            last_frame_time: Instant::now(),
            running: true,
            last_save_time: Instant::now(),
        }
    }

    fn cycle(&mut self) {
        if self.input_check_counter % 16 == 0 {
            if !self.main_display.process_events() {
                match self.memory_bus.mbc.as_mut() {
                    Some(mbc) => {
                        if ! mbc.dirty_sram() {
                            self.running = false;
                            std::process::exit(0);
                        }
                        else {
                            error!("Emulator requested to stop but save SRAM is dirty, not stopping to prevent data loss");
                        }
                    },
                    None => {},
                }
                
            }
        }

        // Process inputs every 2 cycles for responsiveness
        if self.input_check_counter % 2 == 0 {
            let current_keys = self.main_display.get_pressed_keys();
            let mut joypad = self.memory_bus.dmg_io.joypad;

            // Store the selection bits before modifying the joypad
            let select_buttons = joypad.select_buttons;
            let select_dpad = joypad.select_dpad;

            // Reset all buttons to released state
            for button in [
                Button::A,
                Button::B,
                Button::Start,
                Button::Select,
                Button::Up,
                Button::Down,
                Button::Left,
                Button::Right,
            ] {
                joypad.button_released(button);
            }

            let mut pressed = false;

            // Apply current keys
            for key in current_keys {
                match key {
                    Keycode::Up => joypad.button_pressed(Button::Up),
                    Keycode::Down => joypad.button_pressed(Button::Down),
                    Keycode::Left => joypad.button_pressed(Button::Left),
                    Keycode::Right => joypad.button_pressed(Button::Right),
                    Keycode::A => joypad.button_pressed(Button::A),
                    Keycode::B => joypad.button_pressed(Button::B),
                    Keycode::Return => joypad.button_pressed(Button::Start),
                    Keycode::Backspace => joypad.button_pressed(Button::Select),
                    _ => (),
                }
                pressed = true;
            }

            // Restore the selection bits after updating button states
            joypad.select_buttons = select_buttons;
            joypad.select_dpad = select_dpad;

            // Trigger interrupt if state changed
            if pressed {
                self.memory_bus.trigger_interrupt(JOYPAD);
            }

            self.memory_bus.dmg_io.joypad = joypad;
        }

        self.input_check_counter = (self.input_check_counter + 1) % 32;

        let cpu_cycles = self.cpu.cycle(&mut self.memory_bus);

        if self.memory_bus.enabling_ime {
            self.memory_bus.interrupt_master_enable = true;
            self.memory_bus.enabling_ime = false;
        }

        // Update the timer with the number of CPU cycles
        /*if self.memory_bus.borrow().dmg_io.as_ref().unwrap().borrow_mut().timer.cycle(cpu_cycles) {
            // If timer overflows, trigger a Timer interrupt
            self.memory_bus.trigger_interrupt(interupts::Interrupt::TIMER);
        }*/

        let ppu_interrupts = self.memory_bus.dmg_io.ppu.tick(cpu_cycles);
        for interrupt in ppu_interrupts {
            self.memory_bus.trigger_interrupt(interrupt);
        }

        self.memory_bus.cycle(cpu_cycles);
        /*for _ in 0..cpu_cycles {
            self.dma.borrow_mut().dma_tick();
        }*/

        self.cpu.check_interrupts(&mut self.memory_bus);

        if self.previous_frame != self.memory_bus.dmg_io.ppu.current_frame {
            // Update display with the new frame buffer
            self.main_display.update(self.memory_bus.dmg_io.ppu.framebuffer.clone());

            self.frame_count += 1;  // Increment frame count

            // Only calculate FPS once per second
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_time);

            if elapsed.as_secs() >= 1 {
                debug!("FPS: {}", self.frame_count);
                self.frame_count = 0;  // Reset frame count
                self.last_time = now;
            }

            self.previous_frame = self.memory_bus.dmg_io.ppu.current_frame;

            // Frame rate cap to 60 FPS
            let elapsed = now.duration_since(self.last_frame_time);
            if elapsed < self.target_frame_time {
                let sleep_time = self.target_frame_time - elapsed;
                std::thread::sleep(sleep_time);
            }
            self.last_frame_time = Instant::now();
        }

        if self.last_save_time.elapsed() > Duration::from_secs(5) {
            self.memory_bus.mbc.as_mut().unwrap().save_ram();
            self.last_save_time = Instant::now();
        }
    }

}

fn main() {
    // Open the log file
    let _ = env_logger::builder()
        .target(env_logger::Target::Stdout)
        //.target(env_logger::Target::Pipe(Box::new(file)))
        .filter_level(log::LevelFilter::Debug)
        .is_test(false)
        .try_init();

    let emulaor_config = EmulatorConfig::new();
    let boot_rom = match emulaor_config.boot_rom {
        Some(ref path) => {
            Some(load_boot_rom(path.to_string()))
        },
        None => {
            None
        },
    };

    let rom_file = FileDialog::new()
        .add_filter("Gameboy ROM Files", &["gb"])
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
    
    //let boot_rom = Some(load_boot_rom(String::from("roms/boot/dmg0_boot.bin")));
    //let boot_rom = None;

    //let rom = load_rom(String::from("roms/Tetris.gb"));
    //let rom = load_rom(String::from("roms/Dr. Mario.gb"));
    //let rom = load_rom(String::from("roms/Alleyway.gb"));
    //let rom = load_rom(String::from("roms/Legend of Zelda - Links Awakening.gb"));
    //let rom = load_rom(String::from("roms/Super Mario Land.gb"));
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
    //let rom = load_rom(String::from("roms/test/cpu/cpu_instrs.gb"));

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
    //let rom = load_rom(String::from("roms/test/ppu/dmg-acid2.gb")); // PASSED
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

    /*
    *   Timer
    */
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/tima_reload.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/div_write.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/rapid_toggle.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/tim00.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/tim00_div_trigger.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/tim01.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/tim01_div_trigger.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/tim10.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/tim10_div_trigger.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/tim11.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/tim11_div_trigger.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/tima_reload.gb"));
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/tima_write_reloading.gb")); //TODO failed
    //let rom = load_rom(String::from("/home/josh/Documents/rust/gameboy-emulator/roms/test/mooney/mts-20240127-1204-74ae166/acceptance/timer/tma_write_reloading.gb"));
    
    let mut emulator = Emulator::new(emulaor_config, boot_rom, &rom);
    
    // Main loop - no need for separate input handling now
    while emulator.running {
        emulator.cycle();
    }
}

fn load_rom(file_path: String) -> ROM {
    debug!("Loading ROM: {}", &file_path);
    let mut file = File::open(&file_path).expect("ROM file not found");
    let mut buffer: Vec<u8> = Vec::new();

    // Read the file into a buffer
    file.read_to_end(&mut buffer).expect("Error reading file");
    debug!(
            "ROM file size: {} bytes / {} kilobytes",
            buffer.len(),
            buffer.len() / 1024
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
    buffer
}