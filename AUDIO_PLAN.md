# Implementing DMG Audio (APU) Emulation

## Context

The emulator currently has no audio at all — the `0xFF10-0xFF3F` I/O range (APU
registers + wave RAM) falls through `io.rs`'s catch-all match arm into inert
byte storage (`src/io.rs:59-61,97-99`), so games write to sound registers and
nothing happens. This plan estimates the work to add real Game Boy (DMG) audio
output, and lays out a phased approach so the scope can be stopped at any
milestone with a working, audible result rather than an all-or-nothing effort.

**Headline estimate: ~2-4 focused days for a working, decent-sounding
implementation (all 4 channels + mixing), plus ~1-2 more days if
hardware-accurate edge-case behavior is wanted on top.** This is inherently
fuzzier than the timer/PPU work done this session — audio has a cheap
"sounds basically right" plateau and then a long tail of edge cases that are
expensive to chase and hard to notice without either bit-exact test ROMs or
trained ears (no automatic pass/fail signal the way Mooneye's register
convention gave the CPU/timer/PPU fixes).

This codebase's core timing (CPU/timer/PPU interrupts and cycle accounting)
is now solid and verified (13/13 Mooneye timer tests, confirmed visually
against Super Mario Land), so the APU work is purely additive — a new
peripheral plugged into the same `io.rs`/`memory_bus.rs` dispatch pattern
`timer.rs` and `ppu.rs` already use, with a new real-time audio output path
that's genuinely new territory for this project (everything so far runs
synchronously, single-threaded, per-frame).

## Architecture

The DMG APU is four independent channels feeding a shared stereo mixer, all
governed by a 512 Hz "frame sequencer" derived from the CPU clock
(4,194,304 Hz):

- **CH1** (`0xFF10-0xFF14`, NR10-NR14): square wave, duty cycle, volume
  envelope, frequency sweep
- **CH2** (`0xFF16-0xFF19`, NR21-NR24): square wave + envelope, no sweep —
  identical to CH1 minus sweep
- **CH3** (`0xFF1A-0xFF1E` + wave RAM `0xFF30-0xFF3F`, NR30-NR34): 32-sample
  4-bit custom wavetable playback
- **CH4** (`0xFF20-0xFF23`, NR41-NR44): pseudo-random noise via a 15/7-bit
  LFSR, with envelope but no duty/frequency in the square sense
- **Master control**: `NR50` (`0xFF24`, master L/R volume 0-7 + VIN
  passthrough), `NR51` (`0xFF25`, per-channel L/R routing bitmask), `NR52`
  (`0xFF26`, master power bit + read-only per-channel active-status bits)

**Frame sequencer**: an 8-step counter advanced every 8192 T-cycles
(4194304/512), ticking length counters on steps 0/2/4/6 (256 Hz), CH1's
sweep on steps 2/6 (128 Hz), and envelopes on step 7 (64 Hz). Every channel's
length/envelope/sweep behavior hangs off this one table — get it right once,
first.

Each channel produces a 4-bit (0-15) amplitude per T-cycle; the mixer sums
the channels routed to each side per NR51, scales by NR50's per-side volume,
and the result is downsampled from the ~4.19MHz CPU rate to a fixed output
rate (44100 Hz) before being queued to the audio device.

## Module Design

**Single flat `src/apu.rs`, not a subdirectory** — matches this codebase's
existing convention (every peripheral, including the 813-line `ppu.rs`, is
one flat file; `mbc/` is the only subdirectory and exists because MBC really
is a family of swappable trait implementations). Internally, factor out
shared logic used by multiple channels as small private structs —
`LengthCounter`, `Envelope`, `Sweep` (~15-20 lines each) — then build
`SquareChannel { length, envelope, sweep: Option<Sweep>, .. }` (shared by
CH1 with `Some(sweep)` and CH2 with `None`), plus `WaveChannel` and
`NoiseChannel`, all as fields on the top-level `pub struct Apu`.

Public surface mirrors `Timer`'s existing shape (`src/timer.rs`) exactly:

