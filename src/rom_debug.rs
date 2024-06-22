
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
            for c in &self.message {
                print!("DEBUG: {}", c);
            }
            println!();
        }
    }
}