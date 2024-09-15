#[derive(Clone)]
pub(crate) enum TimerFrequency {
    Hz4096 = 1024,
    Hz16384 = 256,
    Hz65536 = 64,
    Hz262144 = 16,
}

pub struct Timer {
    pub frequency: TimerFrequency,
    pub div: u16,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    pub enabled: bool,
}

impl Timer {
    pub fn new(frequency: TimerFrequency) -> Timer {
        Timer {
            frequency,
            div: 0xAC00,
            tima: 0,
            tma: 0,
            tac: 0,
            enabled: false,
        }
    }
    pub fn cycle(&mut self, cycles: u8) -> bool {
        if !self.enabled {
            return false;
        }

        self.div = self.div.wrapping_add(cycles as u16);

        let freq = match self.tac {
            0b00 => TimerFrequency::Hz4096,
            0b01 => TimerFrequency::Hz262144,
            0b10 => TimerFrequency::Hz65536,
            0b11 => TimerFrequency::Hz16384,
            _ => panic!("Invalid timer frequency: {:#X}", self.tac),
        };

        let (new_tima, overflow) = self.tima.overflowing_add(freq as u8);

        if overflow {
            self.tima = self.tma;
            return true;
        }

        self.tima = new_tima;
        false

    }

}
