use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmulatorConfig {
    pub boot_rom: Option<String>,
    pub scale_factor: u32,
}

#[derive(Debug)]
pub enum ConfigError {
    IoError(String),
    ParseError(String),
    SerializeError(String),
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            boot_rom: None,
            scale_factor: 1,
        }
    }
}

impl EmulatorConfig {
    pub fn new() -> Self {
        EmulatorConfig::load_from_file("config.json").unwrap_or_else(|_| EmulatorConfig::default())
    }
    /**
     * Load configuration from a JSON file.
     * If the file does not exist or is malformed, return the default configuration.
    */
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        let config: EmulatorConfig = serde_json::from_str(&contents)
            .map_err(|e| ConfigError::ParseError(e.to_string())).unwrap_or_else(|e| {
            EmulatorConfig::default()
        });

        Ok(config)
    }
}