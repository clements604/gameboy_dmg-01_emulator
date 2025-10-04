use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Buttons: u16 {
        const A      = 1 << 0;
        const B      = 1 << 1;
        const SELECT = 1 << 2;
        const START  = 1 << 3;
        const RIGHT  = 1 << 4;
        const LEFT   = 1 << 5;
        const UP     = 1 << 6;
        const DOWN   = 1 << 7;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Joypad {
    pub select_buttons: bool,
    pub select_dpad: bool,
    pub pressed: Buttons,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            select_buttons: true,
            select_dpad: true,
            pressed: Buttons::empty(), // no buttons pressed
        }
    }

    pub fn is_pressed(&self, button: Buttons) -> bool {
        // Only active if the right group is selected
        if (button.intersects(Buttons::A | Buttons::B | Buttons::START | Buttons::SELECT)
            && !self.select_buttons)
            || (button.intersects(Buttons::UP | Buttons::DOWN | Buttons::LEFT | Buttons::RIGHT)
            && !self.select_dpad)
        {
            self.pressed.contains(button)
        } else {
            false
        }
    }

    pub fn button_pressed(&mut self, button: Buttons) {
        self.pressed.insert(button);
    }

    pub fn button_released(&mut self, button: Buttons) {
        self.pressed.remove(button);
    }

    pub fn set_selection(&mut self, byte: u8) {
        self.select_buttons = byte & (1 << 5) == 0;
        self.select_dpad = byte & (1 << 4) == 0;
    }
}

impl From<Joypad> for u8 {
    fn from(joypad: Joypad) -> u8 {
        let mut result = 0b1100_0000;

        if !joypad.select_buttons {
            result &= !(1 << 5);
        } else {
            result |= 1 << 5;
        }

        if !joypad.select_dpad {
            result &= !(1 << 4);
        } else {
            result |= 1 << 4;
        }

        if !joypad.select_dpad {
            result |= ((!joypad.pressed.contains(Buttons::START)) as u8) << 3
                | ((!joypad.pressed.contains(Buttons::SELECT)) as u8) << 2
                | ((!joypad.pressed.contains(Buttons::B)) as u8) << 1
                | ((!joypad.pressed.contains(Buttons::A)) as u8);
        } else if !joypad.select_buttons {
            result |= ((!joypad.pressed.contains(Buttons::DOWN)) as u8) << 3
                | ((!joypad.pressed.contains(Buttons::UP)) as u8) << 2
                | ((!joypad.pressed.contains(Buttons::LEFT)) as u8) << 1
                | ((!joypad.pressed.contains(Buttons::RIGHT)) as u8);
        } else {
            result |= 0b1111;
        }

        result
    }
}