```rust
pub struct Apu {
    enabled: bool,                    // NR52 bit 7
    frame_sequencer_step: u8,
    frame_sequencer_counter: u16,
    ch1: SquareChannel,
    ch2: SquareChannel,
    ch3: WaveChannel,
    ch4: NoiseChannel,
    nr50: u8,
    nr51: u8,
    sample_cycle_counter: f32,
    pub sample_buffer: Vec<(i16, i16)>,   // drained by emulator.rs each frame
}

impl Apu {
    pub fn new() -> Apu { .. }
    pub fn read(&self, address: u16) -> u8 { .. }
    pub fn write(&mut self, address: u16, value: u8) { .. }
    pub fn cycle(&mut self, cycles: u8) { .. }   // called once per instruction, like ppu.tick()
}
```

`cycle()` steps the frame sequencer and all 4 channels once per T-cycle (a
plain `for _ in 0..cycles` loop — cheap even at the ~20-27T max instruction
length, no need to batch/analytically-step), and accumulates a
`CYCLES_PER_SAMPLE ≈ 95.1` (4194304/44100) counter to know when to push a
new mixed output sample. No anti-aliasing/decimation filter for v1 —
point-sampling at the output boundary is what most simple emulators do and
is adequate for DMG's simple waveforms; a box-filter average is a one-function
Phase 5 upgrade if aliasing turns out to be audible.

**Wiring in** (exactly the pattern `timer`/`ppu` already use):
- `src/io.rs`: add `pub apu: Apu` field, `Apu::new()` in `IO::new()`, and
  `APU_START..=APU_END` (`0xFF10..=0xFF26`) + `WAVE_RAM_START..=WAVE_RAM_END`
  (`0xFF30..=0xFF3F`) match arms in both `read()`/`write()` dispatching to
  `self.apu.read/write(address)` — same shape as the existing
  `PPU_START..=PPU_END` arms (`src/io.rs:57-58,91-96`).
- `src/memory_bus.rs`: **no changes needed** — `0xFF00-0xFF7F` already routes
  unconditionally to `dmg_io.read/write`, so the APU range reaches `io.rs`'s
  dispatcher for free.
- `src/emulator.rs`: add `self.memory_bus.dmg_io.apu.cycle(cpu_cycles)`
  alongside the existing `ppu.tick(cpu_cycles)` call in `Emulator::cycle()`
  (`src/emulator.rs:62-72`) — ticked from the outer loop like PPU, *not*
  threaded through `cpu.rs` like the timer's sub-instruction-precision path,
  since audio sample generation isn't sensitive to which exact T-state
  within an instruction a register write lands on the way TIMA's
  overflow-reload race is.

## SDL2 Audio Integration

**Cargo.toml**: `sdl2 = { version = "0.36.0", features = ["audio"] }` —
currently no features are enabled at all (`Cargo.toml:24`), so
`sdl2::audio::*` doesn't exist yet. Strict, isolated prerequisite — do this
first and confirm it compiles/links before writing any channel logic, to
catch any local SDL2-audio-library snags early.

**Push (`AudioQueue`) over pull (`AudioCallback`)**: this main loop
(`Emulator::cycle()`) is already synchronous and self-paced to 60fps via
`thread::sleep` (`src/emulator.rs:98-104`), generating a bounded, predictable
batch of new samples per frame (~735 samples at 44100Hz/60fps). Queuing that
batch once per frame — in the same block that already does per-frame work
(input polling, display update, FPS accounting) — needs no new thread, no
`Arc<Mutex<..>>`, and no risk of an audio callback thread racing the
emulation thread over `Apu` state. `AudioCallback` would require a
lock-free ring buffer or shared-locked state purely to avoid concurrency
this design doesn't otherwise need. Trade-off: manual under/overrun handling
— check `audio_queue.size()` before each frame's queue and drop/skip if the
backlog exceeds ~4-8 frames (~50-100ms), bounding worst-case latency without
a second thread.

