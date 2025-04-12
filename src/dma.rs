use crate::memory_bus::MemoryBus;
use log::{debug, error};
use std::cell::RefCell;
use std::rc::Rc;
pub struct Dma {
    active: bool,
    dma_byte: u8,
    dma_value: u8,
    dma_delay: u8,
}

impl Dma {
    pub fn new() -> Dma {
        Dma {
            active: false,
            dma_byte: 0,
            dma_value: 0,
            dma_delay: 0,
        }
    }

    pub(crate) fn dma_start(&mut self, start: u8) {
        self.active = true;
        self.dma_byte = 0;
        self.dma_delay = 2;
        self.dma_value = start;
    }
    pub fn dma_tick(&mut self) -> Option<(u16, u16)> {
        if !self.active {
            return None;
        }

        if self.dma_delay > 0 {
            self.dma_delay -= 1;
            return None;
        }

        let src_addr = (self.dma_value as u16 * 0x100) + self.dma_byte as u16;
        let dest_addr = self.dma_byte as u16;
        
        debug!(
            "DMA transfer: {:#X} -> {:#X}",
            (self.dma_value as u16 * 0x100) + self.dma_byte as u16,
            self.dma_byte
        );
        self.dma_byte += 1;
        self.active = self.dma_byte < 0xA0;

        if !self.active {
            debug!("DMA transfer complete");
        }

        Some((src_addr, dest_addr))
    }

    pub fn is_transferring(&self) -> bool {
        self.active
    }
}
