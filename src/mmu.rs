
/*
*   Memory Management Unit -- MMU
*/

pub struct Mmu {

}

impl Mmu {
    pub fn new() -> Self {
        Mmu {
        }
    }

    /*
    *   Read 8-bit byte from a given address
    */
    pub fn rb(&self, addr: u16) -> u8 {
        unimplemented!("MMU - rb");
        0
    }

    /*
    *   Read 16-bit word from a given address
    */
    pub fn rw(&self, addr: u16) -> u8 {
        unimplemented!("MMU - rw");
        0
    }

    /*
    *   Write 8-bit byte to a given address
    */
    pub fn wb(&self, addr: u16, val: u8) {
        unimplemented!("MMU - wb");
    }

    /*
    *   Write 16-bit word to a given address
    */

    pub fn ww(&self, addr: u16, val: u16) {
        unimplemented!("MMU - ww");
    }


}