**Shared `Sdl` context (restructuring needed)**: `MainDisplay::new()`
currently calls `sdl2::init()` itself and doesn't expose the root context
(`src/display.rs`). SDL expects one `init()` per process with subsystems
obtained from that shared context. Restructure:
- `Emulator::new()` calls `sdl2::init()` once, holds the `Sdl` context locally.
- `MainDisplay::new()` signature changes to accept `&sdl2::Sdl` instead of
  calling `sdl2::init()` internally.
- New `src/audio.rs`: `pub struct AudioOutput { queue: sdl2::audio::AudioQueue<i16> }`,
  constructed via `AudioOutput::new(sdl_context: &sdl2::Sdl)` calling
  `sdl_context.audio()` then `audio_subsystem.open_queue::<i16, _>(None, &spec)`.
  Spec: `AudioSpecDesired { freq: Some(44100), channels: Some(2), samples: None }`.
  Call `.resume()` once after construction (SDL queues start paused).
- `Emulator` gains an `audio_output` field alongside `main_display`, both
  built from the one shared context in `Emulator::new()`. Consider
  `Option<AudioOutput>` with graceful fallback (log + continue) rather than
  panicking on init failure — audio devices can be absent in headless/CI
  environments in a way video generally isn't in this project's current usage.

## Phased Plan

| Phase | Scope | Size |
|---|---|---|
| 0 | Cargo `audio` feature + shared `Sdl` context + a raw test tone playing through `AudioQueue` from the real loop, no `Apu` involved | Small (hours) |
| 1 | `src/apu.rs` skeleton: frame sequencer, full register read/write plumbing (`NR50-52` semantics, unmapped-address behavior), wired into `io.rs`/`emulator.rs`; channels silent/stubbed | Small-Medium |
| 2 | Channel 1 & 2 (square + envelope; sweep on CH1) | Medium |
| 3 | Channel 3 (wave + wave RAM) | Medium |
| 4 | Channel 4 (noise + LFSR) | Medium |
| 5 | NR50/NR51 mixing math, DAC-off silences-channel behavior, NR52 power-off register clearing, glitch/pop cleanup, verification pass | Medium, open-ended |

**Phase 0** de-risks the real-time-audio-streaming integration (feature
flag, context sharing, queuing cadence, underrun tuning) completely
independently of any Game Boy-specific logic — if anything about local SDL
audio setup is going to be awkward, this finds out before channel code exists.

**Phase 1** is the closest in shape to this session's `timer.rs` work
(register plumbing with edge-case semantics) — same muscle memory applies.
Verifiable with simple register-poke unit tests before any sound exists.

**Phase 2** is highest-value: square channels are the most common source in
DMG game audio (nearly every game uses CH1/CH2 for melody/SFX) — first
phase that makes a real game audibly "sound like a Game Boy." Known
hardware quirks worth *deliberately deprioritizing* on the first pass
(approximate now, revisit only if a specific game/test demands precision):
- **Envelope "zombie mode"** — writing NR12/NR22 mid-playback can glitch
  the current volume in ways a naive "envelope only updates on trigger"
  model won't reproduce. Rarely audible in normal play.
- **Length counter "extra clock on enable"** — an off-by-one edge case when
  a channel is (re-)enabled right as the frame sequencer is about to clock
  length anyway. Subtle; exactly what Mooneye's `len`/`len_ctr`-style tests
  check for.
- **Sweep's shadow-frequency-register subtlety** — CH1's sweep operates on
  an internal shadow copy of frequency (not NR13/14 directly), with overflow
  checked both at trigger and per-tick. A naive "recompute from NR13/14
  each tick" implementation will diverge on games doing aggressive live
  sweep effects (e.g. "laser" SFX).

**Phase 3**: also implement CH3's independent DAC-enable bit (NR30 bit 7)
and its own 8-bit length counter (wider range than the other channels' —
double check against Pan Docs). Deprioritize: wave-RAM
read/write-while-playing corruption (real hardware exposes the channel's
internal read position when the CPU touches wave RAM mid-playback, and DMG
vs CGB differ here) — treat wave RAM as freely accessible for v1, revisit
only if a target game's custom-waveform trick or a Mooneye
`wave_write_while_on`-style test demands it.

