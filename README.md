# Gameboy emulator

## TODO
### Super Mario Land Defect
Super Mario Land has an issue where the game resets due to a combination of button presses being detectected.

    This *MAY* be due to the joypad, however could be due to timing, interrupts, ect.

### Pokemon Defects
1. Pokemon Red has a defect for window rendering.

    When certain battle animations are tirggered the screen goes blank.

    After this happens once, subsequent window triggering such as the pause menu or Pokemon center dialog are not visible.

2. Pokemon Red has a defect for sprite rendering.

    When the game starts, the Gamefreak animation shows an issue with sprite pixel priority.

## Useful Links

- Guide to follow
  https://rylev.github.io/DMG-01/public/book/introduction.html
- https://emudev.de/gameboy-emulator/testing-our-cpu/

- Guide author presentation
  https://www.youtube.com/watch?v=B7seNuQncvU

- Recomended talk from guide author
  https://www.youtube.com/watch?v=HyzD8pNlpwI

- Rust implemented Gameboy emulator
  https://github.com/Gekkio/mooneye-gb

- Useful guide for Javascript implementation
  https://imrannazar.com/series/gameboy-emulation-in-javascript

- Gameboy development documentation
  - https://gbdev.io/pandocs/OAM.html
  - https://bgb.bircd.org/pandocs.htm

# gameboy_dmg-01_emulator

## References

#### Git repos -- emulators:
- https://lib.rs/crates/gameboy
- https://github.com/rylev/DMG-01/blob/master/lib-dmg-01/src/gpu.rs
- https://github.com/plorefice/gib/blob/master/src/ui/mod.rs
- https://github.com/ThomasRinsma/dromaius/blob/4e40e157511a5ce3d85f8438018e657faa714f00/src/cpu.cc#L213
- https://github.com/rockytriton/LLD_gbemu
- https://github.com/Gekkio/mooneye-gb

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

## Glossary

- Memory Management Unit (MMU)
