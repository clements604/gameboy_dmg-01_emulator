use std::collections::HashMap;
use log::error;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

// Color bit shifts
const RED_SHIFT: u32 = 16;
const GREEN_SHIFT: u32 = 8;
const COLOR_MASK: u32 = 0xFF;

// Window scaling
const DEFAULT_WINDOW_TITLE: &str = "Gameboy DMG Emulator - ESC to exit";

// RGB color constants
const BLACK: Color = Color::RGB(0, 0, 0);

pub struct MainDisplay {
    pub canvas: Canvas<Window>,
    pub event_pump: sdl2::EventPump,
    current_keys: Vec<Keycode>,
    key_state_changed: bool,
    tracked_keys: Vec<Keycode>,
    // Reused every frame to group pixels by colour so they can be drawn with a
    // single `draw_points` call per colour instead of one `draw_point` call per pixel.
    points_by_colour: HashMap<u32, Vec<Point>>,
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

        // On macOS, a plain (non app-bundled) executable has no Info.plist, so
        // without this it launches with NSApplicationActivationPolicyProhibited:
        // no Dock icon, and the window is created but never becomes key/frontmost
        // (it stays behind whatever launched it, e.g. a terminal or editor).
        macos_activate_app();

        let window = video_subsystem
            .window(
                DEFAULT_WINDOW_TITLE,
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

        let mut event_pump = sdl_context.event_pump().unwrap_or_else(|e| {
            panic!("Event pump creation failed: {}", e);
        });

        // Pump once so Cocoa finishes processing the activation request above
        // before we ask the window to raise itself.
        event_pump.pump_events();
        canvas.window_mut().raise();

        MainDisplay {
            canvas,
            event_pump,
            current_keys: Vec::new(),
            key_state_changed: false,
            tracked_keys,
            points_by_colour: HashMap::new(),
        }
    }

    pub fn update(&mut self, tiles: Vec<u32>) {
        // Clear the screen
        self.canvas.set_draw_color(BLACK);
        self.canvas.clear();

        // Group pixels by colour (the DMG palette only has 4 shades) so each
        // colour can be drawn with a single `draw_points` call instead of one
        // `draw_point` call per pixel. SDL2's hardware-accelerated renderers
        // (notably macOS's Metal backend) have very high per-call overhead,
        // so drawing 23,040 individual points tanks the frame rate.
        for bucket in self.points_by_colour.values_mut() {
            bucket.clear();
        }

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let index = y * SCREEN_WIDTH + x;
                if index < tiles.len() {
                    self.points_by_colour
                        .entry(tiles[index])
                        .or_insert_with(Vec::new)
                        .push(Point::new(x as i32, y as i32));
                }
            }
        }

        for (&colour_value, points) in self.points_by_colour.iter() {
            if points.is_empty() {
                continue;
            }
            self.canvas.set_draw_color(get_sdl_colour(colour_value));
            self.canvas.draw_points(points.as_slice()).unwrap_or_else(|e| {
                error!("Failed to draw points: {}", e);
            });
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
    let r = ((colour_u32 >> RED_SHIFT) & 0xFF) as u8;
    let g = ((colour_u32 >> GREEN_SHIFT) & 0xFF) as u8;
    let b = (colour_u32 & COLOR_MASK) as u8;

    Color::RGB(r, g, b)
}

#[cfg(target_os = "macos")]
fn macos_activate_app() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    app.activate();
}

#[cfg(not(target_os = "macos"))]
fn macos_activate_app() {}
