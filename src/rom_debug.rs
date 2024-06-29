use log::{debug, info};
pub struct rom_debug {
    message: Vec<char>,
}

impl rom_debug {
    pub fn new() -> rom_debug {
        rom_debug {
            message: Vec::new(),
        }
    }

    pub fn add_char(&mut self, c: char) {
        self.message.push(c);
    }

    pub fn add_string(&mut self, s: &str) {
        for c in s.chars() {
            self.add_char(c);
        }
    }

    pub fn print(&self) {
        if self.message.len() > 0 {
            let message: String = self.message.iter().collect();
            info!("DEBUG: {}", message);
        }
    }

    pub fn debug_print(&self, s: &str) {
        debug!("DEBUG: {}", s);
    }
}