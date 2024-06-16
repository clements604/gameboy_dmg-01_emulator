use crate::rom::ROM;
use crate::ppu::Ppu;
use log::{debug, error, info};
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Read};

const ROM_BANK_0_START: u16 = 0x0000;
const ROM_BANK_0_END: u16 = 0x3FFF;
const ROM_BANK_0_SIZE: usize = ((ROM_BANK_0_END - ROM_BANK_0_START) + 1) as usize;
const ROM_BANK_N_START: u16 = 0x4000;
const ROM_BANK_N_END: u16 = 0x7FFF;
const ROM_BANK_N_SIZE: usize = ((ROM_BANK_N_END - ROM_BANK_N_START) + 1) as usize;
const VRAM_START: u16 = 0x8000;
const VRAM_END: u16 = 0x9FFF;
const VRAM_SIZE: usize = ((VRAM_END - VRAM_START) + 1) as usize;
const EXTERNAL_RAM_START: u16 = 0xA000;
const EXTERNAL_RAM_END: u16 = 0xBFFF;
const EXTERNAL_RAM_SIZE: usize = ((EXTERNAL_RAM_END - EXTERNAL_RAM_START) + 1) as usize;
const WRAM_0_START: u16 = 0xC000;
const WRAM_0_END: u16 = 0xCFFF;
const WRAM_0_SIZE: usize = (WRAM_1_END - WRAM_0_START + 1) as usize;
const WRAM_1_START: u16 = 0xD000;
const WRAM_1_END: u16 = 0xDFFF;
const WRAM_1_SIZE: usize = ((WRAM_1_END - WRAM_1_START) + 1) as usize;
const ECHO_RAM_START: u16 = 0xE000;
const ECHO_RAM_END: u16 = 0xFDFF;
const ECHO_RAM_SIZE: usize = ((ECHO_RAM_END - ECHO_RAM_START) + 1) as usize;
const OAM_START: u16 = 0xFE00;
const OAM_END: u16 = 0xFE9F;
const OAM_SIZE: usize = ((OAM_END - OAM_START) + 1) as usize;
const UNUSED_START: u16 = 0xFEA0;
const UNUSED_END: u16 = 0xFEFF;
const UNUSED_SIZE: usize = ((UNUSED_END - UNUSED_START) + 1) as usize;
const IO_REGISTERS_START: u16 = 0xFF00;
const IO_REGISTERS_END: u16 = 0xFF7F;
const IO_REGISTERS_SIZE: usize = ((IO_REGISTERS_END - IO_REGISTERS_START) + 1) as usize;
const HRAM_START: u16 = 0xFF80;
const HRAM_END: u16 = 0xFFFE;
const HRAM_SIZE: usize = ((HRAM_END - HRAM_START) + 1) as usize;
pub const INTERRUPT_ENABLE_REGISTER: u16 = 0xFFFF;
pub struct MemoryBus {
    pub rom: [u8; (ROM_BANK_0_END - ROM_BANK_0_START) as usize], // TODO UPDATE
    pub rom_bank_0: [u8; ROM_BANK_0_SIZE],
    pub rom_bank_n: [u8; ROM_BANK_N_SIZE],
    pub vram: [u8; VRAM_SIZE],
    pub external_ram: [u8; EXTERNAL_RAM_SIZE],
    pub wram_0: [u8; WRAM_0_SIZE],
    pub wram_1: [u8; WRAM_1_SIZE],
    pub echo_ram: [u8; ECHO_RAM_SIZE],
    pub oam: [u8; OAM_SIZE],
    pub unused: [u8; UNUSED_SIZE],
    pub io_registers: [u8; IO_REGISTERS_SIZE],
    pub hram: [u8; HRAM_SIZE],
    pub interrupt_enable_register: u8,
    pub ppu: Ppu,
}

