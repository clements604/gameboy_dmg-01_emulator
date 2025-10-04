use log::error;

pub const DARKEST_GREEN: u32 = 0xFF142C38;
//const DARK_GREEN: u32 = 0xFF306230;
const DARK_GREEN: u32 = 0xFF548C70;
//const LIGHT_GREEN: u32 = 0xFF8BAC0F;
const LIGHT_GREEN: u32 = 0xFFACD490;
//pub const LIGHTEST_GREEN: u32 = 0xFF9BBC0F;
pub const LIGHTEST_GREEN: u32 = 0xFFE8FCCC;
pub const DEFAULT_COLOURS: [u32; 4] = [
    LIGHTEST_GREEN, // This would be the color for palette 00
    LIGHT_GREEN,    // This would be the color for palette 01
    DARK_GREEN,     // This would be the color for palette 10
    DARKEST_GREEN   // This would be the color for palette 11
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
            bg_palette: 0xFC,
            obj_palette: [0xFF; 2],
            window_x: 0,
            window_y: 0,
            bg_colours,
            sp1_colours,
            sp2_colours,
        }
    }
    pub fn read(&self, address: u16) -> u8 {
        match address {
            0xFF47 => self.bg_palette,
            0xFF48 => self.obj_palette[0],
            0xFF49 => self.obj_palette[1],
            0xFF4A => self.window_y,
            0xFF4B => self.window_x,
            _ => panic!("Invalid LCD address: {:#X}", address),
        }
    }
    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xFF47 => {
                self.bg_palette = value;
                self.update_palette(value, 0);  // Update bg_colours
            },
            0xFF48 => {
                self.obj_palette[0] = value;
                self.update_palette(value, 1);  // Update sp1_colours
            },
            0xFF49 => {
                self.obj_palette[1] = value;
                self.update_palette(value, 2);  // Update sp2_colours
            },
            0xFF4A => self.window_y = value,
            0xFF4B => self.window_x = value,
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
        palette_colours[0] = DEFAULT_COLOURS[(palette_data & 0x03) as usize];
        palette_colours[1] = DEFAULT_COLOURS[((palette_data >> 2) & 0x03) as usize];
        palette_colours[2] = DEFAULT_COLOURS[((palette_data >> 4) & 0x03) as usize];
        palette_colours[3] = DEFAULT_COLOURS[((palette_data >> 6) & 0x03) as usize];
    }

    pub fn get_bg_color(&self, pixel: u8) -> u32 {
        let color_index = match pixel {
            0 => self.bg_palette & 0x03,
            1 => (self.bg_palette >> 2) & 0x03,
            2 => (self.bg_palette >> 4) & 0x03,
            3 => (self.bg_palette >> 6) & 0x03,
            _ => unreachable!(),
        };

        // Ensure we're using the correct index based on the palette bits
        self.bg_colours[color_index as usize]
    }

    /// Converts a sprite pixel value to a color based on its palette index (0 or 1).
    pub fn get_sprite_color(&self, palette_index: u8, pixel: u8) -> u32 {

        let palette = if palette_index == 0 { self.obj_palette[0] } else { self.obj_palette[1] };
        let color_index = match pixel {
            0 => palette & 0x03,
            1 => (palette >> 2) & 0x03,
            2 => (palette >> 4) & 0x03,
            3 => (palette >> 6) & 0x03,
            _ => unreachable!(),
        };

        // Use the correct color array based on palette index
        if palette_index == 0 {
            self.sp1_colours[color_index as usize]
        } else {
            self.sp2_colours[color_index as usize]
        }
    }
}