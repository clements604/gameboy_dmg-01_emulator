use std::fs;
use std::path::Path;
use serde::{Deserialize, Deserializer, Serialize};
use log::error;
use sdl2::keyboard::Keycode;
use std::collections::HashMap;

const DEFAULT_SCALE_FACTOR: u32 = 1;
const CONFIG_FILE_PATH: &str = "./config.json";
const UP_KEY: &str = "up";
const DOWN_KEY: &str = "down";
const LEFT_KEY: &str = "left";
const RIGHT_KEY: &str = "right";
const A_KEY: &str = "a";
const B_KEY: &str = "b";
const START_KEY: &str = "start";
const SELECT_KEY: &str = "select";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmulatorConfig {
    pub boot_rom: Option<String>,
    pub scale_factor: u32,
    #[serde(
        deserialize_with = "deserialize_keycode_map",
        serialize_with = "serialize_keycode_map"
    )]
    pub key_bindings: HashMap<String, Keycode>,
}

#[derive(Debug)]
pub enum ConfigError {
    IoError,
    SerializeError,
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        let mut key_bindings = HashMap::new();
        key_bindings.insert(UP_KEY.to_string(), Keycode::Up);
        key_bindings.insert(DOWN_KEY.to_string(), Keycode::Down);
        key_bindings.insert(LEFT_KEY.to_string(), Keycode::Left);
        key_bindings.insert(RIGHT_KEY.to_string(), Keycode::Right);
        key_bindings.insert(A_KEY.to_string(), Keycode::A);
        key_bindings.insert(B_KEY.to_string(), Keycode::S);
        key_bindings.insert(START_KEY.to_string(), Keycode::Return);
        key_bindings.insert(SELECT_KEY.to_string(), Keycode::Backspace);
        Self {
            boot_rom: None,
            scale_factor: DEFAULT_SCALE_FACTOR,
            key_bindings,
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
        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read config file, using default. Error: {}", e);
                let config = EmulatorConfig::default();
                EmulatorConfig::save_to_file(&config, CONFIG_FILE_PATH).unwrap_or_else(|e| {
                    error!("Failed to save default config: {:?}", e);
                });
                return Ok(config);
            }
        };

        let config: EmulatorConfig = serde_json::from_str(&contents).unwrap_or_else(|e| {
            error!("Failed to parse config file, using default. Error: {}", e);
            EmulatorConfig::default()
        });

        Ok(config)
    }


    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            error!("Failed to serialize config: {}", e);
            ConfigError::SerializeError
        })?;

        fs::write(path, json).map_err(|e| {
            error!("Failed to write config file: {}", e);
            ConfigError::IoError
        })
    }
}
fn deserialize_keycode_map<'de, D>( deserializer: D) -> Result<HashMap<String, Keycode>, D::Error>
where
    D: Deserializer<'de>,
{
    let string_map: HashMap<String, String> = HashMap::deserialize(deserializer)?;

    string_map
        .into_iter()
        .map(|(key, value)| {
            Keycode::from_name(&value)
                .map(|keycode| (key, keycode))
                .ok_or_else(|| serde::de::Error::custom(format!("Invalid keycode: {}", value)))
        })
        .collect()
}

fn serialize_keycode_map<S>(
    map: &HashMap<String, Keycode>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeMap;
    let mut map_ser = serializer.serialize_map(Some(map.len()))?;
    for (k, v) in map {
        map_ser.serialize_entry(k, &v.name())?;
    }
    map_ser.end()
}