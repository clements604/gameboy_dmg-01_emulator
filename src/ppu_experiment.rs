use std::cell::RefCell;
use crate::interupts;
use crate::interupts::Interrupt;
use crate::CPU::{Flag, FlagsRegister, CPU};
use log::{debug, error, info};
use std::fmt;
use std::rc::Rc;
use crate::display::Display;
use crate::lcd::LCD;
use crate::memory_bus::MemoryBus;

const TILE_START: u16 = 0x8000;
const TILE_END: u16 = 0x97FF;

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
    display: Rc<RefCell<Display>>,

    pub lcdc: u8,
    pub stat: u8,
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub ly: u8,
    pub ly_compare: u8,
}

//Display for Ppu
impl fmt::Display for Ppu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "LY: {},\
            LYC: {},\
            Mode: {},\
            Line Ticks: {},\
            Current Frame: {},\
            Previous Frame Time: {},\
            Start Time: {},\
            Frame Count: {},\
            LCDC: {:#X},\
            STAT: {:#X},\
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
pub struct OamEntry {
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

impl OamEntry {
    pub fn new() -> OamEntry {
        OamEntry {
            y: 0,
            x: 0,
            tile_number: 0,
            flags: OAMFlags::from(0),
        }
    }

}
impl Ppu {
    pub fn new(cpu: Rc<RefCell<CPU>>, lcd: Rc<RefCell<LCD>>, display: Rc<RefCell<Display>>) -> Ppu {
        Ppu {
            oam_ram: [0; 0xA0],
            vram: [0x0000; 0x2000],
            //ly: 0,
            //lyc: 0,
            mode: 2,
            line_ticks: 0,
            current_frame: 0,
            previous_frame_time: 0,
            start_time: 0,
            frame_count: 0,
            cpu,
            lcd,
            display,
            lcdc: 0x91,
            stat: 0,
            scroll_x: 0,
            scroll_y: 0,
            ly: 0,
            ly_compare: 0,
        }
    }
    fn is_bgw_enabled(&self) -> bool {
        self.lcdc & 0x01 != 0
    }

    fn is_obj_enabled(&self) -> bool {
        self.lcdc & 0x02 != 0
    }

    fn get_object_height(&self) -> u8 {
        if self.lcdc & 0x04 != 0 {
            16
        } else {
            8
        }
    }

    fn get_bg_map_area(&self) -> u16 {
        if self.lcdc & 0x08 != 0 {
            0x9C00
        } else {
            0x9800
        }
    }

    fn get_tile_data_area(&self) -> u16 {
        if self.lcdc & 0x10 != 0 {
            0x8000
        } else {
            0x8800
        }
    }

    fn is_window_enabled(&self) -> bool {
        self.lcdc & 0x20 != 0
    }

    fn get_window_map_area(&self) -> u16 {
        if self.lcdc & 0x40 != 0 {
            0x9C00
        } else {
            0x9800
        }
    }
    fn is_lcd_enabled(&self) -> bool {
        self.lcdc & 0x80 != 0
    }

    fn set_mode(&mut self, mode: u8) {
        self.mode = mode;
        self.stat = (self.stat & 0xFC) | mode;//TODO maybe not correct?
    }

