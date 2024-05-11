use log::{debug, error};
use std::fs::File;
use std::io::prelude::*;
use std::{error, fmt, result};

use crate::constants;
use crate::rom;
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

pub struct CPU {
    pub registers: Registers,
    //work_ram: [u8; 0xFFFF],
    work_ram: [u8; 0xFFFF],
    video_ram: [u16; 8192],
    interupt: bool,
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: FlagsRegister::new(),
            h: 0,
            l: 0,
            pc: 0x0100, //0,
            sp: 0,
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
        write!(f, "Registers: A: {:02X} B: {:02X} C: {:02X} D: {:02X} E: {:02X} F: {} H: {:02X} L: {:02X} PC: {:04X} SP: {:04X}", self.a, self.b, self.c, self.d, self.e, self.f, self.h, self.l, self.pc, self.sp)
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

impl CPU {
    pub fn new() -> Self {
        CPU {
            registers: Registers::new(),
            work_ram: [0; 0xFFFF],
            video_ram: [0; 8192],
            interupt: false,
        }
    }

    /*
     *  Load the boot ROM into memory
     */
    pub fn load_boot_rom(&mut self, file_path: String) {
        debug!("Loading boot ROM");
        let mut file = File::open(file_path).expect("Boot ROM file not found");
        let mut buffer: Vec<u8> = Vec::new();

        // Read the file into a buffer
        file.read_to_end(&mut buffer)
            .expect("Error reading boot rom file");
        debug!(
            "Boot ROM file size: {} bytes / {} kilobytes",
            buffer.len(),
            buffer.len() / 1024
        );

        for (i, byte) in buffer.iter().enumerate() {
            self.work_ram[i] = *byte;
        }

        debug!("Boot ROM loaded into memory");
    }

    /*
     *   Load the ROM into memory
     */
    pub fn load_rom(&mut self, file_path: String) {
        debug!("Loading ROM: {}", file_path);
        let mut file = File::open(file_path).expect("ROM file not found");
        let mut buffer: Vec<u8> = Vec::new();

        // Read the file into a buffer
        file.read_to_end(&mut buffer).expect("Error reading file");
        debug!(
            "ROM file size: {} bytes / {} kilobytes",
            buffer.len(),
            buffer.len() / 1024
        );

        let rom = ROM::new(buffer);
        debug!("{}", rom);

        rom.validate_header_checksum().unwrap(); // Panics if the header checksum is invalid
                                                 // Check cartridge type and load the ROM into memory based on the type

        for byte in 0x00..0x3FFF {
            self.work_ram[byte] = rom.rom[byte];
        }

        //for (i, byte) in rom.rom.iter().enumerate() {
        //    self.work_ram[0x0100 + i] = *byte;
        //}

        debug!("ROM Bank 0 loaded into memory");
        if rom.cartridge_type == 0x00 {
            // ROM ONLY
            debug!("ROM ONLY");
        } else {
            debug!("ROM with MBC");
            let rom_banks = rom.load_rom_to_banks();
            self.work_ram[0x4000..=0x7FFF].copy_from_slice(&rom_banks.data[0]); // Load the first bank of the ROM into memory
            debug!(
                "ROM Bank 1 loaded into memory, size: {} bytes",
                rom_banks.data[0].len()
            );
        }
        //debug!("ROM loaded into memory");
    }

