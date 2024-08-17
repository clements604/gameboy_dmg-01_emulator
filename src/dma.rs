use crate::memory_bus::MemoryBus;
use log::{debug, error};
use std::cell::RefCell;
use std::rc::Rc;
pub struct Dma {
    memory_bus: Rc<RefCell<MemoryBus>>,
    active: bool,
    dma_byte: u8,
    dma_value: u8,
    dma_delay: u8,
}

impl Dma {
    pub fn new(memory_bus: Rc<RefCell<MemoryBus>>) -> Dma {
        Dma {
            memory_bus,
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
    pub fn dma_tick(&mut self) {
        if !self.active {
            return;
        }

        if self.dma_delay > 0 {
            self.dma_delay -= 1;
            return;
        }
        
        //let mut memory_bus = self.memory_bus.borrow_mut();
        let value = self.memory_bus.borrow().read_byte((self.dma_value as u16 * 0x100) + self.dma_byte as u16);
        //memory_bus.write_byte(self.dma_byte as u16, value);
        
        self.memory_bus.borrow_mut().ppu.as_ref().unwrap().borrow_mut().oam_write(self.dma_byte as u16, value);
        //self.memory_bus.borrow().ppu_experiment.as_ref().unwrap().borrow_mut().oam_write(self.dma_byte as u16, value);
        
        debug!(
            "DMA transfer: {:#X} -> {:#X}",
            (self.dma_value as u16 * 0x100) + self.dma_byte as u16,
            self.dma_byte
        );
        self.dma_byte += 1;
        self.active = self.dma_byte < 0xA0;

        if !self.active {
            error!("DMA transfer complete");
        }
    }

    pub fn is_transferring(&self) -> bool {
        self.active
    }
}
