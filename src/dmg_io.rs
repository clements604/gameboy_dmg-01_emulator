use std::cell::RefCell;
use std::rc::Rc;
use log::{debug, error, info};
use crate::CPU::CPU;
//use crate::display::Display;
use crate::dma::Dma;
use crate::{joypad, timer};
use crate::lcd::LCD;
use crate::memory_bus::{IO_REGISTERS_START, IO_REGISTERS_SIZE, MemoryBus};
use crate::ppu::Ppu;
use crate::timer::{Timer};

pub struct IO {
    io_registers: [u8; IO_REGISTERS_SIZE],
    serial_data: [char; 2],
    pub ppu: Ppu,
    pub joypad: joypad::Joypad,
    pub timer: Timer,
}

impl IO {
    pub fn new() -> IO {

        IO {
            io_registers: [0; IO_REGISTERS_SIZE],
            serial_data: ['\0'; 2],
            ppu: Ppu::new(),
            joypad: joypad::Joypad::new(),
            timer: Timer::new(),//TODO why have four variants if this is a constant? timer::TimerFrequency::Hz4096
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        debug!("Read from IO address: {:#X}", address);
        match address {
            //0xFF00 => u8::from(self.joypad.clone()),
            0xFF00 => u8::from(self.joypad.clone()),
            0xFF01 => self.serial_data[0] as u8,
            0xFF02 => self.serial_data[1] as u8,
            0xFF04 => {
                let low_byte = (self.timer.div & 0xFF) as u8;
                low_byte
            },
            0xFF05 => self.timer.tima,
            0xFF06 => self.timer.tma,
            0xFF07 => {
                let mut value = self.timer.tac;
                // Frequency is now determined by the lower 2 bits of tac directly
                if self.timer.enabled {
                    value |= 0b100; // Set the enable bit
                }
                value
            },
            0xFF40..=0xFF46 => self.ppu.read(address),
            0xFF47..=0xFF4B => self.ppu.read(address),
            _ => {
                debug!("Reading from IO address: {:#X}", address);
                self.io_registers[(address - IO_REGISTERS_START) as usize]
            }
        }
    }
    pub fn write(&mut self, address: u16, value: u8) {
        //debug!("Write to IO address: {:#X}", address);
        match address {
            0xFF00 => {
                self.joypad = joypad::Joypad::from(value);
            },
            0xFF01 => {
                self.serial_data[0] = value as char;
            },
            0xFF02 => {
                self.serial_data[1] = value as char;
            },
            0xFF04 => {
                //self.timer.div = 0; // Reset DIV when written to
                self.timer.reset_div();
            },
            0xFF05 => self.timer.tima = value,
            0xFF06 => self.timer.tma = value,
            0xFF07 => {
                self.timer.tac = value;
                self.timer.enabled = (value & 0b100) != 0; // Check if bit 2 is set
            },
            0xFF40..=0xFF46 => {

                self.ppu.write(address, value);
            },
            0xFF47..=0xFF4B => {
                self.ppu.write(address, value);
            },
            _ => {
                //debug!("Write to IO address: {:#X}", address);
                self.io_registers[(address - IO_REGISTERS_START) as usize] = value;
            }
        }
        //self.io_registers[(address - IO_REGISTERS_START) as usize] = value;
    }

}
