pub struct Timer {
    internal_div_counter: u16,  // Internal 16-bit counter
    pub div: u8,                // Exposed 8-bit DIV register
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    pub enabled: bool,
    tima_cycles: u32,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            internal_div_counter: 0,
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            enabled: false,
            tima_cycles: 0,
        }
    }

    // Method to write to DIV (always resets to 0)
    pub fn reset_div(&mut self) {
        self.internal_div_counter = 0;
        self.div = 0;
    }

    pub fn set_tac(&mut self, value: u8) {
        self.tac = value;
        self.enabled = (value & 0x04) != 0;
    }

    pub fn cycle(&mut self, cycles: u8) -> bool {
        let cycles_u32 = cycles as u32;
        let mut interrupt = false;

        // Update DIV - always increments regardless of timer being enabled
        let old_high_bit = self.internal_div_counter & 0x100;

        self.internal_div_counter = self.internal_div_counter.wrapping_add(cycles as u16);
        self.div = (self.internal_div_counter >> 8) as u8; // Upper 8 bits become DIV

        // Optional: Detect bit 8 falling edge for TIMA increment (some accurate emulators do this)
        let new_high_bit = self.internal_div_counter & 0x100;
        let falling_edge = old_high_bit != 0 && new_high_bit == 0;

        // Update TIMA based on frequency and enabled state
        if self.enabled {
            // Get frequency from TAC bits 0-1
            let freq = match self.tac & 0x03 {
                0 => 1024, // 4096 Hz
                1 => 16,   // 262144 Hz
                2 => 64,   // 65536 Hz
                3 => 256,  // 16384 Hz
                _ => unreachable!(),
            };

            // Update TIMA
            self.tima_cycles += cycles_u32;
            while self.tima_cycles >= freq {
                self.tima_cycles -= freq;

                let (new_tima, overflow) = self.tima.overflowing_add(1);
                if overflow {
                    self.tima = self.tma; // Load TMA on overflow
                    interrupt = true;    // Set interrupt flag
                } else {
                    self.tima = new_tima;
                }
            }
        }

        interrupt
    }
}