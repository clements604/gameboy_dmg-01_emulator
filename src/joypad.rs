/**
Select buttons: If this bit is 0, then buttons (SsBA) can be read from the lower nibble.

Select d-pad: If this bit is 0, then directional keys can be read from the lower nibble.

The lower nibble is Read-only. Note that, rather unconventionally for the Game Boy, a button being pressed is seen as the corresponding bit being 0, not 1.

If neither buttons nor d-pad is selected ($30 was written), then the low nibble reads $F (all buttons released).
**/

#[derive(Debug, Clone, Copy)]
pub struct Joypad {
    select_buttons: bool,
    select_dpad: bool,
    start_down: bool,
    select_up: bool,
    b_left: bool,
    a_right: bool,
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
            select_buttons: true, // No selection of buttons or d-pad at start
            select_dpad: true,
            start_down: true, // True means not pressed (inverted logic for Game Boy)
            select_up: true,
            b_left: true,
            a_right: true,
        }
    }

    pub fn is_pressed(&self, button: Button) -> bool {
        match button {
            Button::Start => !self.select_buttons & !self.start_down,
            Button::Select => !self.select_buttons & !self.select_up,
            Button::A => !self.select_buttons & !self.a_right,
            Button::B => !self.select_buttons & !self.b_left,
            Button::Up => !self.select_dpad & !self.select_up,
            Button::Down => !self.select_dpad & !self.start_down,
            Button::Left => !self.select_dpad & !self.b_left,
            Button::Right => !self.select_dpad & !self.a_right,
        }
    }

    pub fn button_pressed(&mut self, button: Button) {
        match button {
            Button::Start => {
                self.select_buttons = false;
                self.start_down = false;
            }
            Button::Select => {
                self.select_buttons = false;
                self.select_up = false;
            }
            Button::A => {
                self.select_buttons = false;
                self.a_right = false;
            }
            Button::B => {
                self.select_buttons = false;
                self.b_left = false;
            }
            Button::Up => {
                self.select_dpad = false;
                self.select_up = false;
            }
            Button::Down => {
                self.select_dpad = false;
                self.start_down = false;
            }
            Button::Left => {
                self.select_dpad = false;
                self.b_left = false;
            }
            Button::Right => {
                self.select_dpad = false;
                self.a_right = false;
            }
        }
    }
}

impl std::convert::From<Joypad> for u8 {
    fn from(joypad: Joypad) -> u8 {
        1 << 7
            | 1 << 6
            | !(joypad.select_buttons as u8) << 5
            | !(joypad.select_dpad as u8) << 4
            | !(joypad.start_down as u8) << 3
            | !(joypad.select_up as u8) << 2
            | !(joypad.b_left as u8) << 1
            | !(joypad.a_right as u8)
    }
}

impl std::convert::From<u8> for Joypad {
    fn from(byte: u8) -> Joypad {
        Self {
            select_buttons: (byte & (1 << 5)) != 0,
            select_dpad: (byte & (1 << 4)) != 0,
            start_down: (byte & (1 << 3)) != 0,
            select_up: (byte & (1 << 2)) != 0,
            b_left: (byte & (1 << 1)) != 0,
            a_right: (byte & 1) != 0,
        }
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
        joypad.start_down = false;
        assert_eq!(u8::from(joypad), 0b1101_0111);
    }

    #[test]
    fn test_select_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_buttons = false;
        joypad.select_up = false;
        assert_eq!(u8::from(joypad), 0b11011011);
    }

    #[test]
    fn test_a_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_buttons = false;
        joypad.a_right = false;
        assert_eq!(u8::from(joypad), 0b1101_1110);
    }

    #[test]
    fn test_b_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_buttons = false;
        joypad.b_left = false;
        assert_eq!(u8::from(joypad), 0b1101_1101);
    }

    #[test]
    fn test_dpad_up_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_dpad = false;
        joypad.select_up = false;
        assert_eq!(u8::from(joypad), 0b11101011);
    }

    #[test]
    fn test_dpad_down_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_dpad = false;
        joypad.start_down = false;
        assert_eq!(u8::from(joypad), 0b1110_0111);
    }

    #[test]
    fn test_dpad_left_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_dpad = false;
        joypad.b_left = false;
        assert_eq!(u8::from(joypad), 0b1110_1101);
    }

    #[test]
    fn test_dpad_right_pressed() {
        let mut joypad = Joypad::new();
        joypad.select_dpad = false;
        joypad.a_right = false;
        assert_eq!(u8::from(joypad), 0b1110_1110);
    }

    #[test]
    fn test_is_pressed_true() {
        let mut joypad = Joypad::new();
        joypad.select_buttons = false;
        joypad.start_down = false;
        assert_eq!(joypad.is_pressed(Button::Start), true);
    }

    #[test]
    fn test_is_pressed_false() {
        let mut joypad = Joypad::new();
        joypad.select_dpad = true;
        joypad.select_up = true;
        joypad.b_left = false;
        assert_eq!(joypad.is_pressed(Button::B), false);
    }

    #[test]
    fn test_press_start() {
        let mut joypad = Joypad::new();
        joypad.button_pressed(Button::Start);
        assert_eq!(u8::from(joypad), 0b1101_0111);
    }

    #[test]
    fn test_press_a() {
        let mut joypad = Joypad::new();
        joypad.button_pressed(Button::A);
        assert_eq!(u8::from(joypad), 0b1101_1110);
    }
}
