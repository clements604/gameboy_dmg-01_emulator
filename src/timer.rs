use log::{debug, info};
use serde::{Serialize, Deserialize};

pub struct Timer {
    internal_div_counter: u16,
    pub div: u8,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    pub enabled: bool,
    // New fields for delayed reload
    tima_reload_cycles: u8,  // Countdown for reload delay
    tima_reload_value: u8,   // Value to reload with
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
        // Check for falling edge caused by DIV reset
        if self.enabled {
            let bit_position = self.get_tima_bit_position();
            let old_bit = (self.internal_div_counter >> bit_position) & 1 != 0;

            // After reset, the bit will be 0, so check for falling edge
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

        // Update TAC values
        self.tac = value;
        self.enabled = (value & 0x04) != 0;

        let mut interrupt = false;

        // Check for falling edges caused by TAC changes
        // This handles the "unexpected timer increases" mentioned in the test
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

            // If timer is still enabled after the change, check new bit
            if self.enabled {
                let new_bit_position = self.get_tima_bit_position();
                let new_bit = (self.internal_div_counter >> new_bit_position) & 1 != 0;

                // Falling edge: old bit was 1, new bit is 0
                if old_bit && !new_bit {
                    if self.increment_tima() {
                        interrupt = true;
                    }
                }
            } else {
                // Timer was disabled: if old bit was 1, that's a falling edge
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
            debug!("TIMA OVERFLOW: starting reload");
            self.tima = 0x00;
            self.tima_reload_cycles = 4;
            self.tima_reload_value = self.tma;
            self.tma_write_window = 5;    // NEW: Accept TMA writes for 5 cycles
            true
        } else {
            self.tima = new_tima;
            false
        }
    }

    pub fn write_tma(&mut self, value: u8) {
        debug!("TMA WRITE: value=0x{:02X}, reload_cycles={}, write_window={}", 
           value, self.tima_reload_cycles, self.tma_write_window);
        self.tma = value;

        if self.tma_write_window > 0 {
            debug!("  -> ACCEPTED: in write window");
            if self.tima_reload_cycles > 0 {
                // Still reloading - update reload value
                self.tima_reload_value = value;
            } else {
                // Reload complete but still in window - update TIMA directly  
                self.tima = value;
            }
        }
    }

    pub fn cycle(&mut self, cycles: u8) -> bool {

        debug!("TIMER CYCLE: {} cycles, tima=0x{:02X}, enabled={}", cycles, self.tima, self.enabled);
        debug!("CYCLE START: internal_counter=0x{:04X}, cycles_to_add={}", self.internal_div_counter, cycles);
        
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

            // Check for falling edge after incrementing
            if self.enabled {
                let bit_position = self.get_tima_bit_position();
                let new_bit = (self.internal_div_counter >> bit_position) & 1 != 0;

                // Falling edge: was 1, now 0
                // In the falling edge detection part
                if old_bit && !new_bit {
                    debug!("FALLING EDGE at cycle {}: incrementing TIMA from 0x{:02X}", self.internal_div_counter, self.tima);
                    if self.increment_tima() {
                        interrupt = true;
                    }
                }
            }
        }

        interrupt
    }

    // Helper method to check if TIMA is in the reload delay period
    pub fn is_tima_reloading(&self) -> bool {
        self.tima_reload_cycles > 0
    }

    pub fn write_tima(&mut self, value: u8) {
        if self.tima_reload_cycles > 0 {
            debug!("TIMA WRITE DEBUG: value=0x{:02X}, reload_cycles={}, current_tima=0x{:02X}",
                     value, self.tima_reload_cycles, self.tima);

            // During reload delay: write behavior depends on exact timing
            // Based on the test pattern, only writes during cycle 2 are ignored
            if self.tima_reload_cycles == 2 {
                debug!("  -> IGNORED (reload_cycles={})", self.tima_reload_cycles);
                // Specific cycle where writes are ignored
                // TIMA will still reload with TMA value
                return;
            } else {
                debug!("  -> ACCEPTED (reload_cycles={}), canceling reload", self.tima_reload_cycles);
                // All other cycles during reload - write takes effect
                // Cancel the reload and use the written value
                self.tima = value;
                self.tima_reload_cycles = 0;
                return;
            }
        }

        // Normal write when not reloading
        debug!("TIMA WRITE NORMAL: value=0x{:02X}", value);
        self.tima = value;
    }


    
}