**Phase 4**: reuses Phase 2's `LengthCounter`/`Envelope` machinery — mainly
new work is the LFSR and NR43's divisor/shift-amount timing table (fiddly
bit layout; transcribe Pan Docs' divisor table as a `const DIVISORS: [u16; 8]`
lookup rather than deriving it algebraically). Sequenced last since noise
SFX are generally less noticeable if slightly off than melody channels.

**Phase 5**: two behaviors worth deliberately getting right here (compact,
well-isolated, common real-world bug sources) rather than treating as
optional polish:
- **DAC-off silences the channel independent of its enable flag** — a
  channel can be "active" per NR52 status yet silent because its DAC (upper
  bits of the volume/envelope register) is off; games routinely write a
  volume of 0 expecting immediate silence, not a lingering last sample.
- **NR52 power-off zeroes NR10-NR51** (wave RAM and NR52 itself survive) and
  **while powered off, all writes except to NR52/wave RAM are ignored** —
  some games deliberately power off the APU during timing-sensitive
  sections; getting this backwards can desync those routines.

## Testing / Verification

No APU test ROMs exist locally yet — `roms/mooneye-test-suite/acceptance/`
(vendored 2026-07-14) has `bits/instr/interrupts/oam_dma/ppu/serial/timer/`
but no `apu/`, and no blargg `dmg_sound`/`cgb_sound` ROMs are present.
Three-pronged approach, cheapest first:

1. **Register-level unit tests** (no external ROMs) — e.g. "NR52 power-off
   zeroes NR10-NR51 on readback," "triggering CH1 with DAC off doesn't set
   its NR52 active-status bit," "frame sequencer step timing matches the
   512Hz/8-step table." Fast, deterministic, catches Phase 1/5 logic
   directly. Check whether `timer.rs`/`ppu.rs` have any existing inline
   `#[cfg(test)]` convention to follow before deciding between inline tests
   and a new `tests/apu.rs`.
2. **Mooneye acceptance-ROM tests via the existing harness** — upstream
   Mooneye (Gekkio) has an `acceptance/apu/*.gb` set (length/envelope/sweep/
   wave timing) in versions newer than what's vendored here. Pull/rebuild
   the vendored checkout at a commit that includes them (same RGBDS
   toolchain flow already used for the existing `build/acceptance/timer/*.gb`
   binaries), then add `tests/apu.rs` mirroring `tests/mooneye.rs` /
   `tests/common/mod.rs`'s exact `assert_rom_passes("acceptance/apu", ..)`
   pattern. Note some of Mooneye's sound tests are explicitly
   `manual-only` (require listening) and aren't automatable this way. Worth
   doing as an early Phase 1 sub-task, not deferred to the end, so these
   tests exist to catch regressions *while* Phases 2-4 are written.
3. **By-ear verification** — no automatable signal exists for "does the
   music sound right" the way Mooneye's register-signature convention
   covers CPU/timer/PPU; this is a manual listening pass, done deliberately
   after each phase (Phase 2: a square-only game's menu music; Phase 3: a
   game known for prominent wave-channel bass; Phase 4: noise-heavy
   percussion/SFX). Flag to the user as an inherent limitation of this work
   versus the automatable CPU/timer/PPU fixes done this session.

## Critical Files

- `src/apu.rs` (new) — channels, frame sequencer, register I/O, sample generation
- `src/io.rs` — `pub apu: Apu` field + `0xFF10-0xFF26`/`0xFF30-0xFF3F` dispatch
- `src/emulator.rs` — `apu.cycle(cpu_cycles)` call, per-frame sample draining/queuing, shared `Sdl` context, new `audio_output` field
- `src/display.rs` — `MainDisplay::new()` accepts an externally-created `&sdl2::Sdl` instead of calling `sdl2::init()` itself
- `src/audio.rs` (new) — `AudioOutput` wrapping `sdl2::audio::AudioQueue`
- `Cargo.toml` — enable `sdl2`'s `audio` feature
- `tests/common/mod.rs` + new `tests/apu.rs` — extend the existing Mooneye headless-ROM-runner pattern once APU test ROMs are vendored
