use log::{debug, error, info};
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{self, Read};
use std::{error, fmt, result};

use crate::{constants, rom_debug};
//use crate::display::GPU;
use crate::display;
use crate::rom;
use crate::memory_bus;
use constants::*;
use rom::*;
use crate::rom::ROM;
use memory_bus::MemoryBus;
use crate::interupts::*;

use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug)]
pub struct Registers {
    pub a: u8, // Accumulator register
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: FlagsRegister, // Flags
    pub h: u8,
    pub l: u8,
    pub pc: u16, // Program counter
    pub sp: u16,     // Stack pointer
}

#[derive(Debug, Clone, Copy)]
pub struct FlagsRegister {
    zero: bool,
    subtract: bool,
    half_carry: bool,
    carry: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Flag {
    Z, // Zero flag
    N, // Subtract flag
    H, // Half-carry flag
    C, // Carry flag
}

pub struct CPU/*<'a>*/ {
    pub registers: Registers,
    //work_ram: [u8; 0xFFFFF],
    video_ram: [u16; 8192],
    
    //pub memory_bus: &'a mut MemoryBus,
    pub memory_bus: Rc<RefCell<MemoryBus>>,
    pub halted: bool,
    stopped: bool,
    /*pub interrupt_master_enable: bool,
    pub enabling_ime: bool,
    pub interrupt_enable_register: u8,
    pub interrupt_flags: u8,*/
    rom_debug: rom_debug::rom_debug,
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            a: 0x01,
            b: 0x0,
            c: 0x13,
            d: 0x0,
            e: 0xD8,
            f: FlagsRegister::new(),
            h: 0x01,
            l: 0x4D,
            pc: 0x0100,
            sp: 0xFFFE,
        }
    }

    fn get_af(&self) -> u16 {
        let flags: u16 = self.f.into();
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
        self.f = value.into();
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
            "Registers:
        A: {:02X}
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
        SP: {:04X}
        PC: {:04X}
        F: {}",
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
            self.sp,
            self.pc,
            self.f
        )
    }
}

impl FlagsRegister {
    pub fn new() -> Self {
        FlagsRegister {
            zero: true,
            subtract: false,
            half_carry: true,
            carry: true,
        }
    }

    pub fn set_flag(&mut self, flag: Flag, value: bool) {
        match flag {
            Flag::Z => self.zero = value,
            Flag::N => self.subtract = value,
            Flag::H => self.half_carry = value,
            Flag::C => self.carry = value,
        }
    }

    fn get_flag(&self, flag: Flag) -> bool {
        match flag {
            Flag::Z => self.zero,
            Flag::N => self.subtract,
            Flag::H => self.half_carry,
            Flag::C => self.carry,
        }
    }
}

impl fmt::Display for FlagsRegister {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Flags: Z: {} N: {} H: {} C: {}",
            if self.zero { 1 } else { 0 },
            if self.subtract { 1 } else { 0 },
            if self.half_carry { 1 } else { 0 },
            if self.carry { 1 } else { 0 }
        )
    }
}

impl std::convert::From<FlagsRegister> for u8 {
    fn from(flag: FlagsRegister) -> u8 {
        (if flag.zero { 1 } else { 0 }) << ZERO_FLAG_BYTE_POSITION
            | (if flag.subtract { 1 } else { 0 }) << SUBTRACT_FLAG_BYTE_POSITION
            | (if flag.half_carry { 1 } else { 0 }) << HALF_CARRY_FLAG_BYTE_POSITION
            | (if flag.carry { 1 } else { 0 }) << CARRY_FLAG_BYTE_POSITION
    }
}

impl std::convert::From<FlagsRegister> for u16 {
    fn from(flag: FlagsRegister) -> u16 {
        let mut result: u16 = 0;
        result |= (if flag.zero { 1 } else { 0 }) << ZERO_FLAG_BYTE_POSITION;
        result |= (if flag.subtract { 1 } else { 0 }) << SUBTRACT_FLAG_BYTE_POSITION;
        result |= (if flag.half_carry { 1 } else { 0 }) << HALF_CARRY_FLAG_BYTE_POSITION;
        result |= (if flag.carry { 1 } else { 0 }) << CARRY_FLAG_BYTE_POSITION;
        result
    }
}

impl std::convert::From<u8> for FlagsRegister {
    fn from(byte: u8) -> Self {
        let zero = ((byte >> ZERO_FLAG_BYTE_POSITION) & 0b1) != 0;
        let subtract = ((byte >> SUBTRACT_FLAG_BYTE_POSITION) & 0b1) != 0;
        let half_carry = ((byte >> HALF_CARRY_FLAG_BYTE_POSITION) & 0b1) != 0;
        let carry = ((byte >> CARRY_FLAG_BYTE_POSITION) & 0b1) != 0;

        FlagsRegister {
            zero,
            subtract,
            half_carry,
            carry,
        }
    }
}

impl std::convert::From<u16> for FlagsRegister {
    fn from(byte: u16) -> Self {
        let zero = ((byte >> ZERO_FLAG_BYTE_POSITION) & 0b1) != 0;
        let subtract = ((byte >> SUBTRACT_FLAG_BYTE_POSITION) & 0b1) != 0;
        let half_carry = ((byte >> HALF_CARRY_FLAG_BYTE_POSITION) & 0b1) != 0;
        let carry = ((byte >> CARRY_FLAG_BYTE_POSITION) & 0b1) != 0;

        FlagsRegister {
            zero,
            subtract,
            half_carry,
            carry,
        }
    }
}

impl/*<'a>*/ CPU/*<'a>*/ {
    pub fn new(/*memory_bus: &'a mut MemoryBus*/memory_bus: Rc<RefCell<MemoryBus>>) -> Self {
        CPU {
            registers: Registers::new(),
            video_ram: [0; 8192],
            //gpu: GPU::new(),
            memory_bus,
            halted: false,
            stopped: false,
            /*interrupt_master_enable: false,
            enabling_ime: false,
            interrupt_enable_register: 0,
            interrupt_flags: 0,*/
            rom_debug: rom_debug::rom_debug::new(),
        }
    }

