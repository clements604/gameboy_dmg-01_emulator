#[derive(Debug, Clone, Copy)]
pub struct Joypad {
    // These are controlled by writing to $FF00, not by button presses
    select_buttons: bool,
    select_dpad: bool,
    // Button states
    pub(crate) start: bool,
    pub(crate) select: bool,
    pub(crate) b: bool,
    pub(crate) a: bool,
    pub(crate) up: bool,
    pub(crate) down: bool,
    pub(crate) left: bool,
    pub(crate) right: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Button {
    A,
    B,
    Start,
    Select,
    Up,
    Down,
    Left,
    Right,
}

impl Joypad {
    pub fn new() -> Joypad {
        Joypad {
            select_buttons: true,
            select_dpad: true,
            start: true,
            select: true,
            b: true,
            a: true,
            up: true,
            down: true,
            left: true,
            right: true,
        }
    }

    pub fn is_pressed(&self, button: Button) -> bool {
        match button {
            Button::Start | Button::Select | Button::A | Button::B if !self.select_buttons => {
                match button {
                    Button::Start => !self.start,
                    Button::Select => !self.select,
                    Button::A => !self.a,
                    Button::B => !self.b,
                    _ => false, // This case should never happen due to the match guard
                }
            }
            Button::Up | Button::Down | Button::Left | Button::Right if !self.select_dpad => {
                match button {
                    Button::Up => !self.up,
                    Button::Down => !self.down,
                    Button::Left => !self.left,
                    Button::Right => !self.right,
                    _ => false, // Similarly, this should never happen
                }
            }
            _ => false, // If neither buttons nor d-pad is selected, no button is pressed
        }
    }

    pub fn button_pressed(&mut self, button: Button) {
        match button {
            Button::Start => self.start = false,
            Button::Select => self.select = false,
            Button::A => self.a = false,
            Button::B => self.b = false,
            Button::Up => self.up = false,
            Button::Down => self.down = false,
            Button::Left => self.left = false,
            Button::Right => self.right = false,
        }
    }

    pub fn button_released(&mut self, button: Button) {
        match button {
            Button::Start => self.start = true,
            Button::Select => self.select = true,
            Button::A => self.a = true,
            Button::B => self.b = true,
            Button::Up => self.up = true,
            Button::Down => self.down = true,
            Button::Left => self.left = true,
            Button::Right => self.right = true,
        }
    }

    pub fn set_selection(&mut self, byte: u8) {
        self.select_buttons = byte & (1 << 5) == 0;
        self.select_dpad = byte & (1 << 4) == 0;
    }
}

impl std::convert::From<Joypad> for u8 {
    fn from(joypad: Joypad) -> u8 {
        let mut result = 0b1100_0000; // Bits 7 and 6 are always 1, bits 5 and 4 start as 1 unless selected

        if !joypad.select_dpad {
            result |= 0; // Bit 5 is 0 if buttons selected
        } else {
            result |= 1 << 5;
        }

        if !joypad.select_buttons {
            result |= 0; // Bit 4 is 0 if d-pad selected
        } else {
            result |= 1 << 4;
        }

        if !joypad.select_dpad {
            result |=
                (joypad.start as u8) << 3 |
                    (joypad.select as u8) << 2 |
                    (joypad.b as u8) << 1 |
                    (joypad.a as u8);
        } else if !joypad.select_buttons {
            result |=
                (joypad.down as u8) << 3 |
                    (joypad.up as u8) << 2 |
                    (joypad.left as u8) << 1 |
                    (joypad.right as u8);
        } else {
            // Neither buttons nor d-pad selected, lower nibble should be 1111
            result |= 0b1111;
        }

        result
    }
}

impl std::convert::From<u8> for Joypad {
    fn from(byte: u8) -> Joypad {
        let mut joypad = Joypad::new();
        joypad.set_selection(byte);

        // Only set button states if corresponding selection bit is 0
        if !joypad.select_buttons {
            joypad.start = byte & (1 << 3) == 0;
            joypad.select = byte & (1 << 2) == 0;
            joypad.b = byte & (1 << 1) == 0;
            joypad.a = byte & 1 == 0;
        } else if !joypad.select_dpad {
            joypad.down = byte & (1 << 3) == 0;
            joypad.up = byte & (1 << 2) == 0;
            joypad.left = byte & (1 << 1) == 0;
            joypad.right = byte & 1 == 0;
        }

        joypad
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joypad_init() {
        let joypad = Joypad::new();
        assert_eq!(u8::from(joypad), 0xFF);
    }

    #[test]
    fn test_start_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_buttons = false;
        joypad.start = false;
        assert_eq!(u8::from(joypad), 0b1101_0111);
    }

    #[test]
    fn test_select_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_buttons = false;
        joypad.select = false;
        assert_eq!(u8::from(joypad), 0b11011011);
    }

    #[test]
    fn test_a_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_buttons = false;
        joypad.a = false;
        assert_eq!(u8::from(joypad), 0b1101_1110);
    }

    #[test]
    fn test_b_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_buttons = false;
        joypad.b = false;
        assert_eq!(u8::from(joypad), 0b1101_1101);
    }

    #[test]
    fn test_dpad_up_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_dpad = false;
        joypad.up = false;
        assert_eq!(u8::from(joypad), 0b11101011);
    }

    #[test]
    fn test_dpad_down_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_dpad = false;
        joypad.down = false;
        assert_eq!(u8::from(joypad), 0b1110_0111);
    }

    #[test]
    fn test_dpad_left_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_dpad = false;
        joypad.left = false;
        assert_eq!(u8::from(joypad), 0b1110_1101);
    }

    #[test]
    fn test_dpad_right_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_dpad = false;
        joypad.right = false;
        assert_eq!(u8::from(joypad), 0b1110_1110);
    }

}
