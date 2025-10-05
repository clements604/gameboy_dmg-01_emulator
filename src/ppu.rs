use crate::interupts::Interrupt;
use crate::lcd::LCD;
use crate::lcd::LIGHTEST_GREEN;
use bitflags::bitflags;
use log::{debug, error};
use std::fmt;

const TILE_MAP_0_START: u16 = 0x9800;
const TILE_MAP_1_START: u16 = 0x9C00;
const OAM_MODE: u8 = 2;
const VRAM_MODE: u8 = 3;
const HBLANK_MODE: u8 = 0;
const VBLANK_MODE: u8 = 1;
const LINES_PER_FRAME: u8 = 153;
const TICKS_PER_LINE: u16 = 456;
const Y_RES: usize = 144;
const X_RES: usize = 160;
const TILE_SIZE_BYTES: usize = 16;
const MAX_SPRITES_PER_LINE: usize = 10;
const TILE_WIDTH: usize = 8;
const TILE_HEIGHT_EIGHT: usize = 8;
const TILE_HEIGHT_SIXTEEN: usize = 16;
const TILE_SIZE_EIGHT: usize = 16;
const TILE_SIZE_SIXTEEN: usize = 32;
const UNSIGNED_TILE_START: u16 = 0x8000;
const SIGNED_TILE_START: u16 = 0x8800;
const LYC_COINCIDENCE_FLAG: u8 = 0x04;
const BG_TILEMAP_SIZE: usize = 1024;
const VRAM_SIZE: usize = 0x2000; // 8KB VRAM
const OAM_SIZE: usize = 0xA0; // 160 bytes OAM
const SIGNED_TILE_OFFSET: u8 = 128;
const BG_MAP_SIZE_PIXELS: usize = 256;

pub struct Ppu {
    oam_ram: [u8; OAM_SIZE],
    pub vram: [u8; VRAM_SIZE],
    pub mode: u8,
    line_ticks: u16,
    pub current_frame: u32,
    previous_frame_time: u32,
    start_time: u32,
    frame_count: u16,
    pub lcd: LCD,
    pub lcdc: u8,
    pub stat: u8,
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub ly: u8,
    pub ly_compare: u8,
    window_line_counter: u8,
    pub framebuffer: Vec<u32>,
    interrupts: Vec<Interrupt>,
    prev_stat_line: bool,
}

impl fmt::Display for Ppu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "LY: {:2X},\
            LYC: {:2X},\
            Mode: {},\
            Line Ticks: {},\
            Current Frame: {},\
            Previous Frame Time: {},\
            Start Time: {},\
            Frame Count: {},\
            LCDC: {:2X},\
            STAT: {:2X},\
            Scroll X: {},\
            Scroll Y: {}",
            self.ly,
            self.ly_compare,
            self.mode,
            self.line_ticks,
            self.current_frame,
            self.previous_frame_time,
            self.start_time,
            self.frame_count,
            self.lcdc,
            self.stat,
            self.scroll_x,
            self.scroll_y
        )
    }
}

#[derive(Clone)]
pub struct Sprite {
    y: u8,
    x: u8,
    tile_number: u8,
    flags: OAMFlags,
}

enum StatInterrupt {
    LYC = 0x40,
    OAM = 0x20,
    VBLANK = 0x10,
    HBLANK = 0x08,
}

bitflags! {
    #[derive(Clone)]
    pub struct OAMFlags: u8 {
        const PRIORITY     = 0b1000_0000;
        const Y_FLIP       = 0b0100_0000;
        const X_FLIP       = 0b0010_0000;
        const DMG_PALETTE  = 0b0001_0000;
        const BANK         = 0b0000_1000;
    }
}