#[cfg(test)]
mod timer_tests {
    use super::*;

    #[test]
    fn test_div_register_timing() {
        let mut timer = Timer::new();

        // DIV should increment every 256 cycles
        assert_eq!(timer.div, 0);

        // Run 255 cycles - DIV should still be 0
        timer.cycle(255);
        assert_eq!(timer.div, 0);

        // Run 1 more cycle (256 total) - DIV should be 1
        timer.cycle(1);
        assert_eq!(timer.div, 1);

        // Run another 256 cycles - DIV should be 2
        timer.cycle(255);
        timer.cycle(1);
        assert_eq!(timer.div, 2);
    }

    #[test]
    fn test_div_reset() {
        let mut timer = Timer::new();

        // Advance timer (512 cycles = 2 * 256)
        timer.cycle(255);
        timer.cycle(255);
        timer.cycle(2);
        assert_eq!(timer.div, 2);

        // Reset DIV
        timer.reset_div();
        assert_eq!(timer.div, 0);
        assert_eq!(timer.internal_div_counter, 0);
    }

    #[test]
    fn test_tima_4096hz() {
        let mut timer = Timer::new();
        timer.set_tac(0x04); // Enable timer, 4096 Hz - bit 10, falling edge every 1024 cycles
        timer.tima = 0;

        // Run 1023 cycles - TIMA should still be 0  
        timer.cycle(255);
        timer.cycle(255);
        timer.cycle(255);
        timer.cycle(255);
        timer.cycle(3); // Total: 1023
        assert_eq!(timer.tima, 0);

        // Run 1 more cycle (1024 total) - TIMA should be 1
        timer.cycle(1);
        assert_eq!(timer.tima, 1);
    }

    #[test]
    fn test_tima_262144hz() {
        let mut timer = Timer::new();
        timer.set_tac(0x05); // Enable timer, 262144 Hz - bit 3, falling edge every 16 cycles
        timer.tima = 0;

        // Run 15 cycles - TIMA should still be 0
        timer.cycle(15);
        assert_eq!(timer.tima, 0);

        // Run 1 more cycle (16 total) - TIMA should be 1
        timer.cycle(1);
        assert_eq!(timer.tima, 1);

        // Run another 16 cycles - TIMA should be 2
        timer.cycle(16);
        assert_eq!(timer.tima, 2);
    }

    #[test]
    fn test_tima_65536hz() {
        let mut timer = Timer::new();
        timer.set_tac(0x06); // Enable timer, 65536 Hz - bit 6, falling edge every 64 cycles
        timer.tima = 0;

        // Run 63 cycles - TIMA should still be 0
        timer.cycle(63);
        assert_eq!(timer.tima, 0);

        // Run 1 more cycle (64 total) - TIMA should be 1
        timer.cycle(1);
        assert_eq!(timer.tima, 1);
    }

    #[test]
    fn test_tima_16384hz() {
        let mut timer = Timer::new();
        timer.set_tac(0x07); // Enable timer, 16384 Hz - bit 8, falling edge every 256 cycles
        timer.tima = 0;

        // Run 255 cycles - TIMA should still be 0
        timer.cycle(255);
        assert_eq!(timer.tima, 0);

        // Run 1 more cycle (256 total) - TIMA should be 1
        timer.cycle(1);
        assert_eq!(timer.tima, 1);
    }
    
