
// Constants for the Gameboy's memory map
pub const BOOT_ROM_START: u16 = 0x0000;
pub const BOOT_ROM_END:u16 = 0x00FF;
pub const GAME_ROM_BANK_0_START: u16 = 0x0000;
pub const GAME_ROM_BANK_0_END: u16 = 0x3FFF;
pub const GAME_ROM_BANK_N_START: u16 = 0x4000;
pub const GAME_ROM_BANK_N_END: u16 = 0x7FFF;
pub const TILE_RAM_START: u16 = 0x8000;
pub const TILE_RAM_END: u16 = 0x97FF;
pub const BACKGROUND_RAM_START: u16 = 0x9800;
pub const BACKGROUND_RAM_END: u16 = 0x9FFF;
pub const CARTRIDGE_RAM_START: u16 = 0xA000;
pub const CARTRIDGE_RAM_END: u16 = 0xBFFF;
pub const WORK_RAM_START: u16 = 0xC000;
pub const WORK_RAM_END: u16 = 0xDFFF;
pub const ECHO_RAM_START: u16 = 0xE000;
pub const ECHO_RAM_END: u16 = 0xFDFF;
pub const OAM_START: u16 = 0xFE00; // Object Attribute Memory
pub const OAM_END: u16 = 0xFE9F; // Object Attribute Memory
pub const UNUSED_START: u16 = 0xFEA0;
pub const UNUSED_END: u16 = 0xFEFF;
pub const IO_REGISTERS_START: u16 = 0xFF00;
pub const IO_REGISTERS_END: u16 = 0xFF7F;
pub const HIGH_RAM_START: u16 = 0xFF80;
pub const HIGH_RAM_END: u16 = 0xFFFE;
pub const INTERRUPT_ENABLE_REGISTER: u16 = 0xFFFF;

// Constants for the Gameboy's I/O ranges
pub const JOYPAD_INPUT: u16 = 0xFF00;
pub const SERIAL_TRANSFER_START: u16 = 0xFF01;
pub const SERIAL_TRANSFER_END: u16 = 0xFF02;
pub const TIMER_START: u16 = 0xFF04;
pub const TIMER_END: u16 = 0xFF07;
pub const SOUND_START: u16 = 0xFF10;
pub const SOUND_END: u16 = 0xFF26;
pub const WAVE_START: u16 = 0xFF30;
pub const WAVE_END: u16 = 0xFF3F;
pub const LCD_CONTROL_START: u16 = 0xFF40;
pub const LCD_CONTROL_END: u16 = 0xFF4B;
pub const VRAM_BLANK_SELECT: u16 = 0xFF4F; // TODO first appearance was CGB, so maybe undeeded.
pub const BOOT_ROM_DISABLE: u16 = 0xFF50;
pub const VRAM_DMA_START: u16 = 0xFF51; // TODO first appearance was CGB, so maybe undeeded.
pub const VRAM_DMA_END: u16 = 0xFF55; // TODO first appearance was CGB, so maybe undeeded.
pub const BG_PALETTE_START: u16 = 0xFF68; // TODO first appearance was CGB, so maybe undeeded.
pub const BG_PALETTE_END: u16 = 0xFF6B; // TODO first appearance was CGB, so maybe undeeded.
pub const WRAM_BANK_SELECT: u16 = 0xFF70; // TODO first appearance was CGB, so maybe undeeded.

