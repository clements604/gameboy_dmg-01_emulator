use crate::display::Display;
use crate::lcd::LCD;
use crate::CPU::CPU;
use std::cell::RefCell;
use std::rc::Rc;
use log::{debug, error, info};
use crate::interupts::Interrupt;

const DEFAULT_COLOURS: [u32; 4] = [0xFFFFFFFF, 0xFFAAAAAA, 0xFF555555, 0xFF000000]; // Palettes?
const LINES_PER_FRAME: u8 = 153;
const TICKS_PER_LINE: u16 = 456;
const Y_RES: u8 = 144;
const X_RES: u8 = 160;
const TARGET_FRAME_TIME: u32 = 1000 / 60; // 60 FPS

const STAT_MASK: u8 = 0b01010000;

pub struct Ppu {
    oam_ram: [u8; 0xA0],
    vram: [u8; 0x2000],
    cpu: Rc<RefCell<CPU>>,
    lcd: Rc<RefCell<LCD>>,
    display: Rc<RefCell<Display>>,

    mode: u8,
    lcdc: u8,
    stat: Stat,
    scroll_x: u8,
    scroll_y: u8,
    ly: u8,
    ly_compare: u8,
    cycles: u16,
    display_enabled: bool,

    bg_palette: [u32; 4],
    obj_palette_1: [u32; 4],
    obj_palette_2: [u32; 4],
}

impl Ppu {
    pub fn new(cpu: Rc<RefCell<CPU>>, lcd: Rc<RefCell<LCD>>, display: Rc<RefCell<Display>>) -> Ppu {
        let mut bg_palette = [0; 4];
        let mut obj_palette_1 = [0; 4];
        let mut obj_palette_2 = [0; 4];
        for i in 0..4 {
            bg_palette[i] = DEFAULT_COLOURS[i];
            obj_palette_1[i] = DEFAULT_COLOURS[i];
            obj_palette_2[i] = DEFAULT_COLOURS[i];
        }

        Ppu {
            oam_ram: [0; 0xA0],
            vram: [0; 0x2000],
            cpu,
            lcd,
            display,
            mode: PpuMode::OamMode as u8,
            lcdc: 0x91,
            stat: Stat::new(),
            scroll_x: 0,
            scroll_y: 0,
            ly: 0,
            ly_compare: 0,
            cycles: 0,
            display_enabled: false,
            bg_palette,
            obj_palette_1,
            obj_palette_2,
        }
    }
    pub fn calculate_stat(self) -> u8 { // TODO, this may not be right
        match self.display_enabled {
            true => u8::from(self.stat) | STAT_MASK,
            false => STAT_MASK,
        }
    }
    pub fn oam_read(&self, address: u16) -> u8 {
        debug!("OAM read at address: {:#X}", address);
        self.oam_ram[(address) as usize]
    }
    pub fn oam_write(&mut self, address: u16, value: u8) {
        debug!("OAM data: {:?}", self.oam_ram);
        self.oam_ram[(address) as usize] = value;
    }
    pub fn vram_read(&self, address: u16) -> u8 {
        debug!("VRAM read at address: {:#X}", address);
        self.vram[(address - 0x8000) as usize]
    }
    pub fn vram_write(&mut self, address: u16, value: u8) {
        debug!("VRAM write {:#4X} at address: {:#4X}", value, address);
        self.vram[(address - 0x8000) as usize] = value;
    }
    pub fn cycle(&mut self) {
        /*if ! self.display_enabled {
            error!("Display is not enabled");
            return;
        }*/

        self.cycles += 1;
        match PpuMode::from(self.mode) {
            PpuMode::OamMode => {
                info!("OAM Mode");
                if self.cycles >= 80 {
                    self.mode = PpuMode::VRamMode as u8;
                }
            }
            PpuMode::VRamMode => {
                info!("VRAM Mode");
                if self.cycles >= 172 {
                    self.mode = PpuMode::HBlankMode as u8;
                }
            }
            PpuMode::VBlankMode => {
                info!("VBlank Mode");
                self.ly += 1;
                if self.ly > 153 {
                    self.ly = 0;
                    self.mode = PpuMode::OamMode as u8;
                }
                /*else {
                    self.cycles += 114;
                }*/
                self.check_compare_interrupt();
            }
            PpuMode::HBlankMode => {
                info!("HBlank Mode");
                self.ly += 1;
                if self.ly >= 144 {
                    self.mode = PpuMode::VBlankMode as u8;
                    self.cpu.borrow_mut().trigger_interrupt(Interrupt::VBLANK);
                }
                else {
                    self.mode = PpuMode::OamMode as u8;
                }
                self.display.borrow_mut().ui_update();
                self.check_compare_interrupt();
            }
            _ => panic!("Invalid PPU mode"),
        }
    }

fn check_compare_interrupt(&mut self) {
        if self.ly == self.ly_compare {
            self.stat.coincidence_flag = true;
            if self.stat.lyc_int {
                self.cpu.borrow_mut().trigger_interrupt(Interrupt::LCDSTAT);
            }
        }
        else {
            self.stat.coincidence_flag = false;
        }
    }

}

