use log::{debug, error};

pub struct IO {
    serial_transfer_data: u8,
    serial_transfer_control: u8,
}

impl IO {
    pub fn new() -> IO {
        IO {
            serial_transfer_data: 0,
            serial_transfer_control: 0,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xFF01 => {
                self.serial_transfer_data = value;
            },
            0xFF02 => {
                self.serial_transfer_control = value;
            },
            _ => {
                error!("Write to unhandled IO address: {:#X}", address);
            }
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0xFF01 => {
                self.serial_transfer_data
            },
            0xFF02 => {
                self.serial_transfer_control
            },
            _ => {
                error!("Read from unhandled IO address: {:#X}", address);
                0
            }
        }
    }
}
