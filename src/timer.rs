// Timer frequency-related constants
const FREQ_4096HZ_BIT: u8 = 9;     // 4096 Hz - bit 9 (falling edge every 1024 cycles)
const FREQ_262144HZ_BIT: u8 = 3;   // 262144 Hz - bit 3 (falling edge every 16 cycles)
const FREQ_65536HZ_BIT: u8 = 5;    // 65536 Hz - bit 5 (falling edge every 64 cycles)
const FREQ_16384HZ_BIT: u8 = 7;    // 16384 Hz - bit 7 (falling edge every 256 cycles)

// TAC register masks
const TAC_ENABLED_MASK: u8 = 0x04;
const TAC_FREQUENCY_MASK: u8 = 0x03;

// Timer reload related constants
const TIMA_RELOAD_DELAY: u8 = 4;    // Cycles to wait before reloading TIMA
const TMA_WRITE_WINDOW: u8 = 5;     // Window for TMA writes to affect reload
const TIMA_OVERFLOW_VALUE: u8 = 0x00;
const CRITICAL_RELOAD_CYCLE: u8 = 2; // Critical cycle during reload
const DIV_REGISTER_SHIFT: u8 = 8;

pub struct Timer {
    internal_div_counter: u16,
    pub div: u8,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    pub enabled: bool,
    tima_reload_cycles: u8,
    tima_reload_value: u8,
    tma_write_window: u8,
    tima_just_reloaded: bool,
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
            tima_reload_cycles: 0,
            tima_reload_value: 0,
            tma_write_window: 0,
            tima_just_reloaded: false,
        }
    }

    pub fn reset_div(&mut self) {
        if self.enabled {
            let bit_position = self.get_tima_bit_position();
            let old_bit = (self.internal_div_counter >> bit_position) & 1 != 0;

            if old_bit {
                self.increment_tima();
            }
        }

        self.internal_div_counter = 0;
        self.div = 0;
    }

    pub fn set_tac(&mut self, value: u8) -> bool {
        let old_enabled = self.enabled;
        let old_frequency = self.tac & TAC_FREQUENCY_MASK;

        self.tac = value;
        self.enabled = (value & TAC_ENABLED_MASK) != 0;

        let mut interrupt = false;

        if old_enabled {
            // Get the bit that was being monitored before the change
            let old_bit_position = match old_frequency {
                0 => FREQ_4096HZ_BIT,
                1 => FREQ_262144HZ_BIT,
                2 => FREQ_65536HZ_BIT,
                3 => FREQ_16384HZ_BIT,
                _ => unreachable!(),
            };

            let old_bit = (self.internal_div_counter >> old_bit_position) & 1 != 0;

            if self.enabled {
                let new_bit_position = self.get_tima_bit_position();
                let new_bit = (self.internal_div_counter >> new_bit_position) & 1 != 0;

                if old_bit && !new_bit {
                    if self.increment_tima() {
                        interrupt = true;
                    }
                }
            } else {
                if old_bit {
                    if self.increment_tima() {
                        interrupt = true;
                    }
                }
            }
        }
        interrupt
    }

    fn get_tima_bit_position(&self) -> u8 {
        match self.tac & TAC_FREQUENCY_MASK {
            0 => FREQ_4096HZ_BIT,
            1 => FREQ_262144HZ_BIT,
            2 => FREQ_65536HZ_BIT,
            3 => FREQ_16384HZ_BIT,
            _ => unreachable!(),
        }
    }

    fn increment_tima(&mut self) -> bool {
        let (new_tima, overflow) = self.tima.overflowing_add(1);
        if overflow {
            self.tima = TIMA_OVERFLOW_VALUE;
            self.tima_reload_cycles = TIMA_RELOAD_DELAY;
            self.tima_reload_value = self.tma;
            self.tma_write_window = TMA_WRITE_WINDOW;
            true
        } else {
            self.tima = new_tima;
            false
        }
    }

    pub fn write_tma(&mut self, value: u8) {
        self.tma = value;

        if self.tma_write_window > 0 {
            if self.tima_reload_cycles > 0 {
                self.tima_reload_value = value;
            } else {
                self.tima = value;
            }
        }
    }

    pub fn cycle(&mut self, cycles: u8) -> bool {

        let mut interrupt = false;

        for _ in 0..cycles {
            self.tima_just_reloaded = false;

            // Handle delayed TIMA reload first
            if self.tima_reload_cycles > 0 {
                self.tima_reload_cycles -= 1;
                if self.tima_reload_cycles == 0 {
                    self.tima = self.tima_reload_value;
                    self.tima_just_reloaded = true;
                }
            }

            if self.tma_write_window > 0 {
                self.tma_write_window -= 1;
            }

            // Get current state of the TIMA bit before incrementing
            let old_bit = if self.enabled {
                let bit_position = self.get_tima_bit_position();
                (self.internal_div_counter >> bit_position) & 1 != 0
            } else {
                false
            };

            // Increment internal counter
            self.internal_div_counter = self.internal_div_counter.wrapping_add(1);
            self.div = (self.internal_div_counter >> DIV_REGISTER_SHIFT) as u8;
            
            if self.enabled {
                let bit_position = self.get_tima_bit_position();
                let new_bit = (self.internal_div_counter >> bit_position) & 1 != 0;

                if old_bit && !new_bit {
                    if self.increment_tima() {
                        interrupt = true;
                    }
                }
            }
        }

        interrupt
    }
    pub fn write_tima(&mut self, value: u8) {
        if self.tima_just_reloaded {
            // A write landing on the exact tick TMA was reloaded into TIMA
            // loses the bus conflict; the reload wins and the write is dropped.
            return;
        }

        if self.tima_reload_cycles > 0 {

            if self.tima_reload_cycles == CRITICAL_RELOAD_CYCLE {
                return;
            } else {
                self.tima = value;
                self.tima_reload_cycles = 0;
                return;
            }
        }

        self.tima = value;
    }


    
}
