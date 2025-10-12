# Game Boy DMG-01 Emulator

## Overview
A Game Boy DMG-01 emulator written in Rust.

## Known issues
- No sound support
- No save states
- No serial link support

## Dependencies
- Rust (https://www.rust-lang.org/tools/install)
- SDL2 (https://www.libsdl.org/download-2.0.php)
- SDL2 development libraries (on Ubuntu: `libsdl2-dev`)
- SDL2 bindings for Rust (https://crates.io/crates/sdl2)
- Cargo (comes with Rust installation)

## Build and Run
### Development build
```bash
cargo build
```
### Release build
```bash
cargo build --release
```

## Configuration
Configuration is done via a 'config.json' file.

The file should be in the same directory as the executable.

The boot rom file is optional and can be omitted if not needed.

At first run, a default 'config.json' file will be created if it does not exist.

Configuration options:
- `boot_rom`: Path to the boot ROM file (optional).
- `scale_factor`: Integer scaling factor for the display (default is 2).
- `key_bindings`: Object mapping emulator controls to keyboard keys.

### Example configuration:
```json
{
  "boot_rom": "./dmg0_boot.bin",
  "scale_factor": 2,
  "key_bindings": {
    "up": "Up",
    "down": "Down",
    "left": "Left",
    "right": "Right",
    "a": "A",
    "b": "S",
    "start": "Return",
    "select": "Backspace"
  }
}
```

## Controls
Controls are configured as part of config.json and can be changed to suit your preferences.

The default key bindings are:
- Up: Up Arrow
- Down: Down Arrow
- Left: Left Arrow
- Right: Right Arrow
- A: 'A' key
- B: 'S' key
- Start: Enter key
- Select: Backspace key

## Useful Links

- Guide to follow
  https://rylev.github.io/DMG-01/public/book/introduction.html
- https://emudev.de/gameboy-emulator

- Guide author presentation
  https://www.youtube.com/watch?v=B7seNuQncvU

- Recommended talk from guide author
  https://www.youtube.com/watch?v=HyzD8pNlpwI

- Useful guide for Javascript implementation
  https://imrannazar.com/series/gameboy-emulation-in-javascript

- Gameboy development documentation
  - https://gbdev.io/pandocs/OAM.html
  - https://bgb.bircd.org/pandocs.htm

## References

#### Git repos -- emulators:
- https://lib.rs/crates/gameboy
- https://github.com/rylev/DMG-01
- https://github.com/plorefice/gib/blob/master/src/ui/mod.rs
- https://github.com/ThomasRinsma/dromaius/blob/4e40e157511a5ce3d85f8438018e657faa714f00/src/cpu.cc#L213
- https://github.com/rockytriton/LLD_gbemu
- https://github.com/Gekkio/mooneye-gb
- https://github.com/zeroview/DMG-2025
- https://github.com/mario-hess
- https://github.com/ThomasRinsma/dromaius

#### Git repos - tests:
- https://github.com/mattcurrie/dmg-acid2
- https://github.com/Gekkio/mooneye-test-suite

#### ROM header information

- https://gist.github.com/drhelius/4317698
- https://gbdev.gg8.se/wiki/articles/Gameboy_ROM_Header_Info

#### Boot ROMS

- https://gbdev.gg8.se/files/roms/bootroms/

#### Test ROMS

- https://github.com/retrio/gb-test-roms
- https://github.com/retrio/gb-test-roms/blob/master/cpu_instrs/source/07-jr%2Cjp%2Ccall%2Cret%2Crst.s

#### OAM documentation
- https://hacktix.github.io/GBEDG/ppu/#oam-memory

#### Opcodes
- https://pastraiser.com/cpu/gameboy/gameboy_opcodes.html
