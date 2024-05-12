mod CPU;
mod constants;
mod display;
mod rom;

use log::{debug, error};

fn main() {
    let _ = env_logger::builder()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Debug)
        .is_test(false)
        .try_init();

    let mut cpu = CPU::CPU::new();
    //cpu.load_boot_rom(String::from("roms/boot/dmg0_boot.bin"));

    //cpu.load_rom(String::from("roms/Tetris.gb"));
    //cpu.load_rom(String::from("roms/Pokemon - Red Version (USA, Europe) (SGB Enhanced).gb"));

    cpu.load_rom(String::from("roms/test/cpu_instrs.gb"));
    //cpu.load_rom(String::from("roms/test/cpu/07-jr,jp,call,ret,rst.gb"));

    let mut cycle_count = 0;

    loop {
        cycle_count += 1;
        debug!("Cycle: {}", cycle_count);
        cpu.cycle();
        //cpu.registers.pc += 1;
    }
}
