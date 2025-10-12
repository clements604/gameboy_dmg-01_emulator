use log::debug;

use crate::dma::Dma;
use crate::io::IO;
use crate::interrupts::{Interrupt, InterruptFlags};
use crate::mbc::MBC;
use crate::mbc_factory;
use crate::rom::{ROM, ROMBanks};

const BOOT_ROM_START: u16 = 0x0000;
const BOOT_ROM_END: u16 = 0x00FF;
const BOOT_ROM_SIZE: usize = ((BOOT_ROM_END - BOOT_ROM_START) + 1) as usize;
const ROM_BANK_0_START: u16 = 0x0000;
const ROM_BANK_0_END: u16 = 0x3FFF;
const ROM_BANK_0_SIZE: usize = ((ROM_BANK_0_END - ROM_BANK_0_START) + 1) as usize;
const ROM_BANK_N_START: u16 = 0x4000;
const ROM_BANK_N_END: u16 = 0x7FFF;
const ROM_BANK_N_SIZE: usize = ((ROM_BANK_N_END - ROM_BANK_N_START) + 1) as usize;
pub const VRAM_START: u16 = 0x8000;
pub const VRAM_END: u16 = 0x9FFF;
const EXTERNAL_RAM_START: u16 = 0xA000;
const EXTERNAL_RAM_END: u16 = 0xBFFF;
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
const UNUSED_START: u16 = 0xFEA0;
const UNUSED_END: u16 = 0xFEFF;
pub const IO_REGISTERS_START: u16 = 0xFF00;
pub const IO_REGISTERS_END: u16 = 0xFF7F;
pub const IO_REGISTERS_SIZE: usize = ((IO_REGISTERS_END - IO_REGISTERS_START) + 1) as usize;
const HRAM_START: u16 = 0xFF80;
const HRAM_END: u16 = 0xFFFE;
const HRAM_SIZE: usize = ((HRAM_END - HRAM_START) + 1) as usize;
pub const INTERRUPT_ENABLE_REGISTER: u16 = 0xFFFF;
const ROM_BANK_SELECT_START: u16 = 0x2000;
const IF_REGISTER: u16 = 0xFF0F;
const BOOT_ROM_DISABLE: u16 = 0xFF50;
const DMA_TRANSFER: u16 = 0xFF46;
const BITS_PER_BYTE: usize = 8;

pub struct MemoryBus {
    boot_rom: Vec<u8>,
    rom_banks: ROMBanks,
    rom_bank_0: [u8; ROM_BANK_0_SIZE],
    rom_bank_n: [u8; ROM_BANK_N_SIZE],
    wram_0: [u8; WRAM_0_SIZE],
    wram_1: [u8; WRAM_1_SIZE],
    echo_ram: [u8; ECHO_RAM_SIZE],
    hram: [u8; HRAM_SIZE],
    pub dmg_io: IO,
    dma: Dma,
    boot_rom_enabled: bool,
    pub interrupt_master_enable: bool,
    pub enabling_ime: bool,
    pub interrupt_enable_register: u8,
    pub interrupt_flags: u8,
    pub mbc: Option<Box<dyn MBC>>,
}

impl MemoryBus {
    pub fn new(boot_rom:Option<Vec<u8>>, rom: &ROM) -> MemoryBus {
        let mbc = mbc_factory::create_mbc(rom);
        
        let mut memory_bus = MemoryBus {
            boot_rom_enabled: match boot_rom {
                Some(_) => true,
                None => false
            },
            boot_rom: match boot_rom {
                Some(boot_rom) => {
                    boot_rom
                },
                None => {
                    vec![0; BOOT_ROM_SIZE]
                }
            },
            rom_banks: rom.load_rom_to_banks(),
            rom_bank_0: [0; ROM_BANK_0_SIZE],
            rom_bank_n: [0; ROM_BANK_N_SIZE],
            wram_0: [0; WRAM_0_SIZE],
            wram_1: [0; WRAM_1_SIZE],
            echo_ram: [0; ECHO_RAM_SIZE],
            hram: [0; HRAM_SIZE],
            dmg_io: IO::new(),
            dma: Dma::new(),
            interrupt_master_enable: false,
            enabling_ime: false,
            interrupt_enable_register: 0,
            interrupt_flags: 0,
            mbc: Some(mbc),
        };
        memory_bus.load_rom(&rom.rom);
        memory_bus.update_visible_banks();
        memory_bus
    }