    /*
     *   CPU cycle - fetch, decode, execute
     */
    pub fn cycle(&mut self) -> u8 {
        debug!("##################################################");

        //self.gameboy_doctor_output_log();

        if !self.halted {

            let opcode = self.memory_bus.borrow().read_byte(self.registers.pc);
            debug!("opcode = {:#4X}", opcode);
            debug!("PC = {:#4X}", self.registers.pc);

            self.registers.pc = self.registers.pc.wrapping_add(1);

            self.debug_update();
            self.debug_print();

            match opcode {
                0x00 => {
                    self.op_nop();
                    4
                }
                0x01 => {
                    let nn: u16 = self.read_immediate_short();
                    self.registers.set_bc(nn);
                    12
                }
                0x02 => {
                    self.memory_bus.borrow_mut().write_byte(self.registers.get_bc(), self.registers.a);
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
                    let value = self.read_immediate_byte();
                    self.registers.b = value;
                    8
                }
                0x07 => {
                    let a = self.registers.a;
                    let new_carry = (a & 0x80) != 0;
                    self.registers.a = (a << 1) | if new_carry { 0x01 } else { 0x00 };
                    self.registers.f.set_flag(Flag::C, new_carry);
                    self.registers.f.set_flag(Flag::Z, false); // The Z flag is not affected
                    self.registers.f.set_flag(Flag::N, false);
                    self.registers.f.set_flag(Flag::H, false);
                    4
                }
                0x08 => {
                    let nn: u16 = self.read_immediate_short();
                    self.write_immediate_short(nn, self.registers.sp);
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
                    self.registers.a = self.memory_bus.borrow().read_byte(self.registers.get_bc());
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
                    let value = self.read_immediate_byte();
                    self.registers.c = value;
                    8
                }
                0x0F => {
                    let a = self.registers.a;
                    let carry = (a & 0x01) != 0;
                    self.registers.a = (a >> 1) | (carry as u8) << 7;
                    self.registers.f.set_flag(Flag::C, carry);
                    self.registers.f.set_flag(Flag::Z, false);
                    self.registers.f.set_flag(Flag::N, false);
                    self.registers.f.set_flag(Flag::H, false);
                    4
                }
                0x10 => {
                    // TODO - Implement STOP
                    debug!("CPU stopped");
                    self.stopped = true;
                    4
                }
                0x11 => {
                    let nn: u16 = self.read_immediate_short();
                    self.registers.set_de(nn);
                    12
                }
                0x12 => {
                    self.memory_bus.borrow_mut().write_byte(self.registers.get_de(), self.registers.a);
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
                    let value = self.read_immediate_byte();
                    self.registers.d = value;
                    8
                }
                0x17 => {
                    let carry = self.registers.a & 0x80 != 0;
                    self.registers.a = (self.registers.a << 1)
                        | (if self.registers.f.get_flag(Flag::C) {
                        1
                    } else {
                        0
                    });
                    self.registers.f.set_flag(Flag::Z, false);
                    self.registers.f.set_flag(Flag::N, false);
                    self.registers.f.set_flag(Flag::H, false);
                    self.registers.f.set_flag(Flag::C, carry);
                    4
                }
                0x18 => {
                    let offset = self.read_immediate_byte() as i8;
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
                    self.registers.a = self.memory_bus.borrow().read_byte(self.registers.get_de());
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
                    let value = self.read_immediate_byte();
                    self.registers.e = value;
                    8
                }
                0x1F => {
                    let carry = self.registers.a & 0x01 != 0;
                    self.registers.a = (self.registers.a >> 1)
                        | (if self.registers.f.get_flag(Flag::C) {
                        0x80
                    } else {
                        0
                    });
                    self.registers.f.set_flag(Flag::Z, false);
                    self.registers.f.set_flag(Flag::N, false);
                    self.registers.f.set_flag(Flag::H, false);
                    self.registers.f.set_flag(Flag::C, carry);
                    4
                }
                0x20 => {
                    let offset = self.read_immediate_byte() as i8;
                    if !self.registers.f.get_flag(Flag::Z) {
                        self.op_jr_e(offset);
                        return 12;
                    }
                    return 8;
                }
                0x21 => {
                    let nn: u16 = self.read_immediate_short();
                    self.registers.set_hl(nn);
                    12
                }
                0x22 => {
                    self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), self.registers.a);
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
                    let value = self.read_immediate_byte();
                    self.registers.h = value;
                    8
                }
                0x27 => {
                    self.op_daa();
                    4
                }
                0x28 => {
                    let offset = self.read_immediate_byte() as i8;
                    if self.registers.f.get_flag(Flag::Z) {
                        self.op_jr_e(offset);
                        return 12;
                    }
                    return 8;
                }
                0x29 => {
                    let hl = self.registers.get_hl();
                    let result = self.op_add_r16(hl, hl);
                    self.registers.set_hl(result);
                    8
                }
                0x2A => {
                    self.registers.a = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                    let value = self.read_immediate_byte();
                    self.registers.l = value;
                    8
                }
                0x2F => {
                    self.op_cpl();
                    4
                }
                0x30 => {
                    let offset = self.read_immediate_byte() as i8;
                    if !self.registers.f.get_flag(Flag::C) {
                        self.op_jr_e(offset);
                        return 12;
                    }
                    return 8;
                }
                0x31 => {
                    let nn: u16 = self.read_immediate_short();
                    self.registers.sp = nn;
                    12
                }
                0x32 => {
                    self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), self.registers.a);
                    self.registers
                        .set_hl(self.registers.get_hl().wrapping_sub(1));
                    8
                }
                0x33 => {
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    8
                }
                0x34 => {
                    self.op_inc_hl();
                    12
                }
                0x35 => {
                    self.op_dec_hl();
                    12
                }
                0x36 => {
                    let value = self.read_immediate_byte();
                    self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), value);
                    12
                }
                0x37 => {
                    self.op_scf();
                    4
                }
                0x38 => {
                    let offset = self.read_immediate_byte() as i8;
                    if self.registers.f.get_flag(Flag::C) {
                        self.op_jr_e(offset);
                        return 12;
                    }
                    return 8;
                }
                0x39 => {
                    let hl = self.registers.get_hl();
                    let value = self.registers.sp;
                    let result = self.op_add_r16(hl, value);
                    self.registers.set_hl(result);
                    8
                }
                0x3A => {
                    self.registers.a = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                    self.registers
                        .set_hl(self.registers.get_hl().wrapping_sub(1));
                    8
                }
                0x3B => {
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
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
                    let value = self.read_immediate_byte();
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
                    self.registers.b = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                    self.registers.c = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                    self.registers.d = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                    self.registers.e = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                    self.registers.h = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                    self.registers.l = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                    8
                }
                0x6F => {
                    self.registers.l = self.registers.a;
                    4
                }
                0x70 => {
                    self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), self.registers.b);
                    8
                }
                0x71 => {
                    self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), self.registers.c);
                    8
                }
                0x72 => {
                    self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), self.registers.d);
                    8
                }
                0x73 => {
                    self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), self.registers.e);
                    8
                }
                0x74 => {
                    self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), self.registers.h);
                    8
                }
                0x75 => {
                    self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), self.registers.l);
                    8
                }
                0x76 => {
                    debug!("Halting CPU");
                    self.op_halt();
                    4
                }
                0x77 => {
                    self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), self.registers.a);
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
                    self.registers.a = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                    let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                    let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                    let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                    let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                    let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                    self.op_and_r8(value);
                    8
                }
                0xA7 => {
                    self.registers.a &= self.registers.a;
                    self.registers.f.set_flag(Flag::Z, self.registers.a == 0);
                    self.registers.f.set_flag(Flag::N, false);
                    self.registers.f.set_flag(Flag::H, true);
                    self.registers.f.set_flag(Flag::C, false);
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
                    let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                    self.op_xor_r8(value);
                    8
                }
                0xAF => {
                    self.registers.a = 0x0000;
                    self.registers.f.set_flag(Flag::Z, true);
                    self.registers.f.set_flag(Flag::N, false);
                    self.registers.f.set_flag(Flag::H, false);
                    self.registers.f.set_flag(Flag::C, false);
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
                    let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                    let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                    self.op_cp_r8(value);
                    8
                }
                0xBF => {
                    let result = self.registers.a.wrapping_sub(self.registers.a);
                    self.registers.f.set_flag(Flag::Z, result == 0);
                    self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                    self.registers.f.set_flag(Flag::H, false); // Clear the half-carry flag
                    self.registers.f.set_flag(Flag::C, false); // Clear the carry flag
                    4
                }
                0xC0 => {
                    if !self.registers.f.get_flag(Flag::Z) {
                        self.op_ret();
                        return 20;
                    }
                    return 8;
                }
                0xC1 => {
                    let value = self.op_pop_stack();
                    self.registers.set_bc(value);
                    12
                }
                0xC2 => {
                    let nn: u16 = self.read_immediate_short();
                    if !self.registers.f.get_flag(Flag::Z) {
                        self.op_jp_nn(nn);
                        return 16;
                    }
                    return 12;
                }
                0xC3 => {
                    let nn: u16 = self.read_immediate_short();
                    debug!("Jumping to 0x{:X}", nn);
                    self.op_jp_nn(nn);
                    16
                }
                0xC4 => {
                    let nn: u16 = self.read_immediate_short();
                    if !self.registers.f.get_flag(Flag::Z) {
                        self.op_call_nn(nn);
                        return 24;
                    }
                    return 12;
                }
                0xC5 => {
                    self.op_push_stack(self.registers.get_bc());
                    16
                }
                0xC6 => {
                    self.op_add_d8();
                    8
                }
                0xC7 => {
                    self.op_rst_address(0x0000);
                    16
                }
                0xC8 => {
                    if self.registers.f.get_flag(Flag::Z) {
                        self.op_ret();
                        return 20;
                    }
                    return 8;
                }
                0xC9 => {
                    self.op_ret();
                    16
                }
                0xCA => {
                    let nn: u16 = self.read_immediate_short();
                    if self.registers.f.get_flag(Flag::Z) {
                        self.op_jp_nn(nn);
                        return 16;
                    }
                    return 12;
                }
                0xCB => {
                    // Get the next byte and use it as the extended opcode
                    let extended_opcode = self.read_immediate_byte();
                    let cb_cycles = 4;
                    debug!("0xCB{:X}", extended_opcode);
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
                            let mut value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            self.op_rlc(&mut value);
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), value);
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
                            let mut value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            self.op_rrc(&mut value);
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), value);
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
                            let mut value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            self.op_rl(&mut value);
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), value);
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
                            let mut value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            self.op_rr(&mut value);
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), value);
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
                            let mut value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            self.op_sla(&mut value);
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), value);
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
                            let mut value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            self.op_sra(&mut value);
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), value);
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
                            let mut value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            self.op_swap(&mut value);
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), value);
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
                            let mut value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            self.op_srl(&mut value);
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), value);
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
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
                            let value = self.memory_bus.borrow().read_byte(hl);
                            let result = value & !(1 << 0);
                            self.memory_bus.borrow_mut().write_byte(hl, result);
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
                            let value = self.memory_bus.borrow().read_byte(hl);
                            let result = value & !(1 << 1);
                            self.memory_bus.borrow_mut().write_byte(hl, result);
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
                            let value = self.memory_bus.borrow().read_byte(hl);
                            let result = value & !(1 << 2);
                            self.memory_bus.borrow_mut().write_byte(hl, result);
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
                            let value = self.memory_bus.borrow().read_byte(hl);
                            let result = value & !(1 << 3);
                            self.memory_bus.borrow_mut().write_byte(hl, result);
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
                            let value = self.memory_bus.borrow().read_byte(hl);
                            let result = value & !(1 << 4);
                            self.memory_bus.borrow_mut().write_byte(hl, result);
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
                            let value = self.memory_bus.borrow().read_byte(hl);
                            let result = value & !(1 << 5);
                            self.memory_bus.borrow_mut().write_byte(hl, result);
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
                            let value = self.memory_bus.borrow().read_byte(hl);
                            let result = value & !(1 << 6);
                            self.memory_bus.borrow_mut().write_byte(hl, result);
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
                            let value = self.memory_bus.borrow().read_byte(hl);
                            let result = value & !(1 << 7);
                            self.memory_bus.borrow_mut().write_byte(hl, result);
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
                            let value = self.memory_bus.borrow().read_byte(hl);
                            let result = value | (1 << 0);
                            self.memory_bus.borrow_mut().write_byte(hl, result);
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
                            let value = self.memory_bus.borrow().read_byte(hl);
                            let result = value | (1 << 1);
                            self.memory_bus.borrow_mut().write_byte(hl, result);
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            let mut result = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            result |= value | 1 << 2;
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), result);
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            let mut result = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            result |= value | 1 << 3;
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), result);
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            let mut result = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            result |= value | 1 << 4;
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), result);
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            let mut result = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            result |= value | 1 << 5;
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), result);
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            let mut result = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            result |= value | 1 << 6;
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), result);
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
                            let value = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            let mut result = self.memory_bus.borrow().read_byte(self.registers.get_hl());
                            result |= value | 1 << 7;
                            self.memory_bus.borrow_mut().write_byte(self.registers.get_hl(), result);
                            cb_cycles + 16
                        }
                        0xFF => {
                            self.registers.a |= 1 << 7;
                            cb_cycles + 8
                        }
                        _ => {
                            panic!("Unsupported opcode: 0xCB{:02X}", opcode);
                        }
                    }
                }
                0xCC => {
                    let nn: u16 = self.read_immediate_short();
                    if self.registers.f.get_flag(Flag::Z) {
                        self.op_call_nn(nn);
                        return 24;
                    }
                    return 12;
                }
                0xCD => {
                    let nn: u16 = self.read_immediate_short();
                    debug!("CALL {:04X}", nn);
                    self.op_call_nn(nn);
                    24
                }
                0xCE => {
                    let value = self.read_immediate_byte();
                    self.op_adc_r8(value);
                    8
                }
                0xCF => {
                    self.op_rst_address(0x08);
                    16
                }
                0xD0 => {
                    if !self.registers.f.get_flag(Flag::C) {
                        self.op_ret();
                        return 20;
                    }
                    return 8;
                }
                0xD1 => {
                    let value = self.op_pop_stack();
                    self.registers.set_de(value);
                    12
                }
                0xD2 => {
                    let nn: u16 = self.read_immediate_short();
                    if !self.registers.f.get_flag(Flag::C) {
                        self.op_jp_nn(nn);
                        return 16;
                    }
                    return 12;
                }
                0xD3 => {
                    error!("Unsupported opcode: 0xD3");
                    4
                }
                0xD4 => {
                    let nn: u16 = self.read_immediate_short();
                    if !self.registers.f.get_flag(Flag::C) {
                        self.op_call_nn(nn);
                        return 24;
                    }
                    return 12;
                }
                0xD5 => {
                    self.op_push_stack(self.registers.get_de());
                    16
                }
                0xD6 => {
                    self.op_sub_d8();
                    8
                }
                0xD7 => {
                    self.op_rst_address(0x10);
                    16
                }
                0xD8 => {
                    if self.registers.f.get_flag(Flag::C) {
                        self.op_ret();
                        return 20;
                    }
                    return 8;
                }
                0xD9 => {
                    self.op_ret();
                    self.op_ei();
                    16
                }
                0xDA => {
                    let nn: u16 = self.read_immediate_short();
                    if self.registers.f.get_flag(Flag::C) {
                        self.op_jp_nn(nn);
                        return 16;
                    }
                    return 12;
                }
                0xDB => {
                    error!("Unsupported opcode: 0xDB");
                    4
                }
                0xDC => {
                    let nn: u16 = self.read_immediate_short();
                    if self.registers.f.get_flag(Flag::C) {
                        self.op_call_nn(nn);
                        return 24;
                    }
                    return 12;
                }
                0xDD => {
                    error!("Unsupported opcode: 0xDD");
                    4
                }
                0xDE => {
                    let value = self.read_immediate_byte();
                    self.op_sbc_r8(value);
                    8
                }
                0xDF => {
                    self.op_rst_address(0x18);
                    16
                }
                0xE0 => {
                    let offset = self.read_immediate_byte() as u16;
                    let address = 0xFF00 + offset;
                    debug!("LDH (0xFF00 + {:02X}), A", offset);
                    debug!("Address = {:02X}", address);
                    self.memory_bus.borrow_mut().write_byte(address, self.registers.a);
                    12
                }
                0xE1 => {
                    let value = self.op_pop_stack();
                    self.registers.set_hl(value);
                    12
                }
                0xE2 => {
                    let address = 0xFF00 | self.registers.c as u16;
                    self.memory_bus.borrow_mut().write_byte(address, self.registers.a);
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
                    self.op_push_stack(self.registers.get_hl());
                    16
                }
                0xE6 => {
                    self.op_and_d8();
                    8
                }
                0xE7 => {
                    self.op_rst_address(0x20);
                    16
                }
                0xE8 => {
                    self.op_add_sp_d8();
                    16
                }
                0xE9 => {
                    self.registers.pc = self.registers.get_hl();
                    4
                }
                0xEA => {
                    let nn: u16 = self.read_immediate_short();
                    self.memory_bus.borrow_mut().write_byte(nn, self.registers.a);
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
                    let value = self.read_immediate_byte();
                    self.op_xor_r8(value);
                    8
                }
                0xEF => {
                    self.op_rst_address(0x28);
                    16
                }
                0xF0 => {
                    let value = self.read_immediate_byte();
                    let address = 0xFF00 + value as u16;
                    self.registers.a = self.memory_bus.borrow().read_byte(address);
                    12
                }
                0xF1 => {
                    let value = self.op_pop_stack();
                    self.registers.a = (value >> 8) as u8; // Upper byte to A

                    // Set flags directly
                    self.registers.f.set_flag(Flag::Z, (value & 0x80) != 0); // Bit 7 of F
                    self.registers.f.set_flag(Flag::N, (value & 0x40) != 0); // Bit 6 of F
                    self.registers.f.set_flag(Flag::H, (value & 0x20) != 0); // Bit 5 of F
                    self.registers.f.set_flag(Flag::C, (value & 0x10) != 0); // Bit 4 of F

                    12
                }
                0xF2 => {
                    let address = 0xFF00 | self.registers.c as u16;
                    self.registers.a = self.memory_bus.borrow().read_byte(address);
                    8
                }
                0xF3 => {
                    self.op_di();
                    4
                }
                0xF4 => {
                    error!("Unsupported opcode: 0xF4");
                    4
                }
                0xF5 => {
                    self.op_push_stack(self.registers.get_af());
                    16
                }
                0xF6 => {
                    self.op_or_d8();
                    8
                }
                0xF7 => {
                    self.op_rst_address(0x30);
                    16
                }
                0xF8 => {
                    let value = self.read_immediate_byte() as i8;
                    let sp = self.registers.sp;
                    let result = sp.wrapping_add(value as i16 as u16);
                    self.registers.set_hl(result);
                    self.registers.f.set_flag(Flag::Z, false);
                    self.registers.f.set_flag(Flag::N, false);
                    self.registers.f.set_flag(Flag::H, (sp & 0xF) + (value as u16 & 0xF) > 0xF);
                    self.registers.f.set_flag(Flag::C, (sp & 0xFF) + (value as u16 & 0xFF) > 0xFF);
                    12
                }
                0xF9 => {
                    self.registers.sp = self.registers.get_hl();
                    8
                }
                0xFA => {
                    let nn: u16 = self.read_immediate_short();
                    self.registers.a = self.memory_bus.borrow().read_byte(nn);
                    16
                }
                0xFB => {
                    self.op_ei();
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
                    let value = self.read_immediate_byte();
                    self.op_cp_r8(value);
                    8
                }
                0xFF => {
                    self.op_rst_address(0x0038);
                    16
                }
                _ => {
                    panic!("Unsupported opcode: {:X}", opcode);
                }
            }
        }
        else {
            debug!("CPU halted");

            if u8::from(self.memory_bus.borrow().interrupt_flags) != 0 {
                self.halted = false;
            }

            4
        }

    }

    /*
     *   NOP
     *   No operation.
     */
    fn op_nop(&mut self) {
        debug!("op_nop");
    }

    /*
     *   Read the immediate 16-bit value from memory for the current program counter and program counter + 1.
     */
    fn read_immediate_short(&mut self) -> u16 {
        let lsb = self.memory_bus.borrow().read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        let msb = self.memory_bus.borrow().read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        (msb as u16) << 8 | lsb as u16
    }

    /*
     *   Read the immediate 8-bit value from memory for the current program counter.
     */
    fn read_immediate_byte(&mut self) -> u8 {
        let value = self.memory_bus.borrow().read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        value
    }

    /*
     *   Write the immediate 16-bit value to memory.
     */
    fn write_immediate_short(&mut self, address: u16, value: u16) {
        debug!("write immediate short address {:X} value {:X}", address, value);
        let lsb = (value & 0x00FF) as u8;
        let msb = (value >> 8) as u8;
        self.memory_bus.borrow_mut().write_byte(address, lsb);
        self.memory_bus.borrow_mut().write_byte(address.wrapping_add(1), msb);
        /*match address {
            0x0000..=0x7FFF  => {
                let lsb = (value & 0x00FF) as u8;
                let msb = (value >> 8) as u8;
                self.memory_bus.borrow_mut().write_byte(address, lsb);
                self.memory_bus.borrow_mut().write_byte(address.wrapping_add(1), msb);
            },
            0x8000..=0x9FFF => {
                self.gpu.write_short(address, value);
            },
            _ => {
                panic!("Unsupported address: {:X}", address);

            }
        }*/
    }

    /*
    * 8-bit arithmetic and logical operations
    */
    fn op_inc_r8(&mut self, register: u8) -> u8{
        debug!("op_inc_r8");
        let result = register.wrapping_add(1);
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, (register & 0x0F) + 1 > 0x0F);
        result
    }

    fn op_dec_r8(&mut self, register: u8) -> u8{
        debug!("op_dec_r8");
        let result = register.wrapping_sub(1);
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, true);
        self.registers.f.set_flag(Flag::H, (register & 0x0F) < 1);
        result
    }

    fn op_add_r8(&mut self, value: u8) {
        debug!("op_add_r8");
        let result: u8 = self.registers.a.wrapping_add(value);
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers
            .f
            .set_flag(Flag::H, (self.registers.a & 0x0F) + (value & 0x0F) > 0x0F);
        self.registers
            .f
            .set_flag(Flag::C, (self.registers.a as u16) + (value as u16) > 0xFF);
        self.registers.a = result;
    }

    fn op_add_d8(&mut self) {
        debug!("op_add_r8");
        let value = self.read_immediate_byte();
        let result: u8 = self.registers.a.wrapping_add(value);
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers
            .f
            .set_flag(Flag::H, (self.registers.a & 0x0F) + (value & 0x0F) > 0x0F);
        self.registers
            .f
            .set_flag(Flag::C, (self.registers.a as u16) + (value as u16) > 0xFF);
        self.registers.a = result;
    }

    fn op_sub_r8(&mut self, value: u8) {
        debug!("op_sub_r8");
        let result: u8 = self.registers.a.wrapping_sub(value);
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, true);
        self.registers
            .f
            .set_flag(Flag::H, (self.registers.a & 0x0F) < (value & 0x0F));
        self.registers
            .f
            .set_flag(Flag::C, self.registers.a < value);
        self.registers.a = result;
    }
    
    fn op_sub_d8(&mut self) {
        debug!("op_sub_d8");
        let value = self.read_immediate_byte();
        let result: u8 = self.registers.a.wrapping_sub(value);
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, true);
        self.registers
            .f
            .set_flag(Flag::H, (self.registers.a & 0x0F) < (value & 0x0F));
        self.registers
            .f
            .set_flag(Flag::C, self.registers.a < value);
        self.registers.a = result;
    }

    fn op_or_r8(&mut self, value: u8) {
        debug!("op_or_r8");
        let result: u8 = self.registers.a | value;
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, false);
        self.registers.a = result;
    }
    
    fn op_or_d8(&mut self) {
        debug!("op_or_d8");
        let value = self.read_immediate_byte();
        let result: u8 = self.registers.a | value;
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, false);
        self.registers.a = result;
    }

    fn op_and_r8(&mut self, value: u8) {
        debug!("op_and_r8");
        let result: u8 = self.registers.a & value;
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, true);
        self.registers.f.set_flag(Flag::C, false);
        self.registers.a = result;
    }
    
    fn op_and_d8(&mut self) {
        debug!("op_and_d8");
        let value = self.read_immediate_byte();
        let result: u8 = self.registers.a & value;
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, true);
        self.registers.f.set_flag(Flag::C, false);
        self.registers.a = result;
    }

    fn op_cp_r8(&mut self, value: u8) {
        debug!("op_cp_r8");
        let result: u8 = self.registers.a.wrapping_sub(value);
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, true);
        self.registers.f.set_flag(Flag::H, (self.registers.a & 0x0F) < (value & 0x0F));
        self.registers.f.set_flag(Flag::C, self.registers.a < value);
    }

    fn op_xor_r8(&mut self, value: u8) {
        debug!("op_xor_r8");
        self.registers.a ^= value;
        self.registers.f.set_flag(Flag::Z, self.registers.a == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, false);
    }

    fn op_adc_r8(&mut self, value: u8) {
        debug!("op_adc_r8");
        let carry = if self.registers.f.get_flag(Flag::C) {
            1
        } else {
            0
        } as u8;
        let result = self.registers.a.wrapping_add(value).wrapping_add(carry);
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(
            Flag::H,
            (self.registers.a & 0x0F) + (value & 0x0F) + carry > 0x0F,
        );
        self.registers.f.set_flag(
            Flag::C,
            (self.registers.a as u16) + (value as u16) + (carry as u16) > 0xFF,
        );
        self.registers.a = result;
    }

    // Complement
    fn op_cpl(&mut self) {
        debug!("op_cpl");
        self.registers.a = !self.registers.a;
        self.registers.f.set_flag(Flag::N, true);
        self.registers.f.set_flag(Flag::H, true);
    }

    fn op_ccf(&mut self) {
        debug!("op_ccf");
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, !self.registers.f.get_flag(Flag::C));
    }

    fn op_scf(&mut self) {
        debug!("op_scf");
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, true);
    }

    fn op_daa(&mut self) {
        debug!("op_daa");
        let mut a = self.registers.a;
        if !self.registers.f.get_flag(Flag::N) {
            if self.registers.f.get_flag(Flag::C) || a > 0x99 {
                a = a.wrapping_add(0x60);
                self.registers.f.set_flag(Flag::C, true);
            }
            if self.registers.f.get_flag(Flag::H) || (a & 0x0F) > 0x09 {
                a = a.wrapping_add(0x06);
            }
        } else {
            if self.registers.f.get_flag(Flag::C) {
                a = a.wrapping_sub(0x60);
            }
            if self.registers.f.get_flag(Flag::H) {
                a = a.wrapping_sub(0x06);
            }
        }
        self.registers.f.set_flag(Flag::Z, a == 0);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.a = a;
    }

    fn op_sbc_r8(&mut self, r: u8) {
        let carry = if self.registers.f.get_flag(Flag::C) { 1 } else { 0 } as u8;
        let result = self.registers.a.wrapping_sub(r).wrapping_sub(carry);
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::N, true);
        // Check for half carry by comparing lower nibbles before subtraction
        self.registers.f.set_flag(Flag::H, (self.registers.a & 0x0F) < (r & 0x0F) + carry);
        self.registers.f.set_flag(Flag::C, (self.registers.a as u16) < (r as u16) + (carry as u16));
        self.registers.a = result;
    }

    /*
     *   16-bit arithmetic operations
     */
    fn op_inc_hl(&mut self) {
        debug!("op_inc_hl");
        let address = self.registers.get_hl();
        let mut value = self.memory_bus.borrow_mut().read_byte(address);
        value = value.wrapping_add(1);
        self.memory_bus.borrow_mut().write_byte(address, value);
        self.registers.f.set_flag(Flag::Z, value == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, (value & 0x0F) == 0x00);
    }

    fn op_dec_hl(&mut self) {
        debug!("op_dec_hl");
        let address = self.registers.get_hl();
        let mut value = self.memory_bus.borrow_mut().read_byte(address);
        value = value.wrapping_sub(1);
        self.memory_bus.borrow_mut().write_byte(address, value);
        self.registers.f.set_flag(Flag::Z, value == 0);
        self.registers.f.set_flag(Flag::N, true);
        self.registers.f.set_flag(Flag::H, (value & 0x0F) == 0x0F);
    }

    fn op_add_r16(&mut self, register: u16, value: u16) -> u16{
        debug!("op_add_r16");
        let result = register.wrapping_add(value);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, (register & 0x0FFF) + (value & 0x0FFF) > 0x0FFF);
        self.registers.f.set_flag(Flag::C, (register as u32) + (value as u32) > 0xFFFF);
        result
    }
    fn op_add_sp_d8(&mut self) {
        let value = self.read_immediate_byte() as i8;
        let sp = self.registers.sp as i16;
        let result = sp.wrapping_add(value as i16);
        self.registers.sp = result as u16;
        let half_carry = ((sp & 0x0F) + (value as i16 & 0x0F)) & 0x10 != 0;
        self.registers.f.set_flag(Flag::H, half_carry);
        let carry = (sp & 0xFF) + (value as i16 & 0xFF) > 0xFF;
        self.registers.f.set_flag(Flag::C, carry);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::Z, false);
    }

    /*
     *   JP nn
     *   Unconditional jump to the absolute address specified by the 16-bit operand nn.
     */
    fn op_jp_nn(&mut self, address: u16) {
        debug!("op_jp_nn");
        self.registers.pc = address;
    }

    /*
     *   JR e
     *   Unconditional jump to the relative address specified by the signed 8-bit operand e.
     */
    /*fn op_jr_e(&mut self, offset: i8) {
        debug!(
            "Jumping to 0x{:04X}",
            self.registers.pc.wrapping_add(offset as u16)
        );
        self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
    }*/
    fn op_jr_e(&mut self, offset: i8) {
        debug!("Program counter before jump: 0x{:04X}", self.registers.pc);
        debug!("Offset: 0x{:02X}", offset);
        let new_pc = self.registers.pc.wrapping_add(offset as u16);
        debug!("Jumping to 0x{:04X}", new_pc);
        self.registers.pc = new_pc;
    }

    /*
     *   CALL nn
     *   Unconditional function call to the absolute address specified by the 16-bit operand nn.
     */
    fn op_call_nn(&mut self, address: u16) {
        debug!("op_call_nn");
        self.op_push_stack(self.registers.pc);
        self.registers.pc = address;
    }

    /*
     *   RET
     *   Unconditional return from a function.
     */
    fn op_ret(&mut self) {
        debug!("op_ret");
        let address = self.op_pop_stack();
        debug!("Return address: 0x{:04X}", address);
        self.registers.pc = address;
    }

    /*
     *   HALT
     *   STOP
     *   DI
     *   Disables interrupt handling by setting IME=0 and cancelling any scheduled effects of the EI instruction if any.
     */

    pub fn check_interrupts(&mut self) {
        // Check if interrupts are scheduled to be enabled
        if self.memory_bus.borrow().enabling_ime {
            self.memory_bus.borrow_mut().interrupt_master_enable = true;
            self.memory_bus.borrow_mut().enabling_ime = false;
        }

        if self.memory_bus.borrow().interrupt_master_enable {
            let interrupt_flags: InterruptFlags = self.memory_bus.borrow().interrupt_flags.into();
            let interrupt_enable_register = self.memory_bus.borrow().interrupt_enable_register;

            // Check if the interrupt is both flagged and enabled
            if interrupt_flags.vblank && (interrupt_enable_register & 0x01) != 0 {
                debug!("VBLANK interrupt");
                self.service_interrupt(Interrupt::VBLANK);
            } else if interrupt_flags.lcd_stat && (interrupt_enable_register & 0x02) != 0 {
                debug!("LCDSTAT interrupt");
                self.service_interrupt(Interrupt::LCDSTAT);
            } else if interrupt_flags.timer && (interrupt_enable_register & 0x04) != 0 {
                debug!("TIMER interrupt");
                self.service_interrupt(Interrupt::TIMER);
            } else if interrupt_flags.serial && (interrupt_enable_register & 0x08) != 0 {
                debug!("SERIAL interrupt");
                self.service_interrupt(Interrupt::SERIAL);
            } else if interrupt_flags.joypad && (interrupt_enable_register & 0x10) != 0 {
                debug!("JOYPAD interrupt");
                self.service_interrupt(Interrupt::JOYPAD);
            }
        }
    }

    fn service_interrupt(&mut self, interrupt: Interrupt) {
        let mut interrupts: InterruptFlags = self.memory_bus.borrow().interrupt_flags.into();
        self.memory_bus.borrow_mut().interrupt_master_enable = false;
        let vector_address = match interrupt {
            Interrupt::VBLANK => 0x0040,
            Interrupt::LCDSTAT => 0x0048,
            Interrupt::TIMER => 0x0050,
            Interrupt::SERIAL => 0x0058,
            Interrupt::JOYPAD => 0x0060,
        };
        
        // Added for mooney/mts-20240127-1204-74ae166/acceptance/ei_sequence.gb
        //self.registers.pc += 1;

        // Push the current PC to the stack
        self.op_push_stack(self.registers.pc);

        // Set PC to the interrupt vector
        self.registers.pc = vector_address;

        // Clear the interrupt flag
        match interrupt {
            Interrupt::VBLANK => interrupts.vblank = false,
            Interrupt::LCDSTAT => interrupts.lcd_stat = false,
            Interrupt::TIMER => interrupts.timer = false,
            Interrupt::SERIAL => interrupts.serial = false,
            Interrupt::JOYPAD => interrupts.joypad = false,
        }

        // Update interrupt flags in memory
        self.memory_bus.borrow_mut().interrupt_flags = interrupts.into();
    }

    fn op_halt(&mut self) {
        debug!("op_halt");
        //self.memory_bus.borrow_mut().write_byte(memory_bus::INTERRUPT_ENABLE_REGISTER, 1);
        self.halted = true;
    }
    fn op_stop(&mut self) {
        error!("op_stop");
        /*
        https://gbdev.io/pandocs/Timer_and_Divider_Registers.html
        */
        self.memory_bus.borrow_mut().write_byte(0xFF04, 0);
    }
    fn op_di(&mut self) {
        debug!("op_di");
        self.memory_bus.borrow_mut().interrupt_master_enable = false;
    }

    /*
     *   EI
     *   Schedules interrupt handling to be enabled after the next machine cycle.
     */
    fn op_ei(&mut self) {
        debug!("op_ei");
        self.memory_bus.borrow_mut().enabling_ime = true;
    }

    /*
     *   RCL (Rotate Left Through Carry)
     */
    // TODO add detailed description
    fn op_rlc(&mut self, register: &mut u8) {
        debug!("op_rlc");
        let carry = *register & 0x80 != 0;
        *register = (*register << 1) | (if carry { 1 } else { 0 });
        self.registers.f.set_flag(Flag::Z, *register == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, carry);
    }

    // TODO add detailed description
    /*
     *   Rotate Right through Carry
     */
    fn op_rrc(&mut self, register: &mut u8) {
        debug!("op_rrc");
        let carry = *register & 0x01 != 0;
        *register = (*register >> 1) | (if carry { 0x80 } else { 0 });
        self.registers.f.set_flag(Flag::Z, *register == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, carry);
    }

    // TODO add detailed description
    /*
     *   Rotate Left
     */
    fn op_rl(&mut self, register: &mut u8) {
        debug!("op_rl");
        let carry = self.registers.f.get_flag(Flag::C);
        let new_carry = *register & 0x80 != 0;
        *register = (*register << 1) | (if carry { 1 } else { 0 });
        self.registers.f.set_flag(Flag::Z, *register == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, new_carry);
    }

    // TODO add detailed description
    fn op_rr(&mut self, register: &mut u8) {
        debug!("op_rr");
        let carry = self.registers.f.get_flag(Flag::C);
        let new_carry = *register & 0x01 != 0;
        *register = (*register >> 1) | (if carry { 0x80 } else { 0 });
        self.registers.f.set_flag(Flag::Z, *register == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, new_carry);
    }

    // TODO add detailed description
    fn op_sla(&mut self, register: &mut u8) {
        let carry = *register >> 7;
        *register <<= 1;
        self.registers.f.set_flag(Flag::Z, *register == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, carry == 1);
    }

    // TODO add detailed description
    fn op_sra(&mut self, register: &mut u8) {
        debug!("op_sra");
        let carry = *register & 0x01 != 0;
        *register = (*register & 0x80) | (*register >> 1);
        self.registers.f.set_flag(Flag::Z, *register == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, carry);
    }

    // TODO add detailed description
    fn op_swap(&mut self, register: &mut u8) {
        debug!("op_swap");
        *register = (*register >> 4) | (*register << 4);
        self.registers.f.set_flag(Flag::Z, *register == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, false);
    }

    // TODO add detailed description
    fn op_srl(&mut self, register: &mut u8) {
        debug!("op_srl");
        let carry = *register & 0x01 != 0;
        *register >>= 1;
        self.registers.f.set_flag(Flag::Z, *register == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, carry);
    }

    // TODO add detailed description
    // FIXME Get this to work with RC
    fn op_bit(&mut self, bit: u8, register: u8) {
        debug!("op_bit");
        self.registers.f.set_flag(Flag::Z, (register & (1 << bit)) == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, true);
    }

    /*
     *   Unconditional function call to the absolute fixed address defined by the opcode.
     */
    fn op_rst_address(&mut self, address: u16) {
        debug!("rst_address {}", address);
        self.op_push_stack(self.registers.pc);
        self.registers.pc = address;
    }

    pub fn op_push_stack(&mut self, address: u16) {
        debug!("op_push_stack");
        self.registers.sp = self.registers.sp.wrapping_sub(2);
        self.memory_bus.borrow_mut().write_short(self.registers.sp, address);
    }

    fn op_pop_stack(&mut self) -> u16 {
        debug!("op_pop_stack");
        let value = self.memory_bus.borrow_mut().read_short(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(2);
        value
    }

    fn wait_for_input(&mut self) {
        // Create a buffer to hold the user input
        let mut buffer = [0; 1];

        // Create an instance of Stdin
        let stdin = io::stdin();

        // Lock stdin and get a mutable reference to it
        let mut handle = stdin.lock();

        loop {
            // Read a single byte of input
            match handle.read_exact(&mut buffer) {
                Ok(_) => {
                    // If a key was pressed, break out of the loop
                    break;
                }
                Err(_) => {
                    // Handle any errors (e.g., if reading from stdin fails)
                    println!("An error occurred while reading input.");
                    break;
                }
            }
        }
    }

    fn debug_update(&mut self) {
        if self.memory_bus.borrow().read_byte(0xFF02) == 0x81 {
            self.rom_debug.add_char(self.memory_bus.borrow().read_byte(0xFF01) as char);
            self.memory_bus.borrow_mut().write_byte(0xFF02, 0);
        }
    }

    fn debug_print(&mut self) {
        self.rom_debug.print();
    }

    fn gameboy_doctor_output_log(&mut self) {
        // create or open (append mode) the log file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("gameboy_doctor_output.log")
            .unwrap();

        // write line to file
        writeln!(file, "A:{:02X} F:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:02X} PC:{:04X} PCMEM:{:02X},{:02X},{:02X},{:02X}",
                 self.registers.a,
                 u8::from(self.registers.f),
                 self.registers.b,
                 self.registers.c,
                 self.registers.d,
                 self.registers.e,
                 self.registers.h,
                 self.registers.l,
                 self.registers.sp,
                 self.registers.pc,
                 self.memory_bus.borrow().read_byte(self.registers.pc),
                 self.memory_bus.borrow().read_byte(self.registers.pc.wrapping_add(1)),
                 self.memory_bus.borrow().read_byte(self.registers.pc.wrapping_add(2)),
                 self.memory_bus.borrow().read_byte(self.registers.pc.wrapping_add(3)))
            .unwrap();
        // close file
        file.flush().unwrap();
    }

}