enum PpuMode {
    OamMode = 2,
    VRamMode = 3,
    HBlankMode = 0,
    VBlankMode = 1,
}

impl std::convert::From<u8> for PpuMode {
    fn from(byte: u8) -> Self {
        match byte {
            0 => PpuMode::HBlankMode,
            1 => PpuMode::VBlankMode,
            2 => PpuMode::OamMode,
            3 => PpuMode::VRamMode,
            _ => panic!("Invalid PPU mode"),
        }
    }
}

enum TilePixelValue {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
}

struct TileRow {
    row: [TilePixelValue; 8],
}

impl TileRow {
    pub fn new(row: [TilePixelValue; 8]) -> Self {
        TileRow { row }
    }
}
struct Tile {
    rows: [TileRow; 8],
}

impl Tile {
    pub fn new(rows: [TileRow; 8]) -> Self {
        Tile { rows }
    }
}

struct Palette {

}

struct Layer {}
#[derive(Debug, Clone, Copy)]
struct SpriteFlags {
    priority: bool,
    y_flip: bool,
    x_flip: bool,
    dmg_palette: bool,
    bank: bool,
    cgb_palette: u8,
}

impl SpriteFlags {
    pub fn new() -> Self {
        SpriteFlags {
            priority: false,
            y_flip: false,
            x_flip: false,
            dmg_palette: false,
            bank: false,
            cgb_palette: 0,
        }
    }

}
impl Default for SpriteFlags {
    fn default() -> Self {
        SpriteFlags {
            priority: false,
            y_flip: false,
            x_flip: false,
            dmg_palette: false,
            bank: false,
            cgb_palette: 0,
        }
    }
}

#[derive(Debug)]
pub struct Sprite {
    y: u8,
    x: u8,
    tile_number: u8,
    flags: SpriteFlags,
}
impl Default for Sprite {
    fn default() -> Self {
        Sprite {
            y: 0,
            x: 0,
            tile_number: 0,
            flags: SpriteFlags::default(),
        }
    }
}

