
mod CPU;
mod constants;
mod display;
mod rom;

fn main() {
    let _ = env_logger::builder()
            .target(env_logger::Target::Stdout)
            .filter_level(log::LevelFilter::Debug)
            .is_test(false)
            .try_init();

    let mut cpu = CPU::CPU::new();
    cpu.load_rom(String::from("roms/Pokemon - Red Version (USA, Europe) (SGB Enhanced).gb"));

    


}
