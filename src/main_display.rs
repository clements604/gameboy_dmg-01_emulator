use std::ops::Index;
use log::error;
use minifb::{Key, Scale, Window, WindowOptions};
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
    pub window: minifb::Window,
    buffer: Vec<u32>,
}

impl MainDisplay {
    pub fn new() -> MainDisplay {
        let mut window = Window::new(
            "Gameboy DMG Emulator - ESC to exit",
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
            WindowOptions {
                scale: Scale::X2,
                ..WindowOptions::default()
            },
        )
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });
        window.set_target_fps(FPS);

        MainDisplay { window,
            buffer: vec![RED; SCREEN_WIDTH * SCREEN_HEIGHT]}
    }
    
    

    pub fn update(&mut self, tiles: Vec<u32>/*&Vec<u32>*/) {
        //self.render_tile_map_to_screen(tiles);
        self.window.update_with_buffer(&tiles, SCREEN_WIDTH, SCREEN_HEIGHT).unwrap();
    }

}

pub fn get_mififb_colour(palette: u8) -> u32 {
    match palette {
        0b00 => LIGHTEST_GREEN,
        0b01 => LIGHT_GREEN,
        0b10 => DARK_GREEN,
        0b11 => DARKEST_GREEN,
        _ => RED,
    }
}