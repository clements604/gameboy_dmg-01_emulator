use log::{debug, error, info};
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Read};
use std::{error, fmt, result};

use crate::constants;
use crate::display::GPU;
use crate::display::*;
use crate::rom;
use crate::memory_bus::*;
use constants::*;
use rom::*;


pub struct Registers {
    a: u8, // Accumulator register
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: FlagsRegister, // Flags
    h: u8,
    l: u8,
    pub pc: u16, // Program counter
    sp: u16,     // Stack pointer
}

#[derive(Debug, Clone, Copy)]
struct FlagsRegister {
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

pub struct CPU<'a> {
    pub registers: Registers,
    //work_ram: [u8; 0xFFFFF],
    video_ram: [u16; 8192],
    interupt: bool,
    gpu: GPU,
    memory_bus: &'a mut MemoryBus,
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            a: 0x0,
            b: 0x0,
            c: 0x0,
            d: 0x0,
            e: 0x0,
            f: FlagsRegister::new(),
            h: 0x0,
            l: 0x0,
            pc: 0x0100, //0x0,
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
        AF: {:02X}
        BC: {:02X}
        DE: {:02X}
        HL: {:02X}
        PC: {:04X}
        SP: {:04X}
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
            self.pc,
            self.sp,
            self.f
        )
    }
}

impl FlagsRegister {
    pub fn new() -> Self {
        FlagsRegister {
            zero: false,
            subtract: false,
            half_carry: false,
            carry: false,
        }
    }

