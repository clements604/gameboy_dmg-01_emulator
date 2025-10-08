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
        let old_frequency = self.tac & 0x03;

        self.tac = value;
        self.enabled = (value & 0x04) != 0;

        let mut interrupt = false;

        if old_enabled {
            // Get the bit that was being monitored before the change
            let old_bit_position = match old_frequency {
                0 => 9,  // 4096 Hz
                1 => 3,  // 262144 Hz
                2 => 5,  // 65536 Hz
                3 => 7,  // 16384 Hz
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
        match self.tac & 0x03 {
            0 => 9,  // 4096 Hz - bit 9 (falling edge every 1024 cycles)
            1 => 3,  // 262144 Hz - bit 3 (falling edge every 16 cycles)
            2 => 5,  // 65536 Hz - bit 5 (falling edge every 64 cycles)
            3 => 7,  // 16384 Hz - bit 7 (falling edge every 256 cycles)
            _ => unreachable!(),
        }
    }

    fn increment_tima(&mut self) -> bool {
        let (new_tima, overflow) = self.tima.overflowing_add(1);
        if overflow {
            self.tima = 0x00;
            self.tima_reload_cycles = 4;
            self.tima_reload_value = self.tma;
            self.tma_write_window = 5;
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
            // Handle delayed TIMA reload first
            if self.tima_reload_cycles > 0 {
                self.tima_reload_cycles -= 1;
                if self.tima_reload_cycles == 0 {
                    self.tima = self.tima_reload_value;
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
            self.div = (self.internal_div_counter >> 8) as u8;
            
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
        if self.tima_reload_cycles > 0 {

            if self.tima_reload_cycles == 2 {
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