impl std::convert::From<SpriteFlags> for u8 {
    /*
    FLAGS:
        Bit 7 - Priority: 0 = No, 1 = BG and Window colors 1–3 are drawn over this OBJ
        Bit 6 - Y flip: 0 = Normal, 1 = Entire OBJ is vertically mirrored
        Bit 5 - X flip: 0 = Normal, 1 = Entire OBJ is horizontally mirrored
        Bit 4 - DMG palette [Non CGB Mode only]: 0 = OBP0, 1 = OBP1
        Bit 3 - Bank [CGB Mode Only]: 0 = Fetch tile from VRAM bank 0, 1 = Fetch tile from VRAM bank 1
        Bits 2,1,0 - CGB palette [CGB Mode Only]: Which of OBP0–7 to use
     */
    fn from(flag: SpriteFlags) -> u8 {
        let mut byte = 0;
        if flag.priority {
            byte |= 0b1000_0000;
        }
        if flag.y_flip {
            byte |= 0b0100_0000;
        }
        if flag.x_flip {
            byte |= 0b0010_0000;
        }
        if flag.dmg_palette {
            byte |= 0b0001_0000;
        }
        if flag.bank {
            byte |= 0b0000_1000;
        }
        byte |= flag.cgb_palette;
        byte
    }
}
impl std::convert::From<u8> for SpriteFlags {
    /*
    FLAGS:
        Bit 7 - Priority: 0 = No, 1 = BG and Window colors 1–3 are drawn over this OBJ
        Bit 6 - Y flip: 0 = Normal, 1 = Entire OBJ is vertically mirrored
        Bit 5 - X flip: 0 = Normal, 1 = Entire OBJ is horizontally mirrored
        Bit 4 - DMG palette [Non CGB Mode only]: 0 = OBP0, 1 = OBP1
        Bit 3 - Bank [CGB Mode Only]: 0 = Fetch tile from VRAM bank 0, 1 = Fetch tile from VRAM bank 1
        Bits 2,1,0 - CGB palette [CGB Mode Only]: Which of OBP0–7 to use
     */
    fn from(byte: u8) -> Self {
        let priority = byte & 0b1000_0000 != 0;
        let y_flip = byte & 0b0100_0000 != 0;
        let x_flip = byte & 0b0010_0000 != 0;
        let dmg_palette = byte & 0b0001_0000 != 0;
        let bank = byte & 0b0000_1000 != 0;
        let cgb_palette = byte & 0b0000_0111;
        SpriteFlags {
            priority,
            y_flip,
            x_flip,
            dmg_palette,
            bank,
            cgb_palette,
        }
    }
}

impl Sprite {
    // default

    pub fn new(y: u8, x: u8, tile_number: u8, flags: SpriteFlags) -> Self {
        Sprite {
            y,
            x,
            tile_number,
            flags,
        }
    }
}

struct Stat {
    lyc_int: bool,
    mode_2_oam_interrupt: bool,
    mode_1_vblank_interrupt: bool,
    mode_0_hblank_interrupt: bool,
    coincidence_flag: bool,
    ppu_mode: PpuMode,
}

impl Stat {
    pub fn new() -> Self {
        Stat {
            lyc_int: false,
            mode_2_oam_interrupt: false,
            mode_1_vblank_interrupt: false,
            mode_0_hblank_interrupt: false,
            coincidence_flag: false,
            ppu_mode: PpuMode::OamMode,
        }
    }

}

impl std::convert::From<Stat> for u8 {
    fn from(stat: Stat) -> u8 {
        let mut byte = 0;
        if stat.lyc_int {
            byte |= 0b0100_0000;
        }
        if stat.mode_2_oam_interrupt {
            byte |= 0b0010_0000;
        }
        if stat.mode_1_vblank_interrupt {
            byte |= 0b0001_0000;
        }
        if stat.mode_0_hblank_interrupt {
            byte |= 0b0000_1000;
        }
        if stat.coincidence_flag {
            byte |= 0b0000_0100;
        }
        byte |= stat.ppu_mode as u8;
        byte | STAT_MASK
    }
}

impl std::convert::From<u8> for Stat {
    fn from(byte: u8) -> Self {
        let lyc_int = byte & 0b0100_0000 != 0;
        let mode_2_oam_interrupt = byte & 0b0010_0000 != 0;
        let mode_1_vblank_interrupt = byte & 0b0001_0000 != 0;
        let mode_0_hblank_interrupt = byte & 0b0000_1000 != 0;
        let coincidence_flag = byte & 0b0000_0100 != 0;
        let ppu_mode = PpuMode::from(byte & 0b0000_0011);
        Stat {
            lyc_int,
            mode_2_oam_interrupt,
            mode_1_vblank_interrupt,
            mode_0_hblank_interrupt,
            coincidence_flag,
            ppu_mode: PpuMode::from(ppu_mode),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{memory_bus, rom};

    #[test]
    fn test_scratchpad() {
        assert_eq!(1 | 0, 1);
    }

    #[test]
    fn test_stat_from_u8() {
        let mut stat = Stat::new();

        stat.lyc_int = true;
        stat.mode_1_vblank_interrupt = true;
        stat.ppu_mode = PpuMode::HBlankMode;

        let byte: u8 = stat.into();

        let stat_mask =  1 << 7;

        let calc_stat =

        println!("stat mask {:08b}", byte);

        assert_eq!(byte, 0b0101_0000);
    }

}
