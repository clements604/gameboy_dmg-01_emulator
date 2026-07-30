use bitflags::bitflags;
use log::{debug, error};

const SAMPLE_RATE_HZ: f32 = 44_100.0;
const CYCLES_PER_SAMPLE: f32 = 4_194_304.0 / SAMPLE_RATE_HZ;
// 4 channels * analog range 1.0 * volume 8.0 = 32.0 max magnitude; scales that to i16 range.
const SAMPLE_SCALE: f32 = 1024.0;
// Low cutoff: DMG's HPF is the least aggressive of the DMG/CGB/GBA family
// (Pan Docs "Audio details / Mixer") — this should remove DC bias/pops
// without audibly coloring anything above typical GB music/SFX content.
const HPF_CUTOFF_HZ: f32 = 20.0;

// Maps a channel's raw 0-15 digital DAC input to its analog output, per
// Pan Docs "Audio details / DACs": if the DAC is on, digital 0-15 linearly
// maps to analog 1..-1 — note the slope is *negative* (digital 0 -> analog 1).
// If the DAC is off, it contributes analog 0 (silence) rather than fading
// realistically — a deliberate simplification, since the real fade "is not
// entirely deterministic and varies between models."
//
// Critically, `dac_on` here means the DAC itself (NRx2's top 5 bits, or
// NR30 bit 7 for CH3) — NOT whether the channel is actively generating.
// Per the same page: "a disabled channel outputs 0, which an enabled DAC
// will dutifully convert into analog 1." A channel that's inactive but
// still has its DAC on keeps contributing a nonzero bias to the mix; the
// high-pass filter below is what turns that (and other DC-offset changes,
// like NR51 routing or NR50 volume changing) into a brief pop instead of a
// sustained level shift.
fn dac_output(digital: u8, dac_on: bool) -> f32 {
    if !dac_on {
        return 0.0;
    }
    1.0 - (digital as f32 / 7.5)
}

// One-pole high-pass filter, matching the real APU's per-channel mixer
// output stage (Pan Docs "Audio details / Mixer") — removes the DC bias
// that inactive-but-DAC-on channels and DAC/routing/volume changes
// introduce, turning what would otherwise be a sustained level jump into a
// brief, hardware-accurate pop instead of an audible click every time.
#[derive(Default)]
struct HighPassFilter {
    charge_factor: f32,
    prev_input: f32,
    prev_output: f32,
}

impl HighPassFilter {
    fn new(cutoff_hz: f32, sample_rate: f32) -> HighPassFilter {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz);
        let dt = 1.0 / sample_rate;
        HighPassFilter {
            charge_factor: rc / (rc + dt),
            prev_input: 0.0,
            prev_output: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.charge_factor * (self.prev_output + input - self.prev_input);
        self.prev_input = input;
        self.prev_output = output;
        output
    }
}

// Duty-cycle waveforms, one bit per duty_step (0-7); 1 = high. Pan Docs "Waveforms".
const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75%
];

