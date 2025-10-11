use std::fmt;

use bitflags::bitflags;
use log::error;
use memory_bus::MemoryBus;

use crate::interrupts::*;
use crate::memory_bus;

const INITIAL_PC: u16 = 0x0100;
const INITIAL_SP: u16 = 0xFFFE;

// Interrupt vector addresses
const VBLANK_VECTOR: u16 = 0x0040;
const LCDSTAT_VECTOR: u16 = 0x0048;
const TIMER_VECTOR: u16 = 0x0050;
const SERIAL_VECTOR: u16 = 0x0058;
const JOYPAD_VECTOR: u16 = 0x0060;

// Interrupt masks
const VBLANK_MASK: u8 = 0x01;
const LCDSTAT_MASK: u8 = 0x02;
const TIMER_MASK: u8 = 0x04;
const SERIAL_MASK: u8 = 0x08;
const JOYPAD_MASK: u8 = 0x10;

// Memory mapped IO addresses
const DIVIDER_REGISTER: u16 = 0xFF04;

// Initial register values
const INIT_A: u8 = 0x01;
const INIT_C: u8 = 0x13;
const INIT_E: u8 = 0xD8;
const INIT_H: u8 = 0x01;
const INIT_L: u8 = 0x4D;

// Bit masks
const LSB_MASK: u8 = 0x0F;
const MSB_MASK: u8 = 0xF0;
const BYTE_MSB: u8 = 0x80;
const BYTE_LSB: u8 = 0x01;

struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: FlagsRegister,
    h: u8,
    l: u8
}

bitflags! {
    struct FlagsRegister: u8 {
        const ZERO      = 0b1000_0000;
        const SUBTRACT  = 0b0100_0000;
        const HALF_CARRY= 0b0010_0000;
        const CARRY     = 0b0001_0000;
    }
}

pub struct CPU {
    registers: Registers,
    pub pc: u16, // Program counter
    sp: u16,     // Stack pointer
    halted: bool,
    stopped: bool,
}

impl Registers {

    pub fn new() -> Self {
        let mut f: FlagsRegister = FlagsRegister::empty();
        f.insert(FlagsRegister::ZERO);
        f.insert(FlagsRegister::HALF_CARRY);
        f.insert(FlagsRegister::CARRY);
        Registers {
            a: INIT_A,
            b: 0x0,
            c: INIT_C,
            d: 0x0,
            e: INIT_E,
            f,
            h: INIT_H,
            l: INIT_L
        }
    }

    fn get_af(&self) -> u16 {
        let flags: u16 = self.f.bits() as u16;
        let af: u16 = (self.a as u16) << 8 | flags;
        af
    }

    fn get_bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }

    fn get_de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }

    fn get_hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }

    fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = FlagsRegister::from_bits_truncate(value as u8 & MSB_MASK);
    }

    fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }
}

