#[derive(Clone)]
pub(crate) enum TimerFrequency {
    Hz4096 = 1024,
    Hz16384 = 256,
    Hz65536 = 64,
    Hz262144 = 16,
}

pub struct Timer {
    pub div: u16,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    pub enabled: bool,
    cycle_count: u32, // New field to keep track of cycles
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            div: 0xAC00,
            tima: 0,
            tma: 0,
            tac: 0,
            enabled: false,
            cycle_count: 0,
        }
    }

    pub fn cycle(&mut self, cycles: u8) -> bool {
        self.cycle_count = self.cycle_count.wrapping_add(cycles as u32);

        if !self.enabled {
            return false;
        }

        let freq = match self.tac & 0x03 {
            0b00 => 1024,
            0b01 => 16,
            0b10 => 64,
            0b11 => 256,
            _ => unreachable!(),
        };

        let ticks = self.cycle_count / freq;
        self.cycle_count %= freq;

        for _ in 0..ticks {
            let (new_tima, overflow) = self.tima.overflowing_add(1);
            if overflow {
                self.tima = self.tma;
                return true;
            }
            self.tima = new_tima;
        }

        false
    }
}