// Headless runner for Mooneye timer acceptance test ROMs.
//
// Mooneye ROMs report pass/fail by writing a Fibonacci sequence (pass) or
// repeated 0x42 (fail) into registers B/C/D/E/H/L, then executing `LD B, B`
// (opcode 0x40) as a breakpoint. See roms/mooneye-test-suite/README.markdown
// "Pass/fail reporting".

mod common;

fn assert_rom_passes(rom_name: &str) {
    common::assert_rom_passes("acceptance/timer", rom_name);
}

#[test]
fn div_write() {
    assert_rom_passes("div_write.gb");
}

#[test]
fn rapid_toggle() {
    assert_rom_passes("rapid_toggle.gb");
}

#[test]
fn tim00() {
    assert_rom_passes("tim00.gb");
}

#[test]
fn tim00_div_trigger() {
    assert_rom_passes("tim00_div_trigger.gb");
}

#[test]
fn tim01() {
    assert_rom_passes("tim01.gb");
}

#[test]
fn tim01_div_trigger() {
    assert_rom_passes("tim01_div_trigger.gb");
}

#[test]
fn tim10() {
    assert_rom_passes("tim10.gb");
}

#[test]
fn tim10_div_trigger() {
    assert_rom_passes("tim10_div_trigger.gb");
}

#[test]
fn tim11() {
    assert_rom_passes("tim11.gb");
}

#[test]
fn tim11_div_trigger() {
    assert_rom_passes("tim11_div_trigger.gb");
}

#[test]
fn tima_reload() {
    assert_rom_passes("tima_reload.gb");
}

#[test]
fn tima_write_reloading() {
    assert_rom_passes("tima_write_reloading.gb");
}

#[test]
fn tma_write_reloading() {
    assert_rom_passes("tma_write_reloading.gb");
}
