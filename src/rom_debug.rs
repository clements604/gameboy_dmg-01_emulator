use std::process::exit;
use log::{debug, info, error};

/**
 * Integrates with Blargg's ROM Debugging system.
 */
#[allow(dead_code)]
pub struct RomDebug {
    message: Vec<char>,
}

#[allow(dead_code)]
impl RomDebug {
    pub fn new() -> RomDebug {
        RomDebug {
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
            if message.contains("Passed") {
                info!("PASSED: {}", message);
                //exit(0);
            } else if message.contains("Failed") {
                error!("FAILED: {}", message);
                exit(1);
            }
            debug!("DEBUG: {}", message);
        }
    }

    pub fn debug_print(&self, s: &str) {
        debug!("DEBUG: {}", s);
    }
}