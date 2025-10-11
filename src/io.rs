use crate::joypad;
use crate::interrupts::Interrupt;
use crate::memory_bus::{IO_REGISTERS_START, IO_REGISTERS_SIZE};
use crate::ppu::Ppu;
use crate::timer::Timer;

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
            timer: Timer::new(),
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0xFF00 => u8::from(self.joypad),
            0xFF01 => self.serial_data[0] as u8,
            0xFF02 => self.serial_data[1] as u8,
            0xFF04 => self.timer.div,
            0xFF05 => self.timer.tima,
            0xFF06 => self.timer.tma,
            0xFF07 => {
                let mut value = self.timer.tac;
                if self.timer.enabled {
                    value |= 0b100; // Set the enable bit
                }
                value
            },
            0xFF40..=0xFF46 => self.ppu.read(address),
            0xFF47..=0xFF4B => self.ppu.read(address),
            _ => {
                self.io_registers[(address - IO_REGISTERS_START) as usize]
            }
        }
    }
    pub fn write(&mut self, address: u16, value: u8) -> Option<Interrupt> {
        match address {
            0xFF00 => {
                // Only update the selection bits; button states are managed by input
                let mut joypad = self.joypad;
                joypad.select_buttons = value & (1 << 5) == 0;
                joypad.select_dpad = value & (1 << 4) == 0;
                self.joypad = joypad;
            },
            0xFF01 => {
                self.serial_data[0] = value as char;
            },
            0xFF02 => {
                self.serial_data[1] = value as char;
            },
            0xFF04 => {
                self.timer.reset_div();
            },
            0xFF05 => {
                self.timer.write_tima(value);
            },
            0xFF06 => self.timer.write_tma(value),
            0xFF07 => {
                if self.timer.set_tac(value) {
                    return Some(Interrupt::TIMER)
                }
            },
            0xFF40..=0xFF46 => {

                self.ppu.write(address, value);
            },
            0xFF47..=0xFF4B => {
                self.ppu.write(address, value);
            },
            _ => {
                self.io_registers[(address - IO_REGISTERS_START) as usize] = value;
            }
        }
        None
    }

}
