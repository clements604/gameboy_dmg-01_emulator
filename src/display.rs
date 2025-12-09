use minifb::{Window, WindowOptions, Key};
use log::error;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;
const RED_SHIFT: u32 = 16;
const GREEN_SHIFT: u32 = 8;
const COLOR_MASK: u32 = 0xFF;
const DEFAULT_WINDOW_TITLE: &str = "Gameboy DMG Emulator - ESC to exit";

use std::collections::HashMap;
use crate::joypad::Button;

pub struct MainDisplay {
    pub window: Window,
    pub buffer: Vec<u32>,
}

impl MainDisplay {
    /// Returns a Vec of pressed Game Boy buttons based on the provided key_bindings
    pub fn get_pressed_buttons(&self, key_bindings: &HashMap<String, Key>) -> Vec<Button> {
        let mut pressed = Vec::new();
        let keys = self.window.get_keys();
        for (name, key) in key_bindings.iter() {
            if keys.contains(key) {
                match name.as_str() {
                    "a" => pressed.push(Button::A),
                    "b" => pressed.push(Button::B),
                    "start" => pressed.push(Button::Start),
                    "select" => pressed.push(Button::Select),
                    "up" => pressed.push(Button::Up),
                    "down" => pressed.push(Button::Down),
                    "left" => pressed.push(Button::Left),
                    "right" => pressed.push(Button::Right),
                    _ => {},
                }
            }
        }
        pressed
    }

    pub fn new(scale_factor: usize) -> MainDisplay {
        let width = SCREEN_WIDTH * scale_factor;
        let height = SCREEN_HEIGHT * scale_factor;
        let window = Window::new(
            DEFAULT_WINDOW_TITLE,
            width,
            height,
            WindowOptions {
                resize: false,
                scale: minifb::Scale::X1,
                ..WindowOptions::default()
            },
        ).unwrap_or_else(|e| panic!("Window creation failed: {}", e));
        let buffer = vec![0; SCREEN_WIDTH * SCREEN_HEIGHT];
        MainDisplay { window, buffer }
    }

    pub fn update(&mut self, tiles: &[u32]) {
        for (i, &pixel) in tiles.iter().enumerate().take(self.buffer.len()) {
            self.buffer[i] = pixel;
        }
        self.window.update_with_buffer(&self.buffer, SCREEN_WIDTH, SCREEN_HEIGHT).unwrap_or_else(|e| {
            error!("Failed to update window buffer: {}", e);
        });
    }

    pub fn process_events(&mut self) -> bool {
        self.window.is_open() && !self.window.is_key_down(minifb::Key::Escape)
    }
}
// ...existing code...

#[allow(dead_code)]
pub fn rgb_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << RED_SHIFT) | ((g as u32) << GREEN_SHIFT) | (b as u32)
}