impl fmt::Display for Registers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "A: {:02X}
             B: {:02X}
             C: {:02X}
             D: {:02X}
             E: {:02X}
             H: {:02X}
             L: {:02X}
             AF: {:04X}
             BC: {:04X}
             DE: {:04X}
             HL: {:04X}
             F: {:08b}",
                self.a,
                self.b,
                self.c,
                self.d,
                self.e,
                self.h,
                self.l,
                self.get_af(),
                self.get_bc(),
                self.get_de(),
                self.get_hl(),
                self.f.bits()
        )
    }
}
impl fmt::Display for CPU {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "PC: {:04X}
             SP: {:04X}
             Halted: {}
             Stopped: {}
             Registers: {}",
             self.pc, self.sp, self.halted, self.stopped, self.registers
        )
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            registers: Registers::new(),
            pc: INITIAL_PC,
            sp: INITIAL_SP,
            halted: false,
            stopped: false,
        }
    }

    /*
     *   CPU cycle - fetch, decode, execute
     */
    pub fn cycle(&mut self, memory_bus: &mut MemoryBus) -> u8 {

        if !self.halted {

            let opcode = memory_bus.read_byte(self.pc);
            self.pc = self.pc.wrapping_add(1);

            match opcode {
                0x00 => {
                    self.op_nop();
                    4
                }
                0x01 => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    self.registers.set_bc(nn);
                    12
                }
                0x02 => {
                    memory_bus.write_byte(self.registers.get_bc(), self.registers.a);
                    8
                }
                0x03 => {
                    self.registers
                        .set_bc(self.registers.get_bc().wrapping_add(1));
                    8
                }
                0x04 => {
                    self.registers.b = self.op_inc_r8(self.registers.b);
                    4
                }
                0x05 => {
                    self.registers.b = self.op_dec_r8(self.registers.b);
                    4
                }
                0x06 => {
                    let value = self.read_immediate_byte(memory_bus);
                    self.registers.b = value;
                    8
                }
                0x07 => {
                    let a = self.registers.a;
                    let new_carry = (a & BYTE_MSB) != 0;
                    self.registers.a = (a << 1) | if new_carry { 0x01 } else { 0x00 };
                    self.registers.f.set(FlagsRegister::CARRY, new_carry);
                    self.registers.f.set(FlagsRegister::ZERO, false);
                    self.registers.f.set(FlagsRegister::SUBTRACT, false);
                    self.registers.f.set(FlagsRegister::HALF_CARRY, false);
                    4
                }
                0x08 => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    self.write_immediate_short(memory_bus, nn, self.sp);
                    20
                }
                0x09 => {
                    let hl = self.registers.get_hl();
                    let value = self.registers.get_bc();
                    let result = self.op_add_r16(hl, value);
                    self.registers.set_hl(result);
                    8
                }
                0x0A => {
                    self.registers.a = memory_bus.read_byte(self.registers.get_bc());
                    8
                }
                0x0B => {
                    self.registers.set_bc(self.registers.get_bc().wrapping_sub(1));
                    8
                }
                0x0C => {
                    self.registers.c = self.op_inc_r8(self.registers.c);
                    4
                }
                0x0D => {
                    self.registers.c = self.op_dec_r8(self.registers.c);
                    4
                }
                0x0E => {
                    let value = self.read_immediate_byte(memory_bus);
                    self.registers.c = value;
                    8
                }
                0x0F => {
                    let a = self.registers.a;
                    let carry = (a & 0x01) != 0;
                    self.registers.a = (a >> 1) | (carry as u8) << 7;
                    self.registers.f.set(FlagsRegister::CARRY, carry);
                    self.registers.f.set(FlagsRegister::ZERO, false);
                    self.registers.f.set(FlagsRegister::SUBTRACT, false);
                    self.registers.f.set(FlagsRegister::HALF_CARRY, false);
                    4
                }
                0x10 => {
                    self.op_stop(memory_bus);
                    4
                }
                0x11 => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    self.registers.set_de(nn);
                    12
                }
                0x12 => {
                    memory_bus.write_byte(self.registers.get_de(), self.registers.a);
                    8
                }
                0x13 => {
                    self.registers.set_de(self.registers.get_de().wrapping_add(1));
                    8
                }
                0x14 => {
                    self.registers.d = self.op_inc_r8(self.registers.d);
                    4
                }
                0x15 => {
                    self.registers.d = self.op_dec_r8(self.registers.d);
                    4
                }
                0x16 => {
                    let value = self.read_immediate_byte(memory_bus);
                    self.registers.d = value;
                    8
                }
                0x17 => {
                    let carry = self.registers.a & BYTE_MSB != 0;
                    self.registers.a = (self.registers.a << 1)
                        | (if self.registers.f.contains(FlagsRegister::CARRY) {
                        1
                    } else {
                        0
                    });
                    self.registers.f.set(FlagsRegister::ZERO, false);
                    self.registers.f.set(FlagsRegister::SUBTRACT, false);
                    self.registers.f.set(FlagsRegister::HALF_CARRY, false);
                    self.registers.f.set(FlagsRegister::CARRY, carry);
                    4
                }
                0x18 => {
                    let offset = self.read_immediate_byte(memory_bus) as i8;
                    self.op_jr_e(offset);
                    12
                }
                0x19 => {
                    let hl = self.registers.get_hl();
                    let value = self.registers.get_de();
                    let result = self.op_add_r16(hl, value);
                    self.registers.set_hl(result);
                    8
                }
                0x1A => {
                    self.registers.a = memory_bus.read_byte(self.registers.get_de());
                    8
                }
                0x1B => {
                    self.registers.set_de(self.registers.get_de().wrapping_sub(1));
                    8
                }
                0x1C => {
                    self.registers.e = self.op_inc_r8(self.registers.e);
                    4
                }
                0x1D => {
                    self.registers.e = self.op_dec_r8(self.registers.e);
                    4
                }
                0x1E => {
                    let value = self.read_immediate_byte(memory_bus);
                    self.registers.e = value;
                    8
                }
                0x1F => {
                    let carry = self.registers.a & 0x01 != 0;
                    self.registers.a = (self.registers.a >> 1)
                        | (if self.registers.f.contains(FlagsRegister::CARRY) {
                        0x80
                    } else {
                        0
                    });
                    self.registers.f.set(FlagsRegister::ZERO, false);
                    self.registers.f.set(FlagsRegister::SUBTRACT, false);
                    self.registers.f.set(FlagsRegister::HALF_CARRY, false);
                    self.registers.f.set(FlagsRegister::CARRY, carry);
                    4
                }
                0x20 => {
                    let offset = self.read_immediate_byte(memory_bus) as i8;
                    if !self.registers.f.contains(FlagsRegister::ZERO) {
                        self.op_jr_e(offset);
                        return 12;
                    }
                    8
                }
                0x21 => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    self.registers.set_hl(nn);
                    12
                }
                0x22 => {
                    memory_bus.write_byte(self.registers.get_hl(), self.registers.a);
                    self.registers.set_hl(self.registers.get_hl().wrapping_add(1));
                    8
                }
                0x23 => {
                    self.registers.set_hl(self.registers.get_hl().wrapping_add(1));
                    8
                }
                0x24 => {
                    self.registers.h = self.op_inc_r8(self.registers.h);
                    4
                }
                0x25 => {
                    self.registers.h = self.op_dec_r8(self.registers.h);
                    4
                }
                0x26 => {
                    let value = self.read_immediate_byte(memory_bus);
                    self.registers.h = value;
                    8
                }
                0x27 => {
                    self.op_daa();
                    4
                }
                0x28 => {
                    let offset = self.read_immediate_byte(memory_bus) as i8;
                    if self.registers.f.contains(FlagsRegister::ZERO) {
                        self.op_jr_e(offset);
                        return 12;
                    }
                    8
                }
                0x29 => {
                    let hl = self.registers.get_hl();
                    let result = self.op_add_r16(hl, hl);
                    self.registers.set_hl(result);
                    8
                }
                0x2A => {
                    self.registers.a = memory_bus.read_byte(self.registers.get_hl());
                    self.registers
                        .set_hl(self.registers.get_hl().wrapping_add(1));
                    8
                }
                0x2B => {
                    self.registers
                        .set_hl(self.registers.get_hl().wrapping_sub(1));
                    8
                }
                0x2C => {
                    self.registers.l = self.op_inc_r8(self.registers.l);
                    4
                }
                0x2D => {
                    self.registers.l = self.op_dec_r8(self.registers.l);
                    4
                }
                0x2E => {
                    let value = self.read_immediate_byte(memory_bus);
                    self.registers.l = value;
                    8
                }
                0x2F => {
                    self.op_cpl();
                    4
                }
                0x30 => {
                    let offset = self.read_immediate_byte(memory_bus) as i8;
                    if !self.registers.f.contains(FlagsRegister::CARRY) {
                        self.op_jr_e(offset);
                        return 12;
                    }
                    8
                }
                0x31 => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    self.sp = nn;
                    12
                }
                0x32 => {
                    memory_bus.write_byte(self.registers.get_hl(), self.registers.a);
                    self.registers
                        .set_hl(self.registers.get_hl().wrapping_sub(1));
                    8
                }
                0x33 => {
                    self.sp = self.sp.wrapping_add(1);
                    8
                }
                0x34 => {
                    self.op_inc_hl(memory_bus);
                    12
                }
                0x35 => {
                    self.op_dec_hl(memory_bus);
                    12
                }
                0x36 => {
                    let value = self.read_immediate_byte(memory_bus);
                    memory_bus.write_byte(self.registers.get_hl(), value);
                    12
                }
                0x37 => {
                    self.op_scf();
                    4
                }
                0x38 => {
                    let offset = self.read_immediate_byte(memory_bus) as i8;
                    if self.registers.f.contains(FlagsRegister::CARRY) {
                        self.op_jr_e(offset);
                        return 12;
                    }
                    8
                }
                0x39 => {
                    let hl = self.registers.get_hl();
                    let value = self.sp;
                    let result = self.op_add_r16(hl, value);
                    self.registers.set_hl(result);
                    8
                }
                0x3A => {
                    self.registers.a = memory_bus.read_byte(self.registers.get_hl());
                    self.registers
                        .set_hl(self.registers.get_hl().wrapping_sub(1));
                    8
                }
                0x3B => {
                    self.sp = self.sp.wrapping_sub(1);
                    8
                }
                0x3C => {
                    self.registers.a = self.op_inc_r8(self.registers.a);
                    4
                }
                0x3D => {
                    self.registers.a = self.op_dec_r8(self.registers.a);
                    4
                }
                0x3E => {
                    let value = self.read_immediate_byte(memory_bus);
                    self.registers.a = value;
                    8
                }
                0x3F => {
                    self.op_ccf();
                    4
                }
                0x40 => {
                    self.registers.b = self.registers.b;
                    4
                }
                0x41 => {
                    self.registers.b = self.registers.c;
                    4
                }
                0x42 => {
                    self.registers.b = self.registers.d;
                    4
                }
                0x43 => {
                    self.registers.b = self.registers.e;
                    4
                }
                0x44 => {
                    self.registers.b = self.registers.h;
                    4
                }
                0x45 => {
                    self.registers.b = self.registers.l;
                    4
                }
                0x46 => {
                    self.registers.b = memory_bus.read_byte(self.registers.get_hl());
                    8
                }
                0x47 => {
                    self.registers.b = self.registers.a;
                    4
                }
                0x48 => {
                    self.registers.c = self.registers.b;
                    4
                }
                0x49 => {
                    self.registers.c = self.registers.c;
                    4
                }
                0x4A => {
                    self.registers.c = self.registers.d;
                    4
                }
                0x4B => {
                    self.registers.c = self.registers.e;
                    4
                }
                0x4C => {
                    self.registers.c = self.registers.h;
                    4
                }
                0x4D => {
                    self.registers.c = self.registers.l;
                    4
                }
                0x4E => {
                    self.registers.c = memory_bus.read_byte(self.registers.get_hl());
                    8
                }
                0x4F => {
                    self.registers.c = self.registers.a;
                    4
                }
                0x50 => {
                    self.registers.d = self.registers.b;
                    4
                }
                0x51 => {
                    self.registers.d = self.registers.c;
                    4
                }
                0x52 => {
                    self.registers.d = self.registers.d;
                    4
                }
                0x53 => {
                    self.registers.d = self.registers.e;
                    4
                }
                0x54 => {
                    self.registers.d = self.registers.h;
                    4
                }
                0x55 => {
                    self.registers.d = self.registers.l;
                    4
                }
                0x56 => {
                    self.registers.d = memory_bus.read_byte(self.registers.get_hl());
                    4
                }
                0x57 => {
                    self.registers.d = self.registers.a;
                    4
                }
                0x58 => {
                    self.registers.e = self.registers.b;
                    4
                }
                0x59 => {
                    self.registers.e = self.registers.c;
                    4
                }
                0x5A => {
                    self.registers.e = self.registers.d;
                    4
                }
                0x5B => {
                    self.registers.e = self.registers.e;
                    4
                }
                0x5C => {
                    self.registers.e = self.registers.h;
                    4
                }
                0x5D => {
                    self.registers.e = self.registers.l;
                    4
                }
                0x5E => {
                    self.registers.e = memory_bus.read_byte(self.registers.get_hl());
                    8
                }
                0x5F => {
                    self.registers.e = self.registers.a;
                    4
                }
                0x60 => {
                    self.registers.h = self.registers.b;
                    4
                }
                0x61 => {
                    self.registers.h = self.registers.c;
                    4
                }
                0x62 => {
                    self.registers.h = self.registers.d;
                    4
                }
                0x63 => {
                    self.registers.h = self.registers.e;
                    4
                }
                0x64 => {
                    self.registers.h = self.registers.h;
                    4
                }
                0x65 => {
                    self.registers.h = self.registers.l;
                    4
                }
                0x66 => {
                    self.registers.h = memory_bus.read_byte(self.registers.get_hl());
                    8
                }
                0x67 => {
                    self.registers.h = self.registers.a;
                    4
                }
                0x68 => {
                    self.registers.l = self.registers.b;
                    4
                }
                0x69 => {
                    self.registers.l = self.registers.c;
                    4
                }
                0x6A => {
                    self.registers.l = self.registers.d;
                    4
                }
                0x6B => {
                    self.registers.l = self.registers.e;
                    4
                }
                0x6C => {
                    self.registers.l = self.registers.h;
                    4
                }
                0x6D => {
                    self.registers.l = self.registers.l;
                    4
                }
                0x6E => {
                    self.registers.l = memory_bus.read_byte(self.registers.get_hl());
                    8
                }
                0x6F => {
                    self.registers.l = self.registers.a;
                    4
                }
                0x70 => {
                    memory_bus.write_byte(self.registers.get_hl(), self.registers.b);
                    8
                }
                0x71 => {
                    memory_bus.write_byte(self.registers.get_hl(), self.registers.c);
                    8
                }
                0x72 => {
                    memory_bus.write_byte(self.registers.get_hl(), self.registers.d);
                    8
                }
                0x73 => {
                    memory_bus.write_byte(self.registers.get_hl(), self.registers.e);
                    8
                }
                0x74 => {
                    memory_bus.write_byte(self.registers.get_hl(), self.registers.h);
                    8
                }
                0x75 => {
                    memory_bus.write_byte(self.registers.get_hl(), self.registers.l);
                    8
                }
                0x76 => {
                    self.op_halt();
                    4
                }
                0x77 => {
                    memory_bus.write_byte(self.registers.get_hl(), self.registers.a);
                    8
                }
                0x78 => {
                    self.registers.a = self.registers.b;
                    4
                }
                0x79 => {
                    self.registers.a = self.registers.c;
                    4
                }
                0x7A => {
                    self.registers.a = self.registers.d;
                    4
                }
                0x7B => {
                    self.registers.a = self.registers.e;
                    4
                }
                0x7C => {
                    self.registers.a = self.registers.h;
                    4
                }
                0x7D => {
                    self.registers.a = self.registers.l;
                    4
                }
                0x7E => {
                    self.registers.a = memory_bus.read_byte(self.registers.get_hl());
                    8
                }
                0x7F => {
                    self.registers.a = self.registers.a;
                    4
                }
                0x80 => {
                    self.op_add_r8(self.registers.b);
                    4
                }
                0x81 => {
                    self.op_add_r8(self.registers.c);
                    4
                }
                0x82 => {
                    self.op_add_r8(self.registers.d);
                    4
                }
                0x83 => {
                    self.op_add_r8(self.registers.e);
                    4
                }
                0x84 => {
                    self.op_add_r8(self.registers.h);
                    4
                }
                0x85 => {
                    self.op_add_r8(self.registers.l);
                    4
                }
                0x86 => {
                    let value = memory_bus.read_byte(self.registers.get_hl());
                    self.op_add_r8(value);
                    8
                }
                0x87 => {
                    self.op_add_r8(self.registers.a);
                    4
                }
                0x88 => {
                    self.op_adc_r8(self.registers.b);
                    4
                }
                0x89 => {
                    self.op_adc_r8(self.registers.c);
                    4
                }
                0x8A => {
                    self.op_adc_r8(self.registers.d);
                    4
                }
                0x8B => {
                    self.op_adc_r8(self.registers.e);
                    4
                }
                0x8C => {
                    self.op_adc_r8(self.registers.h);
                    4
                }
                0x8D => {
                    self.op_adc_r8(self.registers.l);
                    4
                }
                0x8E => {
                    let value = memory_bus.read_byte(self.registers.get_hl());
                    self.op_adc_r8(value);
                    8
                }
                0x8F => {
                    self.op_adc_r8(self.registers.a);
                    4
                }
                0x90 => {
                    self.op_sub_r8(self.registers.b);
                    4
                }
                0x91 => {
                    self.op_sub_r8(self.registers.c);
                    4
                }
                0x92 => {
                    self.op_sub_r8(self.registers.d);
                    4
                }
                0x93 => {
                    self.op_sub_r8(self.registers.e);
                    4
                }
                0x94 => {
                    self.op_sub_r8(self.registers.h);
                    4
                }
                0x95 => {
                    self.op_sub_r8(self.registers.l);
                    4
                }
                0x96 => {
                    let value = memory_bus.read_byte(self.registers.get_hl());
                    self.op_sub_r8(value);
                    8
                }
                0x97 => {
                    self.op_sub_r8(self.registers.a);
                    4
                }
                0x98 => {
                    self.op_sbc_r8(self.registers.b);
                    4
                }
                0x99 => {
                    self.op_sbc_r8(self.registers.c);
                    4
                }
                0x9A => {
                    self.op_sbc_r8(self.registers.d);
                    4
                }
                0x9B => {
                    self.op_sbc_r8(self.registers.e);
                    4
                }
                0x9C => {
                    self.op_sbc_r8(self.registers.h);
                    4
                }
                0x9D => {
                    self.op_sbc_r8(self.registers.l);
                    4
                }
                0x9E => {
                    let value = memory_bus.read_byte(self.registers.get_hl());
                    self.op_sbc_r8(value);
                    8
                }
                0x9F => {
                    self.op_sbc_r8(self.registers.a);
                    4
                }
                0xA0 => {
                    self.op_and_r8(self.registers.b);
                    4
                }
                0xA1 => {
                    self.op_and_r8(self.registers.c);
                    4
                }
                0xA2 => {
                    self.op_and_r8(self.registers.d);
                    4
                }
                0xA3 => {
                    self.op_and_r8(self.registers.e);
                    4
                }
                0xA4 => {
                    self.op_and_r8(self.registers.h);
                    4
                }
                0xA5 => {
                    self.op_and_r8(self.registers.l);
                    4
                }
                0xA6 => {
                    let value = memory_bus.read_byte(self.registers.get_hl());
                    self.op_and_r8(value);
                    8
                }
                0xA7 => {
                    self.op_and_r8(self.registers.a);
                    4
                }
                0xA8 => {
                    self.op_xor_r8(self.registers.b);
                    4
                }
                0xA9 => {
                    self.op_xor_r8(self.registers.c);
                    4
                }
                0xAA => {
                    self.op_xor_r8(self.registers.d);
                    4
                }
                0xAB => {
                    self.op_xor_r8(self.registers.e);
                    4
                }
                0xAC => {
                    self.op_xor_r8(self.registers.h);
                    4
                }
                0xAD => {
                    self.op_xor_r8(self.registers.l);
                    4
                }
                0xAE => {
                    let value = memory_bus.read_byte(self.registers.get_hl());
                    self.op_xor_r8(value);
                    8
                }
                0xAF => {
                    self.op_xor_r8(self.registers.a);
                    4
                }
                0xB0 => {
                    self.op_or_r8(self.registers.b);
                    4
                }
                0xB1 => {
                    self.op_or_r8(self.registers.c);
                    4
                }
                0xB2 => {
                    self.op_or_r8(self.registers.d);
                    4
                }
                0xB3 => {
                    self.op_or_r8(self.registers.e);
                    4
                }
                0xB4 => {
                    self.op_or_r8(self.registers.h);
                    4
                }
                0xB5 => {
                    self.op_or_r8(self.registers.l);
                    4
                }
                0xB6 => {
                    let value = memory_bus.read_byte(self.registers.get_hl());
                    self.op_or_r8(value);
                    8
                }
                0xB7 => {
                    self.op_or_r8(self.registers.a);
                    4
                }
                0xB8 => {
                    self.op_cp_r8(self.registers.b);
                    4
                }
                0xB9 => {
                    self.op_cp_r8(self.registers.c);
                    4
                }
                0xBA => {
                    self.op_cp_r8(self.registers.d);
                    4
                }
                0xBB => {
                    self.op_cp_r8(self.registers.e);
                    4
                }
                0xBC => {
                    self.op_cp_r8(self.registers.h);
                    4
                }
                0xBD => {
                    self.op_cp_r8(self.registers.l);
                    4
                }
                0xBE => {
                    let value = memory_bus.read_byte(self.registers.get_hl());
                    self.op_cp_r8(value);
                    8
                }
                0xBF => {
                    self.op_cp_r8(self.registers.a);
                    4
                }
                0xC0 => {
                    if !self.registers.f.contains(FlagsRegister::ZERO) {
                        self.op_ret(memory_bus);
                        return 20;
                    }
                    8
                }
                0xC1 => {
                    let value = self.op_pop_stack(memory_bus);
                    self.registers.set_bc(value);
                    12
                }
                0xC2 => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    if !self.registers.f.contains(FlagsRegister::ZERO) {
                        self.op_jp_nn(nn);
                        return 16;
                    }
                    12
                }
                0xC3 => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    self.op_jp_nn(nn);
                    16
                }
                0xC4 => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    if !self.registers.f.contains(FlagsRegister::ZERO) {
                        self.op_call_nn(memory_bus, nn);
                        return 24;
                    }
                    12
                }
                0xC5 => {
                    self.op_push_stack(memory_bus, self.registers.get_bc());
                    16
                }
                0xC6 => {
                    self.op_add_d8(memory_bus);
                    8
                }
                0xC7 => {
                    self.op_rst_address(memory_bus, 0x0000);
                    16
                }
                0xC8 => {
                    if self.registers.f.contains(FlagsRegister::ZERO) {
                        self.op_ret(memory_bus);
                        return 20;
                    }
                    8
                }
                0xC9 => {
                    self.op_ret(memory_bus);
                    16
                }
                0xCA => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    if self.registers.f.contains(FlagsRegister::ZERO) {
                        self.op_jp_nn(nn);
                        return 16;
                    }
                    12
                }
                0xCB => {
                    // Get the next byte and use it as the extended opcode
                    let extended_opcode = self.read_immediate_byte(memory_bus);
                    let cb_cycles = 4;
                    match extended_opcode {
                        0x00 => {
                            let mut value = self.registers.b;
                            self.op_rlc(&mut value);
                            self.registers.b = value;
                            cb_cycles + 8
                        }
                        0x01 => {
                            let mut value = self.registers.c;
                            self.op_rlc(&mut value);
                            self.registers.c = value;
                            cb_cycles + 8
                        }
                        0x02 => {
                            let mut value = self.registers.d;
                            self.op_rlc(&mut value);
                            self.registers.d = value;
                            cb_cycles + 8
                        }
                        0x03 => {
                            let mut value = self.registers.e;
                            self.op_rlc(&mut value);
                            self.registers.e = value;
                            cb_cycles + 8
                        }
                        0x04 => {
                            let mut value = self.registers.h;
                            self.op_rlc(&mut value);
                            self.registers.h = value;
                            cb_cycles + 8
                        }
                        0x05 => {
                            let mut value = self.registers.l;
                            self.op_rlc(&mut value);
                            self.registers.l = value;
                            cb_cycles + 8
                        }
                        0x06 => {
                            let mut value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_rlc(&mut value);
                            memory_bus.write_byte(self.registers.get_hl(), value);
                            cb_cycles + 16
                        }
                        0x07 => {
                            let mut value = self.registers.a;
                            self.op_rlc(&mut value);
                            self.registers.a = value;
                            cb_cycles + 8
                        }
                        0x08 => {
                            let mut value = self.registers.b;
                            self.op_rrc(&mut value);
                            self.registers.b = value;
                            cb_cycles + 8
                        }
                        0x09 => {
                            let mut value = self.registers.c;
                            self.op_rrc(&mut value);
                            self.registers.c = value;
                            cb_cycles + 8
                        }
                        0x0A => {
                            let mut value = self.registers.d;
                            self.op_rrc(&mut value);
                            self.registers.d = value;
                            cb_cycles + 8
                        }
                        0x0B => {
                            let mut value = self.registers.e;
                            self.op_rrc(&mut value);
                            self.registers.e = value;
                            cb_cycles + 8
                        }
                        0x0C => {
                            let mut value = self.registers.h;
                            self.op_rrc(&mut value);
                            self.registers.h = value;
                            cb_cycles + 8
                        }
                        0x0D => {
                            let mut value = self.registers.l;
                            self.op_rrc(&mut value);
                            self.registers.l = value;
                            cb_cycles + 8
                        }
                        0x0E => {
                            let mut value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_rrc(&mut value);
                            memory_bus.write_byte(self.registers.get_hl(), value);
                            cb_cycles + 16
                        }
                        0x0F => {
                            let mut value = self.registers.a;
                            self.op_rrc(&mut value);
                            self.registers.a = value;
                            cb_cycles + 8
                        }
                        0x10 => {
                            let mut value = self.registers.b;
                            self.op_rl(&mut value);
                            self.registers.b = value;
                            cb_cycles + 8
                        }
                        0x11 => {
                            let mut value = self.registers.c;
                            self.op_rl(&mut value);
                            self.registers.c = value;
                            cb_cycles + 8
                        }
                        0x12 => {
                            let mut value = self.registers.d;
                            self.op_rl(&mut value);
                            self.registers.d = value;
                            cb_cycles + 8
                        }
                        0x13 => {
                            let mut value = self.registers.e;
                            self.op_rl(&mut value);
                            self.registers.e = value;
                            cb_cycles + 8
                        }
                        0x14 => {
                            let mut value = self.registers.h;
                            self.op_rl(&mut value);
                            self.registers.h = value;
                            cb_cycles + 8
                        }
                        0x15 => {
                            let mut value = self.registers.l;
                            self.op_rl(&mut value);
                            self.registers.l = value;
                            cb_cycles + 8
                        }
                        0x16 => {
                            let mut value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_rl(&mut value);
                            memory_bus.write_byte(self.registers.get_hl(), value);
                            cb_cycles + 16
                        }
                        0x17 => {
                            let mut value = self.registers.a;
                            self.op_rl(&mut value);
                            self.registers.a = value;
                            cb_cycles + 8
                        }
                        0x18 => {
                            let mut value = self.registers.b;
                            self.op_rr(&mut value);
                            self.registers.b = value;
                            cb_cycles + 8
                        }
                        0x19 => {
                            let mut value = self.registers.c;
                            self.op_rr(&mut value);
                            self.registers.c = value;
                            cb_cycles + 8
                        }
                        0x1A => {
                            let mut value = self.registers.d;
                            self.op_rr(&mut value);
                            self.registers.d = value;
                            cb_cycles + 8
                        }
                        0x1B => {
                            let mut value = self.registers.e;
                            self.op_rr(&mut value);
                            self.registers.e = value;
                            cb_cycles + 8
                        }
                        0x1C => {
                            let mut value = self.registers.h;
                            self.op_rr(&mut value);
                            self.registers.h = value;
                            cb_cycles + 8
                        }
                        0x1D => {
                            let mut value = self.registers.l;
                            self.op_rr(&mut value);
                            self.registers.l = value;
                            cb_cycles + 8
                        }
                        0x1E => {
                            let mut value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_rr(&mut value);
                            memory_bus.write_byte(self.registers.get_hl(), value);
                            cb_cycles + 16
                        }
                        0x1F => {
                            let mut value = self.registers.a;
                            self.op_rr(&mut value);
                            self.registers.a = value;
                            cb_cycles + 8
                        }
                        0x20 => {
                            let mut value = self.registers.b;
                            self.op_sla(&mut value);
                            self.registers.b = value;
                            cb_cycles + 8
                        }
                        0x21 => {
                            let mut value = self.registers.c;
                            self.op_sla(&mut value);
                            self.registers.c = value;
                            cb_cycles + 8
                        }
                        0x22 => {
                            let mut value = self.registers.d;
                            self.op_sla(&mut value);
                            self.registers.d = value;
                            cb_cycles + 8
                        }
                        0x23 => {
                            let mut value = self.registers.e;
                            self.op_sla(&mut value);
                            self.registers.e = value;
                            cb_cycles + 8
                        }
                        0x24 => {
                            let mut value = self.registers.h;
                            self.op_sla(&mut value);
                            self.registers.h = value;
                            cb_cycles + 8
                        }
                        0x25 => {
                            let mut value = self.registers.l;
                            self.op_sla(&mut value);
                            self.registers.l = value;
                            cb_cycles + 8
                        }
                        0x26 => {
                            let mut value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_sla(&mut value);
                            memory_bus.write_byte(self.registers.get_hl(), value);
                            cb_cycles + 16
                        }
                        0x27 => {
                            let mut value = self.registers.a;
                            self.op_sla(&mut value);
                            self.registers.a = value;
                            cb_cycles + 8
                        }
                        0x28 => {
                            let mut value = self.registers.b;
                            self.op_sra(&mut value);
                            self.registers.b = value;
                            cb_cycles + 8
                        }
                        0x29 => {
                            let mut value = self.registers.c;
                            self.op_sra(&mut value);
                            self.registers.c = value;
                            cb_cycles + 8
                        }
                        0x2A => {
                            let mut value = self.registers.d;
                            self.op_sra(&mut value);
                            self.registers.d = value;
                            cb_cycles + 8
                        }
                        0x2B => {
                            let mut value = self.registers.e;
                            self.op_sra(&mut value);
                            self.registers.e = value;
                            cb_cycles + 8
                        }
                        0x2C => {
                            let mut value = self.registers.h;
                            self.op_sra(&mut value);
                            self.registers.h = value;
                            cb_cycles + 8
                        }
                        0x2D => {
                            let mut value = self.registers.l;
                            self.op_sra(&mut value);
                            self.registers.l = value;
                            cb_cycles + 8
                        }
                        0x2E => {
                            let mut value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_sra(&mut value);
                            memory_bus.write_byte(self.registers.get_hl(), value);
                            cb_cycles + 16
                        }
                        0x2F => {
                            let mut value = self.registers.a;
                            self.op_sra(&mut value);
                            self.registers.a = value;
                            cb_cycles + 8
                        }
                        0x30 => {
                            let mut value = self.registers.b;
                            self.op_swap(&mut value);
                            self.registers.b = value;
                            cb_cycles + 8
                        }
                        0x31 => {
                            let mut value = self.registers.c;
                            self.op_swap(&mut value);
                            self.registers.c = value;
                            cb_cycles + 8
                        }
                        0x32 => {
                            let mut value = self.registers.d;
                            self.op_swap(&mut value);
                            self.registers.d = value;
                            cb_cycles + 8
                        }
                        0x33 => {
                            let mut value = self.registers.e;
                            self.op_swap(&mut value);
                            self.registers.e = value;
                            cb_cycles + 8
                        }
                        0x34 => {
                            let mut value = self.registers.h;
                            self.op_swap(&mut value);
                            self.registers.h = value;
                            cb_cycles + 8
                        }
                        0x35 => {
                            let mut value = self.registers.l;
                            self.op_swap(&mut value);
                            self.registers.l = value;
                            cb_cycles + 8
                        }
                        0x36 => {
                            let mut value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_swap(&mut value);
                            memory_bus.write_byte(self.registers.get_hl(), value);
                            cb_cycles + 16
                        }
                        0x37 => {
                            let mut value = self.registers.a;
                            self.op_swap(&mut value);
                            self.registers.a = value;
                            cb_cycles + 8
                        }
                        0x38 => {
                            let mut value = self.registers.b;
                            self.op_srl(&mut value);
                            self.registers.b = value;
                            cb_cycles + 8
                        }
                        0x39 => {
                            let mut value = self.registers.c;
                            self.op_srl(&mut value);
                            self.registers.c = value;
                            cb_cycles + 8
                        }
                        0x3A => {
                            let mut value = self.registers.d;
                            self.op_srl(&mut value);
                            self.registers.d = value;
                            cb_cycles + 8
                        }
                        0x3B => {
                            let mut value = self.registers.e;
                            self.op_srl(&mut value);
                            self.registers.e = value;
                            cb_cycles + 8
                        }
                        0x3C => {
                            let mut value = self.registers.h;
                            self.op_srl(&mut value);
                            self.registers.h = value;
                            cb_cycles + 8
                        }
                        0x3D => {
                            let mut value = self.registers.l;
                            self.op_srl(&mut value);
                            self.registers.l = value;
                            cb_cycles + 8
                        }
                        0x3E => {
                            let mut value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_srl(&mut value);
                            memory_bus.write_byte(self.registers.get_hl(), value);
                            cb_cycles + 16
                        }
                        0x3F => {
                            let mut value = self.registers.a;
                            self.op_srl(&mut value);
                            self.registers.a = value;
                            cb_cycles + 8
                        }
                        0x40 => {
                            self.op_bit(0, self.registers.b);
                            cb_cycles + 8
                        }
                        0x41 => {
                            self.op_bit(0, self.registers.c);
                            cb_cycles + 8
                        }
                        0x42 => {
                            self.op_bit(0, self.registers.d);
                            cb_cycles + 8
                        }
                        0x43 => {
                            self.op_bit(0, self.registers.e);
                            cb_cycles + 8
                        }
                        0x44 => {
                            self.op_bit(0, self.registers.h);
                            cb_cycles + 8
                        }
                        0x45 => {
                            self.op_bit(0, self.registers.l);
                            cb_cycles + 8
                        }
                        0x46 => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_bit(0, value);
                            cb_cycles + 16
                        }
                        0x47 => {
                            self.op_bit(0, self.registers.a);
                            cb_cycles + 8
                        }
                        0x48 => {
                            self.op_bit(1, self.registers.b);
                            cb_cycles + 8
                        }
                        0x49 => {
                            self.op_bit(1, self.registers.c);
                            cb_cycles + 8
                        }
                        0x4A => {
                            self.op_bit(1, self.registers.d);
                            cb_cycles + 8
                        }
                        0x4B => {
                            self.op_bit(1, self.registers.e);
                            cb_cycles + 8
                        }
                        0x4C => {
                            self.op_bit(1, self.registers.h);
                            cb_cycles + 8
                        }
                        0x4D => {
                            self.op_bit(1, self.registers.l);
                            cb_cycles + 8
                        }
                        0x4E => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_bit(1, value);
                            cb_cycles + 16
                        }
                        0x4F => {
                            self.op_bit(1, self.registers.a);
                            cb_cycles + 8
                        }
                        0x50 => {
                            self.op_bit(2, self.registers.b);
                            cb_cycles + 8
                        }
                        0x51 => {
                            self.op_bit(2, self.registers.c);
                            cb_cycles + 8
                        }
                        0x52 => {
                            self.op_bit(2, self.registers.d);
                            cb_cycles + 8
                        }
                        0x53 => {
                            self.op_bit(2, self.registers.e);
                            cb_cycles + 8
                        }
                        0x54 => {
                            self.op_bit(2, self.registers.h);
                            cb_cycles + 8
                        }
                        0x55 => {
                            self.op_bit(2, self.registers.l);
                            cb_cycles + 8
                        }
                        0x56 => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_bit(2, value);
                            cb_cycles + 16
                        }
                        0x57 => {
                            self.op_bit(2, self.registers.a);
                            cb_cycles + 8
                        }
                        0x58 => {
                            self.op_bit(3, self.registers.b);
                            cb_cycles + 8
                        }
                        0x59 => {
                            self.op_bit(3, self.registers.c);
                            cb_cycles + 8
                        }
                        0x5A => {
                            self.op_bit(3, self.registers.d);
                            cb_cycles + 8
                        }
                        0x5B => {
                            self.op_bit(3, self.registers.e);
                            cb_cycles + 8
                        }
                        0x5C => {
                            self.op_bit(3, self.registers.h);
                            cb_cycles + 8
                        }
                        0x5D => {
                            self.op_bit(3, self.registers.l);
                            cb_cycles + 8
                        }
                        0x5E => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_bit(3, value);
                            cb_cycles + 16
                        }
                        0x5F => {
                            self.op_bit(3, self.registers.a);
                            cb_cycles + 8
                        }
                        0x60 => {
                            self.op_bit(4, self.registers.b);
                            cb_cycles + 8
                        }
                        0x61 => {
                            self.op_bit(4, self.registers.c);
                            cb_cycles + 8
                        }
                        0x62 => {
                            self.op_bit(4, self.registers.d);
                            cb_cycles + 8
                        }
                        0x63 => {
                            self.op_bit(4, self.registers.e);
                            cb_cycles + 8
                        }
                        0x64 => {
                            self.op_bit(4, self.registers.h);
                            cb_cycles + 8
                        }
                        0x65 => {
                            self.op_bit(4, self.registers.l);
                            cb_cycles + 8
                        }
                        0x66 => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_bit(4, value);

                            cb_cycles + 16
                        }
                        0x67 => {
                            self.op_bit(4, self.registers.a);
                            cb_cycles + 8
                        }
                        0x68 => {
                            self.op_bit(5, self.registers.b);
                            cb_cycles + 8
                        }
                        0x69 => {
                            self.op_bit(5, self.registers.c);
                            cb_cycles + 8
                        }
                        0x6A => {
                            self.op_bit(5, self.registers.d);
                            cb_cycles + 8
                        }
                        0x6B => {
                            self.op_bit(5, self.registers.e);
                            cb_cycles + 8
                        }
                        0x6C => {
                            self.op_bit(5, self.registers.h);
                            cb_cycles + 8
                        }
                        0x6D => {
                            self.op_bit(5, self.registers.l);
                            cb_cycles + 8
                        }
                        0x6E => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_bit(5, value);
                            cb_cycles + 16
                        }
                        0x6F => {
                            self.op_bit(5, self.registers.a);
                            cb_cycles + 8
                        }
                        0x70 => {
                            self.op_bit(6, self.registers.b);
                            cb_cycles + 8
                        }
                        0x71 => {
                            self.op_bit(6, self.registers.c);
                            cb_cycles + 8
                        }
                        0x72 => {
                            self.op_bit(6, self.registers.d);
                            cb_cycles + 8
                        }
                        0x73 => {
                            self.op_bit(6, self.registers.e);
                            cb_cycles + 8
                        }
                        0x74 => {
                            self.op_bit(6, self.registers.h);
                            cb_cycles + 8
                        }
                        0x75 => {
                            self.op_bit(6, self.registers.l);
                            cb_cycles + 8
                        }
                        0x76 => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_bit(6, value);
                            cb_cycles + 16
                        }
                        0x77 => {
                            self.op_bit(6, self.registers.a);
                            cb_cycles + 8
                        }
                        0x78 => {
                            self.op_bit(7, self.registers.b);
                            cb_cycles + 8
                        }
                        0x79 => {
                            self.op_bit(7, self.registers.c);
                            cb_cycles + 8
                        }
                        0x7A => {
                            self.op_bit(7, self.registers.d);
                            cb_cycles + 8
                        }
                        0x7B => {
                            self.op_bit(7, self.registers.e);
                            cb_cycles + 8
                        }
                        0x7C => {
                            self.op_bit(7, self.registers.h);
                            cb_cycles + 8
                        }
                        0x7D => {
                            self.op_bit(7, self.registers.l);
                            cb_cycles + 8
                        }
                        0x7E => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            self.op_bit(7, value);
                            cb_cycles + 16
                        }
                        0x7F => {
                            self.op_bit(7, self.registers.a);
                            cb_cycles + 8
                        }
                        0x80 => {
                            self.registers.b &= !(1 << 0);
                            cb_cycles + 8
                        }
                        0x81 => {
                            self.registers.c &= !(1 << 0);
                            cb_cycles + 8
                        }
                        0x82 => {
                            self.registers.d &= !(1 << 0);
                            cb_cycles + 8
                        }
                        0x83 => {
                            self.registers.e &= !(1 << 0);
                            cb_cycles + 8
                        }
                        0x84 => {
                            self.registers.h &= !(1 << 0);
                            cb_cycles + 8
                        }
                        0x85 => {
                            self.registers.l &= !(1 << 0);
                            cb_cycles + 8
                        }
                        0x86 => {
                            let hl = self.registers.get_hl();
                            let value = memory_bus.read_byte(hl);
                            let result = value & !(1 << 0);
                            memory_bus.write_byte(hl, result);
                            cb_cycles + 8
                        }
                        0x87 => {
                            self.registers.a &= !(1 << 0);
                            cb_cycles + 8
                        }
                        0x88 => {
                            self.registers.b &= !(1 << 1);
                            cb_cycles + 8
                        }
                        0x89 => {
                            self.registers.c &= !(1 << 1);
                            cb_cycles + 8
                        }
                        0x8A => {
                            self.registers.d &= !(1 << 1);
                            cb_cycles + 8
                        }
                        0x8B => {
                            self.registers.e &= !(1 << 1);
                            cb_cycles + 8
                        }
                        0x8C => {
                            self.registers.h &= !(1 << 1);
                            cb_cycles + 8
                        }
                        0x8D => {
                            self.registers.l &= !(1 << 1);
                            cb_cycles + 8
                        }
                        0x8E => {
                            let hl = self.registers.get_hl();
                            let value = memory_bus.read_byte(hl);
                            let result = value & !(1 << 1);
                            memory_bus.write_byte(hl, result);
                            cb_cycles + 8
                        }
                        0x8F => {
                            self.registers.a &= !(1 << 1);
                            cb_cycles + 8
                        }
                        0x90 => {
                            self.registers.b &= !(1 << 2);
                            cb_cycles + 8
                        }
                        0x91 => {
                            self.registers.c &= !(1 << 2);
                            cb_cycles + 8
                        }
                        0x92 => {
                            self.registers.d &= !(1 << 2);
                            cb_cycles + 8
                        }
                        0x93 => {
                            self.registers.e &= !(1 << 2);
                            cb_cycles + 8
                        }
                        0x94 => {
                            self.registers.h &= !(1 << 2);
                            cb_cycles + 8
                        }
                        0x95 => {
                            self.registers.l &= !(1 << 2);
                            cb_cycles + 8
                        }
                        0x96 => {
                            let hl = self.registers.get_hl();
                            let value = memory_bus.read_byte(hl);
                            let result = value & !(1 << 2);
                            memory_bus.write_byte(hl, result);
                            cb_cycles + 8
                        }
                        0x97 => {
                            self.registers.a &= !(1 << 2);
                            cb_cycles + 8
                        }
                        0x98 => {
                            self.registers.b &= !(1 << 3);
                            cb_cycles + 8
                        }
                        0x99 => {
                            self.registers.c &= !(1 << 3);
                            cb_cycles + 8
                        }
                        0x9A => {
                            self.registers.d &= !(1 << 3);
                            cb_cycles + 8
                        }
                        0x9B => {
                            self.registers.e &= !(1 << 3);
                            cb_cycles + 8
                        }
                        0x9C => {
                            self.registers.h &= !(1 << 3);
                            cb_cycles + 8
                        }
                        0x9D => {
                            self.registers.l &= !(1 << 3);
                            cb_cycles + 8
                        }
                        0x9E => {
                            let hl = self.registers.get_hl();
                            let value = memory_bus.read_byte(hl);
                            let result = value & !(1 << 3);
                            memory_bus.write_byte(hl, result);
                            cb_cycles + 16
                        }
                        0x9F => {
                            self.registers.a &= !(1 << 3);
                            cb_cycles + 8
                        }
                        0xA0 => {
                            self.registers.b &= !(1 << 4);
                            cb_cycles + 8
                        }
                        0xA1 => {
                            self.registers.c &= !(1 << 4);
                            cb_cycles + 8
                        }
                        0xA2 => {
                            self.registers.d &= !(1 << 4);
                            cb_cycles + 8
                        }
                        0xA3 => {
                            self.registers.e &= !(1 << 4);
                            cb_cycles + 8
                        }
                        0xA4 => {
                            self.registers.h &= !(1 << 4);
                            cb_cycles + 8
                        }
                        0xA5 => {
                            self.registers.l &= !(1 << 4);
                            cb_cycles + 8
                        }
                        0xA6 => {
                            let hl = self.registers.get_hl();
                            let value = memory_bus.read_byte(hl);
                            let result = value & !(1 << 4);
                            memory_bus.write_byte(hl, result);
                            cb_cycles + 8
                        }
                        0xA7 => {
                            self.registers.a &= !(1 << 4);
                            cb_cycles + 8
                        }
                        0xA8 => {
                            self.registers.b &= !(1 << 5);
                            cb_cycles + 8
                        }
                        0xA9 => {
                            self.registers.c &= !(1 << 5);
                            cb_cycles + 8
                        }
                        0xAA => {
                            self.registers.d &= !(1 << 5);
                            cb_cycles + 8
                        }
                        0xAB => {
                            self.registers.e &= !(1 << 5);
                            cb_cycles + 8
                        }
                        0xAC => {
                            self.registers.h &= !(1 << 5);
                            cb_cycles + 8
                        }
                        0xAD => {
                            self.registers.l &= !(1 << 5);
                            cb_cycles + 8
                        }
                        0xAE => {
                            let hl = self.registers.get_hl();
                            let value = memory_bus.read_byte(hl);
                            let result = value & !(1 << 5);
                            memory_bus.write_byte(hl, result);
                            cb_cycles + 16
                        }
                        0xAF => {
                            self.registers.a &= !(1 << 5);
                            cb_cycles + 8
                        }
                        0xB0 => {
                            self.registers.b &= !(1 << 6);
                            cb_cycles + 8
                        }
                        0xB1 => {
                            self.registers.c &= !(1 << 6);
                            cb_cycles + 8
                        }
                        0xB2 => {
                            self.registers.d &= !(1 << 6);
                            cb_cycles + 8
                        }
                        0xB3 => {
                            self.registers.e &= !(1 << 6);
                            cb_cycles + 8
                        }
                        0xB4 => {
                            self.registers.h &= !(1 << 6);
                            cb_cycles + 8
                        }
                        0xB5 => {
                            self.registers.l &= !(1 << 6);
                            cb_cycles + 8
                        }
                        0xB6 => {
                            let hl = self.registers.get_hl();
                            let value = memory_bus.read_byte(hl);
                            let result = value & !(1 << 6);
                            memory_bus.write_byte(hl, result);
                            cb_cycles + 16
                        }
                        0xB7 => {
                            self.registers.a &= !(1 << 6);
                            cb_cycles + 8
                        }
                        0xB8 => {
                            self.registers.b &= !(1 << 7);
                            cb_cycles + 8
                        }
                        0xB9 => {
                            self.registers.c &= !(1 << 7);
                            cb_cycles + 8
                        }
                        0xBA => {
                            self.registers.d &= !(1 << 7);
                            cb_cycles + 8
                        }
                        0xBB => {
                            self.registers.e &= !(1 << 7);
                            cb_cycles + 8
                        }
                        0xBC => {
                            self.registers.h &= !(1 << 7);
                            cb_cycles + 8
                        }
                        0xBD => {
                            self.registers.l &= !(1 << 7);
                            cb_cycles + 8
                        }
                        0xBE => {
                            let hl = self.registers.get_hl();
                            let value = memory_bus.read_byte(hl);
                            let result = value & !(1 << 7);
                            memory_bus.write_byte(hl, result);
                            cb_cycles + 16
                        }
                        0xBF => {
                            self.registers.a &= !(1 << 7);
                            cb_cycles + 8
                        }
                        0xC0 => {
                            self.registers.b |= 1 << 0;
                            cb_cycles + 8
                        }
                        0xC1 => {
                            self.registers.c |= 1 << 0;
                            cb_cycles + 8
                        }
                        0xC2 => {
                            self.registers.d |= 1 << 0;
                            cb_cycles + 8
                        }
                        0xC3 => {
                            self.registers.e |= 1 << 0;
                            cb_cycles + 8
                        }
                        0xC4 => {
                            self.registers.h |= 1 << 0;
                            cb_cycles + 8
                        }
                        0xC5 => {
                            self.registers.l |= 1 << 0;
                            cb_cycles + 8
                        }
                        0xC6 => {
                            let hl = self.registers.get_hl();
                            let value = memory_bus.read_byte(hl);
                            let result = value | (1 << 0);
                            memory_bus.write_byte(hl, result);
                            cb_cycles + 16
                        }
                        0xC7 => {
                            self.registers.a |= 1 << 0;
                            cb_cycles + 8
                        }
                        0xC8 => {
                            self.registers.b |= 1 << 1;
                            cb_cycles + 8
                        }
                        0xC9 => {
                            self.registers.c |= 1 << 1;
                            cb_cycles + 8
                        }
                        0xCA => {
                            self.registers.d |= 1 << 1;
                            cb_cycles + 8
                        }
                        0xCB => {
                            self.registers.e |= 1 << 1;
                            cb_cycles + 8
                        }
                        0xCC => {
                            self.registers.h |= 1 << 1;
                            cb_cycles + 8
                        }
                        0xCD => {
                            self.registers.l |= 1 << 1;
                            cb_cycles + 8
                        }
                        0xCE => {
                            let hl = self.registers.get_hl();
                            let value = memory_bus.read_byte(hl);
                            let result = value | (1 << 1);
                            memory_bus.write_byte(hl, result);
                            cb_cycles + 16
                        }
                        0xCF => {
                            self.registers.a |= 1 << 1;
                            cb_cycles + 8
                        }
                        0xD0 => {
                            self.registers.b |= 1 << 2;
                            cb_cycles + 8
                        }
                        0xD1 => {
                            self.registers.c |= 1 << 2;
                            cb_cycles + 8
                        }
                        0xD2 => {
                            self.registers.d |= 1 << 2;
                            cb_cycles + 8
                        }
                        0xD3 => {
                            self.registers.e |= 1 << 2;
                            cb_cycles + 8
                        }
                        0xD4 => {
                            self.registers.h |= 1 << 2;
                            cb_cycles + 8
                        }
                        0xD5 => {
                            self.registers.l |= 1 << 2;
                            cb_cycles + 8
                        }
                        0xD6 => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            let mut result = memory_bus.read_byte(self.registers.get_hl());
                            result |= value | 1 << 2;
                            memory_bus.write_byte(self.registers.get_hl(), result);
                            cb_cycles + 16
                        }
                        0xD7 => {
                            self.registers.a |= 1 << 2;
                            cb_cycles + 8
                        }
                        0xD8 => {
                            self.registers.b |= 1 << 3;
                            cb_cycles + 8
                        }
                        0xD9 => {
                            self.registers.c |= 1 << 3;
                            cb_cycles + 8
                        }
                        0xDA => {
                            self.registers.d |= 1 << 3;
                            cb_cycles + 8
                        }
                        0xDB => {
                            self.registers.e |= 1 << 3;
                            cb_cycles + 8
                        }
                        0xDC => {
                            self.registers.h |= 1 << 3;
                            cb_cycles + 8
                        }
                        0xDD => {
                            self.registers.l |= 1 << 3;
                            cb_cycles + 8
                        }
                        0xDE => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            let mut result = memory_bus.read_byte(self.registers.get_hl());
                            result |= value | 1 << 3;
                            memory_bus.write_byte(self.registers.get_hl(), result);
                            cb_cycles + 16
                        }
                        0xDF => {
                            self.registers.a |= 1 << 3;
                            cb_cycles + 8
                        }
                        0xE0 => {
                            self.registers.b |= 1 << 4;
                            cb_cycles + 8
                        }
                        0xE1 => {
                            self.registers.c |= 1 << 4;
                            cb_cycles + 8
                        }
                        0xE2 => {
                            self.registers.d |= 1 << 4;
                            cb_cycles + 8
                        }
                        0xE3 => {
                            self.registers.e |= 1 << 4;
                            cb_cycles + 8
                        }
                        0xE4 => {
                            self.registers.h |= 1 << 4;
                            cb_cycles + 8
                        }
                        0xE5 => {
                            self.registers.l |= 1 << 4;
                            cb_cycles + 8
                        }
                        0xE6 => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            let mut result = memory_bus.read_byte(self.registers.get_hl());
                            result |= value | 1 << 4;
                            memory_bus.write_byte(self.registers.get_hl(), result);
                            cb_cycles + 8
                        }
                        0xE7 => {
                            self.registers.a |= 1 << 4;
                            cb_cycles + 8
                        }
                        0xE8 => {
                            self.registers.b |= 1 << 5;
                            cb_cycles + 8
                        }
                        0xE9 => {
                            self.registers.c |= 1 << 5;
                            cb_cycles + 8
                        }
                        0xEA => {
                            self.registers.d |= 1 << 5;
                            cb_cycles + 8
                        }
                        0xEB => {
                            self.registers.e |= 1 << 5;
                            cb_cycles + 8
                        }
                        0xEC => {
                            self.registers.h |= 1 << 5;
                            cb_cycles + 8
                        }
                        0xED => {
                            self.registers.l |= 1 << 5;
                            cb_cycles + 8
                        }
                        0xEE => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            let mut result = memory_bus.read_byte(self.registers.get_hl());
                            result |= value | 1 << 5;
                            memory_bus.write_byte(self.registers.get_hl(), result);
                            cb_cycles + 16
                        }
                        0xEF => {
                            self.registers.a |= 1 << 5;
                            cb_cycles + 8
                        }
                        0xF0 => {
                            self.registers.b |= 1 << 6;
                            cb_cycles + 8
                        }
                        0xF1 => {
                            self.registers.c |= 1 << 6;
                            cb_cycles + 8
                        }
                        0xF2 => {
                            self.registers.d |= 1 << 6;
                            cb_cycles + 8
                        }
                        0xF3 => {
                            self.registers.e |= 1 << 6;
                            cb_cycles + 8
                        }
                        0xF4 => {
                            self.registers.h |= 1 << 6;
                            cb_cycles + 8
                        }
                        0xF5 => {
                            self.registers.l |= 1 << 6;
                            cb_cycles + 8
                        }
                        0xF6 => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            let mut result = memory_bus.read_byte(self.registers.get_hl());
                            result |= value | 1 << 6;
                            memory_bus.write_byte(self.registers.get_hl(), result);
                            cb_cycles + 16
                        }
                        0xF7 => {
                            self.registers.a |= 1 << 6;
                            cb_cycles + 8
                        }
                        0xF8 => {
                            self.registers.b |= 1 << 7;
                            cb_cycles + 8
                        }
                        0xF9 => {
                            self.registers.c |= 1 << 7;
                            cb_cycles + 8
                        }
                        0xFA => {
                            self.registers.d |= 1 << 7;
                            cb_cycles + 8
                        }
                        0xFB => {
                            self.registers.e |= 1 << 7;
                            cb_cycles + 8
                        }
                        0xFC => {
                            self.registers.h |= 1 << 7;
                            cb_cycles + 8
                        }
                        0xFD => {
                            self.registers.l |= 1 << 7;
                            cb_cycles + 8
                        }
                        0xFE => {
                            let value = memory_bus.read_byte(self.registers.get_hl());
                            let mut result = memory_bus.read_byte(self.registers.get_hl());
                            result |= value | 1 << 7;
                            memory_bus.write_byte(self.registers.get_hl(), result);
                            cb_cycles + 16
                        }
                        0xFF => {
                            self.registers.a |= 1 << 7;
                            cb_cycles + 8
                        }
                    }
                }
                0xCC => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    if self.registers.f.contains(FlagsRegister::ZERO) {
                        self.op_call_nn(memory_bus, nn);
                        return 24;
                    }
                    12
                }
                0xCD => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    self.op_call_nn(memory_bus, nn);
                    24
                }
                0xCE => {
                    let value = self.read_immediate_byte(memory_bus);
                    self.op_adc_r8(value);
                    8
                }
                0xCF => {
                    self.op_rst_address(memory_bus, 0x08);
                    16
                }
                0xD0 => {
                    if !self.registers.f.contains(FlagsRegister::CARRY) {
                        self.op_ret(memory_bus);
                        return 20;
                    }
                    8
                }
                0xD1 => {
                    let value = self.op_pop_stack(memory_bus);
                    self.registers.set_de(value);
                    12
                }
                0xD2 => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    if !self.registers.f.contains(FlagsRegister::CARRY) {
                        self.op_jp_nn(nn);
                        return 16;
                    }
                    12
                }
                0xD3 => {
                    error!("Unsupported opcode: 0xD3");
                    4
                }
                0xD4 => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    if !self.registers.f.contains(FlagsRegister::CARRY) {
                        self.op_call_nn(memory_bus, nn);
                        return 24;
                    }
                    12
                }
                0xD5 => {
                    self.op_push_stack(memory_bus, self.registers.get_de());
                    16
                }
                0xD6 => {
                    self.op_sub_d8(memory_bus);
                    8
                }
                0xD7 => {
                    self.op_rst_address(memory_bus, 0x10);
                    16
                }
                0xD8 => {
                    if self.registers.f.contains(FlagsRegister::CARRY) {
                        self.op_ret(memory_bus);
                        return 20;
                    }
                    8
                }
                0xD9 => {
                    self.op_ret(memory_bus);
                    self.op_ei(memory_bus);
                    16
                }
                0xDA => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    if self.registers.f.contains(FlagsRegister::CARRY) {
                        self.op_jp_nn(nn);
                        return 16;
                    }
                    12
                }
                0xDB => {
                    error!("Unsupported opcode: 0xDB");
                    4
                }
                0xDC => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    if self.registers.f.contains(FlagsRegister::CARRY) {
                        self.op_call_nn(memory_bus, nn);
                        return 24;
                    }
                    12
                }
                0xDD => {
                    error!("Unsupported opcode: 0xDD");
                    4
                }
                0xDE => {
                    let value = self.read_immediate_byte(memory_bus);
                    self.op_sbc_r8(value);
                    8
                }
                0xDF => {
                    self.op_rst_address(memory_bus, 0x18);
                    16
                }
                0xE0 => {
                    let offset = self.read_immediate_byte(memory_bus) as u16;
                    let address = 0xFF00 + offset;
                    memory_bus.write_byte(address, self.registers.a);
                    12
                }
                0xE1 => {
                    let value = self.op_pop_stack(memory_bus);
                    self.registers.set_hl(value);
                    12
                }
                0xE2 => {
                    let address = 0xFF00 | self.registers.c as u16;
                    memory_bus.write_byte(address, self.registers.a);
                    8
                }
                0xE3 => {
                    error!("Unsupported opcode: 0xE3");
                    4
                }
                0xE4 => {
                    error!("Unsupported opcode: 0xE4");
                    4
                }
                0xE5 => {
                    self.op_push_stack(memory_bus, self.registers.get_hl());
                    16
                }
                0xE6 => {
                    self.op_and_d8(memory_bus);
                    8
                }
                0xE7 => {
                    self.op_rst_address(memory_bus, 0x20);
                    16
                }
                0xE8 => {
                    self.op_add_sp_d8(memory_bus);
                    16
                }
                0xE9 => {
                    self.pc = self.registers.get_hl();
                    4
                }
                0xEA => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    memory_bus.write_byte(nn, self.registers.a);
                    16
                }
                0xEB => {
                    error!("Unsupported opcode: 0xEB");
                    4
                }
                0xEC => {
                    error!("Unsupported opcode: 0xEC");
                    4
                }
                0xED => {
                    error!("Unsupported opcode: 0xED");
                    4
                }
                0xEE => {
                    let value = self.read_immediate_byte(memory_bus);
                    self.op_xor_r8(value);
                    8
                }
                0xEF => {
                    self.op_rst_address(memory_bus, 0x28);
                    16
                }
                0xF0 => {
                    let value = self.read_immediate_byte(memory_bus);
                    let address = 0xFF00 + value as u16;
                    self.registers.a = memory_bus.read_byte(address);
                    12
                }
                0xF1 => {
                    let value = self.op_pop_stack(memory_bus);
                    self.registers.set_af(value);
                    12
                }
                0xF2 => {
                    let address = 0xFF00 | self.registers.c as u16;
                    self.registers.a = memory_bus.read_byte(address);
                    8
                }
                0xF3 => {
                    self.op_di(memory_bus);
                    4
                }
                0xF4 => {
                    error!("Unsupported opcode: 0xF4");
                    4
                }
                0xF5 => {
                    self.op_push_stack(memory_bus, self.registers.get_af());
                    16
                }
                0xF6 => {
                    self.op_or_d8(memory_bus);
                    8
                }
                0xF7 => {
                    self.op_rst_address(memory_bus, 0x30);
                    16
                }
                0xF8 => {
                    let value = self.read_immediate_byte(memory_bus) as i8;
                    let sp = self.sp;
                    let result = sp.wrapping_add(value as i16 as u16);
                    self.registers.set_hl(result);
                    self.registers.f.set(FlagsRegister::ZERO, false);
                    self.registers.f.set(FlagsRegister::SUBTRACT, false);
                    self.registers.f.set(FlagsRegister::HALF_CARRY, (sp & 0xF) + (value as u16 & 0xF) > 0xF);
                    self.registers.f.set(FlagsRegister::CARRY, (sp & 0xFF) + (value as u16 & 0xFF) > 0xFF);
                    12
                }
                0xF9 => {
                    self.sp = self.registers.get_hl();
                    8
                }
                0xFA => {
                    let nn: u16 = self.read_immediate_short(memory_bus);
                    self.registers.a = memory_bus.read_byte(nn);
                    16
                }
                0xFB => {
                    self.op_ei(memory_bus);
                    4
                }
                0xFC => {
                    error!("Unsupported opcode: 0xFC");
                    4
                }
                0xFD => {
                    error!("Unsupported opcode: 0xFD");
                    4
                }
                0xFE => {
                    let value = self.read_immediate_byte(memory_bus);
                    self.op_cp_r8(value);
                    8
                }
                0xFF => {
                    self.op_rst_address(memory_bus, 0x0038);
                    16
                }
            }
        }
        else {
            if u8::from(memory_bus.interrupt_flags) != 0 {
                self.halted = false;
            }
            4
        }

    }

    /*
     *   NOP
     *   No operation.
     */
    fn op_nop(&mut self) {}

    /*
     *   Read the immediate 16-bit value from memory for the current program counter and program counter + 1.
     */
    fn read_immediate_short(&mut self, memory_bus: &mut MemoryBus) -> u16 {
        let lsb = memory_bus.read_byte(self.pc);
        self.pc = self.pc.wrapping_add(1);
        let msb = memory_bus.read_byte(self.pc);
        self.pc = self.pc.wrapping_add(1);
        (msb as u16) << 8 | lsb as u16
    }

    /*
     *   Read the immediate 8-bit value from memory for the current program counter.
     */
    fn read_immediate_byte(&mut self, memory_bus: &mut MemoryBus) -> u8 {
        let value = memory_bus.read_byte(self.pc);
        self.pc = self.pc.wrapping_add(1);
        value
    }

    /*
     *   Write the immediate 16-bit value to memory.
     */
    fn write_immediate_short(&mut self, memory_bus: &mut MemoryBus, address: u16, value: u16) {
        let lsb = (value & 0x00FF) as u8;
        let msb = (value >> 8) as u8;
        memory_bus.write_byte(address, lsb);
        memory_bus.write_byte(address.wrapping_add(1), msb);
    }

    /*
    * 8-bit arithmetic and logical operations
    */
    
    /*
    *   Increment register value by 1 and set flags accordingly.
    */
    fn op_inc_r8(&mut self, register: u8) -> u8{
        let result = register.wrapping_add(1);
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, (register & LSB_MASK) + 1 > LSB_MASK);
        result
    }

    /*
    *   Decrement register value by 1 and set flags accordingly.
    */
    fn op_dec_r8(&mut self, register: u8) -> u8{
        let result = register.wrapping_sub(1);
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, true);
        self.registers.f.set(FlagsRegister::HALF_CARRY, (register & LSB_MASK) < 1);
        result
    }

    /*
    *   Add value to register A and set flags accordingly.
    */
    fn op_add_r8(&mut self, value: u8) {
        let result: u8 = self.registers.a.wrapping_add(value);
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers
            .f
            .set(FlagsRegister::HALF_CARRY, (self.registers.a & LSB_MASK) + (value & LSB_MASK) > LSB_MASK);
        self.registers
            .f
            .set(FlagsRegister::CARRY,(self.registers.a as u16) + (value as u16) > 0xFF);
        self.registers.a = result;
    }

    /*
    *   Add immediate 8-bit value to register A and set flags accordingly.
    */
    fn op_add_d8(&mut self, memory_bus: &mut MemoryBus) {
        let value = self.read_immediate_byte(memory_bus);
        let result: u8 = self.registers.a.wrapping_add(value);
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers
            .f
            .set(FlagsRegister::HALF_CARRY,(self.registers.a & LSB_MASK) + (value & LSB_MASK) > LSB_MASK);
        self.registers
            .f
            .set(FlagsRegister::CARRY, (self.registers.a as u16) + (value as u16) > 0xFF);
        self.registers.a = result;
    }

    /*
    *   Subtract value from register A and set flags accordingly.
    */
    fn op_sub_r8(&mut self, value: u8) {
        let result: u8 = self.registers.a.wrapping_sub(value);
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, true);
        self.registers
            .f
            .set(FlagsRegister::HALF_CARRY, (self.registers.a & LSB_MASK) < (value & LSB_MASK));
        self.registers
            .f
            .set(FlagsRegister::CARRY, self.registers.a < value);
        self.registers.a = result;
    }
    
    /*
    *   Subtract immediate 8-bit value from register A and set flags accordingly.
    */
    fn op_sub_d8(&mut self, memory_bus: &mut MemoryBus) {
        let value = self.read_immediate_byte(memory_bus);
        let result: u8 = self.registers.a.wrapping_sub(value);
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, true);
        self.registers
            .f
            .set(FlagsRegister::HALF_CARRY, (self.registers.a & LSB_MASK) < (value & LSB_MASK));
        self.registers
            .f
            .set(FlagsRegister::CARRY, self.registers.a < value);
        self.registers.a = result;
    }

    /*
    *   Logical OR between register A and value, store result in register A and set flags accordingly.
    */
    fn op_or_r8(&mut self, value: u8) {
        let result: u8 = self.registers.a | value;
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, false);
        self.registers.a = result;
    }
    
    /*
    *   Logical OR between register A and immediate 8-bit value, store result in register A and set flags accordingly.
    */
    fn op_or_d8(&mut self, memory_bus: &mut MemoryBus) {
        let value = self.read_immediate_byte(memory_bus);
        let result: u8 = self.registers.a | value;
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, false);
        self.registers.a = result;
    }

    /*
    *   Logical AND between register A and value, store result in register A and set flags accordingly.
    */
    fn op_and_r8(&mut self, value: u8) {
        let result: u8 = self.registers.a & value;
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, true);
        self.registers.f.set(FlagsRegister::CARRY, false);
        self.registers.a = result;
    }
    
    /*
    *   Logical AND between register A and immediate 8-bit value, store result in register A and set flags accordingly.
    */
    fn op_and_d8(&mut self, memory_bus: &mut MemoryBus) {
        let value = self.read_immediate_byte(memory_bus);
        let result: u8 = self.registers.a & value;
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, true);
        self.registers.f.set(FlagsRegister::CARRY, false);
        self.registers.a = result;
    }

    /*
    *   Compare value with register A and set flags accordingly (A - value).
    *   A register is unaffected.
    */
    fn op_cp_r8(&mut self, value: u8) {
        let result: u8 = self.registers.a.wrapping_sub(value);
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, true);
        self.registers.f.set(FlagsRegister::HALF_CARRY, (self.registers.a & LSB_MASK) < (value & LSB_MASK));
        self.registers.f.set(FlagsRegister::CARRY, self.registers.a < value);
    }

    /*
    *   Logical XOR between register A and value, store result in register A and set flags accordingly.
    */
    fn op_xor_r8(&mut self, value: u8) {
        self.registers.a ^= value;
        self.registers.f.set(FlagsRegister::ZERO, self.registers.a == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, false);
    }

    /*
    *   Add value and carry flag to register A and set flags accordingly.
    */
    fn op_adc_r8(&mut self, value: u8) {
        let carry = if self.registers.f.contains(FlagsRegister::CARRY) {
            1
        } else {
            0
        } as u8;
        let result = self.registers.a.wrapping_add(value).wrapping_add(carry);
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY,
            (self.registers.a & LSB_MASK) + (value & LSB_MASK) + carry > 0x0F,
        );
        self.registers.f.set(FlagsRegister::CARRY,
            (self.registers.a as u16) + (value as u16) + (carry as u16) > 0xFF,
        );
        self.registers.a = result;
    }

    
    /*
    *   Complement all bits in register A and set flags accordingly.
    */
    fn op_cpl(&mut self) {
        self.registers.a = !self.registers.a;
        self.registers.f.set(FlagsRegister::SUBTRACT, true);
        self.registers.f.set(FlagsRegister::HALF_CARRY, true);
    }

    /*
    *   Complement carry flag.
    */
    fn op_ccf(&mut self) {
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, !self.registers.f.contains(FlagsRegister::CARRY));
    }

    /*
    *   Set carry flag.
    */
    fn op_scf(&mut self) {
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, true);
    }

    /*
    *   Decimal Adjust for Addition (DAA)
    */
    fn op_daa(&mut self) {
        let mut a = self.registers.a;
        if !self.registers.f.contains(FlagsRegister::SUBTRACT) {
            if self.registers.f.contains(FlagsRegister::CARRY) || a > 0x99 {
                a = a.wrapping_add(0x60);
                self.registers.f.set(FlagsRegister::CARRY, true);
            }
            if self.registers.f.contains(FlagsRegister::HALF_CARRY)|| (a & LSB_MASK) > 0x09 {
                a = a.wrapping_add(0x06);
            }
        } else {
            if self.registers.f.contains(FlagsRegister::CARRY) {
                a = a.wrapping_sub(0x60);
            }
            if self.registers.f.contains(FlagsRegister::HALF_CARRY){
                a = a.wrapping_sub(0x06);
            }
        }
        self.registers.f.set(FlagsRegister::ZERO, a == 0);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.a = a;
    }

    /*
    *   Subtract value and carry flag from register A and set flags accordingly.
    */
    fn op_sbc_r8(&mut self, r: u8) {
        let carry = if self.registers.f.contains(FlagsRegister::CARRY) { 1 } else { 0 } as u8;
        let result = self.registers.a.wrapping_sub(r).wrapping_sub(carry);
        self.registers.f.set(FlagsRegister::ZERO, result == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, true);
        // Check for half carry by comparing lower nibbles before subtraction
        self.registers.f.set(FlagsRegister::HALF_CARRY, (self.registers.a & LSB_MASK) < (r & LSB_MASK) + carry);
        self.registers.f.set(FlagsRegister::CARRY, (self.registers.a as u16) < (r as u16) + (carry as u16));
        self.registers.a = result;
    }

    /*
     *   16-bit arithmetic operations
     */
    /*
     *   Increment the 16-bit value at the memory address pointed to by the HL register.
    */
    fn op_inc_hl(&mut self, memory_bus: &mut MemoryBus) {
        let address = self.registers.get_hl();
        let mut value = memory_bus.read_byte(address);
        value = value.wrapping_add(1);
        memory_bus.write_byte(address, value);
        self.registers.f.set(FlagsRegister::ZERO, value == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, (value & LSB_MASK) == 0x00);
    }

    
    /*
    *   Decrement the 16-bit value at the memory address pointed to by the HL register.
    */
    fn op_dec_hl(&mut self, memory_bus: &mut MemoryBus) {
        let address = self.registers.get_hl();
        let mut value = memory_bus.read_byte(address);
        value = value.wrapping_sub(1);
        memory_bus.write_byte(address, value);
        self.registers.f.set(FlagsRegister::ZERO, value == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, true);
        self.registers.f.set(FlagsRegister::HALF_CARRY, (value & LSB_MASK) == 0x0F);
    }

    /*
    *   Add value to 16-bit register and set flags accordingly.
    */
    fn op_add_r16(&mut self, register: u16, value: u16) -> u16{
        let result = register.wrapping_add(value);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, (register & 0x0FFF) + (value & 0x0FFF) > 0x0FFF);
        self.registers.f.set(FlagsRegister::CARRY, (register as u32) + (value as u32) > 0xFFFF);
        result
    }
    
    /*
    *   Add signed immediate 8-bit value to stack pointer and set flags accordingly.
    */
    fn op_add_sp_d8(&mut self, memory_bus: &mut MemoryBus) {
        let value = self.read_immediate_byte(memory_bus) as i8;
        let sp = self.sp as i16;
        let result = sp.wrapping_add(value as i16);
        self.sp = result as u16;
        let half_carry = ((sp & 0x0F) + (value as i16 & LSB_MASK as i16)) & 0x10 != 0;
        self.registers.f.set(FlagsRegister::HALF_CARRY, half_carry);
        let carry = (sp & 0xFF) + (value as i16 & 0xFF) > 0xFF;
        self.registers.f.set(FlagsRegister::CARRY, carry);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::ZERO, false);
    }

    /*
     *   JP nn
     *   Unconditional jump to the absolute address specified by the 16-bit operand nn.
     */
    fn op_jp_nn(&mut self, address: u16) {
        self.pc = address;
    }

    /*
     *   JR e
     *   Unconditional jump to the relative address specified by the signed 8-bit operand e.
     */
    fn op_jr_e(&mut self, offset: i8) {
        let new_pc = self.pc.wrapping_add(offset as u16);
        self.pc = new_pc;
    }

    /*
     *   CALL nn
     *   Unconditional function call to the absolute address specified by the 16-bit operand nn.
     */
    fn op_call_nn(&mut self, memory_bus: &mut MemoryBus, address: u16) {
        self.op_push_stack(memory_bus, self.pc);
        self.pc = address;
    }

    /*
     *   RET
     *   Unconditional return from a function.
     */
    fn op_ret(&mut self, memory_bus: &mut MemoryBus) {
        let address = self.op_pop_stack(memory_bus);
        self.pc = address;
    }
   
    /*
    * Checks and services interrupts if they are enabled and requested.
    */
    pub fn check_interrupts(&mut self, memory_bus: &mut MemoryBus) {
        // Check if interrupts are scheduled to be enabled
        if memory_bus.enabling_ime {
            memory_bus.interrupt_master_enable = true;
            memory_bus.enabling_ime = false;
        }

        if memory_bus.interrupt_master_enable {
            let interrupt_flags: InterruptFlags = memory_bus.interrupt_flags.into();
            let interrupt_enable_register = memory_bus.interrupt_enable_register;

            // Check if the interrupt is both flagged and enabled
            if interrupt_flags.vblank && (interrupt_enable_register & VBLANK_MASK) != 0 {
                self.service_interrupt(memory_bus, Interrupt::VBLANK);
            } else if interrupt_flags.lcd_stat && (interrupt_enable_register & LCDSTAT_MASK) != 0 {
                self.service_interrupt(memory_bus, Interrupt::LCDSTAT);
            } else if interrupt_flags.timer && (interrupt_enable_register & TIMER_MASK) != 0 {
                self.service_interrupt(memory_bus, Interrupt::TIMER);
            } else if interrupt_flags.serial && (interrupt_enable_register & SERIAL_MASK) != 0 {
                self.service_interrupt(memory_bus, Interrupt::SERIAL);
            } else if interrupt_flags.joypad && (interrupt_enable_register & JOYPAD_MASK) != 0 {
                self.service_interrupt(memory_bus, Interrupt::JOYPAD);
            }
        }
    }

    /*
    * Services the specified interrupt by pushing the current PC to the stack,
    */
    fn service_interrupt(&mut self, memory_bus: &mut MemoryBus, interrupt: Interrupt) {
        let mut interrupts: InterruptFlags = memory_bus.interrupt_flags.into();
        memory_bus.interrupt_master_enable = false;
        let vector_address = match interrupt {
            Interrupt::VBLANK => VBLANK_VECTOR,
            Interrupt::LCDSTAT => LCDSTAT_VECTOR,
            Interrupt::TIMER => TIMER_VECTOR,
            Interrupt::SERIAL => SERIAL_VECTOR,
            Interrupt::JOYPAD => JOYPAD_VECTOR,
        };

        // Push the current PC to the stack
        self.op_push_stack(memory_bus, self.pc);

        // Set PC to the interrupt vector
        self.pc = vector_address;

        // Clear the interrupt flag
        match interrupt {
            Interrupt::VBLANK => interrupts.vblank = false,
            Interrupt::LCDSTAT => interrupts.lcd_stat = false,
            Interrupt::TIMER => interrupts.timer = false,
            Interrupt::SERIAL => interrupts.serial = false,
            Interrupt::JOYPAD => interrupts.joypad = false,
        }

        // Update interrupt flags in memory
        memory_bus.interrupt_flags = interrupts.into();
    }

    
    /*
    *   Called for halt opcode.
    *   Sets the CPU into a halted state until an interrupt occurs.
    */
    fn op_halt(&mut self) {
        self.halted = true;
    }
   
   /*
   *   Called for stop opcode.
   *   Stops the CPU and timer until a button is pressed.
   */
    fn op_stop(&mut self, memory_bus: &mut MemoryBus) {
        /*
        https://gbdev.io/pandocs/Timer_and_Divider_Registers.html
        */
        self.stopped = true;
        memory_bus.write_byte(DIVIDER_REGISTER, 0);
    }
    
    /*
    * Sets the interrupt master enable flag to false in memory bus.
    */
    fn op_di(&mut self, memory_bus: &mut MemoryBus) {
        memory_bus.interrupt_master_enable = false;
    }

    /*
     *   EI
     *   Schedules interrupt handling to be enabled after the next machine cycle.
     */
    fn op_ei(&mut self, memory_bus: &mut MemoryBus) {
        memory_bus.enabling_ime = true;
    }

    /*
     * RCL (Rotate Left Through Carry)
     * Shift the register value left by one bit.
     * If the most significant bit is set, then set the lest significant bit to 1 and set the carry flag.
     */
    fn op_rlc(&mut self, register: &mut u8) {
        let carry = *register & BYTE_MSB != 0;
        *register = (*register << 1) | (if carry { 1 } else { 0 });
        self.registers.f.set(FlagsRegister::ZERO, *register == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, carry);
    }

    /*
     * RRC (Rotate Right Through Carry)
     * Shift the register value right by one bit.
     * If the least significant bit is set, move it into the carry flag
     * and set the most significant bit to the previous carry value.
     */
    fn op_rrc(&mut self, register: &mut u8) {
        let carry = *register & BYTE_LSB != 0;
        *register = (*register >> 1) | (if carry { 0x80 } else { 0 });
        self.registers.f.set(FlagsRegister::ZERO, *register == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, carry);
    }

    /*
     * RL (Rotate Left through Carry)
     * Shift the register value left by one bit.
     * The old bit 7 is moved into the Carry flag.
     * The previous Carry flag value is rotated into bit 0.
     */
    fn op_rl(&mut self, register: &mut u8) {
        let carry = self.registers.f.contains(FlagsRegister::CARRY);
        let new_carry = *register & BYTE_MSB != 0;
        *register = (*register << 1) | (if carry { 1 } else { 0 });
        self.registers.f.set(FlagsRegister::ZERO, *register == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, new_carry);
    }

    /*
     * RR (Rotate Right through Carry)
     * Shift the register value right by one bit.
     * The old bit 0 is moved into the Carry flag.
     * The previous Carry flag value is rotated into bit 7.
     */
    fn op_rr(&mut self, register: &mut u8) {
        let carry = self.registers.f.contains(FlagsRegister::CARRY);
        let new_carry = *register & BYTE_LSB != 0;
        *register = (*register >> 1) | (if carry { 0x80 } else { 0 });
        self.registers.f.set(FlagsRegister::ZERO, *register == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, new_carry);
    }

    /*
     * SLA (Shift Left Arithmetic)
     * Shift the register value left by one bit.
     * The old bit 7 is moved into the Carry flag.
     * Bit 0 is always cleared to 0.
     */
    fn op_sla(&mut self, register: &mut u8) {
        let carry = *register >> 7;
        *register <<= 1;
        self.registers.f.set(FlagsRegister::ZERO, *register == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, carry == 1);
    }

    /*
     * SRA (Shift Right Arithmetic)
     * Shift the register value right by one bit.
     * The old bit 0 is moved into the Carry flag.
     * The most significant bit (bit 7) remains unchanged to preserve the sign.
     */
    fn op_sra(&mut self, register: &mut u8) {
        let carry = *register & BYTE_LSB != 0;
        *register = (*register & BYTE_MSB) | (*register >> 1);
        self.registers.f.set(FlagsRegister::ZERO, *register == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, carry);
    }

    /*
     * Swap Nibbles
     * Swap the upper and lower 4-bit nibbles of the register value.
     */
    fn op_swap(&mut self, register: &mut u8) {
        *register = (*register >> 4) | (*register << 4);
        self.registers.f.set(FlagsRegister::ZERO, *register == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, false);
    }

    /*
     * SRL (Shift Right Logical)
     * Shift the register value right by one bit.
     * The old bit 0 is moved into the Carry flag.
     * Bit 7 is always cleared to 0.
     */
    fn op_srl(&mut self, register: &mut u8) {
        let carry = *register & 0x01 != 0;
        *register >>= 1;
        self.registers.f.set(FlagsRegister::ZERO, *register == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, false);
        self.registers.f.set(FlagsRegister::CARRY, carry);
    }

    /*
     * BIT (Test Bit)
     * Test whether the specified bit position is set in the register value.
     * The Zero flag is set if the tested bit is 0.
     */
    fn op_bit(&mut self, bit: u8, register: u8) {
        self.registers.f.set(FlagsRegister::ZERO, (register & (1 << bit)) == 0);
        self.registers.f.set(FlagsRegister::SUBTRACT, false);
        self.registers.f.set(FlagsRegister::HALF_CARRY, true);
    }

    /*
     *   Unconditional function call to the absolute fixed address defined by the opcode.
     */
    fn op_rst_address(&mut self, memory_bus: &mut MemoryBus, address: u16) {
        self.op_push_stack(memory_bus, self.pc);
        self.pc = address;
    }

    /*
    * PUSH
    * Push the 16-bit value onto the stack.
    */
    pub fn op_push_stack(&mut self, memory_bus: &mut MemoryBus, address: u16) {
        self.sp = self.sp.wrapping_sub(2);
        memory_bus.write_short(self.sp, address);
    }

    /*
    * POP
    * Pop the 16-bit value from the stack and return it.
    */
    fn op_pop_stack(&mut self, memory_bus: &mut MemoryBus) -> u16 {
        let value = memory_bus.read_short(self.sp);
        self.sp = self.sp.wrapping_add(2);
        value
    }

}
