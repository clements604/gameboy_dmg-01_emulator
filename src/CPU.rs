
use std::{error, fmt};
use std::fs::File;
use std::io::prelude::*;
use log::{debug, error};

use crate::constants;
use constants::*;
use crate::rom;
use rom::*;


struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: FlagsRegister, // Flags
    h: u8,
    l: u8,
    pc: u16, // Program counter
    sp: u16, // Stack pointer
}

struct FlagsRegister {
    zero: bool,
    subtract: bool,
    half_carry: bool,
    carry: bool
}

#[derive(Debug, Clone, Copy)]
pub enum Flag {
    Z, // Zero flag
    N, // Subtract flag
    H, // Half-carry flag
    C, // Carry flag
}

pub struct CPU {
    registers: Registers,
    work_ram: [u8; 0xFFFF],
    video_ram: [u16; 8192],
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
            pc: 0,
            sp: 0,
        }
    }

    fn get_af(&self) -> u16 {
        let flags: u16 = self.f.into();
        (self.a as u16) << 8 | flags
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

impl FlagsRegister {
    pub fn new() -> Self {
        FlagsRegister {
            zero: false,
            subtract: false,
            half_carry: false,
            carry: false
        }
    }

    fn set_flag(&mut self, flag: Flag, value: bool) {
        match flag {
            Flag::Z => self.zero = value,
            Flag::N => self.subtract = value,
            Flag::H => self.half_carry = value,
            Flag::C => self.carry = value
        }
    }

    fn get_flag(&self, flag: Flag) -> bool {
        match flag {
            Flag::Z => self.zero,
            Flag::N => self.subtract,
            Flag::H => self.half_carry,
            Flag::C => self.carry
        }
    }
    
}

impl std::convert::From<FlagsRegister> for u8  {
    fn from(flag: FlagsRegister) -> u8 {
        (if flag.zero       { 1 } else { 0 }) << ZERO_FLAG_BYTE_POSITION |
        (if flag.subtract   { 1 } else { 0 }) << SUBTRACT_FLAG_BYTE_POSITION |
        (if flag.half_carry { 1 } else { 0 }) << HALF_CARRY_FLAG_BYTE_POSITION |
        (if flag.carry      { 1 } else { 0 }) << CARRY_FLAG_BYTE_POSITION
    }
}

impl std::convert::From<FlagsRegister> for u16  {
    fn from(flag: FlagsRegister) -> u16 {
        let mut result: u16 = 0;
        result |= (if flag.zero       { 1 } else { 0 }) << ZERO_FLAG_BYTE_POSITION;
        result |= (if flag.subtract   { 1 } else { 0 }) << SUBTRACT_FLAG_BYTE_POSITION;
        result |= (if flag.half_carry { 1 } else { 0 }) << HALF_CARRY_FLAG_BYTE_POSITION;
        result |= (if flag.carry      { 1 } else { 0 }) << CARRY_FLAG_BYTE_POSITION;
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
            carry
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
            carry
        }
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            registers: Registers::new(),
            work_ram: [0; 0xFFFF],
            video_ram: [0; 8192],
        }
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
        debug!("ROM file size: {} bytes / {} kilobytes", buffer.len(), buffer.len() / 1024);

        let rom = ROM::new(buffer);
        debug!("{}", rom);

        rom.validate_header_checksum().unwrap(); // Panics if the header checksum is invalid
        // Check cartridge type and load the ROM into memory based on the type
        for byte in 0x00..0x3FFF { // TODO incorrect start and finish for ROM, this would include headers...
            self.work_ram[byte] = rom.rom[byte];
        }
        debug!("ROM Bank 0 loaded into memory");
        if rom.cartridge_type == 0x00 {
            // ROM ONLY
            debug!("ROM ONLY");
        }
        else {
            debug!("ROM with MBC");
            let rom_banks = rom.load_rom_to_banks();
            self.work_ram[0x4000..=0x7FFF].copy_from_slice(&rom_banks.data[0]); // Load the first bank of the ROM into memory
            debug!("ROM Bank 1 loaded into memory, size: {} bytes", rom_banks.data[0].len());
        }
        //debug!("ROM loaded into memory");
    }

    /*
    *   Write data to
    */
    //TODO
    pub fn write_rom(&mut self, address: u16, data: u8) {
        unimplemented!("write_rom");
    }
    
    /*
    *   LD r, r’
    *   Load to the 8-bit register r, data from the 8-bit register r’.
    */
    fn op_ld_r8_r8(&mut self, source: &u8, destination: &mut u8) {
        debug!("source register: {:X}, destination reguster: {:X}", source, destination);
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
        *register = (self.work_ram[self.registers.sp as usize] as u16) << 8 | self.work_ram[(self.registers.sp + 1) as usize] as u16;
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
        self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
    }

    /*
    *   JR cc, e
    *   Conditional jump to the relative address specified by the signed 8-bit operand e, depending on the condition cc.
    */
    // TODO condition likely incorrect
    fn op_jr_cc_e(&mut self, condition: bool, offset: i8) {
        debug!("op_jr_cc_e");
        if condition {
            self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
        }
    }

    /*
    *   CALL nn
    *   Unconditional function call to the absolute address specified by the 16-bit operand nn.
    */
    // TODO logic likely incorrect
    fn op_call_nn(&mut self, address: u16) {
        debug!("op_call_nn");
        //self.op_push_rr(self.registers.pc);
        //self.registers.pc = address;
        self.registers.pc += 1;
        let lsb = (self.registers.pc & 0xFF) as u8;
        self.registers.pc += 1;
        let msb = (self.registers.pc >> 8) as u8;
        let nn: u16 = lsb as u16 | (msb as u16) << 8;
        self.registers.pc = nn;
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
            self.registers.pc += 1;
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
        let lsb = self.work_ram[self.registers.sp as usize];
        self.registers.sp += 1;
        let msb = self.work_ram[self.registers.sp as usize];
        self.registers.sp += 1;
        self.registers.pc = ((msb as u16) << 8) | lsb as u16;
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
        self.work_ram[INTERRUPT_ENABLE_REGISTER as usize] = 0;
    }

    /*
    *   EI
    *   Schedules interrupt handling to be enabled after the next machine cycle.
    */
    fn op_ei(&mut self) {
        debug!("op_ei");
        self.work_ram[INTERRUPT_ENABLE_REGISTER as usize] = 1;
    }

    /*
    *   CCF
    *   Flips the carry flag, and clears the N and H flags.
    */
    fn op_ccf(&mut self) {
        debug!("op_ccf");
        // Flip the carry flag (bit 4)
        self.registers.f.set_flag(Flag::C, !self.registers.f.get_flag(Flag::C));
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

    /*
    *   NOP
    *   No-operation. This instruction doesn’t do anything, but can be used to add a delay of one machine cycle and
    *   increment PC by one.
    */
    // TODO - very likely incorrect
    fn op_daa(&mut self) {
        debug!("op_daa");
    
        let mut adjustment = 0;
        let mut carry_adjustment = 0;
    
        if self.registers.f.get_flag(Flag::H) || (self.registers.a & 0xF) > 9 {
            adjustment |= 0x06;
        }
    
        if self.registers.f.get_flag(Flag::C) || self.registers.a > 0x99 {
            adjustment |= 0x60;
            carry_adjustment |= 0x100;
        }
    
        let result = self.registers.a.wrapping_add(adjustment);
        self.registers.f.set_flag(Flag::Z, result == 0);
        self.registers.f.set_flag(Flag::H, false);
        self.registers.f.set_flag(Flag::C, (result & 0x100) != 0);
    
        self.registers.a = result + carry_adjustment;
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

}
