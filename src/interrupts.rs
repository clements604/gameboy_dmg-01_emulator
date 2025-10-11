const VBLANK_FLAG: u8 = 0x01;
const LCD_STAT_FLAG: u8 = 0x02;
const TIMER_FLAG: u8 = 0x04;
const SERIAL_FLAG: u8 = 0x08;
const JOYPAD_FLAG: u8 = 0x10;

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
            result |= VBLANK_FLAG;
        }
        if flag.lcd_stat {
            result |= LCD_STAT_FLAG;
        }
        if flag.timer {
            result |= TIMER_FLAG;
        }
        if flag.serial {
            result |= SERIAL_FLAG;
        }
        if flag.joypad {
            result |= JOYPAD_FLAG;
        }
        result
    }
}
impl From<u8> for InterruptFlags {
    fn from(byte: u8) -> Self {
        InterruptFlags {
            vblank: byte & VBLANK_FLAG != 0,
            lcd_stat: byte & LCD_STAT_FLAG != 0,
            timer: byte & TIMER_FLAG != 0,
            serial: byte & SERIAL_FLAG != 0,
            joypad: byte & JOYPAD_FLAG != 0,
        }
    }
}
impl From<Interrupt> for u8 {
    fn from(interrupt: Interrupt) -> u8 {
        match interrupt {
            Interrupt::VBLANK => VBLANK_FLAG,
            Interrupt::LCDSTAT => LCD_STAT_FLAG,
            Interrupt::TIMER => TIMER_FLAG,
            Interrupt::SERIAL => SERIAL_FLAG,
            Interrupt::JOYPAD => JOYPAD_FLAG,
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
