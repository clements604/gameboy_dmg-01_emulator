extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::video::Window;

const SCALE_FACTOR: u32 = 1;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

pub struct Display {
    pub sdl_context: sdl2::Sdl,
    canvas: sdl2::render::Canvas<Window>,
    window_width: u32,
    #[allow(dead_code)]
    window_height: u32,
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
