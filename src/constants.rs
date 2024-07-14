
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

// FLAGS
pub const ZERO_FLAG_BYTE_POSITION: u8 = 7;
pub const SUBTRACT_FLAG_BYTE_POSITION: u8 = 6;
pub const HALF_CARRY_FLAG_BYTE_POSITION: u8 = 5;
pub const CARRY_FLAG_BYTE_POSITION: u8 = 4;

pub const CALL_STACK_SIZE: usize = 0x100;
