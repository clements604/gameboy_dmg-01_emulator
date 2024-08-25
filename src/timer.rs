#[derive(Clone)]
pub(crate) enum TimerFrequency {
    Hz4096 = 1024,
    Hz16384 = 256,
    Hz65536 = 64,
    Hz262144 = 16,
}

pub struct Timer {
    pub(crate) frequency: TimerFrequency,
    cycles: usize,
    pub value: u8,
    pub modulo: u8,
    pub enabled: bool,
}

impl Timer {
    pub fn new(frequency: TimerFrequency) -> Timer {
        Timer {
            frequency,
            cycles: 0,
            value: 0,
            modulo: 0,
            enabled: false,
        }
    }
    pub fn cycle(&mut self, cycles: u8) -> bool {
        if !self.enabled {
            return false;
        }
        
        self.cycles += cycles as usize;
        
        let tick_cycles = self.frequency.clone() as usize;
        
        let overflow = if self.cycles > tick_cycles {
            self.cycles = self.cycles % tick_cycles;
            let (value, did_overflow) = self.value.overflowing_add(1);
            self.value = value;
            did_overflow
        }
        else {
            false
        };
        
        if overflow {
            self.value = self.modulo;
        }

        overflow
    }

}
