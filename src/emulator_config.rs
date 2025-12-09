use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use log::error;

const DEFAULT_SCALE_FACTOR: u32 = 2;
const CONFIG_FILE_PATH: &str = "./config.json";

use minifb::Key;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmulatorConfig {
    pub boot_rom: Option<String>,
    pub scale_factor: u32,
    #[serde(default, deserialize_with = "deserialize_keymap", serialize_with = "serialize_keymap")]
    pub key_bindings: HashMap<String, Key>,
    pub logging_level: LoggingLevel,
}
// Custom (de)serializer for minifb::Key
use serde::{Deserializer, Serializer};
fn key_from_str(s: &str) -> Option<Key> {
    use minifb::Key::*;
    match s.to_ascii_lowercase().as_str() {
        "a" => Some(A),
        "b" => Some(B),
        "c" => Some(C),
        "d" => Some(D),
        "e" => Some(E),
        "f" => Some(F),
        "g" => Some(G),
        "h" => Some(H),
        "i" => Some(I),
        "j" => Some(J),
        "k" => Some(K),
        "l" => Some(L),
        "m" => Some(M),
        "n" => Some(N),
        "o" => Some(O),
        "p" => Some(P),
        "q" => Some(Q),
        "r" => Some(R),
        "s" => Some(S),
        "t" => Some(T),
        "u" => Some(U),
        "v" => Some(V),
        "w" => Some(W),
        "x" => Some(X),
        "y" => Some(Y),
        "z" => Some(Z),
        "up" => Some(Up),
        "down" => Some(Down),
        "left" => Some(Left),
        "right" => Some(Right),
        "space" => Some(Space),
        "return" | "enter" => Some(Enter),
        "backspace" => Some(Backspace),
        "escape" => Some(Escape),
        _ => None,
    }
}

fn key_to_str(key: &Key) -> &'static str {
    use minifb::Key::*;
    match key {
        A => "A", B => "B", C => "C", D => "D", E => "E", F => "F", G => "G", H => "H", I => "I", J => "J", K => "K", L => "L", M => "M", N => "N", O => "O", P => "P", Q => "Q", R => "R", S => "S", T => "T", U => "U", V => "V", W => "W", X => "X", Y => "Y", Z => "Z",
        Up => "Up", Down => "Down", Left => "Left", Right => "Right", Space => "Space", Enter => "Enter", Backspace => "Backspace", Escape => "Escape",
        _ => "Unknown"
    }
}

fn deserialize_keymap<'de, D>(deserializer: D) -> Result<HashMap<String, Key>, D::Error>
where D: Deserializer<'de> {
    let raw: HashMap<String, String> = HashMap::deserialize(deserializer)?;
    let mut out = HashMap::new();
    for (k, v) in raw {
        let key = key_from_str(&v).ok_or_else(|| serde::de::Error::custom(format!("Invalid key: {}", v)))?;
        out.insert(k, key);
    }
    Ok(out)
}

fn serialize_keymap<S>(map: &HashMap<String, Key>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer {
    use serde::ser::SerializeMap;
    let mut m = serializer.serialize_map(Some(map.len()))?;
    for (k, v) in map {
        m.serialize_entry(k, key_to_str(v))?;
    }
    m.end()
}

#[derive(Debug)]
pub enum ConfigError {
    IoError,
    SerializeError,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LoggingLevel {
    ERROR,
    INFO,
    DEBUG,
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        let mut key_bindings = HashMap::new();
        key_bindings.insert("up".to_string(), Key::Up);
        key_bindings.insert("down".to_string(), Key::Down);
        key_bindings.insert("left".to_string(), Key::Left);
        key_bindings.insert("right".to_string(), Key::Right);
        key_bindings.insert("a".to_string(), Key::A);
        key_bindings.insert("b".to_string(), Key::S);
        key_bindings.insert("start".to_string(), Key::Enter);
        key_bindings.insert("select".to_string(), Key::Backspace);
        Self {
            boot_rom: None,
            scale_factor: DEFAULT_SCALE_FACTOR,
            key_bindings,
            logging_level: LoggingLevel::ERROR,
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