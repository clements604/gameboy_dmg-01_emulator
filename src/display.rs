extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::video::{Window, WindowContext};

use crate::{memory_bus, ppu};
use crate::memory_bus::MemoryBus;
use log::{debug, error, info};

use sdl2::render::TextureCreator;
use sdl2::render::{Canvas, Texture, WindowCanvas};
use std::cell::RefCell;
use std::rc::Rc;
use sdl2::surface;

const SCALE_FACTOR: u32 = 3;
const B_W_TOGGLE: bool = false;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

pub const COLOUR_WHITE: u8 = 0b11;
pub const COLOUR_LIGHT_GREY: u8 = 0b10;
pub const COLOUR_DARK_GREY: u8 = 0b01;
pub const COLOUR_BLACK: u8 = 0b00;

pub struct Display /*<'a>*/ {
    //memory_bus: &'a mut MemoryBus,
    memory_bus: Rc<RefCell<MemoryBus>>,
    pub sdl_context: sdl2::Sdl,
    main_window: sdl2::render::Canvas<Window>,

    //debug_window: Option<Window>,
    debug_renderer: Option<WindowCanvas>,
    debug_texture_creator: Option<TextureCreator<WindowContext>>,
    debug_texture: Option<Texture<'static>>,
    debug_screen: Option<sdl2::surface::Surface<'static>>,

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

impl Display /*<'a>*/ {
    pub fn new(
        title: &String,
        memory_bus: Rc<RefCell<MemoryBus>>,
        window_width: u32,
        window_height: u32,
    ) -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        let main_window = video_subsystem
            .window(
                title,
                window_width * SCALE_FACTOR,
                window_height * SCALE_FACTOR,
            )
            .position_centered()
            .opengl()
            .build()
            .unwrap();
        let mut main_canvas: sdl2::render::Canvas<Window> =
            main_window.into_canvas().build().unwrap();
        main_canvas.set_scale(10.0, 10.0).unwrap();
        main_canvas.set_draw_color(Color::RGB(0, 0, 0));
        main_canvas.clear();
        main_canvas.present();

        // Debug window start
        let debug_window = video_subsystem
            //.window("Tile debugger", 16 * 8 * SCALE_FACTOR, 32 * 8 * SCALE_FACTOR)
            .window("Tile debugger", (16 * 8 * SCALE_FACTOR) + (16 * SCALE_FACTOR), (32 * 8 * SCALE_FACTOR) + (64 * SCALE_FACTOR))
            .position_centered()
            .build()
            .unwrap();

        let debug_renderer = debug_window.into_canvas().build().unwrap();
        
        let debug_screen = sdl2::surface::Surface::new(
            (16 * 8 * SCALE_FACTOR) + (16 * SCALE_FACTOR),
            (32 * 8 * SCALE_FACTOR) + (64 * SCALE_FACTOR),
            sdl2::pixels::PixelFormatEnum::ARGB8888,
        )
        .unwrap();
        