    fn increment_ly(&mut self) {

        self.ly += 1;

        if self.ly > 154 {
            self.ly = 0; // Prepare for the next frame
        }

        if self.ly == self.ly_compare {
            // set lyc bit
            self.stat |= 0x04;
            if self.is_stat_interrupt_enabled(StatInterrupt::LYC) {
                self.cpu.borrow_mut().trigger_interrupt(Interrupt::LCDSTAT);
            }
        }
        else {
            // clear lyc bit
            self.stat &= !0x04;
        }
    }
    pub fn tick(&mut self, cycles: u8) {

        if !self.is_lcd_enabled() {
            debug!("LCD is disabled");
            self.ly = 0;
            self.set_mode(HBLANK_MODE);
            //self.mode = HBLANK_MODE;//TODO maybe not correct?
            return;
        }

        //debug!("Line ticks: {}", self.line_ticks);
        self.line_ticks += cycles as u16;
        //debug!("Line ticks + cycles: {}", self.line_ticks);

        if self.line_ticks > TICKS_PER_LINE && self.mode != VBLANK_MODE {
            self.ly = self.ly.wrapping_add(1);
            if self.ly <= 144 {
                self.line_ticks = 0;
            }
        }

        match self.line_ticks {
            0..=80 => {
                self.set_mode(OAM_MODE);
            }
            81..=251 => {
                self.set_mode(VRAM_MODE);
            }
            252..=455 => {
                self.set_mode(HBLANK_MODE);
            },
            456..=4559 => {
                self.set_mode(VBLANK_MODE);
            },
            4560.. => {
                self.set_mode(OAM_MODE);
                self.line_ticks = 0;
                if self.ly > 154 {
                    self.ly = 0;
                }
            },
            _ => {
                panic!("Invalid line ticks: {}", self.line_ticks);
            }
        }

        let stat_bit_0_to_2: u8 = match self.ly == self.ly_compare {
            true => 0b100 | self.mode,
            false => self.mode,
        };

        self.stat &= 0b11111000;
        self.stat = (self.stat & 0xF8) | stat_bit_0_to_2;

        if self.mode == OAM_MODE {

        }

        /*match self.mode {
            HBLANK_MODE => { // H-Blank
                if self.line_ticks >= 204 {
                    self.line_ticks = 0;
                    //self.increment_ly();
                    self.ly += 1;
                    
                    if self.ly == Y_RES - 1 {
                        self.set_mode(VBLANK_MODE);
                        //self.display.borrow_mut().render();
                        //TODO render scanline here
                        //self.trigger_interrupt(Interrupt::VBLANK);
                    }
                    else {
                        self.set_mode(OAM_MODE);
                    }
                }
            },
            VBLANK_MODE => { // V-Blank
                if self.line_ticks >= 456 {
                    self.line_ticks = 0;
                    //self.increment_ly();
                    self.ly += 1;
                    
                    if self.ly > 153 {
                        self.set_mode(OAM_MODE);
                        self.current_frame += 1;
                        self.ly = 0;
                        self.calculate_fps();
                    }
                }
            },
            OAM_MODE => { // OAM Search
                if self.line_ticks >= 80 {
                    self.line_ticks = 0;
                    self.set_mode(VRAM_MODE);
                }
            },
            VRAM_MODE => { // VRAM Transfer
               if self.line_ticks >= 172 {
                    self.line_ticks = 0;
                    self.set_mode(HBLANK_MODE);
                   
                   //TODO render scanline here
                }
            },
            _ => panic!("Unknown PPU mode: {}", self.mode),
        }*/
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
            //0xFF41 => self.stat,
            0xFF41 => {
                0b10000000
                    | if self.is_stat_interrupt_enabled(StatInterrupt::LYC) { 1 } else { 0 }  << 6
                    | if self.is_stat_interrupt_enabled(StatInterrupt::OAM) { 1 } else { 0 } << 5
                    | if self.is_stat_interrupt_enabled(StatInterrupt::VBLANK) { 1 } else { 0 } << 4
                    | if self.is_stat_interrupt_enabled(StatInterrupt::HBLANK) { 1 } else { 0 } << 3
                    | if self.ly == self.ly_compare { 1 } else { 0 } << 2
                    | self.mode
            },
            0xFF42 => self.scroll_y,
            0xFF43 => self.scroll_x,
            0xFF44 => self.ly,
            0xFF45 => self.ly_compare,
            0xFF47 => self.lcd.borrow().bg_palette,
            0xFF48 => self.lcd.borrow().obj_palette[0],
            0xFF49 => self.lcd.borrow().obj_palette[1],
            0xFF4A => self.lcd.borrow().window_y,
            0xFF4B => self.lcd.borrow().window_x,
            _ => panic!("Invalid LCD address: {:#X}", address),
        }
    }
    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xFF40 => self.lcdc = value,
            0xFF41 => self.stat = value,
            0xFF42 => self.scroll_y = value,
            0xFF43 => self.scroll_x = value,
            0xFF44 => self.ly = value,
            0xFF45 => self.ly_compare = value,
            0xFF46 => {
                //debug!("DMA transfer start: {:#X}", value);
                self.lcd.borrow_mut().dma.upgrade().unwrap().borrow_mut().dma_start(value);
            }
            0xFF47 => self.lcd.borrow_mut().update_palette(value, 0),
            0xFF48 => self.lcd.borrow_mut().update_palette(value & 0b11111100, 1),
            0xFF49 => self.lcd.borrow_mut().update_palette(value & 0b11111100, 2),
            0xFF4A => self.lcd.borrow_mut().window_y = value,
            0xFF4B => self.lcd.borrow_mut().window_x = value,
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
            interupts::Interrupt::VBLANK => {
                self.cpu.borrow_mut().trigger_interrupt(interupts::Interrupt::VBLANK);
            }
            interupts::Interrupt::LCDSTAT => {
                self.cpu.borrow_mut().trigger_interrupt(interupts::Interrupt::LCDSTAT);
            }
            _ => {
                panic!("Invalid interrupt: {:?}", interrupt);
            }
        }
    }

    fn calculate_fps(&mut self) {
        let end = self.display.borrow().get_ticks();
        let frame_time = end - self.previous_frame_time;

        if frame_time < TARGET_FRAME_TIME {
            self.display.borrow().delay(TARGET_FRAME_TIME - frame_time);
        }

        if end - self.start_time >= 1000 {
            //debug!("FPS: {}", self.frame_count);
            self.start_time = end;
            self.frame_count = 0;
        }

        self.frame_count += 1;
        self.previous_frame_time = self.display.borrow().get_ticks();
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{memory_bus, rom};

    /*#[test]
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

    #[test]
    fn test_oam_cycles() {
        let rom = rom::ROM::new(vec![0; 0x8000]);
        let memory_bus = Rc::new(RefCell::new(memory_bus::MemoryBus::new(None, &rom)));
        let mut cpu = crate::CPU::CPU::new(Rc::clone(&memory_bus));
        let mut ppu = Ppu::new();
        ppu.mode = 2;
        ppu.step(&mut cpu, 1);
        assert_eq!(ppu.cycles, 1);
        ppu.step(&mut cpu, 79);
        assert_eq!(ppu.cycles, 0);
    }

    #[test]
    fn test_lcd_cycles() {
        let rom = rom::ROM::new(vec![0; 0x8000]);
        let memory_bus = Rc::new(RefCell::new(memory_bus::MemoryBus::new(None, &rom)));
        let mut cpu = crate::CPU::CPU::new(Rc::clone(&memory_bus));
        let mut ppu = Ppu::new();
        ppu.mode = 3;
        ppu.step(&mut cpu, 1);
        assert_eq!(ppu.cycles, 1);
        ppu.step(&mut cpu, 171);
        assert_eq!(ppu.cycles, 0);
    }

    #[test]
    fn test_lyc_ly_interrupt() {
        let rom = rom::ROM::new(vec![0; 0x8000]);
        let memory_bus = Rc::new(RefCell::new(memory_bus::MemoryBus::new(None, &rom)));
        let mut cpu = crate::CPU::CPU::new(Rc::clone(&memory_bus));
        let mut ppu = Ppu::new();
        // Case 1: LY matches LYC, interrupt should trigger
        ppu.ly = 100;
        ppu.lyc = 100;
        ppu.stat = 0x40; // LYC=LY interrupt enabled
        ppu.mode = 0; // H-Blank mode
        ppu.step(&mut cpu, 204); // Simulate step to the end of H-Blank
        assert_eq!(ppu.stat & 0x04, 0x04); // Coincidence flag set
                                           // Ensure interrupt was triggered (additional logic needed for full test)

        // Reset PPU state
        ppu.ly = 0;
        ppu.lyc = 0;
        ppu.stat = 0;
        ppu.mode = 0;
        ppu.cycles = 0;

        // Case 2: LY does not match LYC, interrupt should not trigger
        ppu.ly = 100;
        ppu.lyc = 101;
        ppu.stat = 0x40; // LYC=LY interrupt enabled
        ppu.mode = 0; // H-Blank mode
        ppu.step(&mut cpu, 204); // Simulate step to the end of H-Blank
        assert_eq!(ppu.stat & 0x04, 0x00); // Coincidence flag not set
                                           // Ensure interrupt was not triggered (additional logic needed for full test)
    }*/
}
