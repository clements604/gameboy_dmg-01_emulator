use std::fmt;
use lazy_static::lazy_static;
use std::collections::HashMap;
use log::{debug, info};

pub const HEADER_START: u16 = 0x0134;
const TITLE_START: u16 = 0x0134;
const TITLE_END: u16 = 0x0143;
const CGB_FLAG: u16 = 0x0143;
pub const LICENSEE_CODE_START: u16 = 0x144;
pub const LICENSEE_CODE_END: u16 = 0x146;
const SGB_FLAG: u16 = 0x0146;
const CARTRIDGE_TYPE: u16 = 0x0147;
pub const ROM_SIZE: u16 = 0x0148;
pub const RAM_SIZE: u16 = 0x0149;
const DESTINATION_CODE: u16 = 0x014A;
const OLD_LICENSEE_CODE: u16 = 0x014B;
const MASK_ROM_VERSION: u16 = 0x014C;
const HEADER_CHECKSUM: u16 = 0x014D;
const GLOBAL_CHECKSUM_START: u16 = 0x014E;

pub struct ROM {
    pub title: String,
    //pub manufacturer_code: u16,
    pub cgb_flag: u8,
    pub licensee_code: u16,
    pub sgb_flag: u8,
    pub cartridge_type: u8,
    pub rom_size: u8,
    pub ram_size: u8,
    pub destination_code: u8,
    pub old_licensee_code: u8,
    pub mask_rom_version: u8,
    pub header_checksum: u8,
    pub global_checksum: u16,
    pub rom: Vec<u8>,
    pub rom_file_path: String,
}

pub struct ROMBanks {
    pub data: Vec<Vec<u8>>,
}

impl ROMBanks {
    pub fn new() -> ROMBanks {
        ROMBanks {
            data: Vec::new(),
        }
    }
}

impl ROM {
    pub fn new(rom_file_path: String, rom: Vec<u8>) -> ROM {
        let title = String::from_utf8(rom[TITLE_START as usize..TITLE_END as usize].to_vec()).unwrap();
        let licensee_code = u16::from_str_radix(
            &String::from_utf8_lossy(&rom[LICENSEE_CODE_START as usize..LICENSEE_CODE_END as usize]),
            16,
        ).unwrap_or(0);
        let global_checksum = u16::from_le_bytes([rom[GLOBAL_CHECKSUM_START as usize], rom[GLOBAL_CHECKSUM_START as usize + 1]]);
        let rom :ROM =
        ROM {
            title,
            cgb_flag: rom[CGB_FLAG as usize],
            licensee_code,
            sgb_flag: rom[SGB_FLAG as usize],
            cartridge_type: rom[CARTRIDGE_TYPE as usize],
            rom_size: rom[ROM_SIZE as usize],
            ram_size: rom[RAM_SIZE as usize],
            destination_code: rom[DESTINATION_CODE as usize],
            old_licensee_code: rom[OLD_LICENSEE_CODE as usize],
            mask_rom_version: rom[MASK_ROM_VERSION as usize],
            header_checksum: rom[HEADER_CHECKSUM as usize],
            global_checksum,
            rom,
            rom_file_path,
        };
        info!("Loaded ROM:\n{}", rom);
        rom
    }

    fn calculate_header_checksum(&self) -> u8 {
        let mut header_checksum: u8 = 0;
        for x in HEADER_START..HEADER_CHECKSUM {
            header_checksum = header_checksum.wrapping_sub(self.rom[x as usize]).wrapping_sub(1);
        }
        header_checksum
    }

    pub fn validate_header_checksum(&self) -> Result<String, std::io::Error> {
        let calc_header_checksum: u8 = self.calculate_header_checksum();
        debug!("Header checksum: {}", self.header_checksum);
        debug!("Calculated header checksum: {}", calc_header_checksum);
        if self.header_checksum == calc_header_checksum {
            Ok(String::from("Header checksum [OK]"))
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Header checksum [FAIL]"))
        }
    }

