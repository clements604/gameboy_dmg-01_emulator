use log::error;
use sdl2::audio::{AudioQueue, AudioSpecDesired};

const SAMPLE_RATE: i32 = 44_100;
const CHANNELS: u8 = 2;
const BYTES_PER_FRAME: u32 = CHANNELS as u32 * std::mem::size_of::<i16>() as u32;
// Cap the backlog at ~8 frames (~133ms at 60fps) worth of queued audio, so a
// stall (e.g. a debugger pause) doesn't build up unbounded latency — the
// queue is cleared and refilled fresh instead of catching up sample-by-sample.
const MAX_QUEUED_FRAMES: u32 = 8;

pub struct AudioOutput {
    queue: AudioQueue<i16>,
}

impl AudioOutput {
    pub fn new(sdl_context: &sdl2::Sdl) -> Result<AudioOutput, String> {
        let audio_subsystem = sdl_context.audio()?;

        let spec = AudioSpecDesired {
            freq: Some(SAMPLE_RATE),
            channels: Some(CHANNELS),
            samples: None,
        };

        let queue: AudioQueue<i16> = audio_subsystem.open_queue(None, &spec)?;
        queue.resume(); // SDL audio queues start paused.

        Ok(AudioOutput { queue })
    }

    // Interleaves (L, R) pairs into SDL's expected flat sample layout and queues them.
    pub fn queue_samples(&mut self, samples: &[(i16, i16)]) {
        let max_queued_bytes = MAX_QUEUED_FRAMES * (SAMPLE_RATE as u32 / 60) * BYTES_PER_FRAME;
        if self.queue.size() > max_queued_bytes {
            self.queue.clear();
        }

        let mut interleaved = Vec::with_capacity(samples.len() * 2);
        for &(left, right) in samples {
            interleaved.push(left);
            interleaved.push(right);
        }

        if let Err(e) = self.queue.queue_audio(&interleaved) {
            error!("Failed to queue audio samples: {}", e);
        }
    }
}
