use minifb::{Key, Scale, Window, WindowOptions};

const FPS: usize = 60;
const TILE_SIZE: usize = 8;
const BYTES_PER_TILE: usize = 16; // 2 bytes per row * 8 rows
const TOTAL_TILES: usize = 384; // Total tiles for a 16x24 tile map
const WIDTH: usize = TILE_SIZE * 16; // 16 tiles wide
const HEIGHT: usize = TILE_SIZE * 24; // 24 tiles high
const DARKEST_GREEN: u32 = 0xFF0F380F;
const DARK_GREEN: u32 = 0xFF306230;
const LIGHT_GREEN: u32 = 0xFF8BAC0F;
const LIGHTEST_GREEN: u32 = 0xFF9BBC0F;
const RED: u32 = 0xFFFF0000;
pub struct DebugDisplay {
    pub window: minifb::Window,
    buffer: Vec<u32>,
}

impl DebugDisplay {
    pub fn new() -> DebugDisplay {
        let mut window = Window::new(
            "VRAM Viewer - ESC to exit",
            WIDTH,
            HEIGHT,
            WindowOptions {
                scale: Scale::X4,
                ..WindowOptions::default()
            },
        )
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });
        window.set_target_fps(FPS);

        DebugDisplay { window,
            buffer: vec![RED; WIDTH * HEIGHT]}
    }
    fn get_mififb_colour(&mut self, palette: u8) -> u32 {
        match palette {
            0b00 => LIGHTEST_GREEN,
            0b01 => LIGHT_GREEN,
            0b10 => DARK_GREEN,
            0b11 => DARKEST_GREEN,
            _ => RED,
        }
    }
    fn render_tile_to_debug(
        &mut self,
        x_offset: usize,
        y_offset: usize,
        tile_data: &[u8],
    ) {
        for row in 0..TILE_SIZE {
            let lsb = tile_data[row * 2];
            let msb = tile_data[row * 2 + 1];

            for col in 0..TILE_SIZE {
                // Extract the bit for this pixel
                let lsb_bit = (lsb >> (7 - col)) & 1;
                let msb_bit = (msb >> (7 - col)) & 1;
                let pixel_value = (msb_bit << 1) | lsb_bit; // Combine LSB and MSB

                // Where in the array buffer to place the pixel.
                // Multiple tiles per line means each tile's data is not contiguous.
                let buffer_index = (y_offset + row) * WIDTH + (x_offset + col);

                self.buffer[buffer_index] = self.get_mififb_colour(pixel_value);
            }
        }
    }
    fn render_tile_map(&mut self, tiles: &[&[u8]]) {
        self.buffer = vec![crate::tile_map_display::RED; crate::tile_map_display::WIDTH * crate::tile_map_display::HEIGHT];
        for (index, tile) in tiles.iter().enumerate() {
            let tile_x = (index % 16) * TILE_SIZE;
            let tile_y = (index / 16) * TILE_SIZE;
            self.render_tile_to_debug(tile_x, tile_y, tile);
        }
    }
    
    pub fn update(&mut self, tiles: &Vec<&[u8]>) {
        self.render_tile_map(tiles);
        self.window.update_with_buffer(&self.buffer, WIDTH, HEIGHT).unwrap();
    }
}
