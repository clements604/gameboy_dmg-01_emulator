use crate::apu::Apu;
use crate::joypad;
use crate::interrupts::Interrupt;
use crate::memory_bus::{IO_REGISTERS_START, IO_REGISTERS_SIZE};
use crate::ppu::Ppu;
use crate::timer::Timer;

const JOYPAD_REGISTER: u16 = 0xFF00;
const SERIAL_DATA_REGISTER: u16 = 0xFF01;
const SERIAL_CONTROL_REGISTER: u16 = 0xFF02;
const DIV_REGISTER: u16 = 0xFF04;
const TIMA_REGISTER: u16 = 0xFF05;
const TMA_REGISTER: u16 = 0xFF06;
const TAC_REGISTER: u16 = 0xFF07;
const APU_START: u16 = 0xFF10;
const WAVE_PATTERN_START: u16 = 0xFF30;
const WAVE_PATTERN_END: u16 = 0xFF3F;
const APU_END: u16 = 0xFF26;
const PPU_START: u16 = 0xFF40;
const PPU_END: u16 = 0xFF46;
const PPU_PALETTE_START: u16 = 0xFF47;
const PPU_PALETTE_END: u16 = 0xFF4B;
const JOYPAD_BUTTON_SELECT_BIT: u8 = 5;
const JOYPAD_DPAD_SELECT_BIT: u8 = 4;
const TIMER_ENABLE_MASK: u8 = 0b100;

pub struct IO {
    io_registers: [u8; IO_REGISTERS_SIZE],
    serial_data: [char; 2],
    pub ppu: Ppu,
    pub joypad: joypad::Joypad,
    pub timer: Timer,
    pub apu: Apu,
}

impl IO {
    pub fn new() -> IO {

        IO {
            io_registers: [0; IO_REGISTERS_SIZE],
            serial_data: ['\0'; 2],
            ppu: Ppu::new(),
            joypad: joypad::Joypad::new(),
            timer: Timer::new(),
            apu: Apu::new(),
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            JOYPAD_REGISTER => u8::from(self.joypad),
            SERIAL_DATA_REGISTER => self.serial_data[0] as u8,
            SERIAL_CONTROL_REGISTER => self.serial_data[1] as u8,
            DIV_REGISTER => self.timer.div,
            TIMA_REGISTER => self.timer.tima,
            TMA_REGISTER => self.timer.tma,
            TAC_REGISTER => {
                let mut value = self.timer.tac;
                if self.timer.enabled {
                    value |= TIMER_ENABLE_MASK; // Set the enable bit
                }
                value
            },
            APU_START..=APU_END => self.apu.read(address),
            WAVE_PATTERN_START..=WAVE_PATTERN_END => self.apu.read(address),
            PPU_START..=PPU_END => self.ppu.read(address),
            PPU_PALETTE_START..=PPU_PALETTE_END => self.ppu.read(address),
            _ => {
                self.io_registers[(address - IO_REGISTERS_START) as usize]
            }
        }
    }
    pub fn write(&mut self, address: u16, value: u8) -> Option<Interrupt> {
        match address {
            JOYPAD_REGISTER => {
                // Only update the selection bits; button states are managed by input
                let mut joypad = self.joypad;
                joypad.select_buttons = value & (1 << JOYPAD_BUTTON_SELECT_BIT) == 0;
                joypad.select_dpad = value & (1 << JOYPAD_DPAD_SELECT_BIT) == 0;
                self.joypad = joypad;
            },
            SERIAL_DATA_REGISTER => {
                self.serial_data[0] = value as char;
            },
            SERIAL_CONTROL_REGISTER => {
                self.serial_data[1] = value as char;
            },
            DIV_REGISTER => {
                self.timer.reset_div();
            },
            TIMA_REGISTER => {
                self.timer.write_tima(value);
            },
            TMA_REGISTER => self.timer.write_tma(value),
            TAC_REGISTER => {
                if self.timer.set_tac(value) {
                    return Some(Interrupt::TIMER)
                }
            },
            APU_START..=APU_END => self.apu.write(address, value),
            WAVE_PATTERN_START..=WAVE_PATTERN_END => self.apu.write(address, value),
            PPU_START..=PPU_END => {
                self.ppu.write(address, value);
            },
            PPU_PALETTE_START..=PPU_PALETTE_END => {
                self.ppu.write(address, value);
            },
            _ => {
                self.io_registers[(address - IO_REGISTERS_START) as usize] = value;
            }
        }
        None
    }

}
