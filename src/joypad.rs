use bitflags::bitflags;

const BUTTON_SELECT_BIT: u8 = 5;
const DPAD_SELECT_BIT: u8 = 4;
const NO_BUTTONS_PRESSED: u8 = 0b1100_0000;
const ALL_BUTTONS_PRESSED: u8 = 0b1111;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct JoypadFlags: u8 {
        const START  = 1 << 3;
        const SELECT = 1 << 2;
        const B      = 1 << 1;
        const A      = 1 << 0;
        const DOWN   = 1 << 3;
        const UP     = 1 << 2;
        const LEFT   = 1 << 1;
        const RIGHT  = 1 << 0;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Joypad {
    pub(crate) select_buttons: bool,
    pub(crate) select_dpad: bool,
    pub(crate) buttons: JoypadFlags,
    pub(crate) dpad: JoypadFlags,
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
            buttons: JoypadFlags::all(),
            dpad: JoypadFlags::all(),
        }
    }

    pub fn is_pressed(&self, button: Button) -> bool {
        match button {
            Button::Start | Button::Select | Button::A | Button::B if !self.select_buttons => {
                let flag = match button {
                    Button::Start => JoypadFlags::START,
                    Button::Select => JoypadFlags::SELECT,
                    Button::A => JoypadFlags::A,
                    Button::B => JoypadFlags::B,
                    _ => unreachable!(),
                };
                !self.buttons.contains(flag)
            }
            Button::Up | Button::Down | Button::Left | Button::Right if !self.select_dpad => {
                let flag = match button {
                    Button::Up => JoypadFlags::UP,
                    Button::Down => JoypadFlags::DOWN,
                    Button::Left => JoypadFlags::LEFT,
                    Button::Right => JoypadFlags::RIGHT,
                    _ => unreachable!(),
                };
                !self.dpad.contains(flag)
            }
            _ => false,
        }
    }

    pub fn button_pressed(&mut self, button: Button) {
        match button {
            Button::Start => self.buttons.remove(JoypadFlags::START),
            Button::Select => self.buttons.remove(JoypadFlags::SELECT),
            Button::A => self.buttons.remove(JoypadFlags::A),
            Button::B => self.buttons.remove(JoypadFlags::B),
            Button::Up => self.dpad.remove(JoypadFlags::UP),
            Button::Down => self.dpad.remove(JoypadFlags::DOWN),
            Button::Left => self.dpad.remove(JoypadFlags::LEFT),
            Button::Right => self.dpad.remove(JoypadFlags::RIGHT),
        }
    }

    pub fn button_released(&mut self, button: Button) {
        match button {
            Button::Start => self.buttons.insert(JoypadFlags::START),
            Button::Select => self.buttons.insert(JoypadFlags::SELECT),
            Button::A => self.buttons.insert(JoypadFlags::A),
            Button::B => self.buttons.insert(JoypadFlags::B),
            Button::Up => self.dpad.insert(JoypadFlags::UP),
            Button::Down => self.dpad.insert(JoypadFlags::DOWN),
            Button::Left => self.dpad.insert(JoypadFlags::LEFT),
            Button::Right => self.dpad.insert(JoypadFlags::RIGHT),
        }
    }

    pub fn set_selection(&mut self, byte: u8) {
        self.select_buttons = byte & (1 << BUTTON_SELECT_BIT) == 0;
        self.select_dpad = byte & (1 << DPAD_SELECT_BIT) == 0;
    }
}

impl From<Joypad> for u8 {
    fn from(joypad: Joypad) -> u8 {
        let mut result = NO_BUTTONS_PRESSED;

        if !joypad.select_buttons {
            result &= !(1 << BUTTON_SELECT_BIT);
        } else {
            result |= 1 << BUTTON_SELECT_BIT;
        }

        if !joypad.select_dpad {
            result &= !(1 << DPAD_SELECT_BIT);
        } else {
            result |= 1 << DPAD_SELECT_BIT;
        }

        if !joypad.select_dpad {
            result |= joypad.buttons.bits();
        } else if !joypad.select_buttons {
            result |= joypad.dpad.bits();
        } else {
            result |= ALL_BUTTONS_PRESSED;
        }

        result
    }
}

impl From<u8> for Joypad {
    fn from(byte: u8) -> Joypad {
        let mut joypad = Joypad::new();
        joypad.select_buttons = byte & (1 << BUTTON_SELECT_BIT) == 0;
        joypad.select_dpad = byte & (1 << DPAD_SELECT_BIT) == 0;
        joypad
    }
}