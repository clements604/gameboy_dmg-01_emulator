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
use crate::timer::{Timer, TimerFrequency};

pub struct IO {
    io_registers: [u8; IO_REGISTERS_SIZE],
    serial_data: [char; 2],
    pub lcd: Rc<RefCell<LCD>>,
    //dma: Rc<RefCell<Dma>>,
    cpu: Rc<RefCell<CPU>>,
    pub ppu: Rc<RefCell<Ppu>>,
    joypad: joypad::Joypad,
    pub timer: Timer,
}

impl IO {
    pub fn new(/*dma: Rc<RefCell<Dma>>, */cpu: Rc<RefCell<CPU>>, lcd: Rc<RefCell<LCD>>, ppu: Rc<RefCell<Ppu>>) -> IO {

        IO {
            io_registers: [0; IO_REGISTERS_SIZE],
            serial_data: ['\0'; 2],
            lcd,
            //dma,
            cpu: cpu.clone(),
            ppu,
            joypad: joypad::Joypad::new(),
            //timer: Timer::new(cpu.clone()),
            timer: Timer::new(timer::TimerFrequency::Hz4096),//TODO why have four variants if this is a constant?
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {//FIXME remove mut
        //debug!("Read from IO address: {:#X}", address);
        match address {
            0xFF00 => {
                //unimplemented!("Read from JOYPAD");
                u8::from(self.joypad.clone())
            }
            0xFF01 => {
                self.serial_data[0] as u8
            },
            0xFF02 => {
                self.serial_data[1] as u8
            },
            0xFF04 => {
                self.timer.div as u8
            },
            0xFF05 => {
                self.timer.tima
            },
            0xFF06 => {
                self.timer.tma
            },
            0xFF07 => {
                panic!("Tac register read")
            }
            0xFF40..=0xFF46 => {
                self.ppu.as_ref().borrow().read(address)
            },
            0xFF47..0xFF4B => {
                self.lcd.as_ref().borrow().read(address)
            },
            _ => {
                //debug!("Read from IO address: {:#X}", address);
                self.io_registers[(address - IO_REGISTERS_START) as usize]
            }
        }
    }
    pub fn write(&mut self, address: u16, value: u8/*, cpu: &mut CPU*/) {
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
                self.timer.div = 0;
            },
            0xFF05 => {
                self.timer.tima = value;
            },
            0xFF06 => {
                self.timer.tma = value;
            },
            0xFF07 => {
                self.timer.frequency = match value & 0b11 {
                    0b00 => TimerFrequency::Hz4096,
                    0b11 => TimerFrequency::Hz16384,
                    0b10 => TimerFrequency::Hz65536,
                    0b01 => TimerFrequency::Hz262144,
                    //_ => TimerFrequency::Hz262144,
                    _ => panic!("Invalid timer frequency: {:#X}", value),
                };
                self.timer.enabled = (value & 0b100) == 0b100;
            }
            0xFF40..=0xFF46 => {
                self.ppu.as_ref().borrow_mut().write(address, value);
            },
            0xFF47..0xFF4B => {
                self.lcd.as_ref().borrow_mut().write(address, value);
            },
            _ => {
                //debug!("Write to IO address: {:#X}", address);
                self.io_registers[(address - IO_REGISTERS_START) as usize] = value;
            }
        }
        self.io_registers[(address - IO_REGISTERS_START) as usize] = value;
    }

}