    fn set_flag(&mut self, flag: Flag, value: bool) {
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

impl<'a> CPU<'a> {
    pub fn new(memory_bus: &'a mut  MemoryBus) -> Self {
        CPU {
            registers: Registers::new(),
            //work_ram: [0; 0xFFFFF],
            video_ram: [0; 8192],
            interupt: false,
            gpu: GPU::new(),
            //call_stack: vec![0x0000], //TODO should be 0xFFFE?
            memory_bus,
        }
    }

    /*fn debug_print_tile_data(&mut self) {
        let tile_data = &self.work_ram[TILE_RAM_START as usize..=TILE_RAM_END as usize];
        for (i, byte) in tile_data.iter().enumerate() {
            if i % 16 == 0 {
                println!();
            }
            print!("{:02X} ", byte);
        }
    }*/

    /*
     *   CPU cycle - fetch, decode, execute
     */
    pub fn cycle(&mut self) {
        debug!("##################################################");
        debug!("Fetch");

        let opcode = self.memory_bus.read_byte(self.registers.pc);
        debug!("PC [0x{:X}]", self.registers.pc);
        debug!("Opcode [0x{:X}]", opcode);
        //debug!("{}", self.registers);
        //self.debug_print_tile_data();
        self.print_debug();

        self.registers.pc = self.registers.pc.wrapping_add(1);

        //self.wait_for_input();

        if self.memory_bus.read_byte(0xFF02) == 0x81 {
            // TODO Temp for development, delete this
            info!("CPU test [{}]", self.memory_bus.read_byte(0xFF01) as char);
            self.memory_bus.write_byte(0xFF02, 0x0);
            panic!("test reaches here");
        }

        debug!("Decode & Execute");

        match opcode {
            0x00 => {
                self.op_nop();
            }
            0x01 => {
                let nn: u16 = self.read_immediate_short();
                self.registers.set_bc(nn);
            }
            0x02 => {
                self.memory_bus.write_byte(self.registers.get_bc(), self.registers.a);
            }
            0x03 => {
                self.registers
                    .set_bc(self.registers.get_bc().wrapping_add(1));
            }
            0x04 => {
                let old_value = self.registers.b;
                self.registers.b = self.registers.b.wrapping_add(1);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::Z, self.registers.b == 0);
                self.registers
                    .f
                    .set_flag(Flag::H, (old_value & 0x0F) == 0x0F);
            }
            0x05 => {
                let old_value = self.registers.b;
                self.registers.b = self.registers.b.wrapping_sub(1);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(Flag::Z, self.registers.b == 0);
                self.registers.f.set_flag(Flag::H, (old_value & 0x0F) == 0);
            }
            0x06 => {
                let value = self.read_immediate_byte();
                self.registers.b = value;
            }
            0x07 => {
                let old_carry = self.registers.f.get_flag(Flag::C);
                let a = self.registers.a;
                let new_carry = a & 0x80 != 0;
                self.registers.a = (a << 1) | (old_carry as u8);
                self.registers.f.set_flag(Flag::C, new_carry);
                self.registers.f.set_flag(Flag::Z, self.registers.a == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
            }
            0x08 => {
                let nn: u16 = self.read_immediate_short();
                panic!("0x08. nn: 0x{:X}", nn);
                self.write_immediate_short(nn, self.registers.sp);
            }
            0x09 => {
                let hl = self.registers.get_hl();
                let bc = self.registers.get_bc();
                let (result, carry) = hl.overflowing_add(bc);
                self.registers.set_hl(result);
                self.registers.f.set_flag(Flag::N, false);
                self.registers
                    .f
                    .set_flag(Flag::H, (hl & 0x0FFF) + (bc & 0x0FFF) > 0x0FFF);
                self.registers.f.set_flag(Flag::C, carry);
            }
            0x0A => {
                self.registers.a = self.memory_bus.read_byte(self.registers.get_bc());
            }
            0x0B => {
                self.registers.set_bc(self.registers.get_bc() - 1);
            }
            0x0C => {
                let old_value = self.registers.c;
                self.registers.c = self.registers.c.wrapping_add(1);
                self.registers.f.set_flag(Flag::Z, self.registers.c == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers
                    .f
                    .set_flag(Flag::H, (old_value & 0x0F) == 0x0F);
            }
            0x0D => {
                let old_value = self.registers.c;
                self.registers.c = self.registers.c.wrapping_sub(1);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(Flag::Z, self.registers.c == 0);
                self.registers.f.set_flag(Flag::H, (old_value & 0x0F) == 0);
            }
            0x0E => {
                let value = self.read_immediate_byte();
                self.registers.c = value;
            }
            0x0F => {
                let a = self.registers.a;
                let carry = (self.registers.a & 0x01) != 0;
                self.registers.a = (a >> 1) | (carry as u8) << 7;

                self.registers.f.set_flag(Flag::C, carry);

                self.registers.f.set_flag(Flag::Z, false);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
            }
            0x10 => {
                // TODO - Implement STOP
                error!("STOP not implemented");
                //unimplemented!("STOP not implemented");
            }
            0x11 => {
                let nn: u16 = self.read_immediate_short();
                self.registers.set_de(nn);
            }
            0x12 => {
                self.memory_bus.write_byte(self.registers.get_de(), self.registers.a);
            }
            0x13 => {
                self.registers.set_de(self.registers.get_de() + 1);
            }
            0x14 => {
                let old_value = self.registers.d;
                self.registers.d = self.registers.d.wrapping_add(1);
                self.registers.f.set_flag(Flag::Z, self.registers.d == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers
                    .f
                    .set_flag(Flag::H, (old_value & 0x0F) == 0x0F);
            }
            0x15 => {
                let old_value = self.registers.d;
                self.registers.d = self.registers.d.wrapping_sub(1);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(Flag::Z, self.registers.d == 0);
                self.registers.f.set_flag(Flag::H, (old_value & 0x0F) == 0);
            }
            0x16 => {
                let value = self.read_immediate_byte();
                self.registers.d = value;
            }
            0x17 => {
                let carry = self.registers.a & 0x80 != 0;
                self.registers.a = (self.registers.a << 1)
                    | (if self.registers.f.get_flag(Flag::C) {
                        1
                    } else {
                        0
                    });
                self.registers.f.set_flag(Flag::Z, self.registers.a == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, carry);
            }
            0x18 => {
                let offset = self.read_immediate_byte() as i8;
                self.op_jr_e(offset);
            }
            0x19 => {
                let hl = self.registers.get_hl();
                let de = self.registers.get_de();
                let (result, carry) = hl.overflowing_add(de);
                self.registers.set_hl(result);
                self.registers.f.set_flag(Flag::N, false);
                self.registers
                    .f
                    .set_flag(Flag::H, (hl & 0x0FFF) + (de & 0x0FFF) > 0x0FFF);
                self.registers.f.set_flag(Flag::C, carry);
            }
            0x1A => {
                self.registers.a = self.memory_bus.read_byte(self.registers.get_de());
            }
            0x1B => {
                self.registers.set_de(self.registers.get_de() - 1);
            }
            0x1C => {
                let old_value = self.registers.e;
                self.registers.e = self.registers.e.wrapping_add(1);
                self.registers.f.set_flag(Flag::Z, self.registers.e == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers
                    .f
                    .set_flag(Flag::H, (old_value & 0x0F) == 0x0F);
            }
            0x1D => {
                let old_value = self.registers.e;
                self.registers.e = self.registers.e.wrapping_sub(1);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(Flag::Z, self.registers.e == 0);
                self.registers.f.set_flag(Flag::H, (old_value & 0x0F) == 0);
            }
            0x1E => {
                let value = self.read_immediate_byte();
                self.registers.e = value;
            }
            0x1F => {
                let carry = self.registers.a & 0x01 != 0;
                self.registers.a = (self.registers.a >> 1)
                    | (if self.registers.f.get_flag(Flag::C) {
                        0x80
                    } else {
                        0
                    });
                self.registers.f.set_flag(Flag::Z, self.registers.a == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, carry);
            }
            0x20 => {
                let offset = self.read_immediate_byte() as i8;
                if !self.registers.f.get_flag(Flag::Z) {
                    self.op_jr_e(offset);
                }
            }
            0x21 => {
                let nn: u16 = self.read_immediate_short();
                self.registers.set_hl(nn);
            }
            0x22 => {
                self.memory_bus.write_byte(self.registers.get_hl(), self.registers.a);
                self.registers.set_hl(self.registers.get_hl() + 1);
            }
            0x23 => {
                self.registers.set_hl(self.registers.get_hl() + 1);
            }
            0x24 => {
                let old_value = self.registers.h;
                self.registers.h = self.registers.h.wrapping_add(1);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::Z, self.registers.h == 0);
                self.registers
                    .f
                    .set_flag(Flag::H, (old_value & 0x0F) == 0x0F);
            }
            0x25 => {
                let old_value = self.registers.h;
                self.registers.h = self.registers.h.wrapping_sub(1);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(Flag::Z, self.registers.h == 0);
                self.registers.f.set_flag(Flag::H, (old_value & 0x0F) == 0);
            }
            0x26 => {
                let value = self.read_immediate_byte();
                self.registers.h = value;
            }
            0x27 => {
                let mut a = self.registers.a;
                let mut adjust = 0;
                if self.registers.f.get_flag(Flag::H)
                    || (!self.registers.f.get_flag(Flag::N) && (a & 0x0F) > 9)
                {
                    adjust |= 0x06;
                }
                if self.registers.f.get_flag(Flag::C)
                    || (!self.registers.f.get_flag(Flag::N) && a > 0x99)
                {
                    adjust |= 0x60;
                    self.registers.f.set_flag(Flag::C, true);
                }
                a = a.wrapping_add(adjust);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::Z, a == 0);
                self.registers.a = a;
            }
            0x28 => {
                let offset = self.read_immediate_byte() as i8;
                if self.registers.f.get_flag(Flag::Z) {
                    self.op_jr_e(offset);
                }
            }
            0x29 => {
                let hl: u16 = self.registers.get_hl();
                let (result, carry) = hl.overflowing_add(hl);
                self.registers.set_hl(result);
                self.registers.f.set_flag(Flag::N, false);
                self.registers
                    .f
                    .set_flag(Flag::H, (hl & 0x0FFF) + (hl & 0x0FFF) > 0x0FFF);
                self.registers.f.set_flag(Flag::C, carry);
            }
            0x2A => {
                self.registers.a = self.memory_bus.read_byte(self.registers.get_hl());
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_add(1));
            }
            0x2B => {
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_sub(1));
            }
            0x2C => {
                let old_value = self.registers.l;
                self.registers.l = self.registers.l.wrapping_add(1);
                self.registers.f.set_flag(Flag::Z, self.registers.l == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers
                    .f
                    .set_flag(Flag::H, (old_value & 0x0F) == 0x0F);
            }
            0x2D => {
                let old_value = self.registers.l;
                self.registers.l = self.registers.l.wrapping_sub(1);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(Flag::Z, self.registers.l == 0);
                self.registers.f.set_flag(Flag::H, (old_value & 0x0F) == 0);
            }
            0x2E => {
                let value = self.read_immediate_byte();
                self.registers.l = value;
            }
            0x2F => {
                self.registers.a = !self.registers.a;
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(Flag::H, true);
            }
            0x30 => {
                let value = self.memory_bus.read_byte(self.registers.pc) as i8;
                if !self.registers.f.get_flag(Flag::C) {
                    self.registers.pc = self.registers.pc.wrapping_add(value as u16);
                }
            }
            0x31 => {
                let nn: u16 = self.read_immediate_short();
                self.registers.sp = nn;
            }
            0x32 => {
                self.memory_bus.write_byte(self.registers.get_hl(), self.registers.a);
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_sub(1));
            }
            0x33 => {
                self.registers.sp = self.registers.sp.wrapping_add(1);
            }
            0x34 => {
                let hl = self.registers.get_hl();
                let value = self.memory_bus.read_byte(self.registers.get_hl());
                self.registers.f.set_flag(Flag::H, (value & 0x0F) == 0x0F);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::C, value == 0xFF);
                self.memory_bus.write_byte(hl, value.wrapping_add(1));
                self.registers
                    .f
                    .set_flag(Flag::Z, self.memory_bus.read_byte(hl) == 0);
            }
            0x35 => {
                let hl = self.registers.get_hl();
                let value = self.memory_bus.read_byte(hl);
                self.registers.f.set_flag(Flag::H, (value & 0x0F) == 0x00);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(Flag::C, value == 0x00);
                self.memory_bus.write_byte(hl, value.wrapping_sub(1));
                self.registers
                    .f
                    .set_flag(Flag::Z, self.memory_bus.read_byte(hl) == 0);
            }
            0x36 => {
                let value = self.read_immediate_byte();
                self.memory_bus.write_byte(self.registers.get_hl(), value);
            }
            0x37 => {
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, true);
            }
            0x38 => {
                let offset = self.read_immediate_byte();
                if self.registers.f.get_flag(Flag::C) {
                    self.op_jr_e(offset as i8);
                }
            }
            0x39 => {
                let hl = self.registers.get_hl();
                let (result, carry) = hl.overflowing_add(self.registers.sp);
                self.registers.set_hl(result);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(
                    Flag::H,
                    (hl & 0x0FFF) + (self.registers.sp & 0x0FFF) > 0x0FFF,
                );
                self.registers.f.set_flag(Flag::C, carry);
            }
            0x3A => {
                self.registers.a = self.memory_bus.read_byte(self.registers.get_hl());
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_sub(1));
            }
            0x3B => {
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }
            0x3C => {
                let old_value = self.registers.a;
                self.registers.a = self.registers.a.wrapping_add(1);
                self.registers.f.set_flag(Flag::Z, self.registers.a == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers
                    .f
                    .set_flag(Flag::H, (old_value & 0x0F) == 0x0F);
            }
            0x3D => {
                let old_value = self.registers.a;
                self.registers.a = self.registers.a.wrapping_sub(1);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(Flag::Z, self.registers.a == 0);
                self.registers.f.set_flag(Flag::H, (old_value & 0x0F) == 0);
            }
            0x3E => {
                let value = self.read_immediate_byte();
                self.registers.a = value;
            }
            0x3F => {
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers
                    .f
                    .set_flag(Flag::C, !self.registers.f.get_flag(Flag::C));
            }
            0x40 => {
                self.registers.b = self.registers.b;
            }
            0x41 => {
                self.registers.b = self.registers.c;
            }
            0x42 => {
                self.registers.b = self.registers.d;
            }
            0x43 => {
                self.registers.b = self.registers.e;
            }
            0x44 => {
                self.registers.b = self.registers.h;
            }
            0x45 => {
                self.registers.b = self.registers.l;
            }
            0x46 => {
                self.registers.b = self.registers.get_hl() as u8;
            }
            0x47 => {
                self.registers.b = self.registers.a;
            }
            0x48 => {
                self.registers.c = self.registers.b;
            }
            0x49 => {
                self.registers.c = self.registers.c;
            }
            0x4A => {
                self.registers.c = self.registers.d;
            }
            0x4B => {
                self.registers.c = self.registers.e;
            }
            0x4C => {
                self.registers.c = self.registers.h;
            }
            0x4D => {
                self.registers.c = self.registers.l;
            }
            0x4E => {
                self.registers.c = self.registers.get_hl() as u8;
            }
            0x4F => {
                self.registers.c = self.registers.a;
            }
            0x50 => {
                self.registers.d = self.registers.b;
            }
            0x51 => {
                self.registers.d = self.registers.c;
            }
            0x52 => {
                self.registers.d = self.registers.d;
            }
            0x53 => {
                self.registers.d = self.registers.e;
            }
            0x54 => {
                self.registers.d = self.registers.h;
            }
            0x55 => {
                self.registers.d = self.registers.l;
            }
            0x56 => {
                self.registers.d = self.registers.get_hl() as u8;
            }
            0x57 => {
                self.registers.d = self.registers.a;
            }
            0x58 => {
                self.registers.e = self.registers.b;
            }
            0x59 => {
                self.registers.e = self.registers.c;
            }
            0x5A => {
                self.registers.e = self.registers.d;
            }
            0x5B => {
                self.registers.e = self.registers.e;
            }
            0x5C => {
                self.registers.e = self.registers.h;
            }
            0x5D => {
                self.registers.e = self.registers.l;
            }
            0x5E => {
                self.registers.e = self.registers.get_hl() as u8;
            }
            0x5F => {
                self.registers.e = self.registers.a;
            }
            0x60 => {
                self.registers.h = self.registers.b;
            }
            0x61 => {
                self.registers.h = self.registers.c;
            }
            0x62 => {
                self.registers.h = self.registers.d;
            }
            0x63 => {
                self.registers.h = self.registers.e;
            }
            0x64 => {
                self.registers.h = self.registers.h;
            }
            0x65 => {
                self.registers.h = self.registers.l;
            }
            0x66 => {
                self.registers.h = self.registers.get_hl() as u8;
            }
            0x67 => {
                self.registers.h = self.registers.a;
            }
            0x68 => {
                self.registers.l = self.registers.b;
            }
            0x69 => {
                self.registers.l = self.registers.c;
            }
            0x6A => {
                self.registers.l = self.registers.d;
            }
            0x6B => {
                self.registers.l = self.registers.e;
            }
            0x6C => {
                self.registers.l = self.registers.h;
            }
            0x6D => {
                self.registers.l = self.registers.l;
            }
            0x6E => {
                self.registers.l = self.registers.get_hl() as u8;
            }
            0x6F => {
                self.registers.l = self.registers.a;
            }
            0x70 => {
                self.memory_bus.write_byte(self.registers.get_hl(), self.registers.b);
            }
            0x71 => {
                self.memory_bus.write_byte(self.registers.get_hl(), self.registers.c);
            }
            0x72 => {
                self.memory_bus.write_byte(self.registers.get_hl(), self.registers.d);
            }
            0x73 => {
                self.memory_bus.write_byte(self.registers.get_hl(), self.registers.e);
            }
            0x74 => {
                self.memory_bus.write_byte(self.registers.get_hl(), self.registers.h);
            }
            0x75 => {
                self.memory_bus.write_byte(self.registers.get_hl(), self.registers.l);
            }
            0x76 => {
                //unimplemented!("HALT not implemented");
                //TODO
                error!("HALT not implemented");
            }
            0x77 => {
                self.memory_bus.write_byte(self.registers.get_hl(), self.registers.a);
            }
            0x78 => {
                self.registers.a = self.registers.b;
            }
            0x79 => {
                self.registers.a = self.registers.c;
            }
            0x7A => {
                self.registers.a = self.registers.d;
            }
            0x7B => {
                self.registers.a = self.registers.e;
            }
            0x7C => {
                self.registers.a = self.registers.h;
            }
            0x7D => {
                self.registers.a = self.registers.l;
            }
            0x7E => {
                self.registers.a = self.registers.get_hl() as u8;
            }
            0x7F => {
                self.registers.a = self.registers.a;
            }
            0x80 => {
                let (result, carry) = self.registers.a.overflowing_add(self.registers.b);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.b & 0x0F) > 0x0F,
                );
                self.registers.f.set_flag(Flag::C, carry);
                self.registers.a = result;
            }
            0x81 => {
                let (result, carry) = self.registers.a.overflowing_add(self.registers.c);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.c & 0x0F) > 0x0F,
                );
                self.registers.f.set_flag(Flag::C, carry);
                self.registers.a = result;
            }
            0x82 => {
                let (result, carry) = self.registers.a.overflowing_add(self.registers.d);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.d & 0x0F) > 0x0F,
                );
                self.registers.f.set_flag(Flag::C, carry);
                self.registers.a = result;
            }
            0x83 => {
                let (result, carry) = self.registers.a.overflowing_add(self.registers.e);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.e & 0x0F) > 0x0F,
                );
                self.registers.f.set_flag(Flag::C, carry);
                self.registers.a = result;
            }
            0x84 => {
                let (result, carry) = self.registers.a.overflowing_add(self.registers.h);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.h & 0x0F) > 0x0F,
                );
                self.registers.f.set_flag(Flag::C, carry);
                self.registers.a = result;
            }
            0x85 => {
                let (result, carry) = self.registers.a.overflowing_add(self.registers.l);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.l & 0x0F) > 0x0F,
                );
                self.registers.f.set_flag(Flag::C, carry);
                self.registers.a = result;
            }
            0x86 => {
                let value = self.memory_bus.read_byte(self.registers.get_hl());
                let (result, carry) = self.registers.a.overflowing_add(value);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers
                    .f
                    .set_flag(Flag::H, (self.registers.a & 0x0F) + (value & 0x0F) > 0x0F);
                self.registers.f.set_flag(Flag::C, carry);
                self.registers.a = result;
            }
            0x87 => {
                let (result, carry) = self.registers.a.overflowing_add(self.registers.a);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.a & 0x0F) > 0x0F,
                );
                self.registers.f.set_flag(Flag::C, carry);
                self.registers.a = result;
            }
            0x88 => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_add(self.registers.b)
                    .wrapping_add(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false); // Clear the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.b & 0x0F) + carry > 0x0F,
                ); // Set the half-carry flag if there's a carry from bit 3
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) + (self.registers.b as u16) + (carry as u16) > 0xFF,
                ); // Set the carry flag if there's a carry out of the most significant bit
                self.registers.a = result;
            }
            0x89 => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_add(self.registers.c)
                    .wrapping_add(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false); // Clear the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.c & 0x0F) + carry > 0x0F,
                ); // Set the half-carry flag if there's a carry from bit 3
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) + (self.registers.c as u16) + (carry as u16) > 0xFF,
                ); // Set the carry flag if there's a carry out of the most significant bit
                self.registers.a = result;
            }
            0x8A => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_add(self.registers.d)
                    .wrapping_add(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false); // Clear the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.d & 0x0F) + carry > 0x0F,
                ); // Set the half-carry flag if there's a carry from bit 3
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) + (self.registers.d as u16) + (carry as u16) > 0xFF,
                ); // Set the carry flag if there's a carry out of the most significant bit
                self.registers.a = result;
            }
            0x8B => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_add(self.registers.e)
                    .wrapping_add(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false); // Clear the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.e & 0x0F) + carry > 0x0F,
                ); // Set the half-carry flag if there's a carry from bit 3
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) + (self.registers.e as u16) + (carry as u16) > 0xFF,
                ); // Set the carry flag if there's a carry out of the most significant bit
                self.registers.a = result;
            }
            0x8C => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_add(self.registers.h)
                    .wrapping_add(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false); // Clear the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.h & 0x0F) + carry > 0x0F,
                ); // Set the half-carry flag if there's a carry from bit 3
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) + (self.registers.h as u16) + (carry as u16) > 0xFF,
                ); // Set the carry flag if there's a carry out of the most significant bit
                self.registers.a = result;
            }
            0x8D => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_add(self.registers.l)
                    .wrapping_add(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false); // Clear the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.l & 0x0F) + carry > 0x0F,
                ); // Set the half-carry flag if there's a carry from bit 3
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) + (self.registers.l as u16) + (carry as u16) > 0xFF,
                ); // Set the carry flag if there's a carry out of the most significant bit
                self.registers.a = result;
            }
            0x8E => {
                let value = self.memory_bus.read_byte(self.registers.get_hl());
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self.registers.a.wrapping_add(value).wrapping_add(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false); // Clear the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (value & 0x0F) + carry > 0x0F,
                ); // Set the half-carry flag if there's a carry from bit 3
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) + (value as u16) + (carry as u16) > 0xFF,
                ); // Set the carry flag if there's a carry out of the most significant bit
                self.registers.a = result;
            }
            0x8F => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_add(self.registers.a)
                    .wrapping_add(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false); // Clear the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.a & 0x0F) + carry > 0x0F,
                ); // Set the half-carry flag if there's a carry from bit 3
                self.registers
                    .f
                    .set_flag(Flag::C, result < self.registers.a);
                self.registers.a = result;
            }
            0x90 => {
                let (result, borrow) = self.registers.a.overflowing_sub(self.registers.b);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.b & 0x0F),
                );
                self.registers.f.set_flag(Flag::C, borrow);
                self.registers.a = result;
            }
            0x91 => {
                let (result, borrow) = self.registers.a.overflowing_sub(self.registers.c);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.c & 0x0F),
                );
                self.registers.f.set_flag(Flag::C, borrow);
                self.registers.a = result;
            }
            0x92 => {
                let (result, borrow) = self.registers.a.overflowing_sub(self.registers.d);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.d & 0x0F),
                );
                self.registers.f.set_flag(Flag::C, borrow);
                self.registers.a = result;
            }
            0x93 => {
                let (result, borrow) = self.registers.a.overflowing_sub(self.registers.e);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.e & 0x0F),
                );
                self.registers.f.set_flag(Flag::C, borrow);
                self.registers.a = result;
            }
            0x94 => {
                let (result, borrow) = self.registers.a.overflowing_sub(self.registers.h);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.h & 0x0F),
                );
                self.registers.f.set_flag(Flag::C, borrow);
                self.registers.a = result;
            }
            0x95 => {
                let (result, borrow) = self.registers.a.overflowing_sub(self.registers.l);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.l & 0x0F),
                );
                self.registers.f.set_flag(Flag::C, borrow);
                self.registers.a = result;
            }
            0x96 => {
                let value = self.memory_bus.read_byte(self.registers.get_hl());
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self.registers.a.wrapping_sub(value).wrapping_sub(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers
                    .f
                    .set_flag(Flag::H, (self.registers.a & 0x0F) < (value & 0x0F) + carry); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) < (value as u16) + (carry as u16),
                ); // Set the carry flag if there's a borrow out of the most significant bit
                self.registers.a = result;
            }
            0x97 => {
                let (result, borrow) = self.registers.a.overflowing_sub(self.registers.a);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.a & 0x0F),
                );
                self.registers.f.set_flag(Flag::C, borrow);
                self.registers.a = result;
            }
            0x98 => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_sub(self.registers.b)
                    .wrapping_sub(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.b & 0x0F) + carry,
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) < (self.registers.b as u16) + (carry as u16),
                ); // Set the carry flag if there's a borrow out of the most significant bit
                self.registers.a = result;
            }
            0x99 => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_sub(self.registers.c)
                    .wrapping_sub(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.c & 0x0F) + carry,
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) < (self.registers.c as u16) + (carry as u16),
                ); // Set the carry flag if there's a borrow out of the most significant bit
                self.registers.a = result;
            }
            0x9A => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_sub(self.registers.d)
                    .wrapping_sub(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.d & 0x0F) + carry,
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) < (self.registers.d as u16) + (carry as u16),
                ); // Set the carry flag if there's a borrow out of the most significant bit
                self.registers.a = result;
            }
            0x9B => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_sub(self.registers.e)
                    .wrapping_sub(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.e & 0x0F) + carry,
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) < (self.registers.e as u16) + (carry as u16),
                ); // Set the carry flag if there's a borrow out of the most significant bit
                self.registers.a = result;
            }
            0x9C => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_sub(self.registers.h)
                    .wrapping_sub(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.h & 0x0F) + carry,
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) < (self.registers.h as u16) + (carry as u16),
                ); // Set the carry flag if there's a borrow out of the most significant bit
                self.registers.a = result;
            }
            0x9D => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_sub(self.registers.l)
                    .wrapping_sub(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.l & 0x0F) + carry,
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) < (self.registers.l as u16) + (carry as u16),
                ); // Set the carry flag if there's a borrow out of the most significant bit
                self.registers.a = result;
            }
            0x9E => {
                let value = self.memory_bus.read_byte(self.registers.get_hl());
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self.registers.a.wrapping_sub(value).wrapping_sub(carry);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers
                    .f
                    .set_flag(Flag::H, (self.registers.a & 0x0F) < (value & 0x0F) + carry); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16) < (value as u16) + (carry as u16),
                ); // Set the carry flag if there's a borrow out of the most significant bit
                self.registers.a = result;
            }
            0x9F => {
                let result = self.registers.a.wrapping_sub(self.registers.a);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(Flag::H, false); // Clear the half-carry flag
                self.registers.f.set_flag(Flag::C, false); // Clear the carry flag
                self.registers.a = result;
            }
            0xA0 => {
                self.registers.a = self.registers.a & self.registers.b;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, true); // TODO why are we setting this to true for and ops?
                self.registers.f.set_flag(Flag::C, false);
            }
            0xA1 => {
                self.registers.a = self.registers.a & self.registers.c;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, true); // TODO why are we setting this to true for and ops?
                self.registers.f.set_flag(Flag::C, false);
            }
            0xA2 => {
                self.registers.a = self.registers.a & self.registers.d;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, true); // TODO why are we setting this to true for and ops?
                self.registers.f.set_flag(Flag::C, false);
            }
            0xA3 => {
                self.registers.a = self.registers.a & self.registers.e;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, true); // TODO why are we setting this to true for and ops?
                self.registers.f.set_flag(Flag::C, false);
            }
            0xA4 => {
                self.registers.a = self.registers.a & self.registers.h;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, true); // TODO why are we setting this to true for and ops?
                self.registers.f.set_flag(Flag::C, false);
            }
            0xA5 => {
                self.registers.a = self.registers.a & self.registers.l;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, true); // TODO why are we setting this to true for and ops?
                self.registers.f.set_flag(Flag::C, false);
            }
            0xA6 => {
                self.registers.a =
                    self.registers.a & self.memory_bus.read_byte(self.registers.get_hl());

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, true); // TODO why are we setting this to true for and ops?
                self.registers.f.set_flag(Flag::C, false);
            }
            0xA7 => {
                self.registers.a = self.registers.a & self.registers.a;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, true); // TODO why are we setting this to true for and ops?
                self.registers.f.set_flag(Flag::C, false);
            }
            0xA8 => {
                self.registers.a = self.registers.a ^ self.registers.b;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xA9 => {
                self.registers.a = self.registers.a ^ self.registers.c;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xAA => {
                self.registers.a = self.registers.a ^ self.registers.d;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xAB => {
                self.registers.a = self.registers.a ^ self.registers.e;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xAC => {
                self.registers.a = self.registers.a ^ self.registers.h;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xAD => {
                self.registers.a = self.registers.a ^ self.registers.l;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xAE => {
                self.registers.a =
                    self.registers.a ^ self.memory_bus.read_byte(self.registers.get_hl());

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xAF => {
                self.registers.a = 0x0000;
                self.registers.f.set_flag(Flag::Z, true);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xB0 => {
                self.registers.a = self.registers.a | self.registers.b;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xB1 => {
                self.registers.a = self.registers.a | self.registers.c;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xB2 => {
                self.registers.a = self.registers.a | self.registers.d;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xB3 => {
                self.registers.a = self.registers.a | self.registers.e;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xB4 => {
                self.registers.a = self.registers.a | self.registers.h;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xB5 => {
                self.registers.a = self.registers.a | self.registers.l;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xB6 => {
                self.registers.a =
                    self.registers.a | self.memory_bus.read_byte(self.registers.get_hl());

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xB7 => {
                self.registers.a = self.registers.a | self.registers.a;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xB8 => {
                let result = self.registers.a.wrapping_sub(self.registers.b);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.b & 0x0F),
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers
                    .f
                    .set_flag(Flag::C, self.registers.a < self.registers.b); // Set the carry flag if there's a borrow out of the most significant bit
            }
            0xB9 => {
                let result = self.registers.a.wrapping_sub(self.registers.c);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.c & 0x0F),
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers
                    .f
                    .set_flag(Flag::C, self.registers.a < self.registers.c); // Set the carry flag if there's a borrow out of the most significant bit
            }
            0xBA => {
                let result = self.registers.a.wrapping_sub(self.registers.d);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.d & 0x0F),
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers
                    .f
                    .set_flag(Flag::C, self.registers.a < self.registers.d); // Set the carry flag if there's a borrow out of the most significant bit
            }
            0xBB => {
                let result = self.registers.a.wrapping_sub(self.registers.e);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.e & 0x0F),
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers
                    .f
                    .set_flag(Flag::C, self.registers.a < self.registers.e); // Set the carry flag if there's a borrow out of the most significant bit
            }
            0xBC => {
                let result = self.registers.a.wrapping_sub(self.registers.h);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.h & 0x0F),
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers
                    .f
                    .set_flag(Flag::C, self.registers.a < self.registers.h); // Set the carry flag if there's a borrow out of the most significant bit
            }
            0xBD => {
                let result = self.registers.a.wrapping_sub(self.registers.l);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) < (self.registers.l & 0x0F),
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers
                    .f
                    .set_flag(Flag::C, self.registers.a < self.registers.l); // Set the carry flag if there's a borrow out of the most significant bit
            }
            0xBE => {
                let value = self.memory_bus.read_byte(self.registers.get_hl());
                let result = self.registers.a.wrapping_sub(value);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers
                    .f
                    .set_flag(Flag::H, (self.registers.a & 0x0F) < (value & 0x0F)); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(Flag::C, self.registers.a < value); // Set the carry flag if there's a borrow out of the most significant bit
            }
            0xBF => {
                let result = self.registers.a.wrapping_sub(self.registers.a);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(Flag::H, false); // Clear the half-carry flag
                self.registers.f.set_flag(Flag::C, false); // Clear the carry flag
            }
            0xC0 => {
                if !self.registers.f.get_flag(Flag::Z) {
                    self.op_ret();
                }
            }
            0xC1 => {
                self.op_push_stack(self.registers.get_bc());
            }
            0xC2 => {
                let nn: u16 = self.read_immediate_short();
                self.op_jp_nn(nn);
            }
            0xC3 => {
                let nn: u16 = self.read_immediate_short();
                debug!("Jumping to 0x{:X}", nn);
                self.op_jp_nn(nn);
            }
            0xC4 => {
                let nn: u16 = self.read_immediate_short();
                self.op_call_nn(nn);
            }
            0xC5 => {
                self.op_push_stack(self.registers.get_bc());
            }
            0xC6 => {
                let value = self.read_immediate_byte();
                self.op_add_a_n8(value);
            }
            0xC7 => {
                self.op_rst_address(0x0000);
            }
            0xC8 => {
                if self.registers.f.get_flag(Flag::Z) {
                    self.op_ret();
                }
            }
            0xC9 => {
                self.op_ret();
            }
            0xCA => {
                let nn: u16 = self.read_immediate_short();
                self.op_jp_nn(nn);
            }
            0xCB => {
                // Get the next byte and use it as the extended opcode
                let extended_opcode = self.read_immediate_byte();
                debug!("0xCB{:X}", extended_opcode);
                match extended_opcode {
                    0x00 => {
                        let mut value = self.registers.b;
                        self.op_rlc(&mut value);
                        self.registers.b = value;
                    }
                    0x01 => {
                        let mut value = self.registers.c;
                        self.op_rlc(&mut value);
                        self.registers.c = value;
                    }
                    0x02 => {
                        let mut value = self.registers.d;
                        self.op_rlc(&mut value);
                        self.registers.d = value;
                    }
                    0x03 => {
                        let mut value = self.registers.e;
                        self.op_rlc(&mut value);
                        self.registers.e = value;
                    }
                    0x04 => {
                        let mut value = self.registers.h;
                        self.op_rlc(&mut value);
                        self.registers.h = value;
                    }
                    0x05 => {
                        let mut value = self.registers.l;
                        self.op_rlc(&mut value);
                        self.registers.l = value;
                    }
                    0x06 => {
                        let mut value = self.memory_bus.read_byte(self.registers.get_hl());
                        self.op_rlc(&mut value);
                        self.memory_bus.write_byte(self.registers.get_hl(), value);
                    }
                    0x07 => {
                        let mut value = self.registers.a;
                        self.op_rlc(&mut value);
                        self.registers.a = value;
                    }
                    0x08 => {
                        let mut value = self.registers.b;
                        self.op_rrc(&mut value);
                        self.registers.b = value;
                    }
                    0x09 => {
                        let mut value = self.registers.c;
                        self.op_rrc(&mut value);
                        self.registers.c = value;
                    }
                    0x0A => {
                        let mut value = self.registers.d;
                        self.op_rrc(&mut value);
                        self.registers.d = value;
                    }
                    0x0B => {
                        let mut value = self.registers.e;
                        self.op_rrc(&mut value);
                        self.registers.e = value;
                    }
                    0x0C => {
                        let mut value = self.registers.h;
                        self.op_rrc(&mut value);
                        self.registers.h = value;
                    }
                    0x0D => {
                        let mut value = self.registers.l;
                        self.op_rrc(&mut value);
                        self.registers.l = value;
                    }
                    0x0E => {
                        let mut value = self.memory_bus.read_byte(self.registers.get_hl());
                        self.op_rrc(&mut value);
                        self.memory_bus.write_byte(self.registers.get_hl(), value);
                    }
                    0x0F => {
                        let mut value = self.registers.a;
                        self.op_rrc(&mut value);
                        self.registers.a = value;
                    }
                    0x10 => {
                        let mut value = self.registers.b;
                        self.op_rl(&mut value);
                        self.registers.b = value;
                    }
                    0x11 => {
                        let mut value = self.registers.c;
                        self.op_rl(&mut value);
                        self.registers.c = value;
                    }
                    0x12 => {
                        let mut value = self.registers.d;
                        self.op_rl(&mut value);
                        self.registers.d = value;
                    }
                    0x13 => {
                        let mut value = self.registers.e;
                        self.op_rl(&mut value);
                        self.registers.e = value;
                    }
                    0x14 => {
                        let mut value = self.registers.h;
                        self.op_rl(&mut value);
                        self.registers.h = value;
                    }
                    0x15 => {
                        let mut value = self.registers.l;
                        self.op_rl(&mut value);
                        self.registers.l = value;
                    }
                    0x16 => {
                        let mut value = self.memory_bus.read_byte(self.registers.get_hl());
                        self.op_rl(&mut value);
                        self.memory_bus.write_byte(self.registers.get_hl() , value);
                    }
                    0x17 => {
                        let mut value = self.registers.a;
                        self.op_rl(&mut value);
                        self.registers.a = value;
                    }
                    0x18 => {
                        let mut value = self.registers.b;
                        self.op_rr(&mut value);
                        self.registers.b = value;
                    }
                    0x19 => {
                        let mut value = self.registers.c;
                        self.op_rr(&mut value);
                        self.registers.c = value;
                    }
                    0x1A => {
                        let mut value = self.registers.d;
                        self.op_rr(&mut value);
                        self.registers.d = value;
                    }
                    0x1B => {
                        let mut value = self.registers.e;
                        self.op_rr(&mut value);
                        self.registers.e = value;
                    }
                    0x1C => {
                        let mut value = self.registers.h;
                        self.op_rr(&mut value);
                        self.registers.h = value;
                    }
                    0x1D => {
                        let mut value = self.registers.l;
                        self.op_rr(&mut value);
                        self.registers.l = value;
                    }
                    0x1E => {
                        let mut value = self.memory_bus.read_byte(self.registers.get_hl());
                        self.op_rr(&mut value);
                        self.memory_bus.write_byte(self.registers.get_hl(), value);
                    }
                    0x1F => {
                        let mut value = self.registers.a;
                        self.op_rr(&mut value);
                        self.registers.a = value;
                    }
                    0x20 => {
                        let mut value = self.registers.b;
                        self.op_sla(&mut value);
                        self.registers.b = value;
                    }
                    0x21 => {
                        let mut value = self.registers.c;
                        self.op_sla(&mut value);
                        self.registers.c = value;
                    }
                    0x22 => {
                        let mut value = self.registers.d;
                        self.op_sla(&mut value);
                        self.registers.d = value;
                    }
                    0x23 => {
                        let mut value = self.registers.e;
                        self.op_sla(&mut value);
                        self.registers.e = value;
                    }
                    0x24 => {
                        let mut value = self.registers.h;
                        self.op_sla(&mut value);
                        self.registers.h = value;
                    }
                    0x25 => {
                        let mut value = self.registers.l;
                        self.op_sla(&mut value);
                        self.registers.l = value;
                    }
                    0x26 => {
                        let mut value = self.memory_bus.read_byte(self.registers.get_hl());
                        self.op_sla(&mut value);
                        self.memory_bus.write_byte(self.registers.get_hl(), value);
                    }
                    0x27 => {
                        let mut value = self.registers.a;
                        self.op_sla(&mut value);
                        self.registers.a = value;
                    }
                    0x28 => {
                        let mut value = self.registers.b;
                        self.op_sra(&mut value);
                        self.registers.b = value;
                    }
                    0x29 => {
                        let mut value = self.registers.c;
                        self.op_sra(&mut value);
                        self.registers.c = value;
                    }
                    0x2A => {
                        let mut value = self.registers.d;
                        self.op_sra(&mut value);
                        self.registers.d = value;
                    }
                    0x2B => {
                        let mut value = self.registers.e;
                        self.op_sra(&mut value);
                        self.registers.e = value;
                    }
                    0x2C => {
                        let mut value = self.registers.h;
                        self.op_sra(&mut value);
                        self.registers.h = value;
                    }
                    0x2D => {
                        let mut value = self.registers.l;
                        self.op_sra(&mut value);
                        self.registers.l = value;
                    }
                    0x2E => {
                        let mut value = self.memory_bus.read_byte(self.registers.get_hl());
                        self.op_sra(&mut value);
                        self.memory_bus.write_byte(self.registers.get_hl(), value);
                    }
                    0x2F => {
                        let mut value = self.registers.a;
                        self.op_sra(&mut value);
                        self.registers.a = value;
                    }
                    0x30 => {
                        let mut value = self.registers.b;
                        self.op_swap(&mut value);
                        self.registers.b = value;
                    }
                    0x31 => {
                        let mut value = self.registers.c;
                        self.op_swap(&mut value);
                        self.registers.c = value;
                    }
                    0x32 => {
                        let mut value = self.registers.d;
                        self.op_swap(&mut value);
                        self.registers.d = value;
                    }
                    0x33 => {
                        let mut value = self.registers.e;
                        self.op_swap(&mut value);
                        self.registers.e = value;
                    }
                    0x34 => {
                        let mut value = self.registers.h;
                        self.op_swap(&mut value);
                        self.registers.h = value;
                    }
                    0x35 => {
                        let mut value = self.registers.l;
                        self.op_swap(&mut value);
                        self.registers.l = value;
                    }
                    0x36 => {
                        let mut value = self.memory_bus.read_byte(self.registers.get_hl());
                        self.op_swap(&mut value);
                        self.memory_bus.write_byte(self.registers.get_hl(), value);
                    }
                    0x37 => {
                        let mut value = self.registers.a;
                        self.op_swap(&mut value);
                        self.registers.a = value;
                    }
                    0x38 => {
                        let mut value = self.registers.b;
                        self.op_srl(&mut value);
                        self.registers.b = value;
                    }
                    0x39 => {
                        let mut value = self.registers.c;
                        self.op_srl(&mut value);
                        self.registers.c = value;
                    }
                    0x3A => {
                        let mut value = self.registers.d;
                        self.op_srl(&mut value);
                        self.registers.d = value;
                    }
                    0x3B => {
                        let mut value = self.registers.e;
                        self.op_srl(&mut value);
                        self.registers.e = value;
                    }
                    0x3C => {
                        let mut value = self.registers.h;
                        self.op_srl(&mut value);
                        self.registers.h = value;
                    }
                    0x3D => {
                        let mut value = self.registers.l;
                        self.op_srl(&mut value);
                        self.registers.l = value;
                    }
                    0x3E => {
                        let mut value = self.memory_bus.read_byte(self.registers.get_hl());
                        self.op_srl(&mut value);
                        self.memory_bus.write_byte(self.registers.get_hl(), value);
                    }
                    0x3F => {
                        let mut value = self.registers.a;
                        self.op_srl(&mut value);
                        self.registers.a = value;
                    }
                    0x40 => {
                        self.op_bit(0, self.registers.b);
                    }
                    0x41 => {
                        self.op_bit(0, self.registers.c);
                    }
                    0x42 => {
                        self.op_bit(0, self.registers.d);
                    }
                    0x43 => {
                        self.op_bit(0, self.registers.e);
                    }
                    0x44 => {
                        self.op_bit(0, self.registers.h);
                    }
                    0x45 => {
                        self.op_bit(0, self.registers.l);
                    }
                    0x46 => {
                        self.op_bit(0, self.memory_bus.read_byte(self.registers.get_hl()));
                    }
                    0x47 => {
                        self.op_bit(0, self.registers.a);
                    }
                    0x48 => {
                        self.op_bit(1, self.registers.b);
                    }
                    0x49 => {
                        self.op_bit(1, self.registers.c);
                    }
                    0x4A => {
                        self.op_bit(1, self.registers.d);
                    }
                    0x4B => {
                        self.op_bit(1, self.registers.e);
                    }
                    0x4C => {
                        self.op_bit(1, self.registers.h);
                    }
                    0x4D => {
                        self.op_bit(1, self.registers.l);
                    }
                    0x4E => {
                        self.op_bit(1, self.memory_bus.read_byte(self.registers.get_hl()));
                    }
                    0x4F => {
                        self.op_bit(1, self.registers.a);
                    }
                    0x50 => {
                        self.op_bit(2, self.registers.b);
                    }
                    0x51 => {
                        self.op_bit(2, self.registers.c);
                    }
                    0x52 => {
                        self.op_bit(2, self.registers.d);
                    }
                    0x53 => {
                        self.op_bit(2, self.registers.e);
                    }
                    0x54 => {
                        self.op_bit(2, self.registers.h);
                    }
                    0x55 => {
                        self.op_bit(2, self.registers.l);
                    }
                    0x56 => {
                        self.op_bit(2, self.memory_bus.read_byte(self.registers.get_hl()));
                    }
                    0x57 => {
                        self.op_bit(2, self.registers.a);
                    }
                    0x58 => {
                        self.op_bit(3, self.registers.b);
                    }
                    0x59 => {
                        self.op_bit(3, self.registers.c);
                    }
                    0x5A => {
                        self.op_bit(3, self.registers.d);
                    }
                    0x5B => {
                        self.op_bit(3, self.registers.e);
                    }
                    0x5C => {
                        self.op_bit(3, self.registers.h);
                    }
                    0x5D => {
                        self.op_bit(3, self.registers.l);
                    }
                    0x5E => {
                        //self.op_bit(3, self.work_ram[self.registers.get_hl() as usize]);
                        self.op_bit(3,
                                    self.memory_bus.read_byte(self.registers.get_hl())
                        );
                    }
                    0x5F => {
                        self.op_bit(3, self.registers.a);
                    }
                    0x60 => {
                        self.op_bit(4, self.registers.b);
                    }
                    0x61 => {
                        self.op_bit(4, self.registers.c);
                    }
                    0x62 => {
                        self.op_bit(4, self.registers.d);
                    }
                    0x63 => {
                        self.op_bit(4, self.registers.e);
                    }
                    0x64 => {
                        self.op_bit(4, self.registers.h);
                    }
                    0x65 => {
                        self.op_bit(4, self.registers.l);
                    }
                    0x66 => {
                        self.op_bit(4, self.memory_bus.read_byte(self.registers.get_hl()));
                    }
                    0x67 => {
                        self.op_bit(4, self.registers.a);
                    }
                    0x68 => {
                        self.op_bit(5, self.registers.b);
                    }
                    0x69 => {
                        self.op_bit(5, self.registers.c);
                    }
                    0x6A => {
                        self.op_bit(5, self.registers.d);
                    }
                    0x6B => {
                        self.op_bit(5, self.registers.e);
                    }
                    0x6C => {
                        self.op_bit(5, self.registers.h);
                    }
                    0x6D => {
                        self.op_bit(5, self.registers.l);
                    }
                    0x6E => {
                        self.op_bit(5, self.memory_bus.read_byte(self.registers.get_hl()));
                    }
                    0x6F => {
                        self.op_bit(5, self.registers.a);
                    }
                    0x70 => {
                        self.op_bit(6, self.registers.b);
                    }
                    0x71 => {
                        self.op_bit(6, self.registers.c);
                    }
                    0x72 => {
                        self.op_bit(6, self.registers.d);
                    }
                    0x73 => {
                        self.op_bit(6, self.registers.e);
                    }
                    0x74 => {
                        self.op_bit(6, self.registers.h);
                    }
                    0x75 => {
                        self.op_bit(6, self.registers.l);
                    }
                    0x76 => {
                        self.op_bit(6, self.memory_bus.read_byte(self.registers.get_hl()));
                    }
                    0x77 => {
                        self.op_bit(6, self.registers.a);
                    }
                    0x78 => {
                        self.op_bit(7, self.registers.b);
                    }
                    0x79 => {
                        self.op_bit(7, self.registers.c);
                    }
                    0x7A => {
                        self.op_bit(7, self.registers.d);
                    }
                    0x7B => {
                        self.op_bit(7, self.registers.e);
                    }
                    0x7C => {
                        self.op_bit(7, self.registers.h);
                    }
                    0x7D => {
                        self.op_bit(7, self.registers.l);
                    }
                    0x7E => {
                        self.op_bit(7, self.memory_bus.read_byte(self.registers.get_hl()));
                    }
                    0x7F => {
                        self.op_bit(7, self.registers.a);
                    }
                    0x80 => {
                        self.registers.b &= !(1 << 0);
                    }
                    0x81 => {
                        self.registers.c &= !(1 << 0);
                    }
                    0x82 => {
                        self.registers.d &= !(1 << 0);
                    }
                    0x83 => {
                        self.registers.e &= !(1 << 0);
                    }
                    0x84 => {
                        self.registers.h &= !(1 << 0);
                    }
                    0x85 => {
                        self.registers.l &= !(1 << 0);
                    }
                    0x86 => {
                        // TODO test this
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        //self.work_ram[self.registers.get_hl() as usize] &= value & !(1 << 0);
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());;
                        result &= value & !(1 << 0);
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0x87 => {
                        self.registers.a &= !(1 << 0);
                    }
                    0x88 => {
                        self.registers.b &= !(1 << 1);
                    }
                    0x89 => {
                        self.registers.c &= !(1 << 1);
                    }
                    0x8A => {
                        self.registers.d &= !(1 << 1);
                    }
                    0x8B => {
                        self.registers.e &= !(1 << 1);
                    }
                    0x8C => {
                        self.registers.h &= !(1 << 1);
                    }
                    0x8D => {
                        self.registers.l &= !(1 << 1);
                    }
                    0x8E => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());;
                        result &= value & !(1 << 1);
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0x8F => {
                        self.registers.a &= !(1 << 1);
                    }
                    0x90 => {
                        self.registers.b &= !(1 << 2);
                    }
                    0x91 => {
                        self.registers.c &= !(1 << 2);
                    }
                    0x92 => {
                        self.registers.d &= !(1 << 2);
                    }
                    0x93 => {
                        self.registers.e &= !(1 << 2);
                    }
                    0x94 => {
                        self.registers.h &= !(1 << 2);
                    }
                    0x95 => {
                        self.registers.l &= !(1 << 2);
                    }
                    0x96 => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());;
                        result &= value & !(1 << 2);
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0x97 => {
                        self.registers.a &= !(1 << 2);
                    }
                    0x98 => {
                        self.registers.b &= !(1 << 3);
                    }
                    0x99 => {
                        self.registers.c &= !(1 << 3);
                    }
                    0x9A => {
                        self.registers.d &= !(1 << 3);
                    }
                    0x9B => {
                        self.registers.e &= !(1 << 3);
                    }
                    0x9C => {
                        self.registers.h &= !(1 << 3);
                    }
                    0x9D => {
                        self.registers.l &= !(1 << 3);
                    }
                    0x9E => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());;
                        result &= value & !(1 << 3);
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0x9F => {
                        self.registers.a &= !(1 << 3);
                    }
                    0xA0 => {
                        self.registers.b &= !(1 << 4);
                    }
                    0xA1 => {
                        self.registers.c &= !(1 << 4);
                    }
                    0xA2 => {
                        self.registers.d &= !(1 << 4);
                    }
                    0xA3 => {
                        self.registers.e &= !(1 << 4);
                    }
                    0xA4 => {
                        self.registers.h &= !(1 << 4);
                    }
                    0xA5 => {
                        self.registers.l &= !(1 << 4);
                    }
                    0xA6 => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());;
                        result &= value & !(1 << 4);
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0xA7 => {
                        self.registers.a &= !(1 << 4);
                    }
                    0xA8 => {
                        self.registers.b &= !(1 << 5);
                    }
                    0xA9 => {
                        self.registers.c &= !(1 << 5);
                    }
                    0xAA => {
                        self.registers.d &= !(1 << 5);
                    }
                    0xAB => {
                        self.registers.e &= !(1 << 5);
                    }
                    0xAC => {
                        self.registers.h &= !(1 << 5);
                    }
                    0xAD => {
                        self.registers.l &= !(1 << 5);
                    }
                    0xAE => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());;
                        result &= value & !(1 << 5);
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0xAF => {
                        self.registers.a &= !(1 << 5);
                    }
                    0xB0 => {
                        self.registers.b &= !(1 << 6);
                    }
                    0xB1 => {
                        self.registers.c &= !(1 << 6);
                    }
                    0xB2 => {
                        self.registers.d &= !(1 << 6);
                    }
                    0xB3 => {
                        self.registers.e &= !(1 << 6);
                    }
                    0xB4 => {
                        self.registers.h &= !(1 << 6);
                    }
                    0xB5 => {
                        self.registers.l &= !(1 << 6);
                    }
                    0xB6 => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());;
                        result &= value & !(1 << 6);
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0xB7 => {
                        self.registers.a &= !(1 << 6);
                    }
                    0xB8 => {
                        self.registers.b &= !(1 << 7);
                    }
                    0xB9 => {
                        self.registers.c &= !(1 << 7);
                    }
                    0xBA => {
                        self.registers.d &= !(1 << 7);
                    }
                    0xBB => {
                        self.registers.e &= !(1 << 7);
                    }
                    0xBC => {
                        self.registers.h &= !(1 << 7);
                    }
                    0xBD => {
                        self.registers.l &= !(1 << 7);
                    }
                    0xBE => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());;
                        result &= value & !(1 << 7);
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0xBF => {
                        self.registers.a &= !(1 << 7);
                    }
                    0xC0 => {
                        self.registers.b |= 1 << 0;
                    }
                    0xC1 => {
                        self.registers.c |= 1 << 0;
                    }
                    0xC2 => {
                        self.registers.d |= 1 << 0;
                    }
                    0xC3 => {
                        self.registers.e |= 1 << 0;
                    }
                    0xC4 => {
                        self.registers.h |= 1 << 0;
                    }
                    0xC5 => {
                        self.registers.l |= 1 << 0;
                    }
                    0xC6 => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());
                        result |= value | 1 << 0;
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0xC7 => {
                        self.registers.a |= 1 << 0;
                    }
                    0xC8 => {
                        self.registers.b |= 1 << 1;
                    }
                    0xC9 => {
                        self.registers.c |= 1 << 1;
                    }
                    0xCA => {
                        self.registers.d |= 1 << 1;
                    }
                    0xCB => {
                        self.registers.e |= 1 << 1;
                    }
                    0xCC => {
                        self.registers.h |= 1 << 1;
                    }
                    0xCD => {
                        self.registers.l |= 1 << 1;
                    }
                    0xCE => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());
                        result |= value | 1 << 1;
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0xCF => {
                        self.registers.a |= 1 << 1;
                    }
                    0xD0 => {
                        self.registers.b |= 1 << 2;
                    }
                    0xD1 => {
                        self.registers.c |= 1 << 2;
                    }
                    0xD2 => {
                        self.registers.d |= 1 << 2;
                    }
                    0xD3 => {
                        self.registers.e |= 1 << 2;
                    }
                    0xD4 => {
                        self.registers.h |= 1 << 2;
                    }
                    0xD5 => {
                        self.registers.l |= 1 << 2;
                    }
                    0xD6 => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());
                        result |= value | 1 << 2;
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0xD7 => {
                        self.registers.a |= 1 << 2;
                    }
                    0xD8 => {
                        self.registers.b |= 1 << 3;
                    }
                    0xD9 => {
                        self.registers.c |= 1 << 3;
                    }
                    0xDA => {
                        self.registers.d |= 1 << 3;
                    }
                    0xDB => {
                        self.registers.e |= 1 << 3;
                    }
                    0xDC => {
                        self.registers.h |= 1 << 3;
                    }
                    0xDD => {
                        self.registers.l |= 1 << 3;
                    }
                    0xDE => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());
                        result |= value | 1 << 3;
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0xDF => {
                        self.registers.a |= 1 << 3;
                    }
                    0xE0 => {
                        self.registers.b |= 1 << 4;
                    }
                    0xE1 => {
                        self.registers.c |= 1 << 4;
                    }
                    0xE2 => {
                        self.registers.d |= 1 << 4;
                    }
                    0xE3 => {
                        self.registers.e |= 1 << 4;
                    }
                    0xE4 => {
                        self.registers.h |= 1 << 4;
                    }
                    0xE5 => {
                        self.registers.l |= 1 << 4;
                    }
                    0xE6 => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());
                        result |= value | 1 << 4;
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0xE7 => {
                        self.registers.a |= 1 << 4;
                    }
                    0xE8 => {
                        self.registers.b |= 1 << 5;
                    }
                    0xE9 => {
                        self.registers.c |= 1 << 5;
                    }
                    0xEA => {
                        self.registers.d |= 1 << 5;
                    }
                    0xEB => {
                        self.registers.e |= 1 << 5;
                    }
                    0xEC => {
                        self.registers.h |= 1 << 5;
                    }
                    0xED => {
                        self.registers.l |= 1 << 5;
                    }
                    0xEE => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());
                        result |= value | 1 << 5;
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0xEF => {
                        self.registers.a |= 1 << 5;
                    }
                    0xF0 => {
                        self.registers.b |= 1 << 6;
                    }
                    0xF1 => {
                        self.registers.c |= 1 << 6;
                    }
                    0xF2 => {
                        self.registers.d |= 1 << 6;
                    }
                    0xF3 => {
                        self.registers.e |= 1 << 6;
                    }
                    0xF4 => {
                        self.registers.h |= 1 << 6;
                    }
                    0xF5 => {
                        self.registers.l |= 1 << 6;
                    }
                    0xF6 => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());
                        result |= value | 1 << 6;
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0xF7 => {
                        self.registers.a |= 1 << 6;
                    }
                    0xF8 => {
                        self.registers.b |= 1 << 7;
                    }
                    0xF9 => {
                        self.registers.c |= 1 << 7;
                    }
                    0xFA => {
                        self.registers.d |= 1 << 7;
                    }
                    0xFB => {
                        self.registers.e |= 1 << 7;
                    }
                    0xFC => {
                        self.registers.h |= 1 << 7;
                    }
                    0xFD => {
                        self.registers.l |= 1 << 7;
                    }
                    0xFE => {
                        let value = self.memory_bus.read_byte(self.registers.get_hl());
                        let mut result = self.memory_bus.read_byte(self.registers.get_hl());
                        result |= value | 1 << 7;
                        self.memory_bus.write_byte(self.registers.get_hl(), result);
                    }
                    0xFF => {
                        self.registers.a |= 1 << 7;
                    }
                    _ => {
                        panic!("Unsupported opcode: 0xCB{:02X}", opcode);
                    }
                }
            }
            0xCC => {
                let nn: u16 = self.read_immediate_short();
                self.op_call_nn(nn);
            }
            0xCD => {
                let nn: u16 = self.read_immediate_short();
                self.op_call_nn(nn);
            }
            0xCE => {
                let value = self.memory_bus.read_byte(self.registers.pc);
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
            0xCF => {
                self.op_rst_address(0x08);
            }
            0xD0 => {
                if !self.registers.f.get_flag(Flag::C) {
                    self.op_ret();
                }
            }
            0xD1 => {
                //self.op_pop_rr(&mut self.registers.get_de());
                let value = self.op_pop_stack();
                self.registers.set_de(value);
            }
            0xD2 => {
                let nn: u16 = self.read_immediate_short();
                self.op_jp_nn(nn);
            }
            0xD3 => {
                panic!("Unsupported opcode: 0xD3");
            }
            0xD4 => {
                let nn: u16 = self.read_immediate_short();
                self.op_call_nn(nn);
            }
            0xD5 => {
                self.op_push_stack(self.registers.get_de());
            }
            0xD6 => {
                let value = self.read_immediate_byte();
                let (result, overflow) = self.registers.a.overflowing_sub(value);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers
                    .f
                    .set_flag(Flag::H, (self.registers.a & 0x0F) < (value & 0x0F)); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(Flag::C, overflow); // Set the carry flag if there's a borrow out of the most significant bit
                self.registers.a = result;
            }
            0xD7 => {
                self.op_rst_address(0x10);
            }
            0xD8 => {
                if self.registers.f.get_flag(Flag::C) {
                    self.op_ret();
                }
            }
            0xD9 => {
                self.op_ret();
                self.op_ei();
            }
            0xDA => {
                let nn: u16 = self.read_immediate_short();
                self.op_jp_nn(nn);
            }
            0xDB => {
                panic!("Unsupported opcode: 0xDB");
            }
            0xDC => {
                let nn: u16 = self.read_immediate_short();
                self.op_call_nn(nn);
            }
            0xDD => {
                panic!("Unsupported opcode: 0xDD");
            }
            0xDE => {
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_sub(self.memory_bus.read_byte(self.registers.pc));
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F)
                        < (self.memory_bus.read_byte(self.registers.pc) & 0x0F) + carry,
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16)
                        < (self.memory_bus.read_byte(self.registers.pc) as u16) + (carry as u16),
                ); // Set the carry flag if there's a borrow out of the most significant bit
                   //self.registers.pc = self.registers.pc.wrapping_add(1);
                self.registers.a = result;
            }
            0xDF => {
                self.op_rst_address(0x18);
            }
            0xE0 => {
                let offset = self.read_immediate_byte() as u16;
                let address = 0xFF00 + offset;
                self.memory_bus.write_byte(address, self.registers.a);
            }
            0xE1 => {
                let value = self.op_pop_stack();
                self.registers.set_hl(value);
            }
            0xE2 => {
                self.op_ldh_c_a();
            }
            0xE3 => {
                panic!("Unsupported opcode: 0xE3");
            }
            0xE4 => {
                panic!("Unsupported opcode: 0xE4");
            }
            0xE5 => {
                self.op_push_stack(self.registers.get_hl());
            }
            0xE6 => {
                let value = self.read_immediate_byte();
                let result = self.registers.a & value;
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, true);
                self.registers.f.set_flag(Flag::C, false);
                self.registers.a = result;
            }
            0xE7 => {
                self.op_rst_address(0x20);
            }
            0xE8 => {
                let value = self.read_immediate_byte() as i16;
                let sp = self.registers.sp as i16;
                let result = sp.wrapping_add(value);
                self.registers.f.set_flag(Flag::Z, false);
                self.registers.f.set_flag(Flag::N, false);
                self.registers
                    .f
                    .set_flag(Flag::H, (sp & 0x0F) + (value & 0x0F) > 0x0F);
                self.registers
                    .f
                    .set_flag(Flag::C, (sp as i32) + (value as i32) > 0xFF);
                self.registers.sp = result as u16;
            }
            0xE9 => {
                self.op_jp_hl();
            }
            0xEA => {
                let nn: u16 = self.read_immediate_short();
                self.op_ld_nn_a(nn);
            }
            0xEB => {
                panic!("Unsupported opcode: 0xEB");
            }
            0xEC => {
                panic!("Unsupported opcode: 0xEC");
            }
            0xED => {
                panic!("Unsupported opcode: 0xED");
            }
            0xEE => {
                let value = self.read_immediate_byte();
                let result = self.registers.a ^ value;
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
                self.registers.a = result;
            }
            0xEF => {
                self.op_rst_address(0x28);
            }
            0xF0 => {
                let value = self.read_immediate_byte();
                self.op_ldh_a_n8(value);
            }
            0xF1 => {
                let value = self.op_pop_stack();
                self.registers.set_af(value);
            }
            0xF2 => {
                self.op_ldh_a_c();
            }
            0xF3 => {
                self.op_di();
            }
            0xF4 => {
                panic!("Unsupported opcode: 0xF4");
            }
            0xF5 => {
                self.op_push_stack(self.registers.get_af());
            }
            0xF6 => {
                let value = self.read_immediate_byte();
                let result = self.registers.a | value;
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, true);
                self.registers.f.set_flag(Flag::C, false);

                self.registers.a = result;
            }
            0xF7 => {
                self.op_rst_address(0x30);
            }
            0xF8 => {
                let value = self.read_immediate_byte();
                let result = ((self.registers.sp as i16) + (value as i8 as i16)) as u16;

                self.registers.set_hl(result);
            }
            0xF9 => {
                self.op_ld_sp_hl();
            }
            0xFA => {
                let nn: u16 = self.read_immediate_short();
                self.op_ld_a_nn(nn);
            }
            0xFB => {
                self.op_ei();
            }
            0xFC => {
                panic!("Unsupported opcode: 0xFC");
            }
            0xFD => {
                panic!("Unsupported opcode: 0xFD");
            }
            0xFE => {
                let value = self.read_immediate_byte();
                let result = self.registers.a.wrapping_sub(value);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers
                    .f
                    .set_flag(Flag::H, (self.registers.a & 0x0F) < (value & 0x0F)); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(Flag::C, self.registers.a < value); // Set the carry flag if there's a borrow out of the most significant bit
            }
            0xFF => {
                self.op_rst_address(0x0038);
            }
            _ => {
                panic!("Unsupported opcode: {:X}", opcode);
            }
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
        /*let lsb = self.work_ram[self.registers.pc as usize];
        self.registers.pc = self.registers.pc.wrapping_add(1);
        let msb = self.work_ram[self.registers.pc as usize];
        self.registers.pc = self.registers.pc.wrapping_add(1);
        (msb as u16) << 8 | lsb as u16*/
        let lsb = self.memory_bus.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        let msb = self.memory_bus.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        (msb as u16) << 8 | lsb as u16
    }

    /*
     *   Read the 16-bit value from memory for a given address.
     */
    /*fn read_short(&mut self, address: u16) -> u16 {
        match address {
            0x0000..=0x7FFF => {
                let lsb = self.work_ram[address as usize];
                let msb = self.work_ram[address.wrapping_add(1) as usize];
                (msb as u16) << 8 | lsb as u16
            }
            0x8000..=0x9FFF => self.gpu.read_short(address),
            0xA000..=0xBFFF => unimplemented!("From cartridge, switchable bank if any"),
            0xC000..=0xCFFF => {
                unimplemented!("4 KiB Work RAM (WRAM)")
            },
            0xE000..=0xFDFF => {
                debug!("Echo RAM (mirror of C000–DDFF), Nintendo says use of this area is prohibited.");
                return 0x0;
            },
            0xFE00..=0xFE9F => {
                unimplemented!("Sprite attribute table (OAM)")
            },
            0xFEA0..=0xFEFF => {
                debug!("Not Usable");
                return 0x0;
            },
            0xFF00..=0xFF7F => {
                unimplemented!("I/O Registers")
            },
            0xFF80..=0xFFFE => {
                unimplemented!("High RAM (HRAM)")
            },
            0xFFFF => {
                unimplemented!("Interrupt Enable Register")
            },
            _ => panic!("Unsupported address: {:X}", address),
        }

    }*/

    /*
     *   Read the immediate 8-bit value from memory for the current program counter.
     */
    fn read_immediate_byte(&mut self) -> u8 {
        let value = self.memory_bus.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        value
    }

    /*
     *   Write the immediate 16-bit value to memory.
     */
    fn write_immediate_short(&mut self, address: u16, value: u16) {
        info!("write immediate short address {:X} value {:X}", address, value);
        match address {
            0x0000..=0x7FFF  => {
                let lsb = (value & 0x00FF) as u8;
                let msb = (address >> 8) as u8;
                self.memory_bus.write_byte(address, lsb);
                self.memory_bus.write_byte(address.wrapping_add(1), msb);
            },
            0x8000..=0x9FFF => {
                self.gpu.write_short(address, value);
            },
            _ => {
                panic!("Unsupported address: {:X}", address);

            }
        }
    }

    /*
     *   Add the immediate 8-bit value to the 8-bit A register.
     */
    fn op_add_a_n8(&mut self, value: u8) {
        debug!("op_add_a_n8");
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

    /*
     *   LD A, (nn)
     *   Load to the 8-bit A register, data from the absolute address specified by the 16-bit operand nn.
     */
    fn op_ld_a_nn(&mut self, address: u16) {
        debug!("op_ld_a_nn");
        self.registers.a = self.memory_bus.read_byte(address);
    }
    /*
     *    LD (nn), A
     *    Load to the absolute address specified by the 16-bit operand nn, data from the 8-bit A register.
     */
    fn op_ld_nn_a(&mut self, address: u16) {
        debug!("op_ld_nn_a");
        self.memory_bus.write_byte(address, self.registers.a);
    }
    /*
     *   LDH A, (C)
     *   Load to the 8-bit A register, data from the address specified by the 8-bit C register. The full 16-bit absolute
     *   address is obtained by setting the most significant byte to 0xFF and the least significant byte to the value of C,
     *   so the possible range is 0xFF00-0xFFFF.
     */
    //  TODO - Check this is setting the upper and lower bits correctly
    fn op_ldh_a_c(&mut self) {
        debug!("op_ldh_a_c");
        let address = 0xFF00 | self.registers.c as u16;
        self.registers.a = self.memory_bus.read_byte(address);
    }

    /*
     *   LDH (C), A
     *   Load to the address specified by the 8-bit C register, data from the 8-bit A register. The full 16-bit absolute
     *   address is obtained by setting the most significant byte to 0xFF and the least significant byte to the value of C,
     *   so the possible range is 0xFF00-0xFFFF.
     */
    fn op_ldh_c_a(&mut self) {
        debug!("op_ldh_c_a");
        let address = 0xFF00 | self.registers.c as u16;
        self.memory_bus.write_byte(address, self.registers.a);
    }

    /*
     *   LDH A, (n)
     *   Load to the 8-bit A register, data from the address specified by the 8-bit immediate data n. The full 16-bit
     *   absolute address is obtained by setting the most significant byte to 0xFF and the least significant byte to the
     *   value of n, so the possible range is 0xFF00-0xFFFF.
     */
    fn op_ldh_a_n8(&mut self, value: u8) {
        debug!("op_ldh_a_n8");
        //let address = 0xFF00 | value as u16;
        let address = 0xFF00 + value as u16;
        self.registers.a = self.memory_bus.read_byte(address);
    }

    /*
     *   LD SP, HL
     *   Load to the 16-bit SP register, data from the 16-bit HL register.
     */
    fn op_ld_sp_hl(&mut self) {
        debug!("op_ld_sp_hl");
        self.registers.sp = self.registers.get_hl();
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
     *   JP HL
     *   Unconditional jump to the absolute address specified by the 16-bit register HL.
     */
    fn op_jp_hl(&mut self) {
        debug!("op_jp_hl");
        self.registers.pc = self.registers.get_hl();
    }

    /*
     *   JR e
     *   Unconditional jump to the relative address specified by the signed 8-bit operand e.
     */
    fn op_jr_e(&mut self, offset: i8) {
        debug!("op_jr_e");
        debug!(
            "Jumping to 0x{:04X}",
            self.registers.pc.wrapping_add(offset as u16)
        );
        self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
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
        //unimplemented!("op_ret");
        self.registers.pc = self.op_pop_stack();
    }

    /*
     *   HALT
     *   STOP
     *   DI
     *   Disables interrupt handling by setting IME=0 and cancelling any scheduled effects of the EI instruction if any.
     */

    fn op_halt(&mut self) {
        debug!("op_halt");
        self.memory_bus.write_byte(INTERRUPT_ENABLE_REGISTER, 0);
    }
    fn op_stop(&mut self) {
        debug!("op_stop");
        self.memory_bus.write_byte(INTERRUPT_ENABLE_REGISTER, 0);
    }
    fn op_di(&mut self) {
        debug!("op_di");
        self.interupt = false;
    }

    /*
     *   EI
     *   Schedules interrupt handling to be enabled after the next machine cycle.
     */
    fn op_ei(&mut self) {
        debug!("op_ei");
        self.interupt = true;
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
        debug!("op_sla");
        let carry = *register & 0x80 != 0;
        *register <<= 1;
        self.registers.f.set_flag(Flag::Z, *register == 0);
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, carry);
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
    fn op_bit(&mut self, bit: u8, register: u8) {
        debug!("op_bit");
        self.registers
            .f
            .set_flag(Flag::Z, (register & (1 << bit)) == 0);
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

    fn op_push_stack(&mut self, address: u16) {
        debug!("op_push_stack");
        debug!("stack pointer: {:#X}", self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_sub(2);
        debug!("stack pointer: {:#X}", self.registers.sp);
        self.memory_bus.write_byte(self.registers.sp, (address >> 8) as u8);
        self.memory_bus.write_byte(self.registers.sp.wrapping_add(1), address as u8);
    }

    fn op_pop_stack(&mut self) -> u16 {
        debug!("op_pop_stack");
        let value = self.memory_bus.read_short(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(2);
        value
        //TODO with this logic stack pointer isn't required
        //self.call_stack.pop().expect("op_pop_stack stack underflow")
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

    fn print_debug(&mut self) {
        if self.memory_bus.read_byte(0xFF02) == 0x81 {
            info!("{}", self.memory_bus.read_byte(0xFF01) as char);
            panic!("0x81");
            self.memory_bus.write_byte(0xFF02, 0x0);
        }
    }
}
