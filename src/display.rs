extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::video::Window;

use log::{debug, error, info};

const SCALE_FACTOR: u32 = 1;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

pub const COLOUR_WHITE: u8 = 0b11;
pub const COLOUR_LIGHT_GREY: u8 = 0b10;
pub const COLOUR_DARK_GREY: u8 = 0b01;
pub const COLOUR_BLACK: u8 = 0b00;

pub struct Display {
    pub sdl_context: sdl2::Sdl,
    canvas: sdl2::render::Canvas<Window>,
    window_width: u32,
    window_height: u32,
    framebuffer: [u8; SCREEN_WIDTH * SCREEN_HEIGHT * 3], // 3 bytes per pixel, RGB24
    framebuffer_a: [u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4], // 4 bytes per pixel, RGBA24 necessary due to DMG drawing with layers
}

#[derive(Copy, Clone)]
pub enum TilePixelValue {
    White,
    LightGrey,
    DarkGrey,
    Black,
}

type Tile = [[TilePixelValue; 8]; 8];
fn blank_tile() -> Tile {
    [[TilePixelValue::White; 8]; 8]
}

pub struct GPU {
    pub vram: [u8; 8192],      // 8KB of VRAM
    pub tile_set: [Tile; 384], // 384 tiles, 16 bytes each
}

impl GPU {
    pub fn new() -> Self {
        GPU {
            vram: [0; 8192],
            tile_set: [blank_tile(); 384],
        }
    }

    pub fn read_short(&self, address: u16) -> u16 {
        assert!(
            address >= 0x8000 && address < 0x9FFF,
            "Address out of VRAM range"
        );
        let vram_index = (address - 0x8000) as usize;
        let lsb = self.vram[vram_index];
        let msb = self.vram[vram_index.wrapping_add(1)];
        debug!(
            "Read VRAM address {:X} value {:X}",
            address,
            (msb as u16) << 8 | lsb as u16
        );
        (msb as u16) << 8 | lsb as u16
        //self.vram[vram_index]
    }

    pub fn write_short(&mut self, address: u16, value: u16) {
        panic!("VRAM write short not implemented");
        assert!(
            address >= 0x8000 && address <= 0x9FFF,
            "Address out of VRAM range"
        );
        let vram_index = (address - 0x8000) as usize;
        let lsb = (value & 0x00FF) as u8;
        let msb = ((value & 0xFF00) >> 8) as u8;
        info!(
            "Write VRAM address {:X} value {:X}",
            address,
            (msb as u16) << 8 | lsb as u16
        );
        self.vram[vram_index] = lsb;
        self.vram[vram_index.wrapping_add(1)] = msb;
    }
}

impl Display {
    pub fn new(title: &String, window_width: u32, window_height: u32) -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        let window = video_subsystem
            .window(
                title,
                window_width * SCALE_FACTOR,
                window_height * SCALE_FACTOR,
            )
            .position_centered()
            .opengl()
            .build()
            .unwrap();

        let mut canvas: sdl2::render::Canvas<Window> = window.into_canvas().build().unwrap();

        canvas.set_scale(10.0, 10.0).unwrap();
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        Display {
            sdl_context: sdl_context,
            canvas: canvas,
            window_width: window_width,
            window_height: window_height,
            framebuffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT * 3],
            framebuffer_a: [0; SCREEN_WIDTH * SCREEN_HEIGHT * 4],
        }
    }

    pub fn redraw(&mut self, video_buffer: &[u8]) {
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();
        self.canvas.set_draw_color(Color::RGB(255, 255, 255));

        let mut x = 0; // x position of the pixel
        let mut y = 0; // y position of the pixel

        let display_divisor: i32 = (self.window_width as i32 - 1).into();

        for pixel in video_buffer.iter() {
            if *pixel != 0 {
                self.canvas.draw_point(Point::new(x, y)).unwrap();
            }
            if x != 0 && (x % display_divisor) == 0 {
                x = 0;
                y += 1;
            } else {
                x += 1;
            }
        }

        self.canvas.present();
    }
}
