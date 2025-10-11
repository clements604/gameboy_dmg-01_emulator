use log::error;

// Address constants
const BG_PALETTE_ADDR: u16 = 0xFF47;
const OBJ_PALETTE0_ADDR: u16 = 0xFF48;
const OBJ_PALETTE1_ADDR: u16 = 0xFF49;
const WINDOW_Y_ADDR: u16 = 0xFF4A;
const WINDOW_X_ADDR: u16 = 0xFF4B;

// Palette constants
const PALETTE_MASK: u8 = 0x03;
const DEFAULT_BG_PALETTE: u8 = 0xFC;
const DEFAULT_OBJ_PALETTE: u8 = 0xFF;

// Bit shift constants
const PALETTE_SHIFT_2: u8 = 2;
const PALETTE_SHIFT_4: u8 = 4;
const PALETTE_SHIFT_6: u8 = 6;

// Colour constants (ARGB format)
pub const DARKEST_GREEN: u32 = 0xFF142C38;
const DARK_GREEN: u32 = 0xFF548C70;
const LIGHT_GREEN: u32 = 0xFFACD490;
pub const LIGHTEST_GREEN: u32 = 0xFFE8FCCC;
pub const DEFAULT_COLOURS: [u32; 4] = [
    LIGHTEST_GREEN, // This would be the colour for palette 00
    LIGHT_GREEN,    // This would be the colour for palette 01
    DARK_GREEN,     // This would be the colour for palette 10
    DARKEST_GREEN   // This would be the colour for palette 11
];
pub struct LCD {
    pub bg_palette: u8,
    pub obj_palette: [u8; 2],
    pub window_x: u8,
    pub window_y: u8,
    
    pub bg_colours: [u32; 4],
    pub sp1_colours: [u32; 4],
    pub sp2_colours: [u32; 4],
}

impl LCD {
    pub fn new() -> LCD {
        
        let mut bg_colours = [0; 4];
        let mut sp1_colours = [0; 4];
        let mut sp2_colours = [0; 4];
        for i in 0..4 {
            bg_colours[i] = DEFAULT_COLOURS[i];
            sp1_colours[i] = DEFAULT_COLOURS[i];
            sp2_colours[i] = DEFAULT_COLOURS[i];
        }
        
        LCD {
            bg_palette: DEFAULT_BG_PALETTE,
            obj_palette: [DEFAULT_OBJ_PALETTE; 2],
            window_x: 0,
            window_y: 0,
            bg_colours,
            sp1_colours,
            sp2_colours,
        }
    }
    pub fn read(&self, address: u16) -> u8 {
        match address {
            BG_PALETTE_ADDR => self.bg_palette,
            OBJ_PALETTE0_ADDR => self.obj_palette[0],
            OBJ_PALETTE1_ADDR => self.obj_palette[1],
            WINDOW_Y_ADDR => self.window_y,
            WINDOW_X_ADDR => self.window_x,
            _ => panic!("Invalid LCD address: {:#X}", address),
        }
    }
    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            BG_PALETTE_ADDR => {
                self.bg_palette = value;
                self.update_palette(value, 0);  // Update bg_colours
            },
            OBJ_PALETTE0_ADDR => {
                self.obj_palette[0] = value;
                self.update_palette(value, 1);  // Update sp1_colours
            },
            OBJ_PALETTE1_ADDR => {
                self.obj_palette[1] = value;
                self.update_palette(value, 2);  // Update sp2_colours
            },
            WINDOW_Y_ADDR => self.window_y = value,
            WINDOW_X_ADDR => self.window_x = value,
            _ => panic!("Invalid LCD address: {:#X}", address),
        }
    }
    pub fn update_palette(&mut self, palette_data: u8, palette: u8) {
        let mut palette_colours = [0; 4];
        match palette {
            0 => palette_colours = self.bg_colours,
            1 => palette_colours = self.sp1_colours,
            2 => palette_colours = self.sp2_colours,
            _ => {error!("Invalid palette: {}", palette);}
        }
        palette_colours[0] = DEFAULT_COLOURS[(palette_data & PALETTE_MASK) as usize];
        palette_colours[1] = DEFAULT_COLOURS[((palette_data >> PALETTE_SHIFT_2) & PALETTE_MASK) as usize];
        palette_colours[2] = DEFAULT_COLOURS[((palette_data >> PALETTE_SHIFT_4) & PALETTE_MASK) as usize];
        palette_colours[3] = DEFAULT_COLOURS[((palette_data >> PALETTE_SHIFT_6) & PALETTE_MASK) as usize];
    }

    pub fn get_bg_colour(&self, pixel: u8) -> u32 {
        let colour_index = match pixel {
            0 => self.bg_palette & PALETTE_MASK,
            1 => (self.bg_palette >> PALETTE_SHIFT_2) & PALETTE_MASK,
            2 => (self.bg_palette >> PALETTE_SHIFT_4) & PALETTE_MASK,
            3 => (self.bg_palette >> PALETTE_SHIFT_6) & PALETTE_MASK,
            _ => unreachable!(),
        };

        // Ensure we're using the correct index based on the palette bits
        self.bg_colours[colour_index as usize]
    }

    /// Converts a sprite pixel value to a colour based on its palette index (0 or 1).
    pub fn get_sprite_colour(&self, palette_index: u8, pixel: u8) -> u32 {

        let palette = if palette_index == 0 { self.obj_palette[0] } else { self.obj_palette[1] };
        let colour_index = match pixel {
            0 => palette & PALETTE_MASK,
            1 => (palette >> PALETTE_SHIFT_2) & PALETTE_MASK,
            2 => (palette >> PALETTE_SHIFT_4) & PALETTE_MASK,
            3 => (palette >> PALETTE_SHIFT_6) & PALETTE_MASK,
            _ => unreachable!(),
        };

        // Use the correct colour array based on palette index
        if palette_index == 0 {
            self.sp1_colours[colour_index as usize]
        } else {
            self.sp2_colours[colour_index as usize]
        }
    }
}