    pub fn load_rom_to_banks(&self) -> ROMBanks {
        let mut rom_banks: ROMBanks = ROMBanks::new();
        let mut rom_offset: usize = 0; // The current offset in the ROM

        while rom_offset < self.rom.len() {
            let end = std::cmp::min(rom_offset + 0x4000, self.rom.len());
            rom_banks.data.push(self.rom[rom_offset..end].to_vec());
            rom_offset += 0x4000;
        }
        rom_banks
    }
}

impl fmt::Display for ROM {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "")?;
        writeln!(f, "Title: [{}]", self.title)?;
        //writeln!(f, "Manufacturer: {}", self.manufacturer_code)?;
        match self.cgb_flag {
            0x80 => {
                writeln!(f, "CGB Flag: [80 - The game supports CGB enhancements, but is backwards compatible with monochrome Game Boys]")?;
            },
            0xC0 => {
                writeln!(f, "CGB Flag: [C0 - The game works on CGB only, DMG will just ignore the bit 6 value, same as 0x80]")?;
            },
            _ => {
                writeln!(f, "CGB Flag: [{} - No CGB support]", self.cgb_flag)?;
            }
        }
        match self.sgb_flag {
            0x00 => {
                writeln!(f, "SGB Flag: [00 - No SGB support]")?;
            },
            0x03 => {
                writeln!(f, "SGB Flag: [03 - Game supports SGB functions]")?;
            },
            _ => {
                writeln!(f, "SGB Flag: [{} - Unknown]", self.sgb_flag)?;
            }
        }
        match CARTRIDGE_TYPE_MAP.get(&self.cartridge_type) {
            Some(cartridge_type_val) => {
                writeln!(f, "Cartridge type: [{} - {}]", self.cartridge_type, cartridge_type_val)?;
            },
            None => {
                writeln!(f, "Cartridge type: [{} - Unknown]", self.cartridge_type)?;
            }
        }
        match ROM_SIZE_MAP.get(&self.rom_size) {
            Some(rom_size_val) => {
                writeln!(f, "ROM size: [{} - {}]", self.rom_size, rom_size_val)?;
            },
            None => {
                writeln!(f, "ROM size: [{} - Unknown]", self.rom_size)?;
            }
        }
        match SAVE_RAM_SIZE_MAP.get(&self.ram_size) {
            Some(save_ram_size_val) => {
                writeln!(f, "Save RAM size: [{} - {}]", self.ram_size, save_ram_size_val)?;
            },
            None => {
                writeln!(f, "Save RAM size: [{} - Unknown]", self.ram_size)?;
            }
        }
        match self.destination_code {
            0x00 => {
                writeln!(f, "Destination code: [00 - Japan]")?;
            },
            0x01 => {
                writeln!(f, "Destination code: [01 - International]")?;
            },
            _ => {
                writeln!(f, "Destination code: [{} - Unknown]", self.destination_code)?;
            }
        }
        if self.old_licensee_code == 0x33 {
            // Get the licensee code from the header
           match NEW_LICENSEE_MAP.get(&self.licensee_code) {
               Some(licensee) => {
                writeln!(f, "Licensee Code: [{} - {}]", self.licensee_code, licensee)?;
               },
               None => {
                writeln!(f, "Licensee: [{} - Unknown]", self.licensee_code)?;
               }
           }
       } else {
           match OLD_LICENSEE_MAP.get(&self.old_licensee_code) {
               Some(old_licensee) => {
                writeln!(f, "Licensee: [{} - {}]", self.old_licensee_code, old_licensee)?;
               },
               None => {
                writeln!(f, "Licensee: [{} - Unknown]", self.old_licensee_code)?;
               }
           }
       }
        writeln!(f, "Mask ROM Version: [{}]", self.mask_rom_version)?;
        writeln!(f, "Header Checksum: [{}]", self.header_checksum)?;
        let calc_header_checksum: u8 = self.calculate_header_checksum();
        if self.header_checksum == calc_header_checksum {
            writeln!(f, "Header checksum [OK]")?;
        } else {
            writeln!(f, "Header checksum [FAIL]")?;
        };
        writeln!(f, "Global Checksum: [{}]", self.global_checksum)?;
        Ok(())
    }
}