    pub fn cycle(&mut self, cpu_cycles: u8) {
        // Timer
        if self.dmg_io.timer.cycle(1) {
            // If timer overflows, trigger a Timer interrupt
            self.trigger_interrupt(Interrupt::TIMER);
        }
        // OAM
        for _ in 0..cpu_cycles {
            if let Some((src_addr, dest_addr)) = self.dma.dma_tick() {
                let value = self.read_byte(src_addr);
                self.dmg_io.ppu.oam_dma_write(dest_addr, value); // ← Use DMA function
            }
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        match address {
            ROM_BANK_0_START..=ROM_BANK_0_END => {
                if self.boot_rom_enabled && address <= BOOT_ROM_END {
                    self.boot_rom[address as usize]
                } else {
                    self.mbc.as_ref().unwrap().read_byte(address)
                }
            },
            ROM_BANK_N_START..=ROM_BANK_N_END => self.mbc.as_ref().unwrap().read_byte(address),
            VRAM_START..=VRAM_END => self.dmg_io.ppu.vram_read(address),
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => self.mbc.as_ref().unwrap().read_byte(address),
            WRAM_0_START..=WRAM_0_END => self.wram_0[(address - WRAM_0_START) as usize],
            WRAM_1_START..=WRAM_1_END => self.wram_1[(address - WRAM_1_START) as usize],
            ECHO_RAM_START..=ECHO_RAM_END => self.echo_ram[(address - ECHO_RAM_START) as usize],
            OAM_START..=OAM_END => {
                if self.dma.is_transferring() {
                    return 0xFF;
                }
                self.dmg_io.ppu.oam_read(address - OAM_START)
            },
            UNUSED_START..=UNUSED_END => 0xFF,
            IO_REGISTERS_START..=IO_REGISTERS_END => {
                if address ==  IF_REGISTER {
                    return self.interrupt_flags;
                }
                self.dmg_io.read(address)
            }
            HRAM_START..=HRAM_END => self.hram[(address - HRAM_START) as usize],
            INTERRUPT_ENABLE_REGISTER => {
                self.interrupt_enable_register
            }
        }
    }
    pub fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            ROM_BANK_0_START..=ROM_BANK_0_END => {
                if self.boot_rom_enabled && address <= BOOT_ROM_END {
                    self.boot_rom[address as usize] = value;
                } else {
                    self.mbc.as_mut().unwrap().write_byte(address, value);
                    if address >= ROM_BANK_SELECT_START {
                        self.update_visible_banks();
                    }
                }
            },
            ROM_BANK_N_START..=ROM_BANK_N_END => {
                self.mbc.as_mut().unwrap().write_byte(address, value);
                self.update_visible_banks();
            },
            
            VRAM_START..=VRAM_END => {
                self.dmg_io.ppu.vram_write(address, value);
            },
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => self.mbc.as_mut().unwrap().write_byte(address, value),
            WRAM_0_START..=WRAM_0_END => self.wram_0[(address - WRAM_0_START) as usize] = value,
            WRAM_1_START..=WRAM_1_END => self.wram_1[(address - WRAM_1_START) as usize] = value,
            ECHO_RAM_START..=ECHO_RAM_END => {
                self.echo_ram[(address - ECHO_RAM_START) as usize] = value
            }
            OAM_START..=OAM_END => {
                if !self.dma.is_transferring() {
                    self.dmg_io.ppu.oam_write(address - OAM_START, value);
                }
            },
            UNUSED_START..=UNUSED_END => debug!("Write to unused memory"),
            IO_REGISTERS_START..=IO_REGISTERS_END => {
                match address {
                    IF_REGISTER => {
                        self.interrupt_flags = value;
                    },
                    BOOT_ROM_DISABLE => {
                        debug!("Boot ROM disable");
                        self.boot_rom_enabled = false;
                    },
                    DMA_TRANSFER => {
                        self.dma.dma_start(value);
                    },
                    _ => {
                        let interrupt = self.dmg_io.write(address, value);
                        match interrupt {
                            Some(interrupt) => {
                                self.trigger_interrupt(interrupt);
                            },
                            None => {}
                        }
                    }
                }
            },
            HRAM_START..=HRAM_END => {
                self.hram[(address - HRAM_START) as usize] = value
            },
            INTERRUPT_ENABLE_REGISTER => {
                self.interrupt_enable_register = value;
            }
        }
    }

    /*
     *   Read the 16-bit value from memory for a given address.
     */
    pub fn read_short(&mut self, address: u16) -> u16 {
        let lsb = self.read_byte(address);
        let msb = self.read_byte(address.wrapping_add(1));
        (msb as u16) << 8 | lsb as u16
    }
    
    pub fn write_short(&mut self, address: u16, value: u16) {
        let lsb = value as u8;
        let msb = (value >> BITS_PER_BYTE) as u8;
        self.write_byte(address, lsb);
        self.write_byte(address.wrapping_add(1), msb);
    }

    /*
     *   Load the ROM into memory
     */
    pub fn load_rom(&mut self, rom: &Vec<u8>) {
        debug!("Loading ROM");
        let bank_0_end = ROM_BANK_0_SIZE.min(rom.len());
        self.rom_bank_0[..bank_0_end].copy_from_slice(&rom[..bank_0_end]);
        if rom.len() > ROM_BANK_0_SIZE {
            let bank_n_end = ROM_BANK_N_SIZE.min(rom.len() - ROM_BANK_0_SIZE);
            self.rom_bank_n[..bank_n_end].copy_from_slice(&rom[ROM_BANK_0_SIZE..ROM_BANK_0_SIZE + bank_n_end]);
        }
    }

    pub fn update_visible_banks(&mut self) {
        if let Some(mbc) = &self.mbc {
            let rom_bank = mbc.get_rom_bank();

            if let Some(bank_data) = self.rom_banks.data.get(0) {
                if bank_data.len() <= self.rom_bank_0.len() {
                    self.rom_bank_0[..bank_data.len()].copy_from_slice(bank_data);
                    // Zero the rest if bank is smaller than buffer
                    if bank_data.len() < self.rom_bank_0.len() {
                        for i in bank_data.len()..self.rom_bank_0.len() {
                            self.rom_bank_0[i] = 0;
                        }
                    }
                } else {
                    let bank_0_len = self.rom_bank_0.len();
                    self.rom_bank_0.copy_from_slice(&bank_data[..bank_0_len]);
                }
            }
            if let Some(bank_data) = self.rom_banks.data.get(rom_bank) {
                if bank_data.len() <= self.rom_bank_n.len() {
                    self.rom_bank_n[..bank_data.len()].copy_from_slice(bank_data);
                    // Zero the rest if bank is smaller than buffer
                    if bank_data.len() < self.rom_bank_n.len() {
                        for i in bank_data.len()..self.rom_bank_n.len() {
                            self.rom_bank_n[i] = 0;
                        }
                    }
                } else {
                    let bank_n_len = self.rom_bank_n.len();
                    self.rom_bank_n.copy_from_slice(&bank_data[..bank_n_len]);
                }
            }
        }
    }
    pub fn trigger_interrupt(&mut self, interrupt: Interrupt) {
        let mut interrupts: InterruptFlags = self.interrupt_flags.into();
        match interrupt {
            Interrupt::VBLANK => interrupts.vblank = true,
            Interrupt::LCDSTAT => interrupts.lcd_stat = true,
            Interrupt::TIMER => interrupts.timer = true,
            Interrupt::SERIAL => interrupts.serial = true,
            Interrupt::JOYPAD => interrupts.joypad = true,
        }
        self.interrupt_flags = interrupts.into();
    }
    
}
