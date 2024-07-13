use crate::CPU::CPU;

pub enum Interrupt {
    VBlank,
    LCDStat,
    Timer,
    Serial,
    Joypad,
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

pub fn check_interrupts(cpu: &mut CPU, interrupt: Interrupt) -> bool {
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
}

pub fn handle_interrupt(cpu: &mut CPU, interrupt: Interrupt) {
    cpu.op_push_stack(cpu.registers.pc);
    cpu.registers.pc = match interrupt {
        Interrupt::VBlank => 0x40,
        Interrupt::LCDStat => 0x48,
        Interrupt::Timer => 0x50,
        Interrupt::Serial => 0x58,
        Interrupt::Joypad => 0x60,
    };
}

pub fn handle_interrupts(cpu: &mut CPU) {
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
}


