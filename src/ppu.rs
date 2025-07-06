use crate::display::{SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::interupts::Interrupt;
use crate::CPU::{Flag, FlagsRegister, CPU};
use crate::{display, interupts, lcd, main, main_display};
use log::{debug, error, info};
use std::fmt;
use crate::lcd::LCD;
use crate::lcd::LIGHTEST_GREEN;

const TILE_START: u16 = 0x8000;
const TILE_END: u16 = 0x97FF;

const TILE_MAP_0_START: u16 = 0x9800;
const TILE_MAP_1_START: u16 = 0x9C00;

const OAM_MODE: u8 = 2;
const VRAM_MODE: u8 = 3;
const HBLANK_MODE: u8 = 0;
const VBLANK_MODE: u8 = 1;
const LINES_PER_FRAME: u8 = 153;
const TICKS_PER_LINE: u16 = 456;
const Y_RES: u8 = 144;
const X_RES: u8 = 160;
const TARGET_FRAME_TIME: u32 = 1000 / 60; // 60 FPS

pub(crate) const VIEWPORT_WIDTH: usize = 160;
pub(crate) const VIEWPORT_HEIGHT: usize = 144;

pub struct Ppu {
    oam_ram: [u8; 0xA0],
    pub vram: [u8; 0x2000],
    //pub ly: u8, // LY register
    //pub lyc: u8,
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
    pub background_buffer: Vec<u32>,
    pub framebuffer: Vec<u32>,
    pub viewport: Vec<u32>,
    interrupts: Vec<Interrupt>, // Experimental for change in ownership model
}

//Display for Ppu
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

#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    y: u8,
    x: u8,
    tile_number: u8,
    flags: OAMFlags,
    //TODO make each flag its own bit variable

    /*
    FLAGS:
        Bit 7 - Priority: 0 = No, 1 = BG and Window colors 1–3 are drawn over this OBJ
        Bit 6 - Y flip: 0 = Normal, 1 = Entire OBJ is vertically mirrored
        Bit 5 - X flip: 0 = Normal, 1 = Entire OBJ is horizontally mirrored
        Bit 4 - DMG palette [Non CGB Mode only]: 0 = OBP0, 1 = OBP1
        Bit 3 - Bank [CGB Mode Only]: 0 = Fetch tile from VRAM bank 0, 1 = Fetch tile from VRAM bank 1
        Bits 2,1,0 - CGB palette [CGB Mode Only]: Which of OBP0–7 to use
     */
}

enum StatInterrupt {
    LYC = 0x40,
    OAM = 0x20,
    VBLANK = 0x10,
    HBLANK = 0x08,
}

#[derive(Debug, Clone, Copy)]
pub enum OamFlag {
    PRIORITY,
    Y_FLIP,
    X_FLIP,
    DMG_PALETTE,
    BANK,
    CGB_PALETTE,
}

#[derive(Debug, Clone, Copy)]
struct OAMFlags {
    priority: bool,
    y_flip: bool,
    x_flip: bool,
    dmg_palette: bool,
    bank: bool,
    cgb_palette: u8,
}

impl fmt::Display for OAMFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Priority: {}, Y Flip: {}, X Flip: {}, DMG Palette: {}, Bank: {}, CGB Palette: {}",
            if self.priority { "1" } else { "0" },
            if self.y_flip { "1" } else { "0" },
            if self.x_flip { "1" } else { "0" },
            if self.dmg_palette { "1" } else { "0" },
            if self.bank { "1" } else { "0" },
            self.cgb_palette
        )
    }
}

impl std::convert::From<OAMFlags> for u8 {
    fn from(flag: OAMFlags) -> u8 {
        (if flag.priority { 1 } else { 0 } << 7)
            | (if flag.y_flip { 1 } else { 0 } << 6)
            | (if flag.x_flip { 1 } else { 0 } << 5)
            | (if flag.dmg_palette { 1 } else { 0 } << 4)
            | (if flag.bank { 1 } else { 0 } << 3)
            | flag.cgb_palette
    }
}

impl std::convert::From<u8> for OAMFlags {
    fn from(byte: u8) -> Self {
        let priority = byte & 0b1000_0000 != 0;
        let y_flip = byte & 0b0100_0000 != 0;
        let x_flip = byte & 0b0010_0000 != 0;
        let dmg_palette = byte & 0b0001_0000 != 0;
        let bank = byte & 0b0000_1000 != 0;
        let cgb_palette = byte & 0b0000_0111;
        OAMFlags {
            priority,
            y_flip,
            x_flip,
            dmg_palette,
            bank,
            cgb_palette,
        }
    }
}
impl OAMFlags {
    pub fn new() -> Self {
        OAMFlags {
            priority: false,
            y_flip: false,
            x_flip: false,
            dmg_palette: false,
            bank: false,
            cgb_palette: 0,
        }
    }

    pub fn get_flag(&self, flag: OamFlag) -> bool {
        match flag {
            OamFlag::PRIORITY => self.priority,
            OamFlag::Y_FLIP => self.y_flip,
            OamFlag::X_FLIP => self.x_flip,
            OamFlag::DMG_PALETTE => self.dmg_palette,
            OamFlag::BANK => self.bank,
            OamFlag::CGB_PALETTE => self.cgb_palette != 0,
        }
    }

