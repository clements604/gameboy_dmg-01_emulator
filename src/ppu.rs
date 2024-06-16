use log::debug;

pub struct Ppu {
    oam_ram: [u8; 0xA0],
    vram: [u8; 0x2000],
}
#[derive(Debug, Clone, Copy)]
pub struct OamEntry {
    y: u8,
    x: u8,
    tile_number: u8,
    flags: u8,
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

impl OamEntry {
    pub fn new() -> OamEntry {
        OamEntry {
            y: 0,
            x: 0,
            tile_number: 0,
            flags: 0,
        }
    }

    fn to_bytes(&self) -> [u8; 4] {
        [self.y, self.x, self.tile_number, self.flags]
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        OamEntry {
            y: bytes[0],
            x: bytes[1],
            tile_number: bytes[2],
            flags: bytes[3],
        }
    }

    fn get_bg_priority(&self) -> bool {
        self.flags & 0b1000_0000 != 0
    }
    fn set_bg_priority(&mut self, value: bool) {
        if value {
            self.flags |= 0b1000_0000;
        } else {
            self.flags &= 0b0111_1111;
        }
    }

    fn get_y_flip(&self) -> bool {
        self.flags & 0b0100_0000 != 0
    }
    fn set_y_flip(&mut self, value: bool) {
        if value {
            self.flags |= 0b0100_0000;
        } else {
            self.flags &= 0b1011_1111;
        }
    }

    fn get_x_flip(&self) -> bool {
        self.flags & 0b0010_0000 != 0
    }
    fn set_x_flip(&mut self, value: bool) {
        if value {
            self.flags |= 0b0010_0000;
        } else {
            self.flags &= 0b1101_1111;
        }
    }

    fn get_dmg_palette(&self) -> bool {
        self.flags & 0b0001_0000 != 0
    }
    fn set_dmg_palette(&mut self, value: bool) {
        if value {
            self.flags |= 0b0001_0000;
        } else {
            self.flags &= 0b1110_1111;
        }
    }

    fn get_bank(&self) -> bool {
        self.flags & 0b0000_1000 != 0
    }
    fn set_bank(&mut self, value: bool) {
        if value {
            self.flags |= 0b0000_1000;
        } else {
            self.flags &= 0b1111_0111;
        }
    }

    fn get_cgb_palette(&self) -> u8 {
        self.flags & 0b0000_0111
    }
    fn set_cgb_palette(&mut self, value: u8) {
        self.flags &= 0b1111_1000;
        self.flags |= value & 0b0000_0111;
    }
}
impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            oam_ram: [0; 0xA0],
            vram: [0; 0x2000],
        }
    }

    pub fn oam_read(&self, address: u16) -> u8 {
        debug!("OAM read at address: {:#X}", address);
        /*if address < 0xFE00 || address >= 0xFEA0 {
            panic!("Attempt to read from invalid OAM address: {:#X}", address);
        }*/
        self.oam_ram[(address) as usize]
    }

    pub fn oam_write(&mut self, address: u16, value: u8) {
        debug!("OAM write at address: {:#X}", address);
        /*if address < 0xFE00 || address >= 0xFEA0 {
            panic!("Attempt to write to invalid OAM address: {:#X}", address);
        }*/
        self.oam_ram[(address) as usize] = value;
        debug!("OAM data: {:?}", self.oam_ram);
    }

    pub fn vram_read(&self, address: u16) -> u8 {
        debug!("VRAM read at address: {:#X}", address);
        self.vram[(address - 0x8000) as usize]
    }

    pub fn vram_write(&mut self, address: u16, value: u8) {
        debug!("VRAM write at address: {:#X}", address);
        self.vram[(address - 0x8000) as usize] = value;
    }

}