    /*
     *   CPU cycle - fetch, decode, execute
     */
    pub fn cycle(&mut self) {
        debug!("##################################################");
        debug!("Fetch");

        if self.work_ram[0xff02] == 0x81 {
            // TODO Temp for development, delete this
            debug!("CPU test [{}]", self.work_ram[0xff01] as char);
            self.work_ram[0xff02] = 0x0;
        }

        let opcode = self.work_ram[self.registers.pc as usize];
        debug!("Opcode [{}]", opcode);

        self.registers.pc += 1;
        debug!("PC [{}]", self.registers.pc);

        debug!("Decode & Execute");

        match opcode {
            0x00 => {
                self.op_nop();
            }
            0x01 => {
                debug!("op_ld_rr_nn 0x01");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                //self.op_ld_rr_nn(&mut self.registers.set_bc, nn);
                self.registers.set_bc(nn);
            }
            0x02 => {
                debug!("0x02");
                self.work_ram[self.registers.get_bc() as usize] = self.registers.a;
            }
            0x03 => {
                debug!("0x03");
                self.registers.set_bc(self.registers.get_bc() + 1);
            }
            0x04 => {
                debug!("0x04");
                self.registers.b = self.registers.b.wrapping_add(1);
            }
            0x05 => {
                debug!("0x05");
                self.registers.b = self.registers.b.wrapping_sub(1);
            }
            0x06 => {
                debug!("0x06");
                let value = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                self.registers.b = value;
            }
            0x07 => {
                debug!("0x07");
                // TODO RLC
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
                debug!("0x08");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.work_ram[nn as usize] = (self.registers.sp & 0xFF) as u8;
                self.work_ram[(nn + 1) as usize] = (self.registers.sp >> 8) as u8;
            }
            0x09 => {
                debug!("0x09");
                let hl = self.registers.get_hl();
                let bc = self.registers.get_bc();
                self.registers.set_hl(hl + bc);
            }
            0x0A => {
                debug!("0x0A");
                self.registers.a = self.work_ram[self.registers.get_bc() as usize];
            }
            0x0B => {
                debug!("0x0B");
                self.registers.set_bc(self.registers.get_bc() - 1);
            }
            0x0C => {
                debug!("0x0C");
                self.registers.c = self.registers.c.wrapping_add(1);
            }
            0x0D => {
                debug!("0x0D");
                self.registers.c = self.registers.c.wrapping_sub(1);
            }
            0x0E => {
                debug!("0x0E");
                let value = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                self.registers.c = value;
            }
            0x0F => {
                debug!("0x0F");
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
                debug!("0x10");
                unimplemented!("STOP not implemented");
            }
            0x11 => {
                debug!("op_ld_rr_nn 0x11");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.registers.set_de(nn);
            }
            0x12 => {
                debug!("0x12");
                self.work_ram[self.registers.get_de() as usize] = self.registers.a;
            }
            0x13 => {
                debug!("0x13");
                self.registers.set_de(self.registers.get_de() + 1);
            }
            0x14 => {
                debug!("0x14");
                self.registers.d = self.registers.d.wrapping_add(1);
            }
            0x15 => {
                debug!("0x15");
                self.registers.d = self.registers.d.wrapping_sub(1);
            }
            0x16 => {
                debug!("0x16");
                let value = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                self.registers.d = value;
            }
            0x17 => {
                debug!("0x17");

                //self.op_rl(&mut self.registers.a);
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
                debug!("0x18");
                let offset = self.work_ram[self.registers.pc as usize] as i8;
                self.registers.pc += 1;
                self.op_jr_e(offset);
            }
            0x19 => {
                debug!("0x19");
                let hl = self.registers.get_hl();
                let de = self.registers.get_de();
                self.registers.set_hl(hl + de);
            }
            0x1A => {
                debug!("0x1A");
                self.registers.a = self.work_ram[self.registers.get_de() as usize];
            }
            0x1B => {
                debug!("0x1B");
                self.registers.set_de(self.registers.get_de() - 1);
            }
            0x1C => {
                debug!("0x1C");
                self.registers.e = self.registers.e.wrapping_add(1);
            }
            0x1D => {
                debug!("0x1D");
                self.registers.e = self.registers.e.wrapping_sub(1);
            }
            0x1E => {
                debug!("0x1E");
                let value = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                self.registers.e = value;
            }
            0x1F => {
                debug!("0x1F");

                //self.op_rr(&mut self.registers.a);
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
                debug!("0x20");
                let offset = self.work_ram[self.registers.pc as usize] as i8;
                self.registers.pc += 1;
                self.op_jr_cc_e(!self.registers.f.get_flag(Flag::Z), offset);
            }
            0x21 => {
                debug!("op_ld_rr_nn 0x21");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.registers.set_hl(nn);
            }
            0x22 => {
                debug!("0x22");
                self.work_ram[self.registers.get_hl() as usize] = self.registers.a;
                self.registers.set_hl(self.registers.get_hl() + 1);
            }
            0x23 => {
                debug!("0x23");
                self.registers.set_hl(self.registers.get_hl() + 1);
            }
            0x24 => {
                debug!("0x24");
                self.registers.h = self.registers.h.wrapping_add(1);
            }
            0x25 => {
                debug!("0x25");
                self.registers.h = self.registers.h.wrapping_sub(1);
            }
            0x26 => {
                debug!("0x26");
                let value = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                self.registers.h = value;
            }
            0x27 => {
                debug!("0x27");

                //self.op_daa();
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
                debug!("0x28");
                let offset = self.work_ram[self.registers.pc as usize] as i8;
                self.registers.pc += 1;
                self.op_jr_cc_e(self.registers.f.get_flag(Flag::Z), offset);
            }
            0x29 => {
                debug!("0x29");
                let hl = self.registers.get_hl();
                self.registers.set_hl(hl + hl);
            }
            0x2A => {
                debug!("0x2A");
                self.registers.a = self.work_ram[self.registers.get_hl() as usize];
                self.registers.set_hl(self.registers.get_hl() + 1);
            }
            0x2B => {
                debug!("0x2B");
                self.registers.set_hl(self.registers.get_hl() - 1);
            }
            0x2C => {
                debug!("0x2C");
                self.registers.l = self.registers.l.wrapping_add(1);
            }
            0x2D => {
                debug!("0x2D");
                self.registers.l = self.registers.l.wrapping_sub(1);
            }
            0x2E => {
                debug!("0x2E");
                let value = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                self.registers.l = value;
            }
            0x2F => {
                debug!("0x2F");

                //self.op_cpl();
                self.registers.a = !self.registers.a;
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(Flag::H, true);
            }
            0x30 => {
                debug!("0x30");
                let offset = self.work_ram[self.registers.pc as usize] as i8;
                self.registers.pc += 1;
                self.op_jr_cc_e(!self.registers.f.get_flag(Flag::C), offset);
            }
            0x31 => {
                debug!("0x31");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.registers.sp = nn;
            }
            0x32 => {
                debug!("0x32");
                self.work_ram[self.registers.get_hl() as usize] = self.registers.a;
                self.registers.set_hl(self.registers.get_hl() - 1);
            }
            0x33 => {
                debug!("0x33");
                self.registers.sp = self.registers.sp.wrapping_add(1);
            }
            0x34 => {
                debug!("0x34");
                let hl = self.registers.get_hl();
                let value = self.work_ram[hl as usize];
                self.registers.f.set_flag(Flag::H, (value & 0x0F) == 0x0F);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::C, value == 0xFF);
                self.work_ram[hl as usize] = value.wrapping_add(1);
                self.registers
                    .f
                    .set_flag(Flag::Z, self.work_ram[hl as usize] == 0);
            }
            0x35 => {
                debug!("0x35");
                let hl = self.registers.get_hl();
                let value = self.work_ram[hl as usize];
                self.registers.f.set_flag(Flag::H, (value & 0x0F) == 0x00);
                self.registers.f.set_flag(Flag::N, true);
                self.registers.f.set_flag(Flag::C, value == 0x00);
                self.work_ram[hl as usize] = value.wrapping_sub(1);
                self.registers
                    .f
                    .set_flag(Flag::Z, self.work_ram[hl as usize] == 0);
            }
            0x36 => {
                debug!("0x36");
                let value = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                self.work_ram[self.registers.get_hl() as usize] = value;
            }
            0x37 => {
                debug!("0x37");

                //self.op_scf();
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, true);
            }
            0x38 => {
                debug!("0x38");
                let value = self.work_ram[self.registers.pc as usize] as i8;
                self.registers.pc += 1;
                self.op_jr_cc_e(self.registers.f.get_flag(Flag::C), value);
            }
            0x39 => {
                debug!("0x39");
                let hl = self.registers.get_hl();
                let sp = self.registers.sp;
                self.registers.set_hl(hl + sp);
            }
            0x3A => {
                debug!("0x3A");
                self.registers.a = self.work_ram[self.registers.get_hl() as usize];
                self.registers.set_hl(self.registers.get_hl() - 1);
            }
            0x3B => {
                debug!("0x3B");
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }
            0x3C => {
                debug!("0x3C");
                self.registers.a = self.registers.a.wrapping_add(1);
            }
            0x3D => {
                debug!("0x3D");
                self.registers.a = self.registers.a.wrapping_sub(1);
            }
            0x3E => {
                debug!("0x3E");
                let value = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                self.registers.a = value;
            }
            0x3F => {
                debug!("0x3F");

                //self.op_ccf();
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers
                    .f
                    .set_flag(Flag::C, !self.registers.f.get_flag(Flag::C));
            }
            0x40 => {
                debug!("0x40");
                //self.op_ld_r8_r8(&self.registers.b, &mut self.registers.b);

                self.registers.b = self.registers.b;
            }
            0x41 => {
                debug!("0x41");
                self.registers.b = self.registers.c;
            }
            0x42 => {
                debug!("0x42");
                self.registers.b = self.registers.d;
            }
            0x43 => {
                debug!("0x43");
                self.registers.b = self.registers.e;
            }
            0x44 => {
                debug!("0x44");
                self.registers.b = self.registers.h;
            }
            0x45 => {
                debug!("0x45");
                self.registers.b = self.registers.l;
            }
            0x46 => {
                debug!("0x46");
                self.registers.b = self.registers.get_hl() as u8;
            }
            0x47 => {
                debug!("0x47");
                self.registers.b = self.registers.a;
            }
            0x48 => {
                debug!("0x48");
                self.registers.c = self.registers.b;
            }
            0x49 => {
                debug!("0x49");
                //self.op_ld_r8_r8(&self.registers.c, &mut self.registers.c);

                self.registers.c = self.registers.c;
            }
            0x4A => {
                debug!("0x4A");
                self.registers.c = self.registers.d;
            }
            0x4B => {
                debug!("0x4B");
                self.registers.c = self.registers.e;
            }
            0x4C => {
                debug!("0x4C");
                self.registers.c = self.registers.h;
            }
            0x4D => {
                debug!("0x4D");
                self.registers.c = self.registers.l;
            }
            0x4E => {
                debug!("0x4E");
                self.registers.c = self.registers.get_hl() as u8;
            }
            0x4F => {
                debug!("0x4F");
                self.registers.c = self.registers.a;
            }
            0x50 => {
                debug!("0x50");
                self.registers.d = self.registers.b;
            }
            0x51 => {
                debug!("0x51");
                self.registers.d = self.registers.c;
            }
            0x52 => {
                debug!("0x52");
                //self.op_ld_r8_r8(&self.registers.d, &mut self.registers.d);

                self.registers.d = self.registers.d;
            }
            0x53 => {
                debug!("0x53");
                self.registers.d = self.registers.e;
            }
            0x54 => {
                debug!("0x54");
                self.registers.d = self.registers.h;
            }
            0x55 => {
                debug!("0x55");
                self.registers.d = self.registers.l;
            }
            0x56 => {
                debug!("0x56");
                self.registers.d = self.registers.get_hl() as u8;
            }
            0x57 => {
                debug!("0x57");
                self.registers.d = self.registers.a;
            }
            0x58 => {
                debug!("0x58");
                self.registers.e = self.registers.b;
            }
            0x59 => {
                debug!("0x59");
                self.registers.e = self.registers.c;
            }
            0x5A => {
                debug!("0x5A");
                self.registers.e = self.registers.d;
            }
            0x5B => {
                debug!("0x5B");
                //self.op_ld_r8_r8(&self.registers.e, &mut self.registers.e);

                self.registers.e = self.registers.e;
            }
            0x5C => {
                debug!("0x5C");
                self.registers.e = self.registers.h;
            }
            0x5D => {
                debug!("0x5D");
                self.registers.e = self.registers.l;
            }
            0x5E => {
                debug!("0x5E");
                self.registers.e = self.registers.get_hl() as u8;
            }
            0x5F => {
                debug!("0x5F");
                self.registers.e = self.registers.a;
            }
            0x60 => {
                debug!("0x60");
                self.registers.h = self.registers.b;
            }
            0x61 => {
                debug!("0x61");
                self.registers.h = self.registers.c;
            }
            0x62 => {
                debug!("0x62");
                self.registers.h = self.registers.d;
            }
            0x63 => {
                debug!("0x63");
                self.registers.h = self.registers.e;
            }
            0x64 => {
                debug!("0x64");
                //self.op_ld_r8_r8(&self.registers.h, &mut self.registers.h);

                self.registers.h = self.registers.h;
            }
            0x65 => {
                debug!("0x65");
                self.registers.h = self.registers.l;
            }
            0x66 => {
                debug!("0x66");
                self.registers.h = self.registers.get_hl() as u8;
            }
            0x67 => {
                debug!("0x67");
                self.registers.h = self.registers.a;
            }
            0x68 => {
                debug!("0x68");
                self.registers.l = self.registers.b;
            }
            0x69 => {
                debug!("0x69");
                self.registers.l = self.registers.c;
            }
            0x6A => {
                debug!("0x6A");
                self.registers.l = self.registers.d;
            }
            0x6B => {
                debug!("0x6B");
                self.registers.l = self.registers.e;
            }
            0x6C => {
                debug!("0x6C");
                self.registers.l = self.registers.h;
            }
            0x6D => {
                debug!("0x6D");
                //self.op_ld_r8_r8(&self.registers.l, &mut self.registers.l);

                self.registers.l = self.registers.l;
            }
            0x6E => {
                debug!("0x6E");
                self.registers.l = self.registers.get_hl() as u8;
            }
            0x6F => {
                debug!("0x6F");
                self.registers.l = self.registers.a;
            }
            0x70 => {
                debug!("0x70");
                self.work_ram[self.registers.get_hl() as usize] = self.registers.b;
            }
            0x71 => {
                debug!("0x71");
                self.work_ram[self.registers.get_hl() as usize] = self.registers.c;
            }
            0x72 => {
                debug!("0x72");
                self.work_ram[self.registers.get_hl() as usize] = self.registers.d;
            }
            0x73 => {
                debug!("0x73");
                self.work_ram[self.registers.get_hl() as usize] = self.registers.e;
            }
            0x74 => {
                debug!("0x74");
                self.work_ram[self.registers.get_hl() as usize] = self.registers.h;
            }
            0x75 => {
                debug!("0x75");
                self.work_ram[self.registers.get_hl() as usize] = self.registers.l;
            }
            0x76 => {
                debug!("0x76");

                //self.op_halt();
                unimplemented!("HALT not implemented");
            }
            0x77 => {
                debug!("0x77");
                self.work_ram[self.registers.get_hl() as usize] = self.registers.a;
            }
            0x78 => {
                debug!("0x78");
                self.registers.a = self.registers.b;
            }
            0x79 => {
                debug!("0x79");
                self.registers.a = self.registers.c;
            }
            0x7A => {
                debug!("0x7A");
                self.registers.a = self.registers.d;
            }
            0x7B => {
                debug!("0x7B");
                self.registers.a = self.registers.e;
            }
            0x7C => {
                debug!("0x7C");
                self.registers.a = self.registers.h;
            }
            0x7D => {
                debug!("0x7D");
                self.registers.a = self.registers.l;
            }
            0x7E => {
                debug!("0x7E");
                self.registers.a = self.registers.get_hl() as u8;
            }
            0x7F => {
                debug!("0x7F");
                self.registers.a = self.registers.a;
            }
            0x80 => {
                debug!("0x80");
                self.registers.a = self.registers.a.wrapping_add(self.registers.b);
            }
            0x81 => {
                debug!("0x81");
                self.registers.a = self.registers.a.wrapping_add(self.registers.c);
            }
            0x82 => {
                debug!("0x82");
                self.registers.a = self.registers.a.wrapping_add(self.registers.d);
            }
            0x83 => {
                debug!("0x83");
                self.registers.a = self.registers.a.wrapping_add(self.registers.e);
            }
            0x84 => {
                debug!("0x84");
                self.registers.a = self.registers.a.wrapping_add(self.registers.h);
            }
            0x85 => {
                debug!("0x85");
                self.registers.a = self.registers.a.wrapping_add(self.registers.l);
            }
            0x86 => {
                debug!("0x86");
                // TODO other examples / documents show is as much more complex
                let value = self.work_ram[self.registers.get_hl() as usize];
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
            0x87 => {
                debug!("0x87");
                // TODO Test this and migrate to a generic function
                let result = self.registers.a.wrapping_add(self.registers.a);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false); // Clear the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F) + (self.registers.a & 0x0F) > 0x0F,
                ); // Set the half-carry flag if there's a carry from bit 3
                self.registers
                    .f
                    .set_flag(Flag::C, result < self.registers.a);
                self.registers.a = result;
            }
            0x88 => {
                debug!("0x88");
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .b
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
                debug!("0x89");
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .b
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
                debug!("0x8A");
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .b
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
                debug!("0x8B");
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .b
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
                debug!("0x8C");
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .b
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
                debug!("0x8D");
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .b
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
                debug!("0x8E");
                let value = self.work_ram[self.registers.get_hl() as usize];
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
                debug!("0x8F");
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
                debug!("0x90");
                self.registers.a = self.registers.a.wrapping_sub(self.registers.b);
            }
            0x91 => {
                debug!("0x91");
                self.registers.a = self.registers.a.wrapping_sub(self.registers.c);
            }
            0x92 => {
                debug!("0x92");
                self.registers.a = self.registers.a.wrapping_sub(self.registers.d);
            }
            0x93 => {
                debug!("0x93");
                self.registers.a = self.registers.a.wrapping_sub(self.registers.e);
            }
            0x94 => {
                debug!("0x94");
                self.registers.a = self.registers.a.wrapping_sub(self.registers.h);
            }
            0x95 => {
                debug!("0x95");
                self.registers.a = self.registers.a.wrapping_sub(self.registers.l);
            }
            0x96 => {
                debug!("0x96");
                let value = self.work_ram[self.registers.get_hl() as usize];
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
                debug!("0x97");
                self.registers.a = self.registers.a.wrapping_sub(self.registers.a);
            }
            0x98 => {
                debug!("0x98");
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
                debug!("0x99");
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
                debug!("0x9A");
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
                debug!("0x9B");
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
                debug!("0x9C");
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
                debug!("0x9D");
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
                debug!("0x9E");
                let value = self.work_ram[self.registers.get_hl() as usize];
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
                debug!("0x9F");
                let result = self.registers.a.wrapping_sub(self.registers.a);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(Flag::H, false); // Clear the half-carry flag
                self.registers.f.set_flag(Flag::C, false); // Clear the carry flag
                self.registers.a = result;
            }
            0xA0 => {
                debug!("0xA0");
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
                debug!("0xA1");
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
                debug!("0xA2");
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
                debug!("0xA3");
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
                debug!("0xA4");
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
                debug!("0xA5");
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
                debug!("0xA6");
                self.registers.a =
                    self.registers.a & self.work_ram[self.registers.get_hl() as usize];

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
                debug!("0xA7");
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
                debug!("0xA8");
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
                debug!("0xA9");
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
                debug!("0xAA");
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
                debug!("0xAB");
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
                debug!("0xAC");
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
                debug!("0xAD");
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
                debug!("0xAE");
                self.registers.a =
                    self.registers.a ^ self.work_ram[self.registers.get_hl() as usize];

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
                debug!("0xAF");
                self.registers.a = self.registers.a ^ self.registers.a;

                if self.registers.a == 0 {
                    self.registers.f.set_flag(Flag::Z, true);
                } else {
                    self.registers.f.set_flag(Flag::Z, false);
                }
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
            }
            0xB0 => {
                debug!("0xB0");
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
                debug!("0xB1");
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
                debug!("0xB2");
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
                debug!("0xB3");
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
                debug!("0xB4");
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
                debug!("0xB5");
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
                debug!("0xB6");
                self.registers.a =
                    self.registers.a | self.work_ram[self.registers.get_hl() as usize];

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
                debug!("0xB7");
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
                debug!("0xB8");
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
                debug!("0xB9");
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
                debug!("0xBA");
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
                debug!("0xBB");
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
                debug!("0xBC");
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
                debug!("0xBD");
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
                debug!("0xBE");
                let value = self.work_ram[self.registers.get_hl() as usize];
                let result = self.registers.a.wrapping_sub(value);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers
                    .f
                    .set_flag(Flag::H, (self.registers.a & 0x0F) < (value & 0x0F)); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(Flag::C, self.registers.a < value); // Set the carry flag if there's a borrow out of the most significant bit
            }
            0xBF => {
                debug!("0xBF");
                let result = self.registers.a.wrapping_sub(self.registers.a);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(Flag::H, false); // Clear the half-carry flag
                self.registers.f.set_flag(Flag::C, false); // Clear the carry flag
            }
            0xC0 => {
                debug!("0xC0");
                if !self.registers.f.get_flag(Flag::Z) {
                    self.op_ret();
                }
            }
            0xC1 => {
                debug!("0xC1");
                self.op_pop_rr(&mut self.registers.get_bc());
            }
            0xC2 => {
                debug!("0xC2");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.op_jp_nn(nn);
            }
            0xC3 => {
                debug!("0xC3");
                if !self.registers.f.get_flag(Flag::Z) {
                    let lsb = self.work_ram[self.registers.pc as usize];
                    self.registers.pc += 1;
                    let msb = self.work_ram[self.registers.pc as usize];
                    self.registers.pc += 1;
                    let nn: u16 = (msb as u16) << 8 | lsb as u16;
                    debug!("Jumping to 0x{:X}", nn);
                    self.op_jp_nn(nn);
                }
            }
            0xC4 => {
                debug!("0xC4");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.op_call_nn(nn);
            }
            0xC5 => {
                debug!("0xC5");
                self.op_push_rr(self.registers.get_bc());
            }
            0xC6 => {
                debug!("0xC6");
                let value = self.work_ram[self.registers.pc as usize];
                let result = self.registers.a.wrapping_add(value);
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
            0xC7 => {
                debug!("0xC7");
                self.op_rst_address(0x0000);
            }
            0xC8 => {
                debug!("0xC8");
                if self.registers.f.get_flag(Flag::Z) {
                    self.op_ret();
                }
            }
            0xC9 => {
                debug!("0xC9");
                self.op_ret();
            }
            0xCA => {
                debug!("0xCA");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.op_jp_nn(nn);
            }
            0xCB => {
                debug!("0xCB");
                unimplemented!("0xCB"); // TODO
            }
            0xCC => {
                debug!("0xCC");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.op_call_nn(nn);
            }
            0xCD => {
                debug!("0xCD");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.op_call_nn(nn);
            }
            0xCE => {
                debug!("0xCE");
                let value = self.work_ram[self.registers.pc as usize];
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
                debug!("0xCF");
                self.op_rst_address(0x08);
            }
            0xD0 => {
                debug!("0xD0");
                if !self.registers.f.get_flag(Flag::C) {
                    self.op_ret();
                }
            }
            0xD1 => {
                debug!("0xD1");
                self.op_pop_rr(&mut self.registers.get_de());
            }
            0xD2 => {
                debug!("0xD2");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.op_jp_nn(nn);
            }
            0xD3 => {
                debug!("0xD3");
                panic!("Unsupported opcode: 0xD3");
            }
            0xD4 => {
                debug!("0xD4");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.op_call_nn(nn);
            }
            0xD5 => {
                debug!("0xD5");
                self.op_push_rr(self.registers.get_de());
            }
            0xD6 => {
                debug!("0xD6");
                let value = self.work_ram[self.registers.pc as usize];
                let (result, overflow) = self.registers.a.overflowing_sub(value);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers
                    .f
                    .set_flag(Flag::H, (self.registers.a & 0x0F) < (value & 0x0F)); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(Flag::C, overflow); // Set the carry flag if there's a borrow out of the most significant bit
                self.registers.a = result;

                self.registers.pc += 1;
            }
            0xD7 => {
                debug!("0xD7");
                self.op_rst_address(0x10);
            }
            0xD8 => {
                debug!("0xD8");
                if self.registers.f.get_flag(Flag::C) {
                    self.op_ret();
                }
            }
            0xD9 => {
                debug!("0xD9");
                self.op_ret();
                self.op_ei();
            }
            0xDA => {
                debug!("0xDA");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.op_jp_nn(nn);
            }
            0xDB => {
                debug!("0xDB");
                panic!("Unsupported opcode: 0xDB");
            }
            0xDC => {
                debug!("0xDC");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.op_call_nn(nn);
            }
            0xDD => {
                debug!("0xDD");
                panic!("Unsupported opcode: 0xDD");
            }
            0xDE => {
                debug!("0xDE");
                let carry = if self.registers.f.get_flag(Flag::C) {
                    1
                } else {
                    0
                } as u8;
                let result = self
                    .registers
                    .a
                    .wrapping_sub(self.work_ram[self.registers.pc as usize]);
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers.f.set_flag(
                    Flag::H,
                    (self.registers.a & 0x0F)
                        < (self.work_ram[self.registers.pc as usize] & 0x0F) + carry,
                ); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(
                    Flag::C,
                    (self.registers.a as u16)
                        < (self.work_ram[self.registers.pc as usize] as u16) + (carry as u16),
                ); // Set the carry flag if there's a borrow out of the most significant bit
                   //self.registers.pc += 1;
                self.registers.a = result;
            }
            0xDF => {
                debug!("0xDF");
                self.op_rst_address(0x18);
            }
            0xE0 => {
                debug!("0xE0");
                let value = self.work_ram[self.registers.pc as usize];
                let address = 0xFF00 + value as u16;
                self.registers.pc += 1;
                self.work_ram[address as usize] = self.registers.a;
            }
            0xE1 => {
                debug!("0xE1");
                self.op_pop_rr(&mut self.registers.get_hl());
            }
            0xE2 => {
                debug!("0xE2");
                self.op_ldh_c_a();
            }
            0xE3 => {
                debug!("0xE3");
                panic!("Unsupported opcode: 0xE3");
            }
            0xE4 => {
                debug!("0xE4");
                panic!("Unsupported opcode: 0xE4");
            }
            0xE5 => {
                debug!("0xE5");
                self.op_push_rr(self.registers.get_hl());
            }
            0xE6 => {
                debug!("0xE6");
                let value = self.work_ram[self.registers.pc as usize];
                let result = self.registers.a & value;
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, true);
                self.registers.f.set_flag(Flag::C, false);
                self.registers.pc += 1;
                self.registers.a = result;
            }
            0xE7 => {
                debug!("0xE7");
                self.op_rst_address(0x20);
            }
            0xE8 => {
                debug!("0xE8");
                let value = self.work_ram[self.registers.pc as usize] as i16;
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
                self.registers.pc += 1;
            }
            0xE9 => {
                debug!("0xE9");
                self.op_jp_hl();
            }
            0xEA => {
                debug!("0xEA");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.op_ld_nn_a(nn);
            }
            0xEB => {
                debug!("0xEB");
                panic!("Unsupported opcode: 0xEB");
            }
            0xEC => {
                debug!("0xEC");
                panic!("Unsupported opcode: 0xEC");
            }
            0xED => {
                debug!("0xED");
                panic!("Unsupported opcode: 0xED");
            }
            0xEE => {
                debug!("0xEE");
                let value = self.work_ram[self.registers.pc as usize];
                let result = self.registers.a ^ value;
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, false);
                self.registers.f.set_flag(Flag::C, false);
                self.registers.pc += 1;
                self.registers.a = result;
            }
            0xEF => {
                debug!("0xEF");
                self.op_rst_address(0x28);
            }
            0xF0 => {
                debug!("0xF0");
                let value = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                self.op_ldh_a_n8(value);
            }
            0xF1 => {
                debug!("0xF1");
                self.op_pop_rr(&mut self.registers.get_af());
            }
            0xF2 => {
                debug!("0xF2");
                self.op_ldh_a_c();
            }
            0xF3 => {
                debug!("0xF3");
                self.op_di();
            }
            0xF4 => {
                debug!("0xF4");
                panic!("Unsupported opcode: 0xF4");
            }
            0xF5 => {
                debug!("0xF5");
                self.op_push_rr(self.registers.get_af());
            }
            0xF6 => {
                debug!("0xF6");
                let value = self.work_ram[self.registers.pc as usize];
                let result = self.registers.a | value;
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, false);
                self.registers.f.set_flag(Flag::H, true);
                self.registers.f.set_flag(Flag::C, false);
                self.registers.pc += 1;
                self.registers.a = result;
            }
            0xF7 => {
                debug!("0xF7");
                self.op_rst_address(0x30);
            }
            0xF8 => {
                debug!("0xF8");
                let value = self.work_ram[self.registers.pc as usize];
                let result = ((self.registers.sp as i16) + (value as i8 as i16)) as u16;
                self.registers.pc += 1;
                self.registers.set_hl(result);
            }
            0xF9 => {
                debug!("0xF9");
                self.op_ld_sp_hl();
            }
            0xFA => {
                debug!("0xFA");
                let lsb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let msb = self.work_ram[self.registers.pc as usize];
                self.registers.pc += 1;
                let nn: u16 = lsb as u16 | (msb as u16) << 8;
                self.op_ld_a_nn(nn);
            }
            0xFB => {
                debug!("0xFB");
                self.op_ei();
            }
            0xFC => {
                debug!("0xFC");
                panic!("Unsupported opcode: 0xFC");
            }
            0xFD => {
                debug!("0xFD");
                panic!("Unsupported opcode: 0xFD");
            }
            0xFE => {
                debug!("0xFE");
                let value = self.work_ram[self.registers.pc as usize];
                let result = self.registers.a.wrapping_sub(value);
                self.registers.pc += 1;
                self.registers.f.set_flag(Flag::Z, result == 0);
                self.registers.f.set_flag(Flag::N, true); // Set the subtraction flag
                self.registers
                    .f
                    .set_flag(Flag::H, (self.registers.a & 0x0F) < (value & 0x0F)); // Set the half-carry flag if there's a borrow from bit 4
                self.registers.f.set_flag(Flag::C, self.registers.a < value); // Set the carry flag if there's a borrow out of the most significant bit
            }
            0xFF => {
                debug!("0xFF");
                self.op_rst_address(0x38);
            } /*_ => {
                  panic!("Unsupported opcode: {}", format!("{:02X}", opcode));
              }*/
        }
    }

    /*
     *   NOP
     *   No operation.
     */
    fn op_nop(&mut self) {
        debug!("op_nop");
        //debug!("{}", self.registers);
        //debug!("{:?}", self.work_ram);
        //panic!("op_nop"); // TODO temp for debugging
    }

    /*
     *   LD r, r’
     *   Load to the 8-bit register r, data from the 8-bit register r’.
     */
    fn op_ld_r8_r8(&mut self, source: &u8, destination: &mut u8) {
        debug!(
            "source register: {:X}, destination reguster: {:X}",
            source, destination
        );
        *destination = source.clone();
    }

    /*
     *   LD r, n
     *   Load to the 8-bit register r, the immediate data n.
     */
    fn op_ld_r8_n8(&mut self, address: u8, value: &mut u8) {
        debug!("LD address: {:X}, value: {:X}", address, value);
        self.work_ram[address as usize] = *value;
    }

    /*
     *   LD r, (HL)
     *  Load to the 8-bit register r, data from the absolute address specified by the 16-bit register HL.
     */
    fn op_ld_r8_hl(&mut self, register: &mut u8) {
        debug!("op_ld_r8_hl");
        let hlv = self.registers.get_hl();
        debug!("HLV: {:X}", hlv);
        *register = self.work_ram[hlv as usize];
    }

    /*
     *   LD (HL), r
     *   Load to the absolute address specified by the 16-bit register HL, data from the 8-bit register r.
     */
    fn op_ld_hl_r8(&mut self, register: &u8) {
        debug!("op_ld_hl_r8");
        let hlv = self.registers.get_hl();
        debug!("HLV: {:X}", hlv);
        self.work_ram[hlv as usize] = *register;
    }

    /*
     *   LD (HL), n
     *   Load to the absolute address specified by the 16-bit register HL, the immediate data n.
     */
    fn op_ld_hl_n8(&mut self, value: u8) {
        debug!("op_ld_hl_n8");
        let hlv = self.registers.get_hl();
        debug!("HLV: {:X}", hlv);
        self.work_ram[hlv as usize] = value;
    }

    /*
     *   LD A, (BC)
     *   Load to the 8-bit A register, data from the absolute address specified by the 16-bit register BC.
     */
    fn op_ld_a_bc(&mut self) {
        debug!("op_ld_a_bc");
        let bc = self.registers.get_bc();
        self.registers.a = self.work_ram[bc as usize];
    }

    /*
     *   LD A, (DE)
     *   Load to the 8-bit A register, data from the absolute address specified by the 16-bit register DE.
     */
    fn op_ld_a_de(&mut self) {
        debug!("op_ld_a_de");
        let de = self.registers.get_de();
        self.registers.a = self.work_ram[de as usize];
    }

    /*
     *   LD (BC), a
     *   Load to the absolute address specified by the 16-bit register BC, data from the 8-bit A register.
     */
    fn op_ld_bc_a(&mut self) {
        debug!("op_ld_bc_a");
        let bc = self.registers.get_bc();
        self.work_ram[bc as usize] = self.registers.a;
    }

    /*
     *   LD (DE), a
     *   Load to the absolute address specified by the 16-bit register DE, data from the 8-bit A register.
     */
    fn op_ld_de_a(&mut self) {
        debug!("op_ld_de_a");
        let de = self.registers.get_de();
        self.work_ram[de as usize] = self.registers.a;
    }

    /*
     *   LD A, (nn)
     *   Load to the 8-bit A register, data from the absolute address specified by the 16-bit operand nn.
     */
    fn op_ld_a_nn(&mut self, address: u16) {
        debug!("op_ld_a_nn");
        self.registers.a = self.work_ram[address as usize];
    }
    /*
     *    LD (nn), A
     *    Load to the absolute address specified by the 16-bit operand nn, data from the 8-bit A register.
     */
    fn op_ld_nn_a(&mut self, address: u16) {
        debug!("op_ld_nn_a");
        self.work_ram[address as usize] = self.registers.a;
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
        self.registers.a = self.work_ram[address as usize];
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
        self.work_ram[address as usize] = self.registers.a;
    }

    /*
     *   LDH A, (n)
     *   Load to the 8-bit A register, data from the address specified by the 8-bit immediate data n. The full 16-bit
     *   absolute address is obtained by setting the most significant byte to 0xFF and the least significant byte to the
     *   value of n, so the possible range is 0xFF00-0xFFFF.
     */
    fn op_ldh_a_n8(&mut self, value: u8) {
        debug!("op_ldh_a_n8");
        let address = 0xFF00 | value as u16;
        self.registers.a = self.work_ram[address as usize];
    }

    /*
     *   LDH (n), A
     *   Load to the address specified by the 8-bit immediate data n, data from the 8-bit A register. The full 16-bit
     *   absolute address is obtained by setting the most significant byte to 0xFF and the least significant byte to the
     *   value of n, so the possible range is 0xFF00-0xFFFF.
     */
    fn op_ldh_n8_a(&mut self, value: u8) {
        debug!("op_ldh_n8_a");
        let address = 0xFF00 | value as u16;
        self.work_ram[address as usize] = self.registers.a;
    }

    /*
     *   LD A, (HL-)
     *   Load to the 8-bit A register, data from the absolute address specified by the 16-bit register HL. The value of
     *   HL is decremented after the memory read.
     */
    fn op_ld_a_hl_dec(&mut self) {
        debug!("op_ld_a_hl_dec");
        let hlv = self.registers.get_hl();
        self.registers.a = self.work_ram[hlv as usize];
        self.registers.set_hl(hlv - 1);
    }

    /*
     *   LD (HL-), A
     *   Load to the absolute address specified by the 16-bit register HL, data from the 8-bit A register. The value of
     *   HL is decremented after the memory write.
     */
    fn op_ld_hl_dec_a(&mut self) {
        debug!("op_ld_hl_dec_a");
        let hlv = self.registers.get_hl();
        self.work_ram[hlv as usize] = self.registers.a;
        self.registers.set_hl(hlv - 1);
    }

    /*
     *   LD A, (HL+)
     *   Load to the 8-bit A register, data from the absolute address specified by the 16-bit register HL. The value of
     *   HL is incremented after the memory read.
     */
    fn op_ld_a_hl_inc(&mut self) {
        debug!("op_ld_a_hl_inc");
        let hlv = self.registers.get_hl();
        self.registers.a = self.work_ram[hlv as usize];
        self.registers.set_hl(hlv + 1);
    }

    /*
     *   LD (HL+), A
     *   Load to the absolute address specified by the 16-bit register HL, data from the 8-bit A register. The value of
     *   HL is incremented after the memory write.
     */
    fn op_ld_hl_inc_a(&mut self) {
        debug!("op_ld_hl_inc_a");
        let hlv = self.registers.get_hl();
        self.work_ram[hlv as usize] = self.registers.a;
        self.registers.set_hl(hlv + 1);
    }

    /*
     *   LD rr, nn
     *   Load to the 16-bit register rr, the immediate 16-bit data nn.
     */
    // TODO function pointer here?
    fn op_ld_rr_nn(&mut self, register: &mut u16, value: u16) {
        debug!("op_ld_rr_nn");
        *register = value;
    }

    /*
     *   LD (nn), SP
     *   Load to the absolute address specified by the 16-bit operand nn, data from the 16-bit SP register.
     */
    fn op_ld_nn_sp(&mut self, address: u16) {
        debug!("op_ld_nn_sp");
        self.work_ram[address as usize] = (self.registers.sp & 0xFF) as u8;
        self.work_ram[(address + 1) as usize] = (self.registers.sp >> 8) as u8;
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
     *   PUSH rr
     *   Push to the stack memory, data from the 16-bit register rr.
     */
    // TODO - work ram in the stack?
    // TODO function pointer here?
    fn op_push_rr(&mut self, register: u16) {
        debug!("op_push_rr");
        self.registers.sp -= 2;
        self.work_ram[self.registers.sp as usize] = (register >> 8) as u8;
        self.work_ram[(self.registers.sp + 1) as usize] = register as u8;
    }

    /*
     *   POP rr
     *   Pops to the 16-bit register rr, data from the stack memory.
     *   This instruction does not do calculations that affect flags, but POP AF completely replaces the F register
     *   value, so all flags are changed based on the 8-bit data that is read from memory.
     */
    // TODO - work ram in the stack?
    // TODO function pointer here?
    fn op_pop_rr(&mut self, register: &mut u16) {
        debug!("op_pop_rr");
        *register = (self.work_ram[self.registers.sp as usize] as u16) << 8
            | self.work_ram[(self.registers.sp + 1) as usize] as u16;
        self.registers.sp += 2;
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
     *   JP cc, nn
     *   Conditional jump to the absolute address specified by the 16-bit operand nn, depending on the condition cc.
     *   Note that the operand (absolute address) is read even when the condition is false!
     */
    // TODO condition likely incorrect
    fn op_jp_cc_nn(&mut self, condition: bool, address: u16) {
        debug!("op_jp_cc_nn");
        if condition {
            self.registers.pc = address;
        }
    }

    /*
     *   JR e
     *   Unconditional jump to the relative address specified by the signed 8-bit operand e.
     */
    fn op_jr_e(&mut self, offset: i8) {
        debug!("op_jr_e");
        self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
    }

    /*
     *   JR cc, e
     *   Conditional jump to the relative address specified by the signed 8-bit operand e, depending on the condition cc.
     */
    // TODO condition likely incorrect
    fn op_jr_cc_e(&mut self, condition: bool, offset: i8) {
        debug!("op_jr_cc_e");
        if condition {
            self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
        }
    }

    /*
     *   CALL nn
     *   Unconditional function call to the absolute address specified by the 16-bit operand nn.
     */
    // TODO logic likely incorrect
    fn op_call_nn(&mut self, address: u16) {
        debug!("op_call_nn");
        self.op_push_rr(self.registers.pc);
        self.registers.pc = address;
    }

    /*
     *   CALL cc, nn
     *   Conditional function call to the absolute address specified by the 16-bit operand nn, depending on the condition cc.
     */
    // TODO logic likely incorrect
    fn op_call_cc_nn(&mut self, condition: bool, address: u16) {
        debug!("op_call_cc_nn");
        if condition {
            self.registers.pc += 1;
            let lsb = (self.registers.pc & 0xFF) as u8;
            //self.registers.pc += 1;
            let msb = (self.registers.pc >> 8) as u8;
            let nn: u16 = lsb as u16 | (msb as u16) << 8;
            self.registers.pc = nn;
        }
    }

    /*
     *   RET
     *   Unconditional return from a function.
     */
    fn op_ret(&mut self) {
        debug!("op_ret");
        unimplemented!("op_ret");
    }

    /*
     *   RET cc
     *   Conditional return from a function, depending on the condition cc.
     */
    // TODO logic likely incorrect
    fn op_ret_cc(&mut self, condition: bool) {
        debug!("op_ret_cc");
        if condition {
            let lsb = self.work_ram[self.registers.sp as usize];
            self.registers.sp += 1;
            let msb = self.work_ram[self.registers.sp as usize];
            self.registers.sp += 1;
            self.registers.pc = ((msb as u16) << 8) | lsb as u16;
        }
    }

    /*
     *   RETI
     *   Unconditional return from a function. Also enables interrupts by setting IME=1.
     */
    fn op_reti(&mut self) {
        debug!("op_reti");
        self.op_ret();
        self.work_ram[INTERRUPT_ENABLE_REGISTER as usize] = 1;
    }

    /*
     *   RST n
     *   Unconditional function call to the absolute fixed address defined by the opcode.
     */
    // TODO logic likely incorrect
    fn op_rst_n(&mut self, address: u16) {
        debug!("op_rst_n");
        let return_address = self.registers.pc;
        self.op_push_rr(return_address);
        self.registers.pc = address;
    }

    /*
     *   HALT
     *   STOP
     *   DI
     *   Disables interrupt handling by setting IME=0 and cancelling any scheduled effects of the EI instruction if any.
     */

    fn op_halt(&mut self) {
        debug!("op_halt");
        self.work_ram[INTERRUPT_ENABLE_REGISTER as usize] = 0;
    }
    fn op_stop(&mut self) {
        debug!("op_stop");
        self.work_ram[INTERRUPT_ENABLE_REGISTER as usize] = 0;
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
     *   CCF
     *   Flips the carry flag, and clears the N and H flags.
     */
    fn op_ccf(&mut self) {
        debug!("op_ccf");
        // Flip the carry flag (bit 4)
        self.registers
            .f
            .set_flag(Flag::C, !self.registers.f.get_flag(Flag::C));
        //self.registers.f ^= 0x10;
        // Clear the subtract (N) and half-carry (H) flags (bits 6 and 5)
        //self.registers.f &= 0b1001_1111;
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
    }

    /*
     *   SCF
     *   Sets the carry flag, and clears the N and H flags.
     */
    fn op_scf(&mut self) {
        debug!("op_scf");
        // Set the carry flag (bit 4)
        //self.registers.f |= 0x10;
        self.registers.f.set_flag(Flag::C, true);
        // Clear the subtract (N) and half-carry (H) flags (bits 6 and 5)
        //self.registers.f &= 0b1001_1111;
        self.registers.f.set_flag(Flag::N, false);
        self.registers.f.set_flag(Flag::H, false);
    }

    // TODO - very likely incorrect
    // TODO - Put function description comment
    fn op_daa(&mut self) {
        debug!("op_daa");

        let mut adjustment = 0;
        let mut carry_adjustment = 0;

        if self.registers.f.get_flag(Flag::H) || (self.registers.a & 0xF) > 9 {
            adjustment |= 0x06;
        }

        if self.registers.f.get_flag(Flag::C) || self.registers.a > 0x99 {
            adjustment |= 0x60;
            carry_adjustment |= 0x100u16;
        }

        let result = self.registers.a.wrapping_add(adjustment) as u16;
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, (result & 0x100) != 0);

        self.registers.a = (result + carry_adjustment) as u8;
    }

    /*
     *   CPL
     *   Flips all the bits in the 8-bit A register, and sets the N and H flags.
     */
    fn op_cpl(&mut self) {
        debug!("op_cpl");
        self.registers.a = !self.registers.a;
        self.registers.f.set_flag(Flag::N, true);
        self.registers.f.set_flag(Flag::H, true);
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

    /*
     *   Unconditional function call to the absolute fixed address defined by the opcode.
     */
    fn op_rst_address(&mut self, address: u16) {
        debug!("rst_address {}", address);
        self.op_push_rr(self.registers.pc);
        self.registers.pc = address;
    }

    fn op_push_stack(&mut self, address: u16) {
        debug!("op_push_rr");
        let sp = self.registers.sp.wrapping_sub(2);
        let msb = (address >> 8) as u8;
        let lsb = (address & 0xFF) as u8;
        self.work_ram[sp as usize] = msb;
        self.work_ram[(sp + 1) as usize] = lsb;
        self.registers.sp = sp;
    }

    fn op_pop_stack(&mut self) -> u16 {
        debug!("op_pop_rr");
        let sp = self.registers.sp;
        let msb = self.work_ram[sp as usize];
        let lsb = self.work_ram[(sp + 1) as usize];
        let value = (msb as u16) << 8 | lsb as u16;
        value
    }
}