        Display {
            memory_bus: memory_bus,
            sdl_context: sdl_context,
            main_window: main_canvas,
            //debug_window: Some(debug_window),
            debug_texture_creator: match Some(&debug_renderer) {
                Some(renderer) => Some(renderer.texture_creator()),
                None => None,
            },
            debug_texture: None,
            debug_renderer: Some(debug_renderer),
            debug_screen: Some(debug_screen),
            window_width,
            window_height,
            framebuffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT * 3],
            framebuffer_a: [0; SCREEN_WIDTH * SCREEN_HEIGHT * 4],
        }
    }

    pub fn redraw(&mut self, video_buffer: &[u8]) {
        self.main_window.set_draw_color(Color::RGB(0, 0, 0));
        self.main_window.clear();
        self.main_window.set_draw_color(Color::RGB(255, 255, 255));

        let mut x = 0; // x position of the pixel
        let mut y = 0; // y position of the pixel

        let display_divisor: i32 = (self.window_width as i32 - 1).into();

        for pixel in video_buffer.iter() {
            if *pixel != 0 {
                self.main_window.draw_point(Point::new(x, y)).unwrap();
            }
            if x != 0 && (x % display_divisor) == 0 {
                x = 0;
                y += 1;
            } else {
                x += 1;
            }
        }

        self.main_window.present();
    }

    pub fn ui_update(&mut self) {
        self.update_debug_window();
    }
    fn update_debug_window(&mut self) {
        if let Some(renderer) = self.debug_renderer.as_mut() {
            //renderer.clear();//FIXME possibly unneeded and may cause bugs
            let surface = self.debug_screen.as_mut().unwrap();

            //error!("{:?}", self.memory_bus.borrow().ppu.vram);
            //renderer.clear();
            //renderer.present();
            /*let debug_texture_creator = renderer.texture_creator();

            let debug_texture = Some(
                debug_texture_creator
                    .create_texture_streaming(
                        sdl2::pixels::PixelFormatEnum::ARGB8888,
                        (16 * 8 * 1) + (16 * 1), // TODO: scale hardcoded as 1
                        (32 * 8 * 1) + (64 * 1), // TODO: scale hardcoded as 1
                    )
                    .unwrap(),
            );*/

            let (width, height) = renderer.window().drawable_size();
            //FIXME this does nothing!
            let rectangle = Rect::new(0, 0, width, height);
            renderer.set_draw_color(Color::RGB(17, 17, 17));
            surface.fill_rect(rectangle, Color::RGB(17, 17, 17)).unwrap();

            let start_address = 0x8000;
            let mut x_draw = 0;
            let mut y_draw = 0;
            let mut tile_number = 0;

            // Create a separate block to limit the mutable borrow scope

            for y in 0..=23 {
                for x in 0..=15 {
                    //self.display_tile(tile_number, address, x_draw + (x * 1), y_draw + (y * 1)); // TODO: scale hardcoded as 1

                    /* TEMP MOVING display_tile code here because of borrow checker*/

                    /*debug!(
                        "Displaying tile for tile number: {}, x: {}, y: {}",
                        tile_number, x, y
                    );*/

                    let x_coord = x_draw + (x * SCALE_FACTOR) as i32;
                    let y_coord = y_draw + (y * SCALE_FACTOR) as i32;

                    let mut rectangle = sdl2::rect::Rect::new(
                        x_coord, y_coord, SCALE_FACTOR,
                        SCALE_FACTOR,
                    );

                    //let mut rectangle = sdl2::rect::Rect::new(x_draw, y_draw, 8, 8);
                    for tile_y in (0..=15).step_by(2) {

                        //debug!("address: {:4X}", address);
                        let b1 = self.memory_bus.borrow().read_byte(
                            (start_address as u16 + (tile_number as u16 * 16) + tile_y as u16)
                                as u16,
                        );
                        let b2 = self.memory_bus.borrow().read_byte(
                            (start_address as u16 + (tile_number as u16 * 16) + tile_y as u16 + 1)
                                as u16,
                        );
                        for bit in (0..=7).rev() {
                            let hi = (b1 & (1 << bit)) >> bit << 1;
                            let lo = (b2 & (1 << bit)) >> bit;
                                                          //debug!("hi: {:X}, lo: {:X}", hi, lo);
                            let colour = hi | lo;
                            let rgb_color = match colour {
                                
                                0 => Color::RGB(255, 255, 255), // White
                                1 => Color::RGB(192, 192, 192), // Light Gray
                                2 => Color::RGB(96, 96, 96),    // Dark Gray
                                /*
                                    0 => Color::RGB(232, 252, 204), // White
                                    1 => Color::RGB(84, 140, 112), // Light Gray
                                    2 => Color::RGB(20, 44, 56),    // Dark Gray
                                */
                                3 => Color::RGB(0, 0, 0),       // Black
                                _ => Color::RGB(255, 0, 0),     // Red (error case)
                            };
                            renderer.set_draw_color(rgb_color);

                            /*let rectangle = sdl2::rect::Rect::new(
                                x_coord + ((7 - bit) * 1),// TODO: scale hardcoded as 1
                                y_coord + (tile_y / 2 * 1) as i32,// TODO: scale hardcoded as 1
                                1,
                                1,
                            );*/
                            //error!("x_coord: {}, y_coord: {}", x_coord + ((7 - bit) * 1), y_coord + (tile_y / 2 * 1) as i32);
                            rectangle.set_x(x_coord + ((7 - bit) * SCALE_FACTOR) as i32);
                            rectangle.set_y(y_coord + (tile_y / 2 * SCALE_FACTOR) as i32);
                            rectangle.set_width(SCALE_FACTOR);
                            rectangle.set_height(SCALE_FACTOR);

                            surface.fill_rect(rectangle, rgb_color).unwrap();
                        }
                    }

                    x_draw += (8 * SCALE_FACTOR) as i32;
                    tile_number += 1;
                }
                y_draw += (8 * SCALE_FACTOR) as i32;
                x_draw = 0;
            }

            renderer.clear();

            if let Some(debug_screen) = self.debug_screen.as_ref() {
                let mut debug_texture = self
                    .debug_texture_creator
                    .as_ref()
                    .unwrap()
                    .create_texture_from_surface(&debug_screen)
                    .unwrap();
                debug_screen.with_lock(|debug_screen_pixels: &[u8]| {
                    let debug_screen_pitch = debug_screen.pitch();
                    debug_texture
                        .update(None, debug_screen_pixels, debug_screen_pitch as usize)
                        .unwrap();
                    renderer.copy(&debug_texture, None, None).unwrap();
                });

                //renderer.clear();
                renderer.present();
            }
            //self.debug_renderer.as_mut().present();
        }
    }
    
    pub fn get_ticks(&self) -> u32 {
        // Get number of ticks for the main window
        self.sdl_context.timer().unwrap().ticks()
    }
    pub fn delay(&self, ms: u32) {
        self.sdl_context.timer().unwrap().delay(ms);
    }

    /*fn display_tile(&self, start_location: u16, tile_number: u16, x: i32, y: i32) {
        debug!(
            "Displaying tile for tile number: {}, x: {}, y: {}",
            tile_number, x, y
        );

        //let mut rectangle = sdl2::rect::Rect::new(x, y, 8, 8);
        for tile_y in (0..16).step_by(2) {
            let address = start_location as u32 + (tile_number as u32 * 16) + tile_y as u32;
            let b1 = self.memory_bus.borrow().read_byte(address as u16);
            let b2 = self.memory_bus.borrow().read_byte((address + 1) as u16);
            for bit in (0..8).rev() {
                let hi = !!(b1 & (1 << bit)) << 1;
                let lo = !!(b2 & (1 << bit));

                let colour = hi | lo;
                let rgb_color = match colour {
                    0 => Color::RGB(255, 255, 255), // White
                    1 => Color::RGB(192, 192, 192), // Light Gray
                    2 => Color::RGB(96, 96, 96),    // Dark Gray
                    3 => Color::RGB(0, 0, 0),       // Black
                    _ => Color::RGB(255, 0, 0),     // Red (error case)
                };
                self.debug_renderer
                    .as_mut()
                    .unwrap()
                    .set_draw_color(rgb_color);

                let mut rectangle = sdl2::rect::Rect::new(
                    x + ((7 - bit) * 1),
                    y + (tile_y / 2 * 1) as i32,
                    1, //TODO scale hardcoded as 1
                    1,
                ); //TODO scale hardcoded as 1

                /*rectangle.x = x + ((7 - bit) * 1); //TODO scale hardcoded as 1
                rectangle.y = y + (tile_y / 2 * 1) as i32; //TODO scale hardcoded as 1
                rectangle.set_width(1);*/

                self.debug_renderer
                    .as_mut()
                    .unwrap()
                    .fill_rect(rectangle)
                    .unwrap();
            }
        }
    }*/
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{memory_bus, rom};

    //#[test]
    /*fn test_tiles() {
        let mut display = Display::new(
            &String::from("Tile Map"),
            16 * 8 as u32,
            32 * 8 as u32,
        );

    }*/
}
