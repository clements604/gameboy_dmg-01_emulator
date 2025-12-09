use std::time::{Duration, Instant};
use log::{debug, error};
use crate::cpu;
use crate::emulator_config::EmulatorConfig;
use crate::display::MainDisplay;
use crate::memory_bus::MemoryBus;
use crate::rom::ROM;

const INPUT_CHECK_INTERVAL: u32 = 16;
const INPUT_PROCESS_INTERVAL: u32 = 2;
const COUNTER_WRAP: u32 = 32;
const FPS_60_MICROS: u64 = 16742;
const AUTOSAVE_INTERVAL_SECS: u64 = 5;
const BOOT_ROM_START: u16 = 0x0000;
const CART_ROM_START: u16 = 0x0100;

use std::collections::HashMap;
use minifb::Key;

pub struct Emulator {
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
    pub(crate) running: bool,
    last_save_time: Instant,
    key_bindings: HashMap<String, Key>,
}

impl Emulator {
    pub fn new(emulator_config: EmulatorConfig, boot_rom: Option<Vec<u8>>, rom: &ROM) -> Emulator {

        let mut cpu = cpu::CPU::new();
        cpu.pc = if boot_rom.is_some() { BOOT_ROM_START } else { CART_ROM_START };
        let main_display = MainDisplay::new(emulator_config.scale_factor as usize);
        let key_bindings = emulator_config.key_bindings.clone();
        Emulator {
            cpu,
            memory_bus: MemoryBus::new(boot_rom, &rom),
            main_display,
            previous_frame: 0,
            last_time: Instant::now(),
            frame_count: 0,
            input_check_counter: 0,
            // Set target frame time to ~16.67ms (60 FPS)
            target_frame_time: Duration::from_micros(FPS_60_MICROS),
            last_frame_time: Instant::now(),
            running: true,
            last_save_time: Instant::now(),
            key_bindings,
        }
    }

    pub fn cycle(&mut self) {
        if self.input_check_counter % INPUT_CHECK_INTERVAL == 0 {
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
        if self.input_check_counter % INPUT_PROCESS_INTERVAL == 0 {
            let pressed_buttons = self.main_display.get_pressed_buttons(&self.key_bindings);
            let mut joypad = self.memory_bus.dmg_io.joypad;

            // Reset all buttons to released state
            for button in [
                crate::joypad::Button::A,
                crate::joypad::Button::B,
                crate::joypad::Button::Start,
                crate::joypad::Button::Select,
                crate::joypad::Button::Up,
                crate::joypad::Button::Down,
                crate::joypad::Button::Left,
                crate::joypad::Button::Right,
            ] {
                joypad.button_released(button);
            }

            // Press currently pressed buttons
            for button in pressed_buttons {
                joypad.button_pressed(button);
            }

            self.memory_bus.dmg_io.joypad = joypad;
        }

        self.input_check_counter = (self.input_check_counter + 1) % COUNTER_WRAP;

        let cpu_cycles = self.cpu.cycle(&mut self.memory_bus);

        if self.memory_bus.enabling_ime {
            self.memory_bus.interrupt_master_enable = true;
            self.memory_bus.enabling_ime = false;
        }

        let ppu_interrupts = self.memory_bus.dmg_io.ppu.tick(cpu_cycles);
        for interrupt in ppu_interrupts {
            self.memory_bus.trigger_interrupt(interrupt);
        }

        self.memory_bus.cycle(cpu_cycles);

        self.cpu.check_interrupts(&mut self.memory_bus);

        if self.previous_frame != self.memory_bus.dmg_io.ppu.current_frame {
            // Update display with the new frame buffer
            self.main_display.update(&self.memory_bus.dmg_io.ppu.framebuffer);

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

        if self.last_save_time.elapsed() > Duration::from_secs(AUTOSAVE_INTERVAL_SECS) {
            match self.memory_bus.mbc.as_mut().unwrap().save_ram() {
                Ok(_) => {},
                Err(e) => {
                    error!("Failed to auto-save SRAM data: {}", e);
                },
            }
            self.last_save_time = Instant::now();
        }
    }

}