impl<'a> MemoryBus {
    pub fn new(rom: &Vec<u8>) -> MemoryBus {
        let mut memory_bus = MemoryBus {
            rom: [0; (ROM_BANK_0_END - ROM_BANK_0_START) as usize],
            rom_bank_0: [0; ROM_BANK_0_SIZE],
            rom_bank_n: [0; ROM_BANK_N_SIZE],
            vram: [0; VRAM_SIZE],
            external_ram: [0; EXTERNAL_RAM_SIZE],
            wram_0: [0; WRAM_0_SIZE],
            wram_1: [0; WRAM_1_SIZE],
            echo_ram: [0; ECHO_RAM_SIZE],
            oam: [0; OAM_SIZE],
            unused: [0; UNUSED_SIZE],
            io_registers: [0; IO_REGISTERS_SIZE],
            hram: [0; HRAM_SIZE],
            interrupt_enable_register: 0,
            ppu: Ppu::new(),
        };
        //debug!("ROM data to be loaded: {:?}", rom);
        memory_bus.load_rom(rom);

        debug!("Memory bus created");

        //debug!("Memory bus bank 0: {:?}", memory_bus.rom_bank_0);
        //debug!("Memory bus bank n: {:?}", memory_bus.rom_bank_n);

        memory_bus
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        debug!("Reading byte from address {:X}", address);
        match address {
            ROM_BANK_0_START..=ROM_BANK_0_END => self.rom_bank_0[(address) as usize],
            ROM_BANK_N_START..=ROM_BANK_N_END => {
                self.rom_bank_n[(address - ROM_BANK_N_START) as usize]
            }
            VRAM_START..=VRAM_END => {
                //self.vram[(address - VRAM_START) as usize]
                self.ppu.vram_read(address)
            },
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => {
                self.external_ram[(address - EXTERNAL_RAM_START) as usize]
            }
            WRAM_0_START..=WRAM_0_END => self.wram_0[(address - WRAM_0_START) as usize],
            WRAM_1_START..=WRAM_1_END => self.wram_1[(address - WRAM_1_START) as usize],
            ECHO_RAM_START..=ECHO_RAM_END => self.echo_ram[(address - ECHO_RAM_START) as usize],
            OAM_START..=OAM_END => { 
                self.ppu.oam_read(address - OAM_START)
            },
            UNUSED_START..=UNUSED_END => self.unused[(address - UNUSED_START) as usize],
            IO_REGISTERS_START..=IO_REGISTERS_END => {
                self.io_registers[(address - IO_REGISTERS_START) as usize]
            }
            HRAM_START..=HRAM_END => self.hram[(address - HRAM_START) as usize],
            INTERRUPT_ENABLE_REGISTER => {
                self.interrupt_enable_register
            }
            _ => {
                panic!("Unimplemented read_byte in memory bus")
            }
        }
    }
    pub fn write_byte(&mut self, address: u16, value: u8) {
        debug!("Writing byte to address {:X}", address);
        match address {
            ROM_BANK_0_START..=ROM_BANK_0_END => self.rom_bank_0[address as usize] = value,
            ROM_BANK_N_START..=ROM_BANK_N_END => {
                self.rom_bank_n[(address - ROM_BANK_N_START) as usize] = value
            }
            VRAM_START..=VRAM_END => { 
                //self.vram[(address - VRAM_START) as usize] = value;
                self.ppu.vram_write(address, value);
            },
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => {
                self.external_ram[(address - EXTERNAL_RAM_START) as usize] = value
            }
            WRAM_0_START..=WRAM_0_END => self.wram_0[(address - WRAM_0_START) as usize] = value,
            WRAM_1_START..=WRAM_1_END => self.wram_1[(address - WRAM_1_START) as usize] = value,
            ECHO_RAM_START..=ECHO_RAM_END => {
                self.echo_ram[(address - ECHO_RAM_START) as usize] = value
            }
            OAM_START..=OAM_END => {
                self.ppu.oam_write(address - OAM_START, value);
            },
            UNUSED_START..=UNUSED_END => self.unused[(address - UNUSED_START) as usize] = value,
            IO_REGISTERS_START..=IO_REGISTERS_END => self.interrupt_enable_register = value,
            HRAM_START..=HRAM_END => {
                //unimplemented!("HRAM write");
                self.hram[(address - HRAM_START) as usize] = value
            },
            INTERRUPT_ENABLE_REGISTER => self.interrupt_enable_register = value,
            _ => {
                panic!("Unimplemented write_byte in memory bus")
            }
        }
    }

    /*
     *   Read the 16-bit value from memory for a given address.
     */
    /*pub fn read_short(&mut self, address: u16) -> u16 {
        match address {
            ROM_BANK_0_START..=ROM_BANK_0_END => {
                let lsb = self.rom_bank_0[address as usize];
                let msb = self.rom_bank_0[address.wrapping_add(1) as usize];
                (msb as u16) << 8 | lsb as u16
            }
            ROM_BANK_N_START..=ROM_BANK_N_END => {
                let lsb = self.rom_bank_n[(address - ROM_BANK_N_START) as usize];
                let msb = self.rom_bank_n[(address - ROM_BANK_N_START).wrapping_add(1) as usize];
                (msb as u16) << 8 | lsb as u16
            }
            VRAM_START..=VRAM_END => {
                let lsb = self.vram[(address - VRAM_START) as usize];
                let msb = self.vram[(address - VRAM_START).wrapping_add(1) as usize];
                (msb as u16) << 8 | lsb as u16
            }
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => {
                let lsb = self.external_ram[(address - EXTERNAL_RAM_START) as usize];
                let msb =
                    self.external_ram[(address - EXTERNAL_RAM_START).wrapping_add(1) as usize];
                (msb as u16) << 8 | lsb as u16
            }
            WRAM_0_START..=WRAM_0_END => {
                let lsb = self.wram_0[(address - WRAM_0_START) as usize];
                let msb = self.wram_0[(address - WRAM_0_START).wrapping_add(1) as usize];
                (msb as u16) << 8 | lsb as u16
            },
            WRAM_1_START..=WRAM_1_END => {
                let lsb = self.wram_0[(address - WRAM_1_START) as usize];
                let msb = self.wram_0[(address - WRAM_1_START).wrapping_add(1) as usize];
                (msb as u16) << 8 | lsb as u16
            },
            ECHO_RAM_START..=ECHO_RAM_END => {
                debug!(
                    "Echo RAM (mirror of C000–DDFF), Nintendo says use of this area is prohibited."
                );
                return 0x0;
            }
            OAM_START..=OAM_END => {
                let lsb = self.oam[(address - OAM_START) as usize];
                let msb = self.oam[(address - OAM_START).wrapping_add(1) as usize];
                (msb as u16) << 8 | lsb as u16
            }
            UNUSED_START..=UNUSED_END => {
                debug!("Not Usable");
                return 0x0;
            }
            IO_REGISTERS_START..=IO_REGISTERS_END => {
                let lsb = self.io_registers[(address - IO_REGISTERS_START) as usize];
                let msb = self.io_registers[(address - IO_REGISTERS_START).wrapping_add(1) as usize];
                (msb as u16) << 8 | lsb as u16
            }
            HRAM_START..=HRAM_END => {
                let lsb = self.hram[(address - HRAM_START) as usize];
                let msb = self.hram[(address - HRAM_START).wrapping_add(1) as usize];
                (msb as u16) << 8 | lsb as u16
            }
            INTERRUPT_ENABLE_REGISTER => {
                unimplemented!("Interrupt Enable Register")
            }
            _ => panic!("Unsupported address: {:X}", address),
        }
    }*/

    /*
     *   Load the ROM into memory
     */
    pub fn load_rom(&mut self, rom: &Vec<u8>) {
        debug!("Loading ROM");

        let bank_0 = rom[ROM_BANK_0_START as usize..=ROM_BANK_0_END as usize].to_vec();
        self.rom_bank_0 = bank_0.clone().try_into().unwrap();
        debug!("ROM Bank 0 loaded into memory {}", bank_0.len());

        let bank_n = rom[ROM_BANK_N_START as usize..=ROM_BANK_N_END as usize].to_vec();
        self.rom_bank_n = bank_n.clone().try_into().unwrap();
        debug!("ROM Bank N loaded into memory");
    }

    /*
     *  Load the boot ROM into memory
     */
    /*pub fn load_boot_rom(&mut self, file_path: String) {
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

        /*for (i, byte) in buffer.iter().enumerate() {
            self.work_ram[i] = *byte;
        }*/
        self.work_ram[0x0000..0x0100].copy_from_slice(&buffer);

        for i in 0x00..0x100 {
            debug!("byte [0x{:X}] = 0x{:X}", i, self.work_ram[i])
        }

        self.registers.pc = 0x0000;

        debug!("Boot ROM loaded into memory");
    }*/
}
