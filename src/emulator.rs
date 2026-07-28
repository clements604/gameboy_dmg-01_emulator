use std::time::{Duration, Instant};
use log::{debug, error};
use sdl2::keyboard::Keycode;
use crate::cpu;
use crate::emulator_config::EmulatorConfig;
use crate::display::MainDisplay;
use crate::memory_bus::MemoryBus;
use crate::rom::ROM;
use crate::interrupts::Interrupt::JOYPAD;
use crate::joypad::Button;

const FPS_60_MICROS: u64 = 16742;
const AUTOSAVE_INTERVAL_SECS: u64 = 5;
const BOOT_ROM_START: u16 = 0x0000;
const CART_ROM_START: u16 = 0x0100;

pub struct Emulator {
    cpu: cpu::CPU,

    memory_bus: MemoryBus,

    main_display: MainDisplay,

    previous_frame: u32,

    last_time: Instant, // Used for FPS calculation
    frame_count: u32, // Used for FPS calculation

    // Frame rate cap variables
    target_frame_time: Duration,
    last_frame_time: Instant,
    pub(crate) running: bool,
    last_save_time: Instant,

    key_bindings: std::collections::HashMap<String, Keycode>,
}

impl Emulator {
    pub fn new(emulator_config: EmulatorConfig, boot_rom: Option<Vec<u8>>, rom: &ROM) -> Emulator {

        let mut cpu = cpu::CPU::new();
        cpu.pc = if boot_rom.is_some() { BOOT_ROM_START } else { CART_ROM_START };
        let main_display = MainDisplay::new(emulator_config.scale_factor, &emulator_config.key_bindings);

        Emulator {
            cpu,
            memory_bus: MemoryBus::new(boot_rom, &rom),
            main_display,
            previous_frame: 0,
            last_time: Instant::now(),
            frame_count: 0,
            // Set target frame time to ~16.67ms (60 FPS)
            target_frame_time: Duration::from_micros(FPS_60_MICROS),
            last_frame_time: Instant::now(),
            running: true,
            last_save_time: Instant::now(),

            key_bindings: emulator_config.key_bindings,
        }
    }

    pub fn cycle(&mut self) {
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
            // Poll window/input events once per rendered frame (~60Hz) rather than
            // every CPU cycle. Polling hundreds of thousands of times per second
            // added no responsiveness and is expensive on macOS, where SDL's event
            // pump goes through the Cocoa run loop.
            self.process_input();

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

    fn process_input(&mut self) {
        if !self.main_display.process_events() {
            match self.memory_bus.mbc.as_mut() {
                Some(mbc) => {
                    if !mbc.dirty_sram() {
                        self.running = false;
                        std::process::exit(0);
                    } else {
                        error!("Emulator requested to stop but save SRAM is dirty, not stopping to prevent data loss");
                    }
                },
                None => {},
            }
            return;
        }

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

        let button_mappings = [
            ("up", Button::Up),
            ("down", Button::Down),
            ("left", Button::Left),
            ("right", Button::Right),
            ("a", Button::A),
            ("b", Button::B),
            ("start", Button::Start),
            ("select", Button::Select),
        ];

        for key in current_keys {
            for (binding_name, button) in &button_mappings {
                if Some(key) == self.key_bindings.get(*binding_name) {
                    joypad.button_pressed(*button);
                    pressed = true;
                }
            }
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

}