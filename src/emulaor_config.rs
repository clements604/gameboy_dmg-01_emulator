use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use log::error;

const CONFIG_FILE_PATH: &str = "./config.json";

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
        //EmulatorConfig::load_from_file("./config.json.s").unwrap_or_else(|_| EmulatorConfig::default())
        EmulatorConfig::load_from_file(CONFIG_FILE_PATH).expect("Failed to load config file")
    }
    /**
     * Load configuration from a JSON file.
     * If the file does not exist or is malformed, return the default configuration.
    */
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = match fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e.to_string())) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read config file, using default. Error: {:?}", e);
                let config = EmulatorConfig::default();
                EmulatorConfig::save_to_file(&config, CONFIG_FILE_PATH).unwrap_or_else(|e| {
                    error!("Failed to save default config: {:?}", e);
                });
                return Ok(config);
            }
        };

        let config: EmulatorConfig = serde_json::from_str(&contents)
            .map_err(|e| ConfigError::ParseError(e.to_string())).unwrap_or_else(|e| {
            error!("Failed to parse config file, using default. Error: {:?}", e);
            EmulatorConfig::default()
        });

        Ok(config)
    }

    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ConfigError::SerializeError(e.to_string()))?;
        fs::write(path, json)
            .map_err(|e| ConfigError::IoError(e.to_string()))
    }

}