impl fmt::Display for OAMFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Priority: {}, Y Flip: {}, X Flip: {}, DMG Palette: {}, Bank: {}",
            if self.contains(OAMFlags::PRIORITY) {
                "1"
            } else {
                "0"
            },
            if self.contains(OAMFlags::Y_FLIP) {
                "1"
            } else {
                "0"
            },
            if self.contains(OAMFlags::X_FLIP) {
                "1"
            } else {
                "0"
            },
            if self.contains(OAMFlags::DMG_PALETTE) {
                "1"
            } else {
                "0"
            },
            if self.contains(OAMFlags::BANK) {
                "1"
            } else {
                "0"
            },
        )
    }
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            oam_ram: [0; OAM_SIZE],
            vram: [0x0000; VRAM_SIZE],
            mode: OAM_MODE,
            line_ticks: 0,
            current_frame: 0,
            previous_frame_time: 0,
            start_time: 0,
            frame_count: 0,
            lcd: LCD::new(),
            lcdc: 0x91,
            stat: 0x85,
            scroll_x: 0,
            scroll_y: 0,
            ly: 0,
            ly_compare: 0,
            window_line_counter: 0,
            framebuffer: vec![LIGHTEST_GREEN; X_RES * Y_RES],
            interrupts: Vec::new(),
            prev_stat_line: false,
        }
    }

    pub fn lcd_ppu_enabled(&self) -> bool {
        self.lcdc & 0x80 != 0
    }

    fn window_enabled(&self) -> bool {
        self.lcdc & 0x20 != 0
    }
    fn sprite_size(&self) -> u8 {
        if self.lcdc & LYC_COINCIDENCE_FLAG != 0 {
            TILE_HEIGHT_SIXTEEN as u8
        } else {
            TILE_HEIGHT_EIGHT as u8
        }
    }

    fn set_ppu_mode(&mut self, mode: u8) {
        self.stat = (self.stat & 0xFC) | (mode & 0x03); // Update only mode bits
    }

    fn sprites_enabled(&self) -> bool {
        self.lcdc & 0x02 != 0
    }

    fn is_background_enabled(&self) -> bool {
        (self.lcdc & 0x01) != 0
    }

    fn increment_ly(&mut self) {
        self.ly = self.ly.wrapping_add(1);

        // Check for LYC coincidence
        if self.ly == self.ly_compare {
            self.stat |= LYC_COINCIDENCE_FLAG; // Set coincidence flag
            if self.is_stat_interrupt_enabled(StatInterrupt::LYC) {
                self.update_stat_interrupts();
            }
        } else {
            self.stat &= !LYC_COINCIDENCE_FLAG;
        }

        if self.ly >= LINES_PER_FRAME {
            self.ly = 0;
            self.window_line_counter = 0;
            self.current_frame += 1;
            self.change_mode(OAM_MODE);

            self.update_stat_interrupts();
        }
    }

    fn get_current_mode(&self) -> u8 {
        self.mode
    }
    fn change_mode(&mut self, mode: u8) {
        self.mode = mode;
        self.stat = (self.stat & 0xFC) | mode;
        self.update_stat_interrupts();
    }

    pub fn tick(&mut self, cycles: u8) -> Vec<Interrupt> {
        self.interrupts.clear();

        if !self.lcd_ppu_enabled() {
            self.ly = 0;
            self.line_ticks = 0;
            self.window_line_counter = 0;
            self.set_ppu_mode(HBLANK_MODE);
            return self.interrupts.clone();
        }

        self.line_ticks += cycles as u16;
        self.mode = self.get_current_mode();

        match self.mode {
            OAM_MODE => {
                if self.line_ticks >= 80 {
                    // Check for LYC=LY interrupt
                    if self.ly == self.ly_compare {
                        self.stat |= LYC_COINCIDENCE_FLAG;
                    } else {
                        self.stat &= !LYC_COINCIDENCE_FLAG;
                    }
                    self.line_ticks -= 80;
                    self.change_mode(VRAM_MODE);
                }
            }
            VRAM_MODE => {
                if self.line_ticks >= 172 {
                    self.line_ticks -= 172;
                    self.change_mode(HBLANK_MODE);
                }
            }
            HBLANK_MODE => {
                if self.line_ticks >= 204 {
                    self.render_scanline();

                    if self.window_enabled()
                        && self.ly >= self.lcd.window_y
                        && self.lcd.window_x.saturating_sub(7) <= 166
                    {
                        self.window_line_counter = self.window_line_counter.wrapping_add(1);
                    }

                    if self.ly == 143 {
                        self.change_mode(VBLANK_MODE);
                    } else {
                        self.change_mode(OAM_MODE);
                    }

                    self.increment_ly();
                    self.line_ticks -= 204;
                }
            }
            VBLANK_MODE => {
                if self.line_ticks >= TICKS_PER_LINE {
                    if self.ly == Y_RES as u8 {
                        self.interrupts.push(Interrupt::VBLANK);
                    }
                    self.increment_ly();
                    self.line_ticks -= TICKS_PER_LINE;
                }
            }
            _ => {
                panic!("Invalid PPU mode: {}", self.mode);
            }
        }
        self.interrupts.clone()
    }

    fn update_stat_interrupts(&mut self) {
        self.stat = (self.stat & 0b11111100) | self.mode; // Clear mode bits and set current mode

        let enabled = |flag: StatInterrupt| (self.stat & flag as u8) != 0;

        let current_stat_line = (enabled(StatInterrupt::LYC) && self.ly == self.ly_compare)
            || (enabled(StatInterrupt::OAM) && self.mode == OAM_MODE)
            || (enabled(StatInterrupt::VBLANK) && self.mode == VBLANK_MODE)
            || (enabled(StatInterrupt::HBLANK) && self.mode == HBLANK_MODE);

        if current_stat_line && !self.prev_stat_line {
            self.interrupts.push(Interrupt::LCDSTAT);
        }

        self.prev_stat_line = current_stat_line;
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0xFF40 => self.lcdc,
            0xFF41 => self.stat,
            0xFF42 => self.scroll_y,
            0xFF43 => self.scroll_x,
            0xFF44 => self.ly,
            0xFF45 => self.ly_compare,
            0xFF47 => self.lcd.bg_palette,
            0xFF47..=0xFF4B => self.lcd.read(address),
            _ => {
                debug!("Invalid LCD address: {:#X}", address);
                0xFF
            }
        }
    }
    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xFF40 => self.lcdc = value,
            0xFF41 => self.stat = (self.stat & 0x87) | (value & 0x78),
            0xFF42 => self.scroll_y = value,
            0xFF43 => self.scroll_x = value,
            0xFF44 => {
                error!("Attempt to write to read-only register: {:#X}", address);
            }
            0xFF45 => self.ly_compare = value,
            0xFF47..=0xFF4B => self.lcd.write(address, value),
            _ => panic!("Invalid LCD address: {:#X}", address),
        }
    }

    fn is_stat_interrupt_enabled(&self, interrupt: StatInterrupt) -> bool {
        match interrupt {
            StatInterrupt::LYC => self.stat & 0x40 != 0,
            StatInterrupt::OAM => self.stat & 0x20 != 0,
            StatInterrupt::VBLANK => self.stat & 0x10 != 0,
            StatInterrupt::HBLANK => self.stat & 0x08 != 0,
        }
    }
    pub fn oam_read(&self, address: u16) -> u8 {
        self.oam_ram[address as usize]
    }

    pub fn oam_dma_write(&mut self, address: u16, value: u8) {
        self.oam_ram[address as usize] = value;
    }

    pub fn oam_write(&mut self, address: u16, value: u8) {
        if self.mode == OAM_MODE || self.mode == VRAM_MODE {
            return;
        }
        self.oam_ram[address as usize] = value;
    }

    pub fn vram_read(&self, address: u16) -> u8 {
        self.vram[(address - UNSIGNED_TILE_START) as usize]
    }

    pub fn vram_write(&mut self, address: u16, value: u8) {
        self.vram[(address - UNSIGNED_TILE_START) as usize] = value;
    }

    /*
    MAIN DISPLAY START
    */
    fn get_bg_tile_map(&self) -> [u8; BG_TILEMAP_SIZE] {
        let mut tile_map: [u8; BG_TILEMAP_SIZE] = [0; BG_TILEMAP_SIZE];
        let tile_map_base: u16 = if self.get_bg_tile_map_area() {
            TILE_MAP_1_START
        } else {
            TILE_MAP_0_START
        };
        for i in 0..BG_TILEMAP_SIZE {
            tile_map[i] = self.vram_read((tile_map_base as usize + i) as u16);
        }
        tile_map
    }

    fn get_bg_tile_map_area(&self) -> bool {
        self.lcdc & 0x08 != 0
    }

    fn get_tile_data_base(&self) -> u16 {
        if self.lcdc & 0x10 != 0 {
            UNSIGNED_TILE_START
        } else {
            SIGNED_TILE_START
        }
    }

    /* Get the tile data for a given tile index */
    pub fn get_tile_data(&self, tile_index: u8) -> TileData {
        let tile_data_base: u16 = self.get_tile_data_base();

        let tile_address = if tile_data_base == SIGNED_TILE_START {
            let adjusted_index = if tile_index < SIGNED_TILE_OFFSET {
                tile_index + SIGNED_TILE_OFFSET // Map 0-127 to 128-255
            } else {
                tile_index - SIGNED_TILE_OFFSET // Map 128-255 to 0-127
            };
            SIGNED_TILE_START + (adjusted_index as u16 * TILE_SIZE_BYTES as u16)
        } else {
            // Unsigned indexing
            tile_data_base + (tile_index as u16 * TILE_SIZE_BYTES as u16)
        };

        // Background/window tiles are always 8x8
        let mut tile_data = vec![0u8; TILE_SIZE_BYTES];
        for i in 0..TILE_SIZE_BYTES {
            tile_data[i] = self.vram_read(tile_address + i as u16);
        }

        TileData::new(&tile_data, TILE_WIDTH)
    }

    pub fn get_sprite_data(&self, sprite: &Sprite) -> TileData {
        let is_tall = (self.lcdc & LYC_COINCIDENCE_FLAG) != 0; // LCDC bit 2: 0 = 8x8, 1 = 8x16
        let sprite_size = if is_tall {
            TILE_SIZE_SIXTEEN
        } else {
            TILE_SIZE_EIGHT
        };
        let height = if is_tall {
            TILE_HEIGHT_SIXTEEN
        } else {
            TILE_HEIGHT_EIGHT
        };

        let mut sprite_tile_data = vec![0u8; sprite_size];

        let tile_number = if is_tall {
            sprite.tile_number & 0xFE // Clear only bit 0
        } else {
            sprite.tile_number
        };

        let sprite_tile_address =
            UNSIGNED_TILE_START + (tile_number as u16 * TILE_SIZE_EIGHT as u16);

        for i in 0..sprite_size {
            sprite_tile_data[i] = self.vram_read(sprite_tile_address + i as u16);
        }

        TileData::new(&sprite_tile_data, height)
    }

    fn render_background_scanline(&mut self, line: &mut [u32; X_RES]) {
        if self.is_background_enabled() {
            let tile_map = self.get_bg_tile_map();
            for x in 0..X_RES {
                let global_x = (x + self.scroll_x as usize) % BG_MAP_SIZE_PIXELS;
                let global_y = (self.ly as usize + self.scroll_y as usize) % BG_MAP_SIZE_PIXELS;

                let tile_x = global_x / TILE_WIDTH;
                let tile_y = global_y / TILE_HEIGHT_EIGHT;
                let tile_index = tile_map[tile_y * 32 + tile_x];
                let tile = self.get_tile_data(tile_index);

                let row = global_y % TILE_WIDTH;
                let col = global_x % TILE_HEIGHT_EIGHT;
                let pixel = tile.get_pixel(row, col);
                line[x] = self.lcd.get_bg_color(pixel);
            }
        }
    }

    fn render_sprite_scanline(&mut self, line: &mut [u32; X_RES]) {
        if !self.sprites_enabled() {
            return;
        }

        let current_scanline = self.ly as isize;
        let sprite_height = self.sprite_size() as isize;
        let sprite_data = self.get_sprites();

        // Collect sprites that intersect with current scanline (max 10)
        let mut sprites_to_render = Vec::with_capacity(MAX_SPRITES_PER_LINE);
        for sprite in sprite_data.iter() {
            let screen_y = sprite.y as isize - TILE_SIZE_BYTES as isize;
            if screen_y <= current_scanline && (screen_y + sprite_height) > current_scanline {
                sprites_to_render.push(sprite);
                if sprites_to_render.len() >= MAX_SPRITES_PER_LINE {
                    break;
                }
            }
        }

        // Process each pixel in the scanline
        for x in 0..X_RES {
            let mut best_sprite: Option<(&Sprite, u32)> = None; // (sprite, color)

            // Check all sprites for this x position to find the highest priority one
            for sprite in &sprites_to_render {
                let screen_x = sprite.x as isize - TILE_WIDTH as isize;
                let sprite_x_pos = x as isize - screen_x;

                if sprite_x_pos < 0 || sprite_x_pos >= 8 {
                    continue;
                }

                let screen_y = sprite.y as isize - TILE_SIZE_BYTES as isize;
                let mut sprite_row = current_scanline - screen_y;
                let mut sprite_col = sprite_x_pos;

                // Apply flips
                if sprite.flags.contains(OAMFlags::Y_FLIP) {
                    sprite_row = (sprite_height - 1) - sprite_row;
                }

                if sprite.flags.contains(OAMFlags::X_FLIP) {
                    sprite_col = 7 - sprite_col;
                }

                let sprite_tile_data = self.get_sprite_data(sprite);
                let sprite_pixel =
                    sprite_tile_data.get_pixel(sprite_row as usize, sprite_col as usize);

                // Skip transparent pixels
                if sprite_pixel == 0 {
                    continue;
                }

                let sprite_palette_index = if sprite.flags.contains(OAMFlags::DMG_PALETTE) {
                    1
                } else {
                    0
                };
                let sprite_color = self
                    .lcd
                    .get_sprite_color(sprite_palette_index, sprite_pixel);

                // Determine if this sprite should be drawn based on priority
                let should_draw = if sprite.flags.contains(OAMFlags::PRIORITY) {
                    // Behind background - only draw if background is transparent
                    line[x] == LIGHTEST_GREEN
                } else {
                    // Above background - always draw
                    true
                };

                if should_draw {
                    // Check if this sprite has higher priority than current best
                    let has_higher_priority = match &best_sprite {
                        None => true,
                        Some((best, _)) => {
                            // X-coordinate priority: smaller X wins
                            sprite.x < best.x
                        }
                    };

                    if has_higher_priority {
                        best_sprite = Some((sprite, sprite_color));
                    }
                }
            }

            // Draw the highest priority sprite (if any)
            if let Some((_, color)) = best_sprite {
                line[x] = color;
            }
        }
    }
    fn render_window_scanline(&mut self, line: &mut [u32; X_RES]) {
        let window_x = self.lcd.window_x.saturating_sub(7);

        // Early return if window is not visible
        if !self.window_enabled() || self.ly < self.lcd.window_y || self.lcd.window_x > 166 {
            return;
        }

        // Calculate window line (Y position within the window)
        let window_line = self.window_line_counter;
        let w_tile_y = (window_line / TILE_HEIGHT_EIGHT as u8) as u16; // Which tile row in the window we're on

        // Get window tile map base address (LCDC bit 6)
        let tile_map_base: u16 = if self.lcdc & 0x40 != 0 {
            0x9C00 // Window Tile Map at 0x9C00-0x9FFF
        } else {
            0x9800 // Window Tile Map at 0x9800-0x9BFF
        };

        // Get addressing mode (LCDC bit 4)
        let data_area_is_8800 = self.lcdc & 0x10 == 0;

        // Calculate tile row line we're rendering (0-7)
        let tile_line = window_line % TILE_HEIGHT_EIGHT as u8;

        // Render window pixels for this scanline
        for screen_x in window_x..X_RES as u8 {
            // Calculate the offset into the window tile map
            let tile_map_offset =
                ((screen_x as u16 - window_x as u16) / TILE_WIDTH as u16) + (w_tile_y * 32);

            // Get the tile ID from the window tile map
            let mut tile_id = self.vram_read(tile_map_base + tile_map_offset);

            // Adjust tile ID for 0x8800 addressing mode
            if data_area_is_8800 {
                tile_id = tile_id.wrapping_add(SIGNED_TILE_OFFSET);
            }

            // Calculate base address for tile data
            let base_address: u16 = if data_area_is_8800 {
                SIGNED_TILE_START
            } else {
                UNSIGNED_TILE_START
            };

            // Calculate address of this specific row of the tile
            let tile_address = base_address + (tile_id as u16 * 16) + (tile_line as u16 * 2);

            // Read the pixel data for this row
            let tile_low = self.vram_read(tile_address);
            let tile_high = self.vram_read(tile_address + 1);

            // Calculate which pixel of the tile we need (0-7)
            let pixel_x = (screen_x as u16 - window_x as u16) % TILE_WIDTH as u16;

            // Get the color bits
            let pixel_bit_position = 7 - (pixel_x as u8);
            let colour_bit_0 = (tile_low >> pixel_bit_position) & 0x1;
            let colour_bit_1 = (tile_high >> pixel_bit_position) & 0x1;
            let colour_id = (colour_bit_1 << 1) | colour_bit_0;

            // Get the color and render
            let colour = self.lcd.get_bg_color(colour_id);

            line[screen_x as usize] = colour;
        }
    }
    fn render_scanline(&mut self) {
        let ly = self.ly as usize;
        let mut line = [LIGHTEST_GREEN; X_RES];

        self.render_background_scanline(&mut line);
        self.render_window_scanline(&mut line);
        self.render_sprite_scanline(&mut line);

        self.framebuffer[ly * X_RES..(ly + 1) * X_RES].copy_from_slice(&line);
    }

    /*
    MAIN DISPLAY END
    */

    /*
    SPRITE DISPLAY START
    */
    fn get_sprites(&self) -> Vec<Sprite> {
        let mut sprites: Vec<Sprite> = Vec::with_capacity(40);

        for i in 0..40 {
            let offset = (i * 4) as u16;
            let y = self.oam_read(offset);
            let x = self.oam_read(offset + 1);
            let tile_number = self.oam_read(offset + 2);
            let flags_byte = self.oam_read(offset + 3);

            // Use from_bits_truncate to safely convert u8 -> OAMFlags
            let flags = OAMFlags::from_bits_truncate(flags_byte);

            sprites.push(Sprite {
                y,
                x,
                tile_number,
                flags,
            });
        }

        sprites
    }

    /*
    SPRITE DISPLAY END
    */
}

// Struct to represent the tile data
pub struct TileData {
    data: Vec<u8>,
    height: usize, // 8 or 16 pixels
    width: usize,
}

impl TileData {
    pub fn new(data: &[u8], height: usize) -> Self {
        assert!(height == TILE_HEIGHT_EIGHT || height == TILE_HEIGHT_SIXTEEN, "Sprite height must be 8 or 16");
        assert_eq!(
            data.len(),
            height * 2,
            "Data size must match height (2 bytes per row)"
        );

        TileData {
            data: data.to_vec(),
            height,
            width: 8,
        }
    }

    // Get pixel color (0-3) at row and col
    pub fn get_pixel(&self, row: usize, col: usize) -> u8 {
        assert!(row < self.height, "Row out of bounds");
        assert!(col < self.width, "Column out of bounds");

        let plane1 = self.data[row * 2];
        let plane2 = self.data[row * 2 + 1];

        let bit_position = 7 - col;
        let low_bit = (plane1 >> bit_position) & 0x01;
        let high_bit = (plane2 >> bit_position) & 0x01;

        (high_bit << 1) | low_bit
    }
}
