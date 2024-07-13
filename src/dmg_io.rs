use log::{debug, error};
use crate::CPU::CPU;
use crate::memory_bus::{IO_REGISTERS_START, IO_REGISTERS_SIZE};

pub struct IO {
    io_registers: [u8; IO_REGISTERS_SIZE],
    serial_data: [char; 2],
}

impl IO {
    pub fn new() -> IO {
        IO {
            io_registers: [0; IO_REGISTERS_SIZE],
            serial_data: ['\0'; 2],
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        debug!("Read from IO address: {:#X}", address);
        match address {
            0xFF01 => {
                self.serial_data[0] as u8
            },
            0xFF02 => {
                self.serial_data[1] as u8
            },
            _ => {
                self.io_registers[(address - IO_REGISTERS_START) as usize]
            }
        }
    }
    pub fn write(&mut self, address: u16, value: u8, cpu: &mut CPU) {
        debug!("Write to IO address: {:#X}", address);
        match address {
            0xFF01 => {
                self.serial_data[0] = value as char;
            },
            0xFF02 => {
                self.serial_data[1] = value as char;
            },
            0xFF0F => {
                debug!("Interrupt flag write: {:#X}", value);
                cpu.interrupt_flags = value;
            },
            _ => {
                self.io_registers[(address - IO_REGISTERS_START) as usize] = value;
            }
        }
    }

}
