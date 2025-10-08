
#[derive(Debug, PartialEq, Clone)]
pub enum Interrupt {
    VBLANK,
    LCDSTAT,
    TIMER,
    SERIAL,
    JOYPAD,
}
#[derive(Copy, Clone, Debug)]
pub struct InterruptFlags {
    pub vblank: bool,
    pub lcd_stat: bool,
    pub timer: bool,
    pub serial: bool,
    pub joypad: bool,
}

impl InterruptFlags {
    pub fn new() -> Self {
        InterruptFlags {
            vblank: false,
            lcd_stat: false,
            timer: false,
            serial: false,
            joypad: false,
        }
    }
}
impl From<InterruptFlags> for u8 {
    fn from(flag: InterruptFlags) -> u8 {
        let mut result = 0;
        if flag.vblank {
            result |= 0x01;
        }
        if flag.lcd_stat {
            result |= 0x02;
        }
        if flag.timer {
            result |= 0x04;
        }
        if flag.serial {
            result |= 0x08;
        }
        if flag.joypad {
            result |= 0x10;
        }
        result
    }
}
impl From<u8> for InterruptFlags {
    fn from(byte: u8) -> Self {
        InterruptFlags {
            vblank: byte & 0x01 != 0,
            lcd_stat: byte & 0x02 != 0,
            timer: byte & 0x04 != 0,
            serial: byte & 0x08 != 0,
            joypad: byte & 0x10 != 0,
        }
    }
}
impl From<Interrupt> for u8 {
    fn from(interrupt: Interrupt) -> u8 {
        match interrupt {
            Interrupt::VBLANK => 0x01,
            Interrupt::LCDSTAT => 0x02,
            Interrupt::TIMER => 0x04,
            Interrupt::SERIAL => 0x08,
            Interrupt::JOYPAD => 0x10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_byte_from_interrupt() {
        let interrupt = Interrupt::VBLANK;
        let byte: u8 = interrupt.into();
        assert_eq!(byte, 0b0000_0001);
        
        let interrupt = Interrupt::LCDSTAT;
        let byte: u8 = interrupt.into();
        assert_eq!(byte, 0b0000_0010);
        
        let interrupt = Interrupt::TIMER;
        let byte: u8 = interrupt.into();
        assert_eq!(byte, 0b0000_0100);
        
        let interrupt = Interrupt::SERIAL;
        let byte: u8 = interrupt.into();
        assert_eq!(byte, 0b0000_1000);
        
        let interrupt = Interrupt::JOYPAD;
        let byte: u8 = interrupt.into();
        assert_eq!(byte, 0b0001_0000);
    }
}
