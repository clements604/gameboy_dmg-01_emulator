use std::cell::RefCell;
use crate::{display, interupts, main, main_display};
use crate::interupts::Interrupt;
use crate::CPU::{Flag, FlagsRegister, CPU};
use log::{debug, error, info};
use std::fmt;
use std::rc::Rc;
use minifb::Key::P;
use crate::display::LIGHTEST_GREEN;
//use crate::display::Display;
use crate::lcd::LCD;
use crate::main_display::get_mififb_colour;
use crate::memory_bus::MemoryBus;

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
const TARGET_FRAME_TIME : u32 = 1000 / 60; // 60 FPS

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
    cpu: Rc<RefCell<CPU>>,
    lcd: Rc<RefCell<LCD>>,
    //display: Rc<RefCell<Display>>,

    pub lcdc: u8,
    pub stat: u8,
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub ly: u8,
    pub ly_compare: u8,

    pub background_buffer: Vec<u32>,
    pub viewport: Vec<u32>,
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
    pub fn new(cpu: Rc<RefCell<CPU>>, lcd: Rc<RefCell<LCD>>/*, display: Rc<RefCell<Display>>*/) -> Ppu {
        Ppu {
            oam_ram: [0; 0xA0],
            vram: [0x0000; 0x2000],
            mode: 2,
            line_ticks: 0,
            current_frame: 0,
            previous_frame_time: 0,
            start_time: 0,
            frame_count: 0,
            cpu,
            lcd,
            //display,
            lcdc: 0x91,
            stat: 0,
            scroll_x: 0,
            scroll_y: 0,
            ly: 0,
            ly_compare: 0,

            background_buffer: vec![main_display::RED; main_display::WIDTH * main_display::HEIGHT],
            viewport: vec![main_display::RED; main_display::VIEWPORT_WIDTH * main_display::VIEWPORT_HEIGHT],
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
        self.stat = (self.stat & 0xFC) | mode;
    }

    fn get_oam_int_select(&self) -> bool {
        self.stat & 0x20 != 0
    }

    fn sprites_enabled(&self) -> bool {
        self.lcdc & 0x02 != 0
    }

    fn bg_window_enabled(&self) -> bool {
        self.lcdc & 0x01 != 0
    }

    fn increment_ly(&mut self) {
        self.ly = self.ly.wrapping_add(1);

        if self.ly == self.ly_compare {
            self.stat |= 0x04;
            // check for stat interrupt
            if self.is_stat_interrupt_enabled(StatInterrupt::LYC) {
                self.cpu.borrow_mut().trigger_interrupt(Interrupt::LCDSTAT);
            }
        } else {
            self.stat &= !0x04; // Reset ly coincidence flag
        }
    }

    pub fn tick(&mut self, cycles: u8) {
        debug!("PPU tick with cycles: {}", cycles);

        if !self.lcd_ppu_enabled() {
            debug!("LCD is disabled");
            //self.ly = 0;
            //self.line_ticks = 0;
            //self.set_ppu_mode(OAM_MODE);
            return;
        }

        self.line_ticks += cycles as u16;
        debug!("Line ticks: {}", self.line_ticks);

        // Handle full line completion (ticks >= 456)
        if self.line_ticks > 456 && self.mode != VBLANK_MODE {
            self.ly = self.ly.wrapping_add(1);  // Move to the next line

            // Handle mode transitions based on the line (LY)
            if self.ly <= 144 {
                self.line_ticks = 0;
            }
        }

        match self.mode {
            HBLANK_MODE => {
                if self.stat & 0x08 != 0 || self.stat & 0x40 != 0 {
                    self.cpu.borrow_mut().trigger_interrupt(Interrupt::LCDSTAT);
                }
                if self.line_ticks >= 204 { // 376 - mode 3’s (VRAM_MODE) duration
                    //self.ly = self.ly.wrapping_add(1);
                    self.increment_ly();
                    if self.ly >= Y_RES {
                        self.mode = VBLANK_MODE;
                        self.cpu.borrow_mut().trigger_interrupt(Interrupt::VBLANK);
                        self.update_stat_interrupts();
                        self.current_frame += 1;
                        //self.calculate_fps();
                    } else {
                        self.mode = OAM_MODE;
                    }
                    self.line_ticks = 0;
                }
            }
            VBLANK_MODE => {
                if self.stat & 0x10 != 0 || self.stat & 0x40 != 0 {
                    self.cpu.borrow_mut().trigger_interrupt(Interrupt::LCDSTAT);
                }
                if self.line_ticks >= TICKS_PER_LINE { // 4560 / 10 scanlines
                    //self.ly = self.ly.wrapping_add(1);
                    self.increment_ly();
                    if self.ly >= LINES_PER_FRAME {
                        self.mode = OAM_MODE;
                        self.ly = 0;
                        //TODO add main display update here
                    }
                    self.line_ticks = 0;
                }
            }
            OAM_MODE => {
                if self.stat & 0x20 != 0 || self.stat & 0x40 != 0 {
                    self.cpu.borrow_mut().trigger_interrupt(Interrupt::LCDSTAT);
                }
                if self.line_ticks >= 80 {
                    self.mode = VRAM_MODE;
                }
            }
            VRAM_MODE => {
                if self.stat & 0x30 != 0 || self.stat & 0x40 != 0 {
                    self.cpu.borrow_mut().trigger_interrupt(Interrupt::LCDSTAT);
                }
                if self.line_ticks >= 172 { // Between 172 and 289 line_ticks (with conditions).
                    self.mode = HBLANK_MODE;
                    self.update_stat_interrupts();
                }
            }
            _ => {
                panic!("Invalid PPU mode: {}", self.mode);
            }
        }

        // Update STAT register bits 0-2
        let stat_bit_0_to_2: u8 = match self.ly == self.ly_compare {
            true => 0b100 | self.mode,
            false => self.mode,
        } as u8;
        debug!("STAT bits 0-2: {:#X}", stat_bit_0_to_2);
        self.stat = (self.stat & 0b11111000) | stat_bit_0_to_2;
        debug!("STAT: {:#X}", self.stat);
    }

    fn update_stat_interrupts(&mut self) {
        let lcd = self.lcd.borrow();
        let lyc_ly_coincidence = self.ly == self.ly_compare;
        if lyc_ly_coincidence {
            self.stat |= 0x04; // Set coincidence flag
        } else {
            self.stat &= !0x04; // Clear coincidence flag
        }

        if (self.stat & 0x40 != 0 && lyc_ly_coincidence) || // LYC=LY interrupt
            (self.stat & 0x20 != 0 && self.mode == 2) || // OAM interrupt
            (self.stat & 0x10 != 0 && self.mode == 1) || // V-Blank interrupt
            (self.stat & 0x08 != 0 && self.mode == 0) { // H-Blank interrupt
            self.cpu.borrow_mut().trigger_interrupt(Interrupt::LCDSTAT);
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
            0xFF47 => self.lcd.borrow().bg_palette,
            _ => panic!("Invalid LCD address: {:#X}", address),
        }
    }
    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xFF40 => self.lcdc = value,
            0xFF41 => self.stat = value,
            0xFF42 => self.scroll_y = value,
            0xFF43 => self.scroll_x = value,
            0xFF44 => {
                error!("Attempt to write to read-only register: {:#X}", address);
                self.ly = value;
            }
            0xFF45 => self.ly_compare = value,
            0xFF46 => {
                debug!("DMA transfer start: {:#X}", value);
                self.lcd.borrow_mut().dma.upgrade().unwrap().borrow_mut().dma_start(value);
            },
            0xFF47 => self.lcd.borrow_mut().bg_palette = value,
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
        //debug!("OAM read at address: {:#X}", address);
        /*if address < 0xFE00 || address >= 0xFEA0 {
            panic!("Attempt to read from invalid OAM address: {:#X}", address);
        }*/
        self.oam_ram[(address) as usize]
    }

    pub fn oam_write(&mut self, address: u16, value: u8) {
        //debug!("OAM write at address: {:#X}", address);
        /*if address < 0xFE00 || address >= 0xFEA0 {
            panic!("Attempt to write to invalid OAM address: {:#X}", address);
        }*/
        self.oam_ram[(address) as usize] = value;
        //debug!("OAM data: {:?}", self.oam_ram);
    }

    pub fn vram_read(&self, address: u16) -> u8 {
        //debug!("VRAM read at address: {:#X}", address);
        self.vram[(address - 0x8000) as usize]
    }

    pub fn vram_write(&mut self, address: u16, value: u8) {
        //debug!("VRAM write {:#4X} at address: {:#4X}", value, address);
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

    pub fn get_tile_set(&self) -> [[u8; 16] ; 256] {
        let mut tile_set = [[0; 16]; 256];//256 tiles

        for tile_number in 0..256 {
            debug!("{:4X}", 0x8000 + (tile_number * 16));
            tile_set[tile_number] = self.get_tile_from_memory((0x8000 + (tile_number * 16)) as u16); // TODO this needs to be dynamic based on tile map value from memory?
        }
        debug!("Tile set {:?}", tile_set);
        tile_set
    }

    fn get_tile_from_memory(&self, address: u16) -> [u8; 16] {
        let mut tile = [0; 16];

        for x in 0..16 {
            tile[x] = self.vram_read(address + x as u16);
        }
        debug!("Returning tile {:?}", tile);
        tile
    }
    
    pub fn get_tile_map(&self) -> Vec<&[u8]>{
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
        let palette = self.lcd.borrow().bg_palette;
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
        info!("BG Tile Map: {:#X}", bg_tile_map);

        for x in 0..1024 {
            tile_map.push(self.vram_read(bg_tile_map + x as u16));
        }
        tile_map
    }

    pub fn populate_background_tiles(&self) -> Vec<[u8;16]>{
        let mut tiles: Vec<[u8;16]> = Vec::new();
        let tile_map = self.get_background_tile_map();
        let background_tile_set = self.get_tile_set();

        for tile in tile_map.iter() {
            info!("Tile: {:#X}", tile);
            let tile_data = background_tile_set[*tile as usize].clone();
            //info!("BG Tiles: {:?}", tile_data);

            tiles.push(tile_data);
        }
        //info!("Number of tiles: {}", tiles.len());
        //info!("Tile Map: {:?}", tiles);

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
        info!("Tile Map Base: {:#X}", tile_map_base);
        for i in 0..1024 {
            tile_map[i] = self.vram_read((tile_map_base as usize + i) as u16);
        }
        tile_map
    }
    pub fn populate_background_buffer(&mut self) {
        let tile_set = self.get_tile_set();
        let tile_map = self.get_bg_tile_map();

        for (index, tile_item) in tile_map.iter().enumerate() {
            let tile = tile_set[*tile_item as usize];
            let mfb_tile = self.convert_tile_to_minifb_format(tile);

            for (x, pixel) in mfb_tile.iter().enumerate() {
                let h_offset = (x % 8) + ((index % 32) * 8);
                let v_offset = (x / 8) + ((index / 32) * 8) * main_display::WIDTH;
                self.background_buffer[h_offset + v_offset] = *pixel;
            }
        }
    }
    pub fn convert_tile_to_minifb_format(&mut self, tile_data: [u8; 16]) -> Vec<u32>{
        let mut minifb_tile: Vec<u32> = vec![main_display::RED; 64];
        for x in (0..tile_data.len()).step_by(2) {
            let lsb = tile_data[x];
            let msb = tile_data[x + 1];
            for bit in 0..8 {
                let bit_lsb = lsb & (1 << bit) != 0;
                let bit_msb = msb & (1 << bit) != 0;
                let pair = ((bit_lsb as u8) << 1) | (bit_msb as u8);
                let bgp_palette = self.convert_pixel_to_bgb_palette(pair);
                let mfb_pixel = main_display::get_mififb_colour(bgp_palette);
                //info!("MFB Pixel: {:#X}", mfb_pixel);
                minifb_tile[(x / 2 * 8) + (7 - bit) as usize] = mfb_pixel;
            }
        }
    minifb_tile
    }
    fn convert_pixel_to_bgb_palette(&self, pixel: u8) -> u8 {
        let palette = self.lcd.borrow().bg_palette;
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
    pub fn render_viewport(&mut self) -> Vec<u32> {
        const TILE_SIZE: usize = 8;
        const VIEWPORT_WIDTH: usize = 160;
        const VIEWPORT_HEIGHT: usize = 144;
        const TILE_MAP_WIDTH: usize = 32;
        let scx = self.scroll_x as usize; // Scroll X
        let scy = self.scroll_y as usize; // Scroll Y

        // Calculate the starting tile and pixel offsets
        let start_tile_x = scx / TILE_SIZE;
        let start_tile_y = scy / TILE_SIZE;
        let pixel_offset_x = scx % TILE_SIZE;
        let pixel_offset_y = scy % TILE_SIZE;

        let mut framebuffer: Vec<u32> = vec![LIGHTEST_GREEN; VIEWPORT_WIDTH * VIEWPORT_HEIGHT];

        // Loop through the 20x18 tiles visible in the viewport
        for tile_y in 0..18 {
            for tile_x in 0..20 {
                // Calculate the position in the tile map
                let map_x = (start_tile_x + tile_x) % TILE_MAP_WIDTH;
                let map_y = (start_tile_y + tile_y) % TILE_MAP_WIDTH;
                let tile_index = self.get_tile_index(map_x, map_y);

                // Fetch the tile data from VRAM
                let tile_data = self.get_tile_data(tile_index);

                // Extract pixel data, handling partial tiles at the edges
                for row in 0..TILE_SIZE {
                    let screen_y = (tile_y * TILE_SIZE + row).wrapping_sub(pixel_offset_y);

                    if screen_y < VIEWPORT_HEIGHT {
                        for col in 0..TILE_SIZE {
                            let screen_x = (tile_x * TILE_SIZE + col).wrapping_sub(pixel_offset_x);

                            if screen_x < VIEWPORT_WIDTH {
                                let pixel = tile_data.get_pixel(row, col);
                                let color = main_display::get_mififb_colour(pixel);
                                framebuffer[screen_y * VIEWPORT_WIDTH + screen_x] = color;
                            }
                        }
                    }
                }
            }
        }
        framebuffer
    }
    pub fn get_tile_index(&self, map_x: usize, map_y: usize) -> u8 {
        const TILE_MAP_WIDTH: usize = 32;  // 32 tiles per row in the tile map
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

    // Get the tile data for a given tile index
    pub fn get_tile_data(&self, tile_index: u8) -> TileData {
        const TILE_DATA_AREA_0: u16 = 0x8000; // Unsigned region
        const TILE_DATA_AREA_1: u16 = 0x8800; // Signed region
        const TILE_SIZE_BYTES: usize = 16; // 16 bytes per tile (2 bytes per row for 8 rows)

        // Determine the base address of the tile data area (based on LCDC register)
        let tile_data_base: u16 = if self.get_bg_tile_map_area() {
            TILE_DATA_AREA_1 // 0x8800 - signed indices
        } else {
            TILE_DATA_AREA_0 // 0x8000 - unsigned indices
        };
        
        let tile_address = if tile_data_base == TILE_DATA_AREA_1 {
            // Treat tile_index as signed if using the 0x8800-0x97FF region
            let signed_tile_index = tile_index as i16;
            // To avoid overflow, cast to i16, then calculate address safely
            (TILE_DATA_AREA_1 as i16 + signed_tile_index * TILE_SIZE_BYTES as i16) as u16
        } else {
            TILE_DATA_AREA_0 + (tile_index as u16 * TILE_SIZE_BYTES as u16)
        };
        
        let mut tile_data = [0u8; TILE_SIZE_BYTES];
        for i in 0..TILE_SIZE_BYTES {
            tile_data[i] = self.vram_read(tile_address + i as u16);
        }
        self.get_sprites();
        TileData::new(tile_data)
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
            //let address = 0xFE00 + i * 4;
            let address = i * 4;
            let y = self.oam_read(address) as u8;
            let x = self.oam_read(address + 1) as u8;
            let tile_number = self.oam_read(address + 2) as u8;
            let flags = OAMFlags::from(self.oam_read(address + 3));
            sprites.push(Sprite { y, x, tile_number, flags });
        }
        info!("Sprites: {:?}", sprites);
        sprites
    }
    /*
    SPRITE DISPLAY END
    */

}

// Struct to represent the tile data
pub struct TileData {
    data: [u8; main_display::TILE_SIZE * 2],
}

impl TileData {
    // Constructor for TileData
    pub fn new(data: [u8; main_display::TILE_SIZE * 2]) -> Self {
        TileData { data }
    }

    // Get pixel color (0-3) at row and col
    pub fn get_pixel(&self, row: usize, col: usize) -> u8 {
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