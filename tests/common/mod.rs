// Shared headless runner for Mooneye test suite ROMs.
//
// Mooneye ROMs report pass/fail by writing a Fibonacci sequence (pass) or
// repeated 0x42 (fail) into registers B/C/D/E/H/L, then executing `LD B, B`
// (opcode 0x40) as a breakpoint. See roms/mooneye-test-suite/README.markdown
// "Pass/fail reporting".

use std::fs;
use std::path::Path;

use gameboy_emulator::cpu::CPU;
use gameboy_emulator::memory_bus::MemoryBus;
use gameboy_emulator::rom::ROM;

const PASS_SIGNATURE: (u8, u8, u8, u8, u8, u8) = (3, 5, 8, 13, 21, 34);
const LD_B_B_OPCODE: u8 = 0x40;
const MAX_CYCLES: u64 = 60_000_000; // ~14s of emulated time, generous safety net

pub enum RomOutcome {
    Pass,
    Fail((u8, u8, u8, u8, u8, u8)),
    Timeout,
}

pub fn run_rom(path: &Path) -> RomOutcome {
    let bytes = fs::read(path).expect("failed to read test ROM");
    let rom = ROM::new(path.to_string_lossy().to_string(), bytes);

    let mut cpu = CPU::new();
    let mut memory_bus = MemoryBus::new(None, &rom);

    let mut cycles_run: u64 = 0;

    loop {
        let cpu_cycles = cpu.cycle(&mut memory_bus);
        cycles_run += cpu_cycles as u64;

        let ppu_interrupts = memory_bus.dmg_io.ppu.tick(cpu_cycles);
        for interrupt in ppu_interrupts {
            memory_bus.trigger_interrupt(interrupt);
        }

        memory_bus.cycle(cpu_cycles);
        cpu.check_interrupts(&mut memory_bus);

        // Detect the `LD B, B` breakpoint opcode about to be executed.
        if memory_bus.read_byte(cpu.pc) == LD_B_B_OPCODE {
            let signature = cpu.debug_bcdehl();
            return if signature == PASS_SIGNATURE {
                RomOutcome::Pass
            } else {
                RomOutcome::Fail(signature)
            };
        }

        if cycles_run > MAX_CYCLES {
            return RomOutcome::Timeout;
        }
    }
}

pub fn assert_rom_passes(sub_dir: &str, rom_name: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("roms/mooneye-test-suite/build")
        .join(sub_dir)
        .join(rom_name);

    match run_rom(&path) {
        RomOutcome::Pass => {}
        RomOutcome::Fail(sig) => panic!(
            "{rom_name} FAILED, register signature (b,c,d,e,h,l) = {sig:?}"
        ),
        RomOutcome::Timeout => panic!("{rom_name} TIMED OUT waiting for LD B,B breakpoint"),
    }
}