/*
*   New Licenseee
*/
lazy_static! {
    pub static ref NEW_LICENSEE_MAP: HashMap<u16, String> = {
        let mut map = HashMap::new();
        map.insert(0x00, String::from("none"));
        map.insert(0x01, String::from("nintendo"));
        map.insert(0x08, String::from("capcom"));
        map.insert(0x13, String::from("electronic arts"));
        map.insert(0x18, String::from("hudsonsoft"));
        map.insert(0x19, String::from("b-ai"));
        map.insert(0x20, String::from("kss"));
        map.insert(0x22, String::from("pow"));
        map.insert(0x24, String::from("pcm complete"));
        map.insert(0x25, String::from("san-x"));
        map.insert(0x28, String::from("kemco japan"));
        map.insert(0x29, String::from("seta"));
        map.insert(0x30, String::from("viacom"));
        map.insert(0x31, String::from("nintendo"));
        map.insert(0x32, String::from("bandia"));
        map.insert(0x33, String::from("ocean/acclaim"));
        map.insert(0x34, String::from("konami"));
        map.insert(0x35, String::from("hector"));
        map.insert(0x37, String::from("taito"));
        map.insert(0x38, String::from("hudson"));
        map.insert(0x39, String::from("banpresto"));
        map.insert(0x41, String::from("ubi soft"));
        map.insert(0x42, String::from("atlus"));
        map.insert(0x44, String::from("malibu"));
        map.insert(0x46, String::from("angel"));
        map.insert(0x47, String::from("pullet-proof"));
        map.insert(0x49, String::from("irem"));
        map.insert(0x50, String::from("absolute"));
        map.insert(0x51, String::from("acclaim"));
        map.insert(0x52, String::from("activision"));
        map.insert(0x53, String::from("american sammy"));
        map.insert(0x54, String::from("konami"));
        map.insert(0x55, String::from("hi tech entertainment"));
        map.insert(0x56, String::from("ljn"));
        map.insert(0x57, String::from("matchbox"));
        map.insert(0x58, String::from("mattel"));
        map.insert(0x59, String::from("milton bradley"));
        map.insert(0x60, String::from("titus"));
        map.insert(0x61, String::from("virgin"));
        map.insert(0x64, String::from("lucasarts"));
        map.insert(0x67, String::from("ocean"));
        map.insert(0x69, String::from("electronic arts"));
        map.insert(0x70, String::from("infogrames"));
        map.insert(0x71, String::from("interplay"));
        map.insert(0x72, String::from("broderbund"));
        map.insert(0x73, String::from("sculptured"));
        map.insert(0x75, String::from("sci"));
        map.insert(0x78, String::from("t*hq"));
        map.insert(0x79, String::from("accolade"));
        map.insert(0x80, String::from("misawa"));
        map.insert(0x83, String::from("lozc"));
        map.insert(0x86, String::from("tokuma shoten i*"));
        map.insert(0x87, String::from("tsukuda ori*"));
        map.insert(0x91, String::from("chun soft"));
        map.insert(0x92, String::from("video system"));
        map.insert(0x93, String::from("ocean/acclaim"));
        map.insert(0x95, String::from("varie"));
        map.insert(0x96, String::from("yonezawa/s'pal"));
        map.insert(0x97, String::from("kaneko"));
        map.insert(0x99, String::from("pack in soft"));

        map
    };
}

