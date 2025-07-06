use std::collections::HashMap;
use log::error;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use std::ops::Index;
use std::time::Duration;
use crate::display::{SCREEN_HEIGHT, SCREEN_WIDTH};

const FPS: usize = 60;
pub(crate) const TILE_SIZE: usize = 8;
const BYTES_PER_TILE: usize = 16; // 2 bytes per row * 8 rows
const TOTAL_TILES: usize = 1024; // Total tiles for a 32x32 tile map
pub const WIDTH: usize = TILE_SIZE * 32; // 32 tiles wide
pub const HEIGHT: usize = TILE_SIZE * 32; // 32 tiles high
pub const VIEWPORT_WIDTH: usize = TILE_SIZE * 20; // 20 tiles wide
pub const VIEWPORT_HEIGHT: usize = TILE_SIZE * 18; // 18 tiles high
const DARKEST_GREEN: u32 = 0xFF0F380F;
const DARK_GREEN: u32 = 0xFF306230;
const LIGHT_GREEN: u32 = 0xFF8BAC0F;
pub(crate) const LIGHTEST_GREEN: u32 = 0xFF9BBC0F;
pub(crate) const RED: u32 = 0xFFFF0000;

pub struct MainDisplay {
    pub canvas: Canvas<Window>,
    pub event_pump: sdl2::EventPump,
    scale: u32,
    keys_to_check: [Keycode; 8],
    current_keys: Vec<Keycode>,  // Currently pressed keys
    key_state_changed: bool,     // Flag for optimization
}

impl MainDisplay {
    pub fn new() -> MainDisplay {

        let mut current_key_states = HashMap::new();

        // Initialize keys we care about
        current_key_states.insert(Keycode::Up, false);
        current_key_states.insert(Keycode::Down, false);
        current_key_states.insert(Keycode::Left, false);
        current_key_states.insert(Keycode::Right, false);
        current_key_states.insert(Keycode::A, false);
        current_key_states.insert(Keycode::B, false);
        current_key_states.insert(Keycode::Return, false);
        current_key_states.insert(Keycode::Backspace, false);
        
        let sdl_context = sdl2::init().unwrap_or_else(|e| {
            panic!("SDL initialization failed: {}", e);
        });

        let video_subsystem = sdl_context.video().unwrap_or_else(|e| {
            panic!("Video subsystem initialization failed: {}", e);
        });

        let scale = 2;
        let window = video_subsystem
            .window(
                "Gameboy DMG Emulator - ESC to exit",
                (SCREEN_WIDTH as u32) * scale,
                (SCREEN_HEIGHT as u32) * scale,
            )
            .position_centered()
            .build()
            .unwrap_or_else(|e| {
                panic!("Window creation failed: {}", e);
            });

        let mut canvas = window.into_canvas().build().unwrap_or_else(|e| {
            panic!("Canvas creation failed: {}", e);
        });

        // Set logical size to maintain correct aspect ratio with scaling
        canvas
            .set_logical_size(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
            .unwrap_or_else(|e| {
                panic!("Setting logical size failed: {}", e);
            });

        let event_pump = sdl_context.event_pump().unwrap_or_else(|e| {
            panic!("Event pump creation failed: {}", e);
        });

        let keys_to_check = [
            Keycode::A,      // A button
            Keycode::B,      // B button
            Keycode::Return, // Start
            Keycode::Space,  // Select
            Keycode::Up,     // Up
            Keycode::Down,   // Down
            Keycode::Left,   // Left
            Keycode::Right,  // Right
        ];

        MainDisplay {
            canvas,
            event_pump,

            scale,
            keys_to_check,
            current_keys: Vec::new(),
            key_state_changed: false,
        }
    }

    pub fn update(&mut self, tiles: Vec<u32>) {
        // Clear the screen
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();

        // Render tiles
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let index = y * SCREEN_WIDTH + x;
                if index < tiles.len() {
                    let color = get_sdl_colour(tiles[index]);
                    self.canvas.set_draw_color(color);
                    self.canvas.draw_point((x as i32, y as i32)).unwrap_or_else(|e| {
                        error!("Failed to draw point: {}", e);
                    });
                }
            }
        }

        // Present the rendered frame
        self.canvas.present();

        // Cap the frame rate
        //::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / FPS as u32)); // FIXME was 1_000_000_000u32 / FPS
    }

    pub fn process_events(&mut self) -> bool {
        false;
        let mut running = true;

        // Process window events (quit, etc.)
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => running = false,
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => running = false,
                _ => {}
            }
        }

        // Get current keyboard state (more reliable than tracking events)
        let keyboard_state = self.event_pump.keyboard_state();
        let mut new_keys = Vec::new();

        // Check each key we care about
        if keyboard_state.is_scancode_pressed(Scancode::Up) { new_keys.push(Keycode::Up); }
        if keyboard_state.is_scancode_pressed(Scancode::Down) { new_keys.push(Keycode::Down); }
        if keyboard_state.is_scancode_pressed(Scancode::Left) { new_keys.push(Keycode::Left); }
        if keyboard_state.is_scancode_pressed(Scancode::Right) { new_keys.push(Keycode::Right); }
        if keyboard_state.is_scancode_pressed(Scancode::A) { new_keys.push(Keycode::A); }
        if keyboard_state.is_scancode_pressed(Scancode::B) { new_keys.push(Keycode::B); }
        if keyboard_state.is_scancode_pressed(Scancode::Return) { new_keys.push(Keycode::Return); }
        if keyboard_state.is_scancode_pressed(Scancode::Backspace) { new_keys.push(Keycode::Backspace); }

        // Check if key state has changed
        self.key_state_changed = new_keys != self.current_keys;

        // Update current keys
        self.current_keys = new_keys;

        running
    }

    // Simplified get_pressed_keys
    pub fn get_pressed_keys(&self) -> &Vec<Keycode> {
        &self.current_keys
    }

    // Check if key state changed
    pub fn has_key_state_changed(&self) -> bool {
        self.key_state_changed
    }
}

pub fn get_sdl_colour(color_u32: u32) -> Color {
    // Extract RGB components from the u32 value
    let r = ((color_u32 >> 16) & 0xFF) as u8;
    let g = ((color_u32 >> 8) & 0xFF) as u8;
    let b = (color_u32 & 0xFF) as u8;

    Color::RGB(r, g, b)
}

// Replacement for the minifb get_mififb_colour function
pub fn get_gb_colour(palette: u8) -> u32 {
    match palette {
        0b00 => LIGHTEST_GREEN,
        0b01 => LIGHT_GREEN,
        0b10 => DARK_GREEN,
        0b11 => DARKEST_GREEN,
        _ => RED,
    }
}

// Helper function to convert palette index to SDL2 Color
pub fn get_palette_colour(palette: u8) -> Color {
    match palette {
        0b00 => sdl_from_u32(LIGHTEST_GREEN),
        0b01 => sdl_from_u32(LIGHT_GREEN),
        0b10 => sdl_from_u32(DARK_GREEN),
        0b11 => sdl_from_u32(DARKEST_GREEN),
        _ => Color::RGB(255, 0, 0), // RED
    }
}

// Helper function to convert u32 to SDL2 Color
fn sdl_from_u32(color_u32: u32) -> Color {
    let r = ((color_u32 >> 16) & 0xFF) as u8;
    let g = ((color_u32 >> 8) & 0xFF) as u8;
    let b = (color_u32 & 0xFF) as u8;
    Color::RGB(r, g, b)
}

// Helper function to convert SDL2 Color to u32
pub fn sdl_to_u32(color: Color) -> u32 {
    let (r, g, b) = color.rgb();
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}