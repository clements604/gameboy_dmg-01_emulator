use minifb::{Key, Scale, Window, WindowOptions};

const FPS: usize = 60;
const TILE_SIZE: usize = 8;
const BYTES_PER_TILE: usize = 16; // 2 bytes per row * 8 rows
const TOTAL_TILES: usize = 1024; // Total tiles for a 32x32 tile map
const WIDTH: usize = TILE_SIZE * 32; // 32 tiles wide
const HEIGHT: usize = TILE_SIZE * 32; // 32 tiles high
const DARKEST_GREEN: u32 = 0xFF0F380F;
const DARK_GREEN: u32 = 0xFF306230;
const LIGHT_GREEN: u32 = 0xFF8BAC0F;
const LIGHTEST_GREEN: u32 = 0xFF9BBC0F;
const RED: u32 = 0xFFFF0000;

/*
    * This struct is responsible for rendering the background tiles to the screen.
    * For debugging purposes only.
 */
pub struct BackgroundDisplay {
    pub window: minifb::Window,
    buffer: Vec<u32>,
}

impl BackgroundDisplay {
    pub fn new() -> BackgroundDisplay {
        let mut window = Window::new(
            "Gameboy DMG Emulator - ESC to exit",
            WIDTH,
            HEIGHT,
            WindowOptions {
                scale: Scale::X2,
                ..WindowOptions::default()
            },
        )
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });
        window.set_target_fps(FPS);

        BackgroundDisplay { window,
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

    pub fn render_tile_map_to_screen(
        &mut self,
        tile_map: &Vec<&[u8]>,
    ) {
        // Iterate over the 32x32 grid (1024 tiles total)
        for tile_y in 0..32 {
            for tile_x in 0..32 {
                // Calculate the index of the current tile in the tile_map
                let tile_index = tile_y * 32 + tile_x;

                // Get the tile data from the tile_map (16 bytes per tile)
                let tile_data = tile_map[tile_index];

                // Calculate the x and y offsets for this tile on the screen
                let x_offset = tile_x * TILE_SIZE;
                let y_offset = tile_y * TILE_SIZE;

                // Render this tile to the screen buffer
                self.render_tile_to_debug(x_offset, y_offset, tile_data);
            }
        }

        // After rendering all tiles, push the buffer to the minifb window
        self.window.update_with_buffer(&self.buffer, WIDTH, HEIGHT)
            .expect("Failed to update window");
    }

    pub fn render_tile_to_debug(
        &mut self,
        x_offset: usize,
        y_offset: usize,
        tile_data: &[u8],
    ) {
        for row in 0..TILE_SIZE {
            // Each tile consists of two bytes per row, LSB and MSB
            let lsb = tile_data[row * 2]; // Least significant byte
            let msb = tile_data[row * 2 + 1]; // Most significant byte

            for col in 0..TILE_SIZE {
                // Extract the bit for this pixel from the LSB and MSB
                let lsb_bit = (lsb >> (7 - col)) & 1; // Get the bit from the LSB
                let msb_bit = (msb >> (7 - col)) & 1; // Get the bit from the MSB
                let pixel_value = (msb_bit << 1) | lsb_bit; // Combine LSB and MSB to get the pixel value

                // Calculate where to place the pixel in the buffer
                let buffer_index = (y_offset + row) * WIDTH + (x_offset + col);

                // Check bounds to prevent out-of-bounds access
                if buffer_index < self.buffer.len() {
                    self.buffer[buffer_index] = self.get_mififb_colour(pixel_value);
                }
            }
        }
    }


    pub fn update(&mut self, tiles: &Vec<&[u8]>) {
        self.render_tile_map_to_screen(tiles);
        self.window.update_with_buffer(&self.buffer, WIDTH, HEIGHT).unwrap();
    }
}