/*
*   Cartridge type
*/
lazy_static! {
    pub static ref CARTRIDGE_TYPE_MAP: HashMap<u8, String> = {
        let mut map = HashMap::new();
        map.insert(0x00, String::from("ROM"));
        map.insert(0x01, String::from("MBC1"));
        map.insert(0x02, String::from("MBC1+RAM"));
        map.insert(0x03, String::from("MBC1+RAM+BATTERY"));
        map.insert(0x05, String::from("MBC2"));
        map.insert(0x06, String::from("MBC2+BATTERY"));
        map.insert(0x08, String::from("ROM+RAM"));
        map.insert(0x09, String::from("ROM+RAM+BATTERY"));
        map.insert(0x0B, String::from("MMM01"));
        map.insert(0x0C, String::from("MMM01+RAM"));
        map.insert(0x0D, String::from("MMM01+RAM+BATTERY"));
        map.insert(0x0F, String::from("MBC3+TIMER+BATTERY"));
        map.insert(0x10, String::from("MBC3+TIMER+RAM+BATTERY"));
        map.insert(0x11, String::from("MBC3"));
        map.insert(0x12, String::from("MBC3+RAM"));
        map.insert(0x13, String::from("MBC3+RAM+BATTERY"));
        map.insert(0x15, String::from("MBC4"));
        map.insert(0x16, String::from("MBC4+RAM"));
        map.insert(0x17, String::from("MBC4+RAM+BATTERY"));
        map.insert(0x19, String::from("MBC5"));
        map.insert(0x1A, String::from("MBC5+RAM"));
        map.insert(0x1B, String::from("MBC5+RAM+BATTERY"));
        map.insert(0x1C, String::from("MBC5+RUMBLE"));
        map.insert(0x1D, String::from("MBC5+RUMBLE+RAM"));
        map.insert(0x1E, String::from("MBC5+RUMBLE+RAM+BATTERY"));
        map.insert(0xFC, String::from("POCKET CAMERA"));
        map.insert(0xFD, String::from("Bandai TAMA5"));
        map.insert(0xFE, String::from("HuC3"));
        map.insert(0xFF, String::from("HuC1+RAM+BATTERY"));
        map
    };
}

/*
*   Rom Size
*/
lazy_static! {
    pub static ref ROM_SIZE_MAP: HashMap<u8, String> = {
        let mut map = HashMap::new();
        map.insert(0x00, String::from("32k"));
        map.insert(0x01, String::from("64k"));
        map.insert(0x02, String::from("128k"));
        map.insert(0x03, String::from("256k"));
        map.insert(0x04, String::from("512k"));
        map.insert(0x05, String::from("1024k"));
        map.insert(0x06, String::from("2048k"));
        map.insert(0x07, String::from("4096k"));
        map
    };
}

/*
*   Save Ram Size
*/
lazy_static! {
    pub static ref SAVE_RAM_SIZE_MAP: HashMap<u8, String> = {
        let mut map = HashMap::new();
        map.insert(0x00, String::from("0k"));
        map.insert(0x01, String::from("2k"));
        map.insert(0x02, String::from("8k"));
        map.insert(0x03, String::from("32k"));
        map
    };
}

