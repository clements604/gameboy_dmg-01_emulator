use std::cell::RefCell;
use std::rc::Rc;
use log::{debug, info};
use crate::CPU::CPU;
use crate::interupts::Interrupt;

pub struct Timer {
    div: u16,
    tima: u16,
    tma: u16,
    tac: u16,
    cpu: Rc<RefCell<CPU>>,
}

impl Timer {
    pub fn new(cpu: Rc<RefCell<CPU>>) -> Timer {
        Timer {
            div: 0xAC00,
            tima: 0,
            tma: 0,
            tac: 0,
            cpu,
        }
    }
    pub fn read(&self, address: u16) -> u16 {
        match address {
            0xFF04 => self.div >> 8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac,
            _ => panic!("Invalid Timer address: {:#X}", address),
        }
    }
    pub fn write(&mut self, address: u16, value: u16) {
        match address {
            0xFF04 => self.div = 0,
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => self.tac = value,
            _ => panic!("Invalid Timer address: {:#X}", address),
        }
    }
    pub fn tick(&mut self) {
        let previous_div = self.div;
        self.div = self.div.wrapping_add(1);
        debug!("Previous DIV: {:#X}, New DIV: {:#X}", previous_div, self.div);

        let timer_update: bool = match self.tac & 0x3 {
            0 => (previous_div & (1 << 9) != 0) && (self.div & (1 << 9) == 0),
            1 => (previous_div & (1 << 3) != 0) && (self.div & (1 << 3) == 0),
            2 => (previous_div & (1 << 5) != 0) && (self.div & (1 << 5) == 0),
            3 => (previous_div & (1 << 7) != 0) && (self.div & (1 << 7) == 0),
            _ => panic!("Invalid Timer frequency: {:#X}", self.tac & 0x3),
        };
        debug!("Timer update: {}", timer_update);
        
        if timer_update && self.tac & (1<<2) != 0 {
            self.tima = self.tima.wrapping_add(1);
            if self.tima == 0xFF {
                self.tima = self.tma;
                self.cpu.borrow_mut().trigger_interrupt(Interrupt::TIMER);
            }
        }

    }
}