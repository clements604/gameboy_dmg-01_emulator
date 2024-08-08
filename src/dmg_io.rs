use std::cell::RefCell;
use std::rc::Rc;
use log::{debug, error, info};
use crate::CPU::CPU;
use crate::display::Display;
use crate::dma::Dma;
use crate::lcd::LCD;
use crate::memory_bus::{IO_REGISTERS_START, IO_REGISTERS_SIZE, MemoryBus};

pub struct IO {
    io_registers: [u8; IO_REGISTERS_SIZE],
    serial_data: [char; 2],
    pub lcd: Rc<RefCell<LCD>>,
    dma: Rc<RefCell<Dma>>,
    cpu: Rc<RefCell<CPU>>,
}

impl IO {
    pub fn new(dma: Rc<RefCell<Dma>>, cpu: Rc<RefCell<CPU>>, lcd: Rc<RefCell<LCD>>) -> IO {
        IO {
            io_registers: [0; IO_REGISTERS_SIZE],
            serial_data: ['\0'; 2],
            lcd,
            dma,
            cpu,
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {//FIXME remove mut
        debug!("Read from IO address: {:#X}", address);
        match address {
            0xFF00 => {
                unimplemented!("Read from JOYPAD");
            }
            0xFF01 => {
                self.serial_data[0] as u8
            },
            0xFF02 => {
                self.serial_data[1] as u8
            },
            0xFF40..=0xFF4B => {
                self.lcd.as_ref().borrow().read(address)
            },
            _ => {
                debug!("Read from IO address: {:#X}", address);
                self.io_registers[(address - IO_REGISTERS_START) as usize]
            }
        }
    }
    pub fn write(&mut self, address: u16, value: u8/*, cpu: &mut CPU*/) {
        debug!("Write to IO address: {:#X}", address);
        match address {
            0xFF01 => {
                self.serial_data[0] = value as char;
            },
            0xFF02 => {
                self.serial_data[1] = value as char;
            },
            0xFF40..=0xFF4B => {
                self.lcd.as_ref().borrow_mut().write(address, value);
            },
            _ => {
                debug!("Write to IO address: {:#X}", address);
                self.io_registers[(address - IO_REGISTERS_START) as usize] = value;
            }
        }
        self.io_registers[(address - IO_REGISTERS_START) as usize] = value;
    }

}