// NR43 divisor code (bits 0-2) -> divisor lookup, per Pan Docs.
const DIVISORS: [u16; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

pub struct Apu {
    enabled: bool, // NR52 bit 7
    frame_sequencer_step: u8,
    frame_sequencer_counter: u16,
    ch1: SquareChannel,
    ch2: SquareChannel,
    ch3: WaveChannel,
    ch4: NoiseChannel,
    nr50: u8,
    nr51: u8,
    sample_cycle_counter: f32,
    hpf_left: HighPassFilter,
    hpf_right: HighPassFilter,
    pub sample_buffer: Vec<(i16, i16)>, // drained by emulator.rs each frame
}

#[derive(Default)]
struct SquareChannel {
    enabled: bool,         // channel active per NR52, independent of DAC
    duty: u8,              // 2-bit duty cycle select, NRx1 bits 6-7
    duty_step: u8,         // 0-7 position in the 8-step waveform
    frequency: u16,        // 11-bit period value, NRx3 + NRx4 bits 0-2
    frequency_timer: u16,  // T-cycles until next duty_step advance
    length: LengthCounter, // NRx1 bits 0-5 (6-bit) + NRx4 bit 6 (enable)
    envelope: Envelope,    // NRx2
    sweep: Option<Sweep>,  // Some(..) for CH1 (NR10), None for CH2
}

#[derive(Default)]
struct LengthCounter {
    counter: u16,  // 6-bit (CH1/CH2/CH4) or 8-bit (CH3) depending on max
    enabled: bool, // NRx4 bit 6 ("length enable")
}

#[derive(Default)]
struct Sweep {
    period: u8,            // NR10 bits 4-6, 0-7 (0 = sweep disabled)
    direction: bool,       // NR10 bit 3: true = decrease (subtract), false = increase (add)
    shift: u8,             // NR10 bits 0-2, 0-7
    shadow_frequency: u16, // internal copy, distinct from NR13/NR14
    enabled: bool,
    timer: u8,
}

#[derive(Default)]
struct Envelope {
    initial_volume: u8, // NRx2 bits 4-7, 0-15
    direction: bool,    // NRx2 bit 3: true = increase, false = decrease
    period: u8,         // NRx2 bits 0-2, 0-7 (0 = envelope disabled)
    current_volume: u8,
    timer: u8, // counts down `period` frame-sequencer-step-7 ticks
}

#[derive(Default)]
struct WaveChannel {
    enabled: bool,
    dac_enabled: bool,     // NR30 bit 7 — independent of `enabled`
    length: LengthCounter, // 8-bit, NR31
    volume_shift: u8,      // NR32 bits 5-6: 0=mute,1=100%,2=50%,3=25%
    frequency: u16,        // 11-bit, NR33 + NR34 bits 0-2
    frequency_timer: u16,
    position: u8,       // 0-31, current sample index into wave RAM
    wave_ram: [u8; 16], // 0xFF30-0xFF3F, 32 4-bit samples packed as 16 bytes
}

#[derive(Default)]
struct NoiseChannel {
    enabled: bool,
    length: LengthCounter, // 6-bit, NR41
    envelope: Envelope,    // NR42
    clock_shift: u8,       // NR43 bits 4-7
    lfsr_width_mode: bool, // NR43 bit 3: true = 7-bit, false = 15-bit
    divisor_code: u8,      // NR43 bits 0-2, indexes DIVISORS lookup
    lfsr: u16,             // 15-bit linear feedback shift register
    frequency_timer: u16,
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct Nr52: u8 {
        const AUDIO_ON = 0b1000_0000;
        const CH4_ON   = 0b0000_1000;
        const CH3_ON   = 0b0000_0100;
        const CH2_ON   = 0b0000_0010;
        const CH1_ON   = 0b0000_0001;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct Nr10: u8 {
        const DIRECTION = 0b0000_1000;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct Nr11: u8 {
        const WAVE_DUTY = 0b1100_0000;
        const INITIAL_LENGTH_TIMER   = 0b0011_1111;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct Nr12: u8 {
        const INITIAL_VOLUME = 0b1111_0000;
        const DIRECTION      = 0b0000_1000;
        const PACE           = 0b0000_0111;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct Nr14: u8 {
        const TRIGGER        = 0b1000_0000;
        const LENGTH_ENABLE  = 0b0100_0000;
        const FREQUENCY_HIGH = 0b0000_0111;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct Nr30: u8 {
        const DAC_ENABLE = 0b1000_0000;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct Nr32: u8 {
        const VOLUME = 0b0110_0000;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct Nr43: u8 {
        const CLOCK_SHIFT   = 0b1111_0000;
        const LFSR_WIDTH    = 0b0000_1000;
        const CLOCK_DIVIDER = 0b0000_0111;
    }
}

impl Apu {
    // Debug-only: current 0-15 amplitude for ch1-ch4, for the debug visualiser.
    #[cfg(debug_assertions)]
    pub fn channel_amplitudes(&self) -> [u8; 4] {
        [
            self.ch1.amplitude(),
            self.ch2.amplitude(),
            self.ch3.amplitude(),
            self.ch4.amplitude(),
        ]
    }

    pub fn new() -> Apu {
        Apu {
            enabled: false,
            frame_sequencer_step: 0,
            frame_sequencer_counter: 0,
            ch1: SquareChannel::new(true),
            ch2: SquareChannel::new(false),
            ch3: WaveChannel::default(),
            ch4: NoiseChannel::default(),
            nr50: 0,
            nr51: 0,
            sample_cycle_counter: 0.0,
            hpf_left: HighPassFilter::new(HPF_CUTOFF_HZ, SAMPLE_RATE_HZ),
            hpf_right: HighPassFilter::new(HPF_CUTOFF_HZ, SAMPLE_RATE_HZ),
            sample_buffer: Vec::new(),
        }
    }
    pub fn read(&self, address: u16) -> u8 {
        match address {
            0xFF10 => {
                let sweep = self.ch1.sweep.as_ref().expect("Channel 1 sweep was None");
                Nr10::from_state(sweep.period, sweep.direction, sweep.shift)
            }
            0xFF11 => Nr11::from_state(self.ch1.duty, 0) | 0b0011_1111,
            0xFF12 => Nr12::from_state(
                self.ch1.envelope.initial_volume,
                self.ch1.envelope.direction,
                self.ch1.envelope.period,
            ),
            0xFF13 => 0xFF, // NR13: frequency low, write-only
            0xFF14 => Nr14::from_state(self.ch1.length.enabled),
            0xFF15 => 0xFF, // unused
            0xFF16 => Nr11::from_state(self.ch2.duty, 0) | 0b0011_1111,
            0xFF17 => Nr12::from_state(
                self.ch2.envelope.initial_volume,
                self.ch2.envelope.direction,
                self.ch2.envelope.period,
            ),
            0xFF18 => 0xFF, // NR23: frequency low, write-only
            0xFF19 => Nr14::from_state(self.ch2.length.enabled),
            0xFF1A => Nr30::from_state(self.ch3.dac_enabled),
            0xFF1B => 0xFF, // NR31: length timer, write-only
            0xFF1C => Nr32::from_state(self.ch3.volume_shift),
            0xFF1D => 0xFF, // NR33: frequency low, write-only
            0xFF1E => Nr14::from_state(self.ch3.length.enabled),
            0xFF1F => 0xFF, // unused
            0xFF20 => 0xFF, // NR41: length timer, write-only
            0xFF21 => Nr12::from_state(
                self.ch4.envelope.initial_volume,
                self.ch4.envelope.direction,
                self.ch4.envelope.period,
            ),
            0xFF22 => Nr43::from_state(
                self.ch4.clock_shift,
                self.ch4.lfsr_width_mode,
                self.ch4.divisor_code,
            ),
            0xFF23 => Nr14::from_state(self.ch4.length.enabled),
            0xFF24 => self.nr50, // NR50: Master volume & VIN panning
            0xFF25 => self.nr51, // NR51: Sound panning
            0xFF26 => Nr52::from_state(
                // NR52: Audio master control
                self.enabled,
                self.ch1.enabled,
                self.ch2.enabled,
                self.ch3.enabled,
                self.ch4.enabled,
            )
            .bits(),
            0xFF27..=0xFF2F => 0xFF, // unused
            0xFF30..=0xFF3F => self.ch3.wave_ram[(address - 0xFF30) as usize],
            _ => {
                error!("Invalid read address at 0x{:4X}", address);
                0
            }
        }
    }
    pub fn write(&mut self, address: u16, value: u8) {
        // While powered off, NR10-NR51 ignore writes and read back as cleared,
        // except NR52 itself (handled below) and, on DMG, the length-timer
        // bits within NR11/NR21/NR31/NR41 (handled in their own arms).
        // Wave RAM (0xFF30-0xFF3F) is outside this range and always writable.
        if !self.enabled
            && (0xFF10..=0xFF25).contains(&address)
            && !matches!(address, 0xFF11 | 0xFF16 | 0xFF1B | 0xFF20)
        {
            debug!("APU is off, ignoring write to 0x{:04X}", address);
            return;
        }
        match address {
            0xFF10 => {
                let (period, direction, shift) = Nr10::to_state(value);
                if let Some(sweep) = self.ch1.sweep.as_mut() {
                    sweep.period = period;
                    sweep.direction = direction;
                    sweep.shift = shift;
                }
            }
            0xFF11 => {
                let (duty, initial_length_timer) = Nr11::to_state(value);
                if self.enabled {
                    self.ch1.duty = duty;
                }
                self.ch1.length.counter = 64 - initial_length_timer as u16;
            }
            0xFF12 => {
                let (initial_volume, direction, period) = Nr12::to_state(value);
                self.ch1.envelope.initial_volume = initial_volume;
                self.ch1.envelope.direction = direction;
                self.ch1.envelope.period = period;
            }
            0xFF13 => {
                // NR13: frequency low, write-only
                self.ch1.frequency = (self.ch1.frequency & 0xFF00) | value as u16;
            }
            0xFF14 => {
                let (trigger, length_enable, frequency_high) = Nr14::to_state(value);
                self.ch1.length.enabled = length_enable;
                self.ch1.frequency = (self.ch1.frequency & 0x00FF) | ((frequency_high as u16) << 8);
                if trigger {
                    self.ch1.trigger();
                }
            }
            0xFF15 => {} // unused
            0xFF16 => {
                let (duty, initial_length_timer) = Nr11::to_state(value);
                if self.enabled {
                    self.ch2.duty = duty;
                }
                self.ch2.length.counter = 64 - initial_length_timer as u16;
            }
            0xFF17 => {
                let (initial_volume, direction, period) = Nr12::to_state(value);
                self.ch2.envelope.initial_volume = initial_volume;
                self.ch2.envelope.direction = direction;
                self.ch2.envelope.period = period;
            }
            0xFF18 => {
                // NR23: frequency low, write-only
                self.ch2.frequency = (self.ch2.frequency & 0xFF00) | value as u16;
            }
            0xFF19 => {
                let (trigger, length_enable, frequency_high) = Nr14::to_state(value);
                self.ch2.length.enabled = length_enable;
                self.ch2.frequency = (self.ch2.frequency & 0x00FF) | ((frequency_high as u16) << 8);
                if trigger {
                    self.ch2.trigger();
                }
            }
            0xFF1A => {
                self.ch3.dac_enabled = Nr30::to_state(value);
            }
            0xFF1B => {
                // NR31: length timer, write-only, 8-bit range
                self.ch3.length.counter = 256 - value as u16;
            }
            0xFF1C => {
                self.ch3.volume_shift = Nr32::to_state(value);
            }
            0xFF1D => {
                // NR33: frequency low, write-only
                self.ch3.frequency = (self.ch3.frequency & 0xFF00) | value as u16;
            }
            0xFF1E => {
                let (trigger, length_enable, frequency_high) = Nr14::to_state(value);
                self.ch3.length.enabled = length_enable;
                self.ch3.frequency = (self.ch3.frequency & 0x00FF) | ((frequency_high as u16) << 8);
                if trigger {
                    self.ch3.trigger();
                }
            }
            0xFF1F => {} // unused
            0xFF20 => {
                // NR41: length timer, write-only, bits 0-5
                self.ch4.length.counter = 64 - (value & 0b0011_1111) as u16;
            }
            0xFF21 => {
                let (initial_volume, direction, period) = Nr12::to_state(value);
                self.ch4.envelope.initial_volume = initial_volume;
                self.ch4.envelope.direction = direction;
                self.ch4.envelope.period = period;
            }
            0xFF22 => {
                let (clock_shift, lfsr_width_mode, divisor_code) = Nr43::to_state(value);
                self.ch4.clock_shift = clock_shift;
                self.ch4.lfsr_width_mode = lfsr_width_mode;
                self.ch4.divisor_code = divisor_code;
            }
            0xFF23 => {
                let (trigger, length_enable, _) = Nr14::to_state(value);
                self.ch4.length.enabled = length_enable;
                if trigger {
                    self.ch4.trigger();
                }
            }
            0xFF24 => self.nr50 = value, // NR50: Master volume & VIN panning
            0xFF25 => self.nr51 = value, // NR51: Sound panning
            0xFF26 => {
                // CHn-on bits (0-3) are read-only status; writes only affect
                // the power bit (7) — writing them does not enable/disable channels.
                let (audio_on, ..) = Nr52::to_state(value);
                if self.enabled && !audio_on {
                    // Powering off clears NR10-NR51, but not NR52 or wave RAM.
                    self.ch1 = SquareChannel::new(true);
                    self.ch2 = SquareChannel::new(false);
                    self.ch3.enabled = false;
                    self.ch3.dac_enabled = false;
                    self.ch3.length = LengthCounter::default();
                    self.ch3.volume_shift = 0;
                    self.ch3.frequency = 0;
                    self.ch3.frequency_timer = 0;
                    self.ch3.position = 0;
                    self.ch4 = NoiseChannel::default();
                    self.nr50 = 0;
                    self.nr51 = 0;
                }
                self.enabled = audio_on;
            }
            0xFF27..=0xFF2F => {} // unused
            0xFF30..=0xFF3F => {
                self.ch3.wave_ram[(address - 0xFF30) as usize] = value;
            }
            _ => {
                error!(
                    "Invalid write address at 0x{:4X} with value {}",
                    address, value
                );
            }
        }
    }
    pub fn cycle(&mut self, cycles: u8) {
        for _ in 0..cycles {
            //Frame sequencer stepping
            self.frame_sequencer_counter += 1;
            if self.frame_sequencer_counter == 8192 {
                self.frame_sequencer_counter = 0;
                self.frame_sequencer_step = (self.frame_sequencer_step + 1) % 8;
                match self.frame_sequencer_step {
                    0 | 2 | 4 | 6 => {
                        if self.ch1.length.tick() {
                            self.ch1.enabled = false;
                        }
                        if self.ch2.length.tick() {
                            self.ch2.enabled = false;
                        }
                        if self.ch3.length.tick() {
                            self.ch3.enabled = false;
                        }
                        if self.ch4.length.tick() {
                            self.ch4.enabled = false;
                        }
                    }
                    _ => {}
                }
                if matches!(self.frame_sequencer_step, 2 | 6) {
                    if let Some(sweep) = self.ch1.sweep.as_mut() {
                        if sweep.tick(&mut self.ch1.frequency) {
                            self.ch1.enabled = false;
                        }
                    }
                }
                if self.frame_sequencer_step == 7 {
                    self.ch1.envelope.tick();
                    self.ch2.envelope.tick();
                    self.ch4.envelope.tick();
                }
            }

            // Per-channel period stepping: each channel advances its own waveform
            // position independently, once per T-cycle.
            self.ch1.step();
            self.ch2.step();
            self.ch3.step();
            self.ch4.step();

            // Downsample to the output rate and push a mixed sample.
            self.sample_cycle_counter += 1.0;
            if self.sample_cycle_counter >= CYCLES_PER_SAMPLE {
                self.sample_cycle_counter -= CYCLES_PER_SAMPLE;
                let sample = self.generate_sample();
                self.sample_buffer.push(sample);
            }
        }
    }

    fn generate_sample(&mut self) -> (i16, i16) {
        let ch1 = dac_output(self.ch1.amplitude(), self.ch1.dac_on());
        let ch2 = dac_output(self.ch2.amplitude(), self.ch2.dac_on());
        let ch3 = dac_output(self.ch3.amplitude(), self.ch3.dac_enabled);
        let ch4 = dac_output(self.ch4.amplitude(), self.ch4.dac_on());

        let mut left = 0.0_f32;
        let mut right = 0.0_f32;
        if self.nr51 & 0b0001_0000 != 0 {
            left += ch1;
        }
        if self.nr51 & 0b0010_0000 != 0 {
            left += ch2;
        }
        if self.nr51 & 0b0100_0000 != 0 {
            left += ch3;
        }
        if self.nr51 & 0b1000_0000 != 0 {
            left += ch4;
        }
        if self.nr51 & 0b0000_0001 != 0 {
            right += ch1;
        }
        if self.nr51 & 0b0000_0010 != 0 {
            right += ch2;
        }
        if self.nr51 & 0b0000_0100 != 0 {
            right += ch3;
        }
        if self.nr51 & 0b0000_1000 != 0 {
            right += ch4;
        }

        // NR50 volume is 0-7; +1 so 0 still audibly attenuates rather than muting entirely.
        let left_volume = ((self.nr50 >> 4) & 0b111) as f32 + 1.0;
        let right_volume = (self.nr50 & 0b111) as f32 + 1.0;
        left *= left_volume;
        right *= right_volume;

        // High-pass filter removes the DC bias that inactive-but-DAC-on channels
        // (and NR50/NR51 changes) introduce, per Pan Docs "Audio details / Mixer".
        left = self.hpf_left.process(left);
        right = self.hpf_right.process(right);

        // Max magnitude (4 channels * 1.0) * (volume 8.0) * SAMPLE_SCALE is right at
        // i16::MAX; `as i16` on floats saturates rather than overflowing, so this is
        // safe even at the boundary.
        let left_sample = (left * SAMPLE_SCALE) as i16;
        let right_sample = (right * SAMPLE_SCALE) as i16;
        (left_sample, right_sample)
    }
}

impl SquareChannel {
    pub fn new(sweep_required: bool) -> SquareChannel {
        let sweep: Option<Sweep> = match sweep_required {
            true => Some(Sweep::default()),
            false => None,
        };
        SquareChannel {
            enabled: false,
            duty: 0,
            duty_step: 0,
            frequency: 0,
            frequency_timer: 0,
            length: LengthCounter::default(),
            envelope: Envelope::default(),
            sweep: sweep,
        }
    }

    // Advances duty_step once frequency_timer expires. Called once per T-cycle.
    fn step(&mut self) {
        if self.frequency_timer > 0 {
            self.frequency_timer -= 1;
        }
        if self.frequency_timer == 0 {
            self.frequency_timer = (2048 - self.frequency) * 4;
            self.duty_step = (self.duty_step + 1) % 8;
        }
    }

    fn amplitude(&self) -> u8 {
        if !self.enabled {
            return 0;
        }
        DUTY_TABLE[self.duty as usize][self.duty_step as usize] * self.envelope.current_volume
    }

    fn dac_on(&self) -> bool {
        self.envelope.initial_volume != 0 || self.envelope.direction
    }

    // NRx4 bit 7. No-op (channel stays off) if the DAC is off. Reloads the
    // length counter only if it's currently zero, resets frequency_timer and
    // the envelope, and (CH1 only) reloads the sweep's shadow frequency/timer
    // with an immediate overflow check. duty_step is deliberately *not* reset —
    // real hardware continues from wherever it was.
    fn trigger(&mut self) {
        if !self.dac_on() {
            return;
        }
        self.enabled = true;
        if self.length.counter == 0 {
            self.length.counter = 64;
        }
        self.frequency_timer = (2048 - self.frequency) * 4;
        self.envelope.current_volume = self.envelope.initial_volume;
        self.envelope.timer = if self.envelope.period == 0 {
            8
        } else {
            self.envelope.period
        };
        if let Some(sweep) = self.sweep.as_mut() {
            sweep.shadow_frequency = self.frequency;
            sweep.timer = if sweep.period == 0 { 8 } else { sweep.period };
            sweep.enabled = sweep.period != 0 || sweep.shift != 0;
            if sweep.shift > 0 {
                let delta = sweep.shadow_frequency >> sweep.shift;
                let candidate = if sweep.direction {
                    sweep.shadow_frequency.wrapping_sub(delta)
                } else {
                    sweep.shadow_frequency.wrapping_add(delta)
                };
                if candidate > 2047 {
                    self.enabled = false;
                }
            }
        }
    }
}

impl WaveChannel {
    // Advances position once frequency_timer expires. Called once per T-cycle.
    fn step(&mut self) {
        if self.frequency_timer > 0 {
            self.frequency_timer -= 1;
        }
        if self.frequency_timer == 0 {
            self.frequency_timer = (2048 - self.frequency) * 2;
            self.position = (self.position + 1) % 32;
        }
    }

    fn amplitude(&self) -> u8 {
        if !self.enabled || !self.dac_enabled {
            return 0;
        }
        let byte = self.wave_ram[(self.position / 2) as usize];
        let raw = if self.position % 2 == 0 {
            byte >> 4
        } else {
            byte & 0x0F
        };
        let shift = match self.volume_shift {
            0 => 4, // mute
            1 => 0, // 100%
            2 => 1, // 50%
            3 => 2, // 25%
            _ => 4,
        };
        raw >> shift
    }

    // NRx4 bit 7. No-op if the DAC (NR30 bit 7) is off. Reloads the length
    // counter only if it's currently zero, resets frequency_timer, and resets
    // position to the start of wave RAM.
    fn trigger(&mut self) {
        if !self.dac_enabled {
            return;
        }
        self.enabled = true;
        if self.length.counter == 0 {
            self.length.counter = 256;
        }
        self.frequency_timer = (2048 - self.frequency) * 2;
        self.position = 0;
    }
}

impl NoiseChannel {
    // Advances the LFSR once frequency_timer expires. Called once per T-cycle.
    fn step(&mut self) {
        if self.frequency_timer > 0 {
            self.frequency_timer -= 1;
        }
        if self.frequency_timer == 0 {
            self.frequency_timer = DIVISORS[self.divisor_code as usize] << self.clock_shift;
            let bit = (self.lfsr & 1) ^ ((self.lfsr >> 1) & 1);
            self.lfsr >>= 1;
            self.lfsr |= bit << 14;
            if self.lfsr_width_mode {
                self.lfsr = (self.lfsr & !(1 << 6)) | (bit << 6);
            }
        }
    }

    fn amplitude(&self) -> u8 {
        if !self.enabled {
            return 0;
        }
        let bit = (!self.lfsr & 1) as u8;
        bit * self.envelope.current_volume
    }

    fn dac_on(&self) -> bool {
        self.envelope.initial_volume != 0 || self.envelope.direction
    }

    // NRx4 bit 7. No-op (channel stays off) if the DAC is off. Reloads the
    // length counter only if it's currently zero, resets frequency_timer and
    // the envelope, and resets the LFSR to all-1s (0x7FFF) per Pan Docs.
    fn trigger(&mut self) {
        if !self.dac_on() {
            return;
        }
        self.enabled = true;
        if self.length.counter == 0 {
            self.length.counter = 64;
        }
        self.frequency_timer = DIVISORS[self.divisor_code as usize] << self.clock_shift;
        self.lfsr = 0x7FFF;
        self.envelope.current_volume = self.envelope.initial_volume;
        self.envelope.timer = if self.envelope.period == 0 {
            8
        } else {
            self.envelope.period
        };
    }
}

impl LengthCounter {
    // Called at 256 Hz (frame sequencer steps 0/2/4/6).
    // Returns true when the counter just reached zero — the channel should be disabled.
    fn tick(&mut self) -> bool {
        if self.enabled && self.counter > 0 {
            self.counter -= 1;
            self.counter == 0
        } else {
            false
        }
    }
}

impl Envelope {
    // Called at 64 Hz (frame sequencer step 7). period == 0 disables automatic stepping.
    fn tick(&mut self) {
        if self.period == 0 {
            return;
        }
        if self.timer > 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.timer = self.period;
            if self.direction && self.current_volume < 15 {
                self.current_volume += 1;
            } else if !self.direction && self.current_volume > 0 {
                self.current_volume -= 1;
            }
        }
    }
}

impl Sweep {
    // Called at 128 Hz (frame sequencer steps 2/6) with CH1's live frequency.
    // Returns true if the sweep calculation overflowed 2047 — the channel should be disabled.
    // Simplification: only calculates/checks overflow once per tick; real hardware
    // does this twice (with the second result discarded but its overflow check kept).
    fn tick(&mut self, frequency: &mut u16) -> bool {
        if self.timer > 0 {
            self.timer -= 1;
        }
        if self.timer != 0 {
            return false;
        }
        self.timer = if self.period == 0 { 8 } else { self.period };
        if !self.enabled || self.period == 0 {
            return false;
        }
        let delta = self.shadow_frequency >> self.shift;
        let new_frequency = if self.direction {
            self.shadow_frequency.wrapping_sub(delta)
        } else {
            self.shadow_frequency.wrapping_add(delta)
        };
        if new_frequency > 2047 {
            return true;
        }
        if self.shift > 0 {
            self.shadow_frequency = new_frequency;
            *frequency = new_frequency;
        }
        false
    }
}

impl Nr52 {
    fn from_state(audio_on: bool, ch1: bool, ch2: bool, ch3: bool, ch4: bool) -> Self {
        let mut flags = Nr52::empty();
        flags.set(Nr52::AUDIO_ON, audio_on);
        flags.set(Nr52::CH1_ON, ch1);
        flags.set(Nr52::CH2_ON, ch2);
        flags.set(Nr52::CH3_ON, ch3);
        flags.set(Nr52::CH4_ON, ch4);
        flags
    }
    fn to_state(value: u8) -> (bool, bool, bool, bool, bool) {
        let flags = Nr52::from_bits_truncate(value);
        (
            flags.contains(Nr52::AUDIO_ON),
            flags.contains(Nr52::CH1_ON),
            flags.contains(Nr52::CH2_ON),
            flags.contains(Nr52::CH3_ON),
            flags.contains(Nr52::CH4_ON),
        )
    }
}

impl Nr10 {
    // period: bits 4-6, direction: bit 3, shift: bits 0-2
    fn from_state(period: u8, direction: bool, shift: u8) -> u8 {
        let mut flags = Nr10::empty();
        flags.set(Nr10::DIRECTION, direction);
        flags.bits() | ((period & 0b111) << 4) | (shift & 0b111)
    }
    fn to_state(value: u8) -> (u8, bool, u8) {
        let flags = Nr10::from_bits_truncate(value);
        let period = (value >> 4) & 0b111;
        let shift = value & 0b111;
        (period, flags.contains(Nr10::DIRECTION), shift)
    }
}

impl Nr11 {
    fn from_state(wave_duty: u8, initial_length_timer: u8) -> u8 {
        ((wave_duty & 0b11) << 6) | (initial_length_timer & 0b0011_1111)
    }
    fn to_state(value: u8) -> (u8, u8) {
        let wave_duty = (value >> 6) & 0b11;
        let initial_length_timer = value & 0b0011_1111;
        (wave_duty, initial_length_timer)
    }
}

impl Nr12 {
    // initial_volume: bits 4-7, direction: bit 3, period: bits 0-2
    fn from_state(initial_volume: u8, direction: bool, period: u8) -> u8 {
        let mut flags = Nr12::empty();
        flags.set(Nr12::DIRECTION, direction);
        flags.bits() | ((initial_volume & 0b1111) << 4) | (period & 0b111)
    }
    fn to_state(value: u8) -> (u8, bool, u8) {
        let flags = Nr12::from_bits_truncate(value);
        let initial_volume = (value >> 4) & 0b1111;
        let period = value & 0b111;
        (initial_volume, flags.contains(Nr12::DIRECTION), period)
    }
}

impl Nr14 {
    // trigger: bit 7 (write-only), length_enable: bit 6, frequency_high: bits 0-2 (write-only)
    // unreadable bits (7, 5-0) always read back as 1
    fn from_state(length_enable: bool) -> u8 {
        let mut flags = Nr14::empty();
        flags.set(Nr14::LENGTH_ENABLE, length_enable);
        flags.bits() | 0b1011_1111
    }
    fn to_state(value: u8) -> (bool, bool, u8) {
        let flags = Nr14::from_bits_truncate(value);
        let frequency_high = value & 0b111;
        (
            flags.contains(Nr14::TRIGGER),
            flags.contains(Nr14::LENGTH_ENABLE),
            frequency_high,
        )
    }
}

impl Nr30 {
    fn from_state(dac_enabled: bool) -> u8 {
        let mut flags = Nr30::empty();
        flags.set(Nr30::DAC_ENABLE, dac_enabled);
        flags.bits() | 0b0111_1111
    }
    fn to_state(value: u8) -> bool {
        Nr30::from_bits_truncate(value).contains(Nr30::DAC_ENABLE)
    }
}

impl Nr32 {
    // volume: bits 5-6, rest unused (reads as 1)
    fn from_state(volume: u8) -> u8 {
        ((volume & 0b11) << 5) | 0b1001_1111
    }
    fn to_state(value: u8) -> u8 {
        (value >> 5) & 0b11
    }
}

impl Nr43 {
    // clock_shift: bits 4-7, lfsr_width: bit 3, clock_divider: bits 0-2
    fn from_state(clock_shift: u8, lfsr_width_mode: bool, clock_divider: u8) -> u8 {
        let mut flags = Nr43::empty();
        flags.set(Nr43::LFSR_WIDTH, lfsr_width_mode);
        flags.bits() | ((clock_shift & 0b1111) << 4) | (clock_divider & 0b111)
    }
    fn to_state(value: u8) -> (u8, bool, u8) {
        let flags = Nr43::from_bits_truncate(value);
        let clock_shift = (value >> 4) & 0b1111;
        let clock_divider = value & 0b111;
        (clock_shift, flags.contains(Nr43::LFSR_WIDTH), clock_divider)
    }
}
