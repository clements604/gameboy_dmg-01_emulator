
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
    f: u8, // Flags
    h: u8,
    l: u8,
    pc: u16, // Program counter
    sp: u16, // Stack pointer
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
            f: 0,
            h: 0,
            l: 0,
            pc: 0,
            sp: 0,
        }
    }

    fn get_af(&self) -> u16 {
        (self.a as u16) << 8 | self.f as u16
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
        self.f = value as u8;
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
    * nop - Do nothing
    */
    pub fn nop(&mut self) {
        debug!("NOP");
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

}