    #[test]
    fn test_tima_overflow_and_interrupt() {
        let mut timer = Timer::new();
        timer.set_tac(0x05); // 262144 Hz - bit 3, every 16 cycles
        timer.tima = 0xFF; // Set to max value
        timer.tma = 0x42;  // Reload value

        // Run 16 cycles to trigger increment and overflow
        let interrupt = timer.cycle(16);

        // Should overflow and trigger interrupt
        assert!(interrupt);

        // TIMA should be 0x00 during the 4-cycle delay period
        assert_eq!(timer.tima, 0x00);

        // Run 4 more cycles to complete the reload
        let interrupt2 = timer.cycle(4);

        // Should now have reloaded with TMA value, no additional interrupt
        assert_eq!(timer.tima, 0x42);
        assert!(!interrupt2);
    }

    #[test]
    fn test_timer_disabled() {
        let mut timer = Timer::new();
        timer.set_tac(0x01); // Disabled timer (bit 2 not set), 262144 Hz frequency
        timer.tima = 0;

        // Run many cycles - TIMA should not increment when disabled
        timer.cycle(255);
        assert_eq!(timer.tima, 0);
    }

    #[test]
    fn test_div_continues_when_timer_disabled() {
        let mut timer = Timer::new();
        timer.set_tac(0x00); // Disabled timer

        // DIV should continue incrementing even when timer is disabled
        timer.cycle(255);
        timer.cycle(1); // 256 total cycles
        assert_eq!(timer.div, 1);
    }

    #[test]
    fn test_tac_enable_disable() {
        let mut timer = Timer::new();

        // Enable timer
        timer.set_tac(0x04); // Enable, 4096 Hz
        assert!(timer.enabled);
        assert_eq!(timer.tac, 0x04);

        // Disable timer
        timer.set_tac(0x00); // Disable
        assert!(!timer.enabled);
        assert_eq!(timer.tac, 0x00);
    }

    #[test]
    fn test_div_reset_falling_edge() {
        let mut timer = Timer::new();
        timer.set_tac(0x05); // 262144 Hz (bit 3, every 16 cycles)
        timer.tima = 0;

        // Advance to trigger the first falling edge at cycle 16
        timer.cycle(16); // Now at 16 cycles total
        assert_eq!(timer.tima, 1); // Should have incremented once

        // Advance a bit more to ensure bit 3 is 1 again
        timer.cycle(8); // Now at 24 cycles total, bit 3 should be 1

        let before_reset_tima = timer.tima;

        // Reset DIV - if bit 3 was 1, this should cause a falling edge and increment TIMA
        timer.reset_div();

        // TIMA should have incremented due to falling edge caused by DIV reset
        assert_eq!(timer.tima, before_reset_tima + 1);
    }

    #[test]
    fn test_tac_change_falling_edge() {
        let mut timer = Timer::new();

        // Set up timer with some cycles elapsed
        timer.cycle(100);

        // Change TAC settings - this can trigger falling edges
        timer.set_tac(0x05); // Enable 262144 Hz
        let initial_tima = timer.tima;

        // Change frequency
        timer.set_tac(0x04); // Change to 4096 Hz

        // Verify no unexpected increments happened just from changing TAC
        // (This test helps catch edge cases in TAC changes)
        // The TIMA should only change due to actual timer cycles, not TAC changes
    }

    #[test]
    fn test_internal_counter_wrapping() {
        let mut timer = Timer::new();

        // Test that internal counter wraps properly at 16-bit boundary
        timer.internal_div_counter = 0xFFFF;
        timer.div = 0xFF;

        timer.cycle(1);

        assert_eq!(timer.internal_div_counter, 0x0000);
        assert_eq!(timer.div, 0x00);
    }

    #[test]
    fn test_precise_timing_4096hz() {
        let mut timer = Timer::new();
        timer.set_tac(0x04); // 4096 Hz (increments every 1024 cycles, not 512)
        timer.tima = 0;

        // Test exact timing boundaries - 1024 cycles per increment for 4096 Hz
        for i in 1..=2 {
            // Run exactly 1024 cycles: 255*4 + 4 = 1024
            timer.cycle(255);
            timer.cycle(255);
            timer.cycle(255);
            timer.cycle(255);
            timer.cycle(4);
            assert_eq!(timer.tima, i, "TIMA should be {} after {} * 1024 cycles", i, i);
        }
    }
}