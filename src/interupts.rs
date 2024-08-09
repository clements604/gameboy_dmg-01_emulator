use crate::constants::{CARRY_FLAG_BYTE_POSITION, HALF_CARRY_FLAG_BYTE_POSITION, SUBTRACT_FLAG_BYTE_POSITION, ZERO_FLAG_BYTE_POSITION};
use crate::CPU::{CPU, FlagsRegister};

#[derive(Debug)]
pub enum Interrupt {
    VBLANK,
    LCDSTAT,
    TIMER,
    SERIAL,
    JOYPAD,
}
#[derive(Copy, Clone, Debug)]
pub struct InterruptFlags {
    pub vblank: bool,
    pub lcd_stat: bool,
    pub timer: bool,
    pub serial: bool,
    pub joypad: bool,
}

impl InterruptFlags {
    pub fn new() -> Self {
        InterruptFlags {
            vblank: false,
            lcd_stat: false,
            timer: false,
            serial: false,
            joypad: false,
        }
    }
}
impl std::convert::From<InterruptFlags> for u8 {
    fn from(flag: InterruptFlags) -> u8 {
        let mut result = 0;
        if flag.vblank {
            result |= 0x01;
        }
        if flag.lcd_stat {
            result |= 0x02;
        }
        if flag.timer {
            result |= 0x04;
        }
        if flag.serial {
            result |= 0x08;
        }
        if flag.joypad {
            result |= 0x10;
        }
        result
    }
}
impl std::convert::From<u8> for InterruptFlags {
    fn from(byte: u8) -> Self {
        InterruptFlags {
            vblank: byte & 0x01 != 0,
            lcd_stat: byte & 0x02 != 0,
            timer: byte & 0x04 != 0,
            serial: byte & 0x08 != 0,
            joypad: byte & 0x10 != 0,
        }
    }
}

/*
pub struct Interrupts {
    pub interrupt_enable: u8,
    pub interrupt_flag: u8,
    pub interrupt_master_enable: bool,
}

impl Interrupts{
    pub fn new() -> Self {
        Interrupts {
            interrupt_enable: 0,
            interrupt_flag: 0,
            interrupt_master_enable: false,
        }
    }*/

/*pub fn check_interrupts(cpu: &mut CPU, interrupt: Interrupt) -> bool {
    let interrupt_flag = match interrupt {
        Interrupt::VBlank => 0x1,
        Interrupt::LCDStat => 0x2,
        Interrupt::Timer => 0x4,
        Interrupt::Serial => 0x8,
        Interrupt::Joypad => 0x10,
    };
    if cpu.interrupt_flags & interrupt_flag != 0 && cpu.interrupt_enable_register & interrupt_flag != 0 {
        handle_interrupt(cpu, interrupt);
        cpu.interrupt_flags &= !interrupt_flag;
        cpu.halted = false;
        cpu.interrupt_master_enable = false;
        return true;
    }
    false
}*/

/*pub fn handle_interrupt(cpu: &mut CPU, interrupt: Interrupt) {
    cpu.op_push_stack(cpu.registers.pc);
    cpu.registers.pc = match interrupt {
        Interrupt::VBlank => 0x40,
        Interrupt::LCDStat => 0x48,
        Interrupt::Timer => 0x50,
        Interrupt::Serial => 0x58,
        Interrupt::Joypad => 0x60,
    };
}*/

/*pub fn handle_interrupts(cpu: &mut CPU) {
    if cpu.interrupt_master_enable {
        if check_interrupts(cpu, Interrupt::VBlank) {
            return;
        }
        if check_interrupts(cpu, Interrupt::LCDStat) {
            return;
        }
        if check_interrupts(cpu, Interrupt::Timer) {
            return;
        }
        if check_interrupts(cpu, Interrupt::Serial) {
            return;
        }
        if check_interrupts(cpu, Interrupt::Joypad) {
            return;
        }
    }
}*/


