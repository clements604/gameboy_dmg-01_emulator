use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

const WINDOW_TITLE: &str = "APU Debug";
const WINDOW_WIDTH: u32 = 160;
const WINDOW_HEIGHT: u32 = 240;
const BAR_COUNT: u32 = 4;
const BAR_GAP: u32 = 8;
const MAX_AMPLITUDE: u32 = 15; // channels output a 4-bit (0-15) amplitude

const BACKGROUND: Color = Color::RGB(20, 20, 20);
const BAR_COLOUR: Color = Color::RGB(0, 220, 0);

// Debug-only: a small window showing each APU channel's current amplitude
// (0-15) as a vertical bar, so channel behaviour can be sanity-checked
// before real audio output (Phase 0) exists. Not compiled into release builds.
pub struct DebugDisplay {
    canvas: Canvas<Window>,
}

impl DebugDisplay {
    // `main_window` is used only to position this window immediately to its right.
    pub fn new(sdl_context: &sdl2::Sdl, main_window: &Window) -> DebugDisplay {
        let video_subsystem = sdl_context.video().unwrap_or_else(|e| {
            panic!("Video subsystem initialization failed: {}", e);
        });

        let (main_x, main_y) = main_window.position();
        let (main_width, _) = main_window.size();

        let window = video_subsystem
            .window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
            .position(main_x + main_width as i32, main_y)
            .build()
            .unwrap_or_else(|e| {
                panic!("Debug window creation failed: {}", e);
            });

        let canvas = window.into_canvas().build().unwrap_or_else(|e| {
            panic!("Debug canvas creation failed: {}", e);
        });

        DebugDisplay { canvas }
    }

    pub fn update(&mut self, channel_amplitudes: [u8; 4]) {
        self.canvas.set_draw_color(BACKGROUND);
        self.canvas.clear();

        let bar_width = (WINDOW_WIDTH - BAR_GAP * (BAR_COUNT + 1)) / BAR_COUNT;

        self.canvas.set_draw_color(BAR_COLOUR);
        for (i, &amplitude) in channel_amplitudes.iter().enumerate() {
            let bar_height = (amplitude as u32 * WINDOW_HEIGHT) / MAX_AMPLITUDE;
            let x = (BAR_GAP + i as u32 * (bar_width + BAR_GAP)) as i32;
            let y = (WINDOW_HEIGHT - bar_height) as i32;
            let rect = Rect::new(x, y, bar_width, bar_height);
            self.canvas.fill_rect(rect).unwrap_or_else(|e| {
                log::error!("Failed to draw debug bar: {}", e);
            });
        }

        self.canvas.present();
    }
}
