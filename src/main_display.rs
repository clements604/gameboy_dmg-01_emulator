use std::collections::HashMap;
use log::error;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

pub struct MainDisplay {
    pub canvas: Canvas<Window>,
    pub event_pump: sdl2::EventPump,
    current_keys: Vec<Keycode>,
    key_state_changed: bool,
    tracked_keys: Vec<Keycode>,
}

impl MainDisplay {
    pub fn new(scale_factor: u32, key_bindings: &HashMap<String, Keycode>) -> MainDisplay {

        let mut current_key_states = HashMap::new();
        let tracked_keys: Vec<Keycode> = key_bindings.values().copied().collect();

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
        
        let window = video_subsystem
            .window(
                "Gameboy DMG Emulator - ESC to exit",
                (SCREEN_WIDTH as u32) * scale_factor,
                (SCREEN_HEIGHT as u32) * scale_factor,
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

        MainDisplay {
            canvas,
            event_pump,
            current_keys: Vec::new(),
            key_state_changed: false,
            tracked_keys,
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
                    let colour = get_sdl_colour(tiles[index]);
                    self.canvas.set_draw_color(colour);
                    self.canvas.draw_point((x as i32, y as i32)).unwrap_or_else(|e| {
                        error!("Failed to draw point: {}", e);
                    });
                }
            }
        }
        
        self.canvas.present();
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
        for keycode in &self.tracked_keys {
            if let Some(scancode) = Scancode::from_keycode(*keycode) {
                if keyboard_state.is_scancode_pressed(scancode) {
                    new_keys.push(*keycode);
                }
            }
        }

        // Check if key state has changed
        self.key_state_changed = new_keys != self.current_keys;

        // Update current keys
        self.current_keys = new_keys;

        running
    }
    
    pub fn get_pressed_keys(&self) -> &Vec<Keycode> {
        &self.current_keys
    }

}

pub fn get_sdl_colour(colour_u32: u32) -> Color {
    // Extract RGB components from the u32 value
    let r = ((colour_u32 >> 16) & 0xFF) as u8;
    let g = ((colour_u32 >> 8) & 0xFF) as u8;
    let b = (colour_u32 & 0xFF) as u8;

    Color::RGB(r, g, b)
}