    pub fn set_flag(&mut self, flag: OamFlag, value: bool) {
        match flag {
            OamFlag::PRIORITY => self.priority = value,
            OamFlag::Y_FLIP => self.y_flip = value,
            OamFlag::X_FLIP => self.x_flip = value,
            OamFlag::DMG_PALETTE => self.dmg_palette = value,
            OamFlag::BANK => self.bank = value,
            OamFlag::CGB_PALETTE => self.cgb_palette = value as u8,
        }
    }
}

impl Sprite {
    pub fn new() -> Sprite {
        Sprite {
            y: 0,
            x: 0,
            tile_number: 0,
            flags: OAMFlags::from(0),
        }
    }
}
impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            oam_ram: [0; 0xA0],
            vram: [0x0000; 0x2000],
            mode: OAM_MODE,
            line_ticks: 0,
            current_frame: 0,
            previous_frame_time: 0,
            start_time: 0,
            frame_count: 0,
            
            lcd: lcd::LCD::new(),

            lcdc: 0x91,
            stat: 0x85,
            scroll_x: 0,
            scroll_y: 0,
            ly: 0,
            ly_compare: 0,
            window_line_counter: 0,
            framebuffer: vec![LIGHTEST_GREEN; VIEWPORT_WIDTH * VIEWPORT_HEIGHT],
            background_buffer: vec![main_display::RED; main_display::WIDTH * main_display::HEIGHT],
            viewport: vec![
                main_display::RED;
                main_display::VIEWPORT_WIDTH * main_display::VIEWPORT_HEIGHT
            ],
            interrupts: Vec::new(),
        }
    }

    pub(crate) fn lcd_ppu_enabled(&self) -> bool {
        self.lcdc & 0x80 != 0
    }

    fn window_enabled(&self) -> bool {
        self.lcdc & 0x20 != 0
    }

    fn bg_window_tile_data(&self) -> u16 {
        if self.lcdc & 0x10 != 0 {
            0x8000
        } else {
            0x8800
        }
    }

    fn bg_tile_map(&self) -> u16 {
        if self.lcdc & 0x08 != 0 {
            0x9C00
        } else {
            0x9800
        }
    }

    fn sprite_size(&self) -> u8 {
        if self.lcdc & 0x04 != 0 {
            16
        } else {
            8
        }
    }

    /**
    Mode 2 int select
     */
    fn get_lyc_int_select(&self) -> bool {
        self.stat & 0x40 != 0
    }
    fn set_lyc_int_select(&mut self, value: bool) {
        if value {
            self.stat |= 0x40;
        } else {
            self.stat &= !0x40;
        }
    }

    /**
    Mode 1 int select
     */
    fn get_vblank_int_select(&self) -> bool {
        self.stat & 0x10 != 0
    }
    fn set_vblank_int_select(&mut self, value: bool) {
        if value {
            self.stat |= 0x10;
        } else {
            self.stat &= !0x10;
        }
    }

    /**
    Mode 0 int select
     */
    fn get_hblank_int_select(&self) -> bool {
        self.stat & 0x08 != 0
    }
    fn set_hblank_int_select(&mut self, value: bool) {
        if value {
            self.stat |= 0x08;
        } else {
            self.stat &= !0x08;
        }
    }

    /**
    LYC == LY
     */
    fn lyc_equals_ly(&self) -> bool {
        self.ly == self.ly_compare
    }

    /**
    PPU mode
     */
    fn get_ppu_mode(&self) -> u8 {
        self.stat & 0x03
    }
    fn set_ppu_mode(&mut self, mode: u8) {
        self.stat = (self.stat & 0xFC) | (mode & 0x03); // Update only mode bits
    }

    fn get_oam_int_select(&self) -> bool {
        self.stat & 0x20 != 0
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
            self.stat |= 0x04; // Set coincidence flag
            if self.is_stat_interrupt_enabled(StatInterrupt::LYC) {
                //self.cpu.borrow_mut().trigger_interrupt(Interrupt::LCDSTAT);
                self.interrupts.push(Interrupt::LCDSTAT);
            }
        } else {
            self.stat &= !0x04; // Clear coincidence flag
        }

        if self.ly >= LINES_PER_FRAME {
            self.ly = 0;
            self.window_line_counter = 0;
            self.current_frame += 1;
            self.change_mode(OAM_MODE);
        }
        self.update_stat_interrupts(); // Ensure this is called to check for interrupts
    }

    fn get_current_mode(&self) -> u8 {
        // TODO: Implement this
        //unimplemented!("get_current_mode")
        return self.mode
    }
    fn change_mode(&mut self, mode: u8) {
        self.mode = mode;
        debug!("PPU mode changed to: {}. STAT before change: {:2X}, STAT after change: {:2X}", mode, self.stat, (self.stat & 0xFC) | mode);
        self.stat = (self.stat & 0xFC) | mode; // Only change mode bits
        self.update_stat_interrupts();
    }

    pub fn tick(&mut self, cycles: u8) -> Vec<Interrupt> {
        debug!("PPU tick with cycles: {}", cycles);

        self.interrupts.clear();

        if !self.lcd_ppu_enabled() {
            debug!("LCD is disabled");
            self.ly = 0;
            self.line_ticks = 0;
            self.window_line_counter = 0;
            self.set_ppu_mode(HBLANK_MODE);
            return self.interrupts.clone();
        }

        self.line_ticks += cycles as u16;
        debug!("Line ticks: {}", self.line_ticks);

        self.mode = self.get_current_mode();

        match self.mode {
            OAM_MODE => {
                if self.line_ticks >= 80 {
                    // Check for OAM interrupt
                    if self.is_stat_interrupt_enabled(StatInterrupt::OAM) {
                        //self.cpu.borrow_mut().trigger_interrupt(Interrupt::LCDSTAT);
                        self.interrupts.push(Interrupt::LCDSTAT);
                    }

                    // Check for LYC=LY interrupt
                    if self.ly == self.ly_compare {
                        self.stat |= 0x04;
                    } else {
                        self.stat &= !0x04;
                    }

                    // Mode transition
                    self.line_ticks -= 80;
                    self.change_mode(VRAM_MODE);
                }
            }
            VRAM_MODE => {
                if self.line_ticks >= 172 {
                    // Mode transition
                    self.line_ticks -= 172;
                    self.change_mode(HBLANK_MODE);
                }
            }
            HBLANK_MODE => {
                if self.line_ticks >= 204 {
                    // Check for H-Blank interrupt
                    if self.is_stat_interrupt_enabled(StatInterrupt::HBLANK) {
                        //self.cpu.borrow_mut().trigger_interrupt(Interrupt::LCDSTAT);
                        self.interrupts.push(Interrupt::LCDSTAT);
                    }

                    // Render scanline
                    self.render_scanline();

                    if self.window_enabled() && self.ly >= self.lcd.window_y && self.lcd.window_x.saturating_sub(7) <= 166 {
                        self.window_line_counter = self.window_line_counter.wrapping_add(1);
                    }

                    // Mode transition
                    if self.ly == 143 {
                        debug!("Transition to VBLANK, STAT before: {:#X}, {:8b}", self.stat, self.stat);
                        self.change_mode(VBLANK_MODE);
                        debug!("STAT after: {:#X}, {:8b}", self.stat, self.stat);
                    } else {
                        //self.increment_line_counter(self.ly);
                        self.change_mode(OAM_MODE);
                    }

                    self.increment_ly();
                    self.line_ticks -= 204;

                }
            }
            VBLANK_MODE => {
                if self.line_ticks >= 456 {
                    if self.ly == 144 {
                        //self.cpu.borrow_mut().trigger_interrupt(Interrupt::VBLANK);
                        self.interrupts.push(Interrupt::VBLANK);
                    }

                    // Final line of V-Blank
                    /*if self.ly == 153 {
                        self.change_mode(OAM_MODE);
                        self.ly = 0;
                        self.current_frame += 1;
                    }
                    else {
                        self.increment_ly();
                    }*/
                    self.increment_ly();
                    self.line_ticks -= 456;
                }
            }
            _ => {
                panic!("Invalid PPU mode: {}", self.mode);
            }
        }
        return self.interrupts.clone();
    }

    fn update_stat_interrupts(&mut self) {
        // Update STAT register's mode bits
        self.stat = (self.stat & 0b11111100) | self.mode;

        if (self.stat & 0x40 != 0 && self.ly == self.ly_compare) || // LYC=LY interrupt
            (self.stat & 0x20 != 0 && self.mode == 2) || // OAM interrupt
            (self.stat & 0x10 != 0 && self.mode == 1) || // V-Blank interrupt
            (self.stat & 0x08 != 0 && self.mode == 0)
        {
            //self.cpu.borrow_mut().trigger_interrupt(Interrupt::LCDSTAT);
            self.interrupts.push(Interrupt::LCDSTAT);
        }
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
                //self.ly = 90;
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

    fn lcdc_background_priority(&self) -> bool {
        self.lcdc & 0x01 == 0
    }

    pub fn oam_read(&self, address: u16) -> u8 {
        //debug!("OAM read at address: {:#X}", address);
        /*if address < 0xFE00 || address >= 0xFEA0 {
            panic!("Attempt to read from invalid OAM address: {:#X}", address);
        }*/
        self.oam_ram[(address) as usize]
    }

    pub fn oam_write(&mut self, address: u16, value: u8) {
        //debug!("OAM write at address: {:#X}", address);
        if self.mode == OAM_MODE || self.mode == VRAM_MODE {
            debug!("Attempt to write to OAM during mode {}", self.mode);
            return;
        }
        self.oam_ram[(address) as usize] = value;
        //debug!("OAM data: {:?}", self.oam_ram);
    }

    pub fn vram_read(&self, address: u16) -> u8 {
        //debug!("VRAM read at address: {:#X}", address);
        self.vram[(address - 0x8000) as usize]
    }

    pub fn vram_write(&mut self, address: u16, value: u8) {
        //debug!("VRAM write {:#4X} at address: {:#4X}", value, address);
        //TODO this won't work when boot rom is enabled
        /*if self.mode == VRAM_MODE || self.mode == OAM_MODE {
            debug!("Attempt to write to VRAM during mode {}", self.mode);
            return;
        }*/
        self.vram[(address - 0x8000) as usize] = value;
        //debug!("VRAM data: {:?}", self.vram);
    }

    fn trigger_interrupt(&mut self, interrupt: interupts::Interrupt) {
        //debug!("Triggering interrupt: {:?}", interrupt);
        match interrupt {
            Interrupt::VBLANK => {
                self.trigger_interrupt(interupts::Interrupt::VBLANK);
            }
            Interrupt::LCDSTAT => {
                self.trigger_interrupt(interupts::Interrupt::LCDSTAT);
            }
            _ => {
                panic!("Invalid interrupt: {:?}", interrupt);
            }
        }
    }

    pub fn get_tile_set(&self) -> [[u8; 16]; 256] {
        let mut tile_set = [[0; 16]; 256]; //256 tiles

        for tile_number in 0..256 {
            //debug!("{:4X}", 0x8000 + (tile_number * 16));
            tile_set[tile_number] = self.get_tile_from_memory((0x8000 + (tile_number * 16)) as u16);
            // TODO this needs to be dynamic based on tile map value from memory?
        }

        tile_set
    }

    fn get_tile_from_memory(&self, address: u16) -> [u8; 16] {
        let mut tile = [0; 16];

        for x in 0..16 {
            tile[x] = self.vram_read(address + x as u16);
        }
        tile
    }

    pub fn get_tile_map(&self) -> Vec<&[u8]> {
        /*let mut tile_map: [u8; 1024] = [0; 1024];

        for i in 0..1024 {
            tile_map[i] = self.cpu.borrow().memory_bus.borrow().read_byte((0x9800 + i) as u16);
        }

        tile_map*/

        let mut tiles: Vec<&[u8]> = Vec::new();
        for i in 0..384 {
            let tile_start = i * 16;
            tiles.push(&self.vram[tile_start..tile_start + 16]);
        }

        tiles
    }

    fn get_bgp_palette(&self, bit_pair: u8) -> u8 {
        let palette = self.lcd.bg_palette;
        match bit_pair {
            0b00 => palette & 0b0000_0011,
            0b01 => (palette & 0b0000_1100) >> 2,
            0b10 => (palette & 0b0011_0000) >> 4,
            0b11 => (palette & 0b1100_0000) >> 4,
            _ => palette & 0b0000_0011,
        }
    }

    pub fn get_debug_background_tile_map(&self) -> Vec<&[u8]> {
        /*
        This function retrieves the tile data for all 32x32 tiles
        in the background tile map for debugging purposes.
        */

        let mut tiles: Vec<&[u8]> = Vec::new();

        // Background tile map range in VRAM: 0x1800 to 0x1BFF (32x32 = 1024 tiles)
        for i in 0x1800..=0x1BFF {
            let tile_index = self.vram[i] as usize; // Fetch tile index

            // The tile data starts at tile_index * 16 (16 bytes per tile)
            let tile_start = tile_index * 16;

            // Prevent out-of-bounds access
            if tile_start + 16 <= self.vram.len() {
                tiles.push(&self.vram[tile_start..tile_start + 16]);
            }
        }

        tiles
    }

    pub fn get_background_tile_map(&self) -> Vec<u8> {
        /*
        This function gets the memory addresses of the in order tiles that make up the background
        */

        let mut tile_map: Vec<u8> = Vec::new();
        let bg_tile_map = self.bg_tile_map();
        debug!("BG Tile Map: {:#X}", bg_tile_map);

        for x in 0..1024 {
            tile_map.push(self.vram_read(bg_tile_map + x as u16));
        }
        tile_map
    }

    /*fn get_window_tile_map(&self) -> Vec<u8> {
        let mut tile_map: Vec<u8> = Vec::new();
        let tile_map_base = if self.lcdc & (1 << 6) != 0 {
            0x9C00
        } else {
            0x9800
        };
        for x in 0..1024 {
            tile_map.push(self.vram_read(tile_map_base + x as u16));
        }
        tile_map
    }*/

    fn get_window_tiles(&self) -> Vec<TileData>{
        // 0 = 0x9800-0x9BFF in hardware (0x1800-0x1BFF in VRAM array)
        // 1 = 0x9C00-0x9FFF in hardware (0x1C00-0x1FFF in VRAM array)

        let mut tiles: Vec<TileData> = Vec::new();

        let window_tile_map_addr = if self.lcdc & 0x40 != 0 {
            0x1C00
        } else {
            0x1800
        };
        info!("Window Tile Map Base: {:#X}", window_tile_map_addr);

        let unsigned_tile_ids = self.lcdc & 0x10 != 0;
        info!("Unsigned Tile IDs: {}", unsigned_tile_ids);

        let tile_base_address = if unsigned_tile_ids {
            0x0000
        }
        else {
            0x0800
        };
        info!("Tile Base Address: {:#X}", tile_base_address);

        for y in 0..32 {
            for x in 0..32 {
                let tile_map_index = window_tile_map_addr + (y * 32) + x;

                // Get the tile ID from the tile map
                let tile_id = self.vram[tile_map_index] as u16 & 0xFF;

                // Calculate the address of the actual tile data
                let tile_data_address = if unsigned_tile_ids {
                    // Unsigned mode (LCDC bit 4 = 1): Tiles at 0x8000-0x8FFF
                    // In VRAM array, this is offset 0x0000-0x0FFF
                    (tile_id * 16) // Base address is already 0 in this case
                } else {
                    // Signed mode (LCDC bit 4 = 0)
                    // If tile_id < 128, it's a positive number (0 to 127)
                    // If tile_id >= 128, it's a negative number (equivalent to -128 to -1)
                    if tile_id < 128 {
                        // Positive tile IDs (0-127) -> 0x9000-0x97FF
                        // In VRAM array, this is offset 0x1000-0x17FF
                        0x1000 + (tile_id * 16)
                    } else {
                        // Negative tile IDs (128-255 as -128 to -1) -> 0x8800-0x8FFF
                        // In VRAM array, this is offset 0x0800-0x0FFF
                        0x0800 + ((tile_id - 128) * 16)
                    }
                };

                // Read the tile data
                let mut tile_data: [u8; 16] = [0; 16];
                for pixel in 0..16 {
                    tile_data[pixel] = self.vram[(tile_data_address + pixel as u16) as usize];
                }
                let tile = TileData::new(&tile_data, 8);
                info!("tile {:?}", tile.data);
                tiles.push(tile);
            }
        }

        tiles
    }
    pub(crate) fn get_window_tile_map(&self) -> Vec<u8> {
        let tile_map_base = if self.lcdc & (1 << 6) != 0 {
            0x9C00
        } else {
            0x9800
        };
        (0..1024).map(|x| self.vram_read(tile_map_base + x as u16)).collect()
    }

    pub fn populate_background_tiles(&self) -> Vec<[u8; 16]> {
        let mut tiles: Vec<[u8; 16]> = Vec::new();
        let tile_map = self.get_background_tile_map();
        let background_tile_set = self.get_tile_set();

        for tile in tile_map.iter() {
            //debug!("Tile: {:#X}", tile);
            let tile_data = background_tile_set[*tile as usize].clone();
            //debug!("BG Tiles: {:?}", tile_data);

            tiles.push(tile_data);
        }
        //debug!("Number of tiles: {}", tiles.len());
        //debug!("Tile Map: {:?}", tiles);

        tiles
    }

    /*
    MAIN DISPLAY START
    */
    fn get_bg_tile_map(&self) -> [u8; 1024] {
        let mut tile_map: [u8; 1024] = [0; 1024];
        let tile_map_base: u16 = if self.get_bg_tile_map_area() {
            TILE_MAP_1_START
        } else {
            TILE_MAP_0_START
        };
        debug!("Tile Map Base: {:#X}", tile_map_base);
        for i in 0..1024 {
            tile_map[i] = self.vram_read((tile_map_base as usize + i) as u16);
        }
        tile_map
    }

    fn get_bg_tile_map_for_scanline(&self, scanline: u8) -> [u8; 32] {
        let tile_map_base: u16 = if self.get_bg_tile_map_area() {
            TILE_MAP_1_START
        } else {
            TILE_MAP_0_START
        };
        debug!(
            "Tile Map Base for scanline {}: {:#X}",
            scanline, tile_map_base
        );

        let row = ((scanline as u16 + self.scroll_y as u16) / 8) % 32; // Adjust for scroll_y
        let start_address = tile_map_base + row * 32;

        let mut tile_row: [u8; 32] = [0; 32];
        for i in 0..32 {
            tile_row[i] = self.vram_read(start_address + i as u16);
        }
        tile_row
    }

    fn get_window_tile_map_for_scanline(&self, scanline: u8) -> [u8; 32] {
        // First check if window is enabled and the scanline is within window area
        if !self.window_enabled() || scanline < self.lcd.window_y {
            return [0; 32]; // Return empty array if window not visible on this scanline
        }

        let tile_map_base: u16 = if self.lcdc & 0x40 != 0 { // LCDC bit 6
            0x9C00 // Window Tile Map at 0x9C00-0x9FFF
        } else {
            0x9800 // Window Tile Map at 0x9800-0x9BFF
        };

        debug!(
        "Window Tile Map Base for scanline {}: {:#X}",
        scanline, tile_map_base
    );

        // Calculate which row of the window we're drawing
        // Window is positioned relative to WY register
        let window_row = (scanline as u16 - self.lcd.window_y as u16) / 8;

        // Unlike background, window doesn't wrap, but we'll cap at 32 rows max
        if window_row >= 32 {
            return [0; 32]; // Beyond the window's vertical limit
        }

        let start_address = tile_map_base + window_row * 32;

        let mut tile_row = [0; 32];
        for (i, tile) in tile_row.iter_mut().enumerate() {
            *tile = self.vram_read(start_address + i as u16);
        }

        tile_row
    }

    pub fn populate_background_buffer(&mut self) {
        let tile_set = self.get_tile_set();
        let tile_map = self.get_bg_tile_map();

        for (index, tile_item) in tile_map.iter().enumerate() {
            let tile = tile_set[*tile_item as usize];
            let mfb_tile = self.convert_tile_to_sdl_format(tile);

            for (x, pixel) in mfb_tile.iter().enumerate() {
                let h_offset = (x % 8) + ((index % 32) * 8);
                let v_offset = (x / 8) + ((index / 32) * 8) * main_display::WIDTH;
                self.background_buffer[h_offset + v_offset] = *pixel;
            }
        }
    }
    pub fn convert_tile_to_sdl_format(&mut self, tile_data: [u8; 16]) -> Vec<u32> {
        let mut sdl_tile: Vec<u32> = vec![main_display::RED; 64];
        for x in (0..tile_data.len()).step_by(2) {
            let lsb = tile_data[x];
            let msb = tile_data[x + 1];
            for bit in 0..8 {
                let bit_lsb = lsb & (1 << bit) != 0;
                let bit_msb = msb & (1 << bit) != 0;
                let pair = ((bit_lsb as u8) << 1) | (bit_msb as u8);
                let bgp_palette = self.convert_pixel_to_bgb_palette(pair);
                let pixel_color = main_display::get_gb_colour(bgp_palette);
                //debug!("SDL Pixel: {:#X}", pixel_color);
                sdl_tile[(x / 2 * 8) + (7 - bit) as usize] = pixel_color;
            }
        }
        sdl_tile
    }
    fn convert_pixel_to_bgb_palette(&self, pixel: u8) -> u8 {
        let palette = self.lcd.bg_palette;
        match pixel {
            0b00 => palette & 0b0000_0011,
            0b01 => (palette & 0b0000_1100) >> 2,
            0b10 => (palette & 0b0011_0000) >> 4,
            0b11 => (palette & 0b1100_0000) >> 6,
            _ => palette & 0b0000_0011,
        }
    }

    /*
    Returns the framebuffer for the Gameboy's visible viewport
     */
    pub fn get_tile_index(&self, map_x: usize, map_y: usize) -> u8 {
        const TILE_MAP_WIDTH: usize = 32; // 32 tiles per row in the tile map
                                          // Determine which tile map is being used based on the LCDC register
        let tile_map_base: u16 = if self.get_bg_tile_map_area() {
            TILE_MAP_1_START
        } else {
            TILE_MAP_0_START
        };

        // Calculate the position in the tile map
        let tile_map_offset = map_y * TILE_MAP_WIDTH + map_x;
        let tile_map_address = tile_map_base + tile_map_offset as u16;

        // Read the tile index from memory (assumes you have a method to read from VRAM)
        self.vram_read(tile_map_address)
    }

    fn get_bg_tile_map_area(&self) -> bool {
        self.lcdc & 0x08 != 0
    }

    fn get_tile_data_base(&self) -> u16 {
        if self.lcdc & 0x10 != 0 {
            0x8000 // Unsigned region
        } else {
            0x8800 // Signed region
        }
    }

    // Get the tile data for a given tile index
    pub fn get_tile_data(&self, tile_index: u8) -> TileData {
        const TILE_SIZE_BYTES: usize = 16; // 16 bytes per tile (2 bytes per row for 8 rows)
        let tile_data_base: u16 = self.get_tile_data_base();

        let tile_address = if tile_data_base == 0x8800 {
            // For signed indexing, adjust the base to start from 0x9000 for index 0
            let adjusted_index = if tile_index < 128 {
                tile_index + 128 // Map 0-127 to 128-255
            } else {
                tile_index - 128 // Map 128-255 to 0-127
            };
            0x8800 + (adjusted_index as u16 * TILE_SIZE_BYTES as u16)
        } else {
            // Unsigned indexing
            tile_data_base + (tile_index as u16 * TILE_SIZE_BYTES as u16)
        };

        // Background/window tiles are always 8x8
        let mut tile_data = vec![0u8; TILE_SIZE_BYTES];
        for i in 0..TILE_SIZE_BYTES {
            tile_data[i] = self.vram_read(tile_address + i as u16);
        }

        TileData::new(&tile_data, 8) // Always 8 pixels tall for background/window tiles
    }

    pub fn get_sprite_data(&self, sprite: &Sprite) -> TileData {
        let is_tall = (self.lcdc & 0x04) != 0; // LCDC bit 2: 0 = 8x8, 1 = 8x16
        let sprite_size = if is_tall { 32 } else { 16 }; // 32 bytes for 8x16, 16 bytes for 8x8
        let height = if is_tall { 16 } else { 8 };

        let mut sprite_tile_data = vec![0u8; sprite_size];

        // For 8x16 sprites, tile_number points to the first tile, second tile follows immediately
        // Note: For 8x16 sprites, the LSB of tile_number is ignored (effectively tile_number & 0xFE)
        let tile_number = if is_tall {
            sprite.tile_number & 0xFE // Clear only bit 0
        } else {
            sprite.tile_number
        };
        // Sprite tiles always start at 0x8000, using unsigned indices
        let sprite_tile_address = 0x8000 + (tile_number as u16 * 16); // Each tile is 16 bytes

        for i in 0..sprite_size {
            sprite_tile_data[i] = self.vram_read(sprite_tile_address + i as u16);
        }

        TileData::new(&sprite_tile_data, height)
    }

    fn render_background_scanline(&mut self, line: &mut [u32; 160]) {
        if self.is_background_enabled() {
            //let ly = self.ly as usize;
            //let mut line = [LIGHTEST_GREEN; 160];
            let tile_map = self.get_bg_tile_map();
            for x in 0..160 {
                let global_x = (x + self.scroll_x as usize) % 256;
                let global_y = (self.ly as usize + self.scroll_y as usize) % 256;

                let tile_x = global_x / 8;
                let tile_y = global_y / 8;
                let tile_index = tile_map[tile_y * 32 + tile_x];
                let tile = self.get_tile_data(tile_index);

                let row = global_y % 8;
                let col = global_x % 8;
                let pixel = tile.get_pixel(row, col);
                line[x] = self.lcd.get_bg_color(pixel);
            }

            //self.framebuffer[ly * 160..(ly + 1) * 160].copy_from_slice(&line);
        }
    }

    fn render_sprite_scanline(&mut self, line: &mut [u32; 160]) {
        if !self.sprites_enabled() {
            return;
        }

        let current_scanline = self.ly as isize;
        let sprite_height = self.sprite_size() as isize;
        let sprite_data = self.get_sprites();

        // Collect sprites that intersect with current scanline (max 10)
        let mut sprites_to_render = Vec::with_capacity(10);
        for sprite in sprite_data.iter() {
            let screen_y = sprite.y as isize - 16;
            if screen_y <= current_scanline && (screen_y + sprite_height) > current_scanline {
                sprites_to_render.push(*sprite);
                if sprites_to_render.len() >= 10 {
                    break;
                }
            }
        }

        // Process each pixel in the scanline
        for x in 0..160 {
            let mut best_sprite: Option<(Sprite, u32)> = None; // (sprite, color)

            // Check all sprites for this x position to find the highest priority one
            for sprite in &sprites_to_render {
                let screen_x = sprite.x as isize - 8;
                let sprite_x_pos = x as isize - screen_x;

                if sprite_x_pos < 0 || sprite_x_pos >= 8 {
                    continue;
                }

                let screen_y = sprite.y as isize - 16;
                let mut sprite_row = current_scanline - screen_y;
                let mut sprite_col = sprite_x_pos;

                // Apply flips
                if sprite.flags.y_flip {
                    sprite_row = (sprite_height - 1) - sprite_row;
                }
                if sprite.flags.x_flip {
                    sprite_col = 7 - sprite_col;
                }

                let sprite_tile_data = self.get_sprite_data(sprite);
                let sprite_pixel = sprite_tile_data.get_pixel(sprite_row as usize, sprite_col as usize);

                // Skip transparent pixels
                if sprite_pixel == 0 {
                    continue;
                }

                let sprite_palette_index = if sprite.flags.dmg_palette { 1 } else { 0 };
                let sprite_color = self.lcd.get_sprite_color(sprite_palette_index, sprite_pixel);

                // Determine if this sprite should be drawn based on priority
                let should_draw = if sprite.flags.priority {
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
                        best_sprite = Some((*sprite, sprite_color));
                    }
                }
            }

            // Draw the highest priority sprite (if any)
            if let Some((_, color)) = best_sprite {
                line[x] = color;
            }
        }
    }
    fn render_window_scanline(&mut self, line: &mut [u32; 160]) {

        let window_x = self.lcd.window_x.wrapping_sub(7);
        
        // Early return if window is not visible
        if !self.window_enabled() || self.ly < self.lcd.window_y || window_x > 166 {
            return;
        }

        // Calculate window line (Y position within the window)
        let window_line = self.window_line_counter;
        let w_tile_y = (window_line / 8) as u16; // Which tile row in the window we're on

        // Get window tile map base address (LCDC bit 6)
        let tile_map_base: u16 = if self.lcdc & 0x40 != 0 {
            0x9C00 // Window Tile Map at 0x9C00-0x9FFF
        } else {
            0x9800 // Window Tile Map at 0x9800-0x9BFF
        };

        // Get addressing mode (LCDC bit 4)
        let data_area_is_8800 = self.lcdc & 0x10 == 0;

        // Calculate tile row line we're rendering (0-7)
        let tile_line = window_line % 8;

        // Render window pixels for this scanline
        for screen_x in window_x..160 {
            // Only draw window pixels if we're at or past window_x - 7
            /*if screen_x < window_x {
                continue;
            }*/

            // Calculate the offset into the window tile map
            let tile_map_offset = (
                ((screen_x as u16 - window_x as u16) / 8) +
                    (w_tile_y * 32)
            ) as u16;

            // Get the tile ID from the window tile map
            let mut tile_id = self.vram_read(tile_map_base + tile_map_offset);

            // Adjust tile ID for 0x8800 addressing mode
            if data_area_is_8800 {
                tile_id = tile_id.wrapping_add(128);
            }

            // Calculate base address for tile data
            let base_address: u16 = if data_area_is_8800 {
                0x8800
            } else {
                0x8000
            };

            // Calculate address of this specific row of the tile
            let tile_address = base_address + (tile_id as u16 * 16) + (tile_line as u16 * 2);

            // Read the pixel data for this row
            let tile_low = self.vram_read(tile_address);
            let tile_high = self.vram_read(tile_address + 1);

            // Calculate which pixel of the tile we need (0-7)
            let pixel_x = (screen_x as u16 - window_x as u16) % 8;

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

    // Helper function to get window tile map address
    fn get_window_tile_map_addr(&self) -> u16 {
        if self.lcdc & 0x40 != 0 { // LCDC bit 6
            0x9C00 // Window Tile Map at 0x9C00-0x9FFF
        } else {
            0x9800 // Window Tile Map at 0x9800-0x9BFF
        }
    }

    // Helper function to get background/window tile data area
    fn get_bgw_data_area(&self) -> u16 {
        if self.lcdc & 0x10 != 0 { // LCDC bit 4
            0x8000 // Tile data at 0x8000-0x8FFF (unsigned)
        } else {
            0x8800 // Tile data at 0x8800-0x97FF (signed)
        }
    }

    fn render_scanline(&mut self) {
        let ly = self.ly as usize;
        let mut line = [LIGHTEST_GREEN; 160];

        self.render_background_scanline(&mut line);
        self.render_window_scanline(&mut line);
        self.render_sprite_scanline(&mut line);

        self.framebuffer[ly * 160..(ly + 1) * 160].copy_from_slice(&line);

    }

    /*
    MAIN DISPLAY END
    */

    /*
    SPRITE DISPLAY START
    */
    fn get_sprites(&self) -> Vec<Sprite> {
        let mut sprites: Vec<Sprite> = Vec::new();

        for i in 0..40 {
            let offset = (i * 4) as u16;  // Make the intent clear
            let y = self.oam_read(offset);
            let x = self.oam_read(offset + 1);
            let tile_number = self.oam_read(offset + 2);
            let flags = OAMFlags::from(self.oam_read(offset + 3));

            sprites.push(Sprite { y, x, tile_number, flags });
        }

        sprites
    }
    /*
    SPRITE DISPLAY END
    */
}

// Struct to represent the tile data
pub struct TileData {
    data: Vec<u8>,    // Use Vec to allow variable size
    height: usize,    // 8 or 16 pixels
    width: usize,     // Typically 8 pixels for Gameboy
}


impl TileData {
    // Constructor for TileData
    pub fn new(data: &[u8], height: usize) -> Self {
        assert!(height == 8 || height == 16, "Sprite height must be 8 or 16");
        assert_eq!(data.len(), height * 2, "Data size must match height (2 bytes per row)");

        TileData {
            data: data.to_vec(),
            height,
            width: 8
        }
    }

    // Get pixel color (0-3) at row and col
    pub fn get_pixel(&self, row: usize, col: usize) -> u8 {
        assert!(row < self.height, "Row out of bounds");
        assert!(col < self.width, "Column out of bounds");

        // Each row is represented by 2 bytes (bitplanes)
        let plane1 = self.data[row * 2];
        let plane2 = self.data[row * 2 + 1];

        // Extract the relevant bit for this column
        let bit_position = 7 - col; // Pixels are stored MSB first
        let low_bit = (plane1 >> bit_position) & 0x01;
        let high_bit = (plane2 >> bit_position) & 0x01;

        // Combine the two bits to get the pixel value (0-3)
        (high_bit << 1) | low_bit
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

/*fn calculate_fps(&mut self) {
        let end = self.display.borrow().get_ticks();
        let frame_time = end - self.previous_frame_time;

        if frame_time < TARGET_FRAME_TIME {
            self.display.borrow().delay(TARGET_FRAME_TIME - frame_time);
        }

        if end - self.start_time >= 1000 {
            debug!("FPS: {}", self.frame_count);
            self.start_time = end;
            self.frame_count = 0;
        }

        self.frame_count += 1;
        self.previous_frame_time = self.display.borrow().get_ticks();
    }

}*/
/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{memory_bus, rom};

    #[test]
    fn test_hblank_cycles() {
        let rom = rom::ROM::new(vec![0; 0x8000]);
        let memory_bus = Rc::new(RefCell::new(memory_bus::MemoryBus::new(None, &rom)));
        let mut cpu = crate::CPU::CPU::new(Rc::clone(&memory_bus));
        let mut ppu = Ppu::new();
        ppu.mode = 0;
        ppu.step(&mut cpu, 1);
        assert_eq!(ppu.cycles, 1);

        ppu.step(&mut cpu, 203);
        assert_eq!(ppu.cycles, 0);
    }

    #[test]
    fn test_vblank_cycles() {
        let rom = rom::ROM::new(vec![0; 0x8000]);
        let memory_bus = Rc::new(RefCell::new(memory_bus::MemoryBus::new(None, &rom)));
        let mut cpu = crate::CPU::CPU::new(Rc::clone(&memory_bus));
        let mut ppu = Ppu::new();
        ppu.mode = 1;
        ppu.step(&mut cpu, 1);
        assert_eq!(ppu.cycles, 1);
        ppu.step(&mut cpu, 455);
        assert_eq!(ppu.cycles, 0);
    }

    }
}
*/