/*
*   Old licensee code
*/
lazy_static! {
    pub static ref OLD_LICENSEE_MAP: HashMap<u8, String> = {
        let mut map = HashMap::new();
        map.insert(0x00, String::from("None"));
        map.insert(0x01, String::from("Nintendo"));
        map.insert(0x08, String::from("Capcom"));
        map.insert(0x09, String::from("Hot-B"));
        map.insert(0x0A, String::from("Jaleco"));
        map.insert(0x0B, String::from("Coconuts Japan"));
        map.insert(0x0C, String::from("Elite Systems"));
        map.insert(0x13, String::from("EA (Electronic Arts)"));
        map.insert(0x18, String::from("Hudsonsoft"));
        map.insert(0x19, String::from("ITC Entertainment"));
        map.insert(0x1A, String::from("Yanoman"));
        map.insert(0x1D, String::from("Japan Clary"));
        map.insert(0x1F, String::from("Virgin Interactive"));
        map.insert(0x24, String::from("PCM Complete"));
        map.insert(0x25, String::from("San-X"));
        map.insert(0x28, String::from("Kotobuki Systems"));
        map.insert(0x29, String::from("Seta"));
        map.insert(0x30, String::from("Infogrames"));
        map.insert(0x31, String::from("Nintendo"));
        map.insert(0x32, String::from("Bandai"));
        map.insert(0x33, String::from("Indicates that the New licensee code should be used instead."));
        map.insert(0x34, String::from("Konami"));
        map.insert(0x35, String::from("HectorSoft"));
        map.insert(0x38, String::from("Capcom"));
        map.insert(0x39, String::from("Banpresto"));
        map.insert(0x3C, String::from(".Entertainment i"));
        map.insert(0x3E, String::from("Gremlin"));
        map.insert(0x41, String::from("Ubisoft"));
        map.insert(0x42, String::from("Atlus"));
        map.insert(0x44, String::from("Malibu"));
        map.insert(0x46, String::from("Angel"));
        map.insert(0x47, String::from("Spectrum Holoby"));
        map.insert(0x49, String::from("Irem"));
        map.insert(0x4A, String::from("Virgin Interactive"));
        map.insert(0x4D, String::from("Malibu"));
        map.insert(0x4F, String::from("U.S. Gold"));
        map.insert(0x50, String::from("Absolute"));
        map.insert(0x51, String::from("Acclaim"));
        map.insert(0x52, String::from("Activision"));
        map.insert(0x53, String::from("American Sammy"));
        map.insert(0x54, String::from("GameTek"));
        map.insert(0x55, String::from("Park Place"));
        map.insert(0x56, String::from("LJN"));
        map.insert(0x57, String::from("Matchbox"));
        map.insert(0x59, String::from("Milton Bradley"));
        map.insert(0x5A, String::from("Mindscape"));
        map.insert(0x5B, String::from("Romstar"));
        map.insert(0x5C, String::from("Naxat Soft"));
        map.insert(0x5D, String::from("Tradewest"));
        map.insert(0x60, String::from("Titus"));
        map.insert(0x61, String::from("Virgin Interactive"));
        map.insert(0x67, String::from("Ocean Interactive"));
        map.insert(0x69, String::from("EA (Electronic Arts)"));
        map.insert(0x6E, String::from("Elite Systems"));
        map.insert(0x6F, String::from("Electro Brain"));
        map.insert(0x70, String::from("Infogrames"));
        map.insert(0x71, String::from("Interplay"));
        map.insert(0x72, String::from("Broderbund"));
        map.insert(0x73, String::from("Sculptered Soft"));
        map.insert(0x75, String::from("The Sales Curve"));
        map.insert(0x78, String::from("t.hq"));
        map.insert(0x79, String::from("Accolade"));
        map.insert(0x7A, String::from("Triffix Entertainment"));
        map.insert(0x7C, String::from("Microprose"));
        map.insert(0x7F, String::from("Kemco"));
        map.insert(0x80, String::from("Misawa Entertainment"));
        map.insert(0x83, String::from("Lozc"));
        map.insert(0x86, String::from("Tokuma Shoten Intermedia"));
        map.insert(0x8B, String::from("Bullet-Proof Software"));
        map.insert(0x8C, String::from("Vic Tokai"));
        map.insert(0x8E, String::from("Ape"));
        map.insert(0x8F, String::from("I’Max"));
        map.insert(0x91, String::from("Chunsoft Co."));
        map.insert(0x92, String::from("Video System"));
        map.insert(0x93, String::from("Tsubaraya Productions Co."));
        map.insert(0x95, String::from("Varie Corporation"));
        map.insert(0x96, String::from("Yonezawa/S’Pal"));
        map.insert(0x97, String::from("Kaneko"));
        map.insert(0x99, String::from("Arc"));
        map.insert(0x9A, String::from("Nihon Bussan"));
        map.insert(0x9B, String::from("Tecmo"));
        map.insert(0x9C, String::from("Imagineer"));
        map.insert(0x9D, String::from("Banpresto"));
        map.insert(0x9F, String::from("Nova"));
        map.insert(0xA1, String::from("Hori Electric"));
        map.insert(0xA2, String::from("Bandai"));
        map.insert(0xA4, String::from("Konami"));
        map.insert(0xA6, String::from("Kawada"));
        map.insert(0xA7, String::from("Takara"));
        map.insert(0xA9, String::from("Technos Japan"));
        map.insert(0xAA, String::from("Broderbund"));
        map.insert(0xAC, String::from("Toei Animation"));
        map.insert(0xAD, String::from("Toho"));
        map.insert(0xAF, String::from("Namco"));
        map.insert(0xB0, String::from("Acclaim"));
        map.insert(0xB1, String::from("ASCII or Nexsoft"));
        map.insert(0xB2, String::from("Bandai"));
        map.insert(0xB4, String::from("Square Enix"));
        map.insert(0xB6, String::from("HAL Laboratory"));
        map.insert(0xB7, String::from("SNK"));
        map.insert(0xB9, String::from("Pony Canyon"));
        map.insert(0xBA, String::from("Culture Brain"));
        map.insert(0xBB, String::from("Sunsoft"));
        map.insert(0xBD, String::from("Sony Imagesoft"));
        map.insert(0xBF, String::from("Sammy"));
        map.insert(0xC0, String::from("Taito"));
        map.insert(0xC2, String::from("Kemco"));
        map.insert(0xC3, String::from("Squaresoft"));
        map.insert(0xC4, String::from("Tokuma Shoten Intermedia"));
        map.insert(0xC5, String::from("Data East"));
        map.insert(0xC6, String::from("Tonkinhouse"));
        map.insert(0xC8, String::from("Koei"));
        map.insert(0xC9, String::from("UFL"));
        map.insert(0xCA, String::from("Ultra"));
        map.insert(0xCB, String::from("Vap"));
        map.insert(0xCC, String::from("Use Corporation"));
        map.insert(0xCD, String::from("Meldac"));
        map.insert(0xCE, String::from(".Pony Canyon or"));
        map.insert(0xCF, String::from("Angel"));
        map.insert(0xD0, String::from("Taito"));
        map.insert(0xD1, String::from("Sofel"));
        map.insert(0xD2, String::from("Quest"));
        map.insert(0xD3, String::from("Sigma Enterprises"));
        map.insert(0xD4, String::from("ASK Kodansha Co."));
        map.insert(0xD6, String::from("Naxat Soft"));
        map.insert(0xD7, String::from("Copya System"));
        map.insert(0xD9, String::from("Banpresto"));
        map.insert(0xDA, String::from("Tomy"));
        map.insert(0xDB, String::from("LJN"));
        map.insert(0xDD, String::from("NCS"));
        map.insert(0xDE, String::from("Human"));
        map.insert(0xDF, String::from("Altron"));
        map.insert(0xE0, String::from("Jaleco"));
        map.insert(0xE1, String::from("Towa Chiki"));
        map.insert(0xE2, String::from("Yutaka"));
        map.insert(0xE3, String::from("Varie"));
        map.insert(0xE5, String::from("Epcoh"));
        map.insert(0xE7, String::from("Athena"));
        map.insert(0xE8, String::from("Asmik ACE Entertainment"));
        map.insert(0xE9, String::from("Natsume"));
        map.insert(0xEA, String::from("King Records"));
        map.insert(0xEB, String::from("Atlus"));
        map.insert(0xEC, String::from("Epic/Sony Records"));
        map.insert(0xEE, String::from("IGS"));
        map.insert(0xF0, String::from("A Wave"));
        map.insert(0xF3, String::from("Extreme Entertainment"));
        map.insert(0xFF, String::from("LJN"));

        map
    };
}
