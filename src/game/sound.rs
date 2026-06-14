use std::f32::consts::PI;
use std::time::Duration;
use rodio::Source;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundType {
    Warning,
    Laser,
    Missile,
    Explosion,
    Speedboat,
}

// Fast LCG random number generator for procedural sound generation
struct Lcg {
    state: u32,
}

impl Lcg {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_f32(&mut self) -> f32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.state as f32) / (u32::MAX as f32) * 2.0 - 1.0
    }
}

pub struct SynthSound {
    sample_rate: u32,
    duration: Duration,
    time: f32,
    sound_type: SoundType,
    volume: f32,
    current_sample: usize,
    total_samples: usize,
    lcg: Lcg,
}

impl SynthSound {
    pub fn new(sample_rate: u32, duration: Duration, sound_type: SoundType, volume: f32) -> Self {
        let total_samples = (sample_rate as f64 * duration.as_secs_f64()) as usize;
        let seed = match sound_type {
            SoundType::Warning => 1,
            SoundType::Laser => 2,
            SoundType::Missile => 3,
            SoundType::Explosion => 4,
            SoundType::Speedboat => 5,
        };
        Self {
            sample_rate,
            duration,
            time: 0.0,
            sound_type,
            volume,
            current_sample: 0,
            total_samples,
            lcg: Lcg::new(seed),
        }
    }
}

impl Iterator for SynthSound {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_sample >= self.total_samples {
            return None;
        }

        let progress = self.current_sample as f32 / self.total_samples as f32;
        let mut val;

        match self.sound_type {
            SoundType::Warning => {
                let mut freq = 880.0_f32;
                if ((self.time * 8.0) as i32) % 2 == 0 {
                    freq = 660.0;
                }
                val = (2.0 * PI * freq * self.time).sin();
                if ((self.time * 4.0) as i32) % 2 == 0 {
                    val = 0.0;
                }
                if progress > 0.85 {
                    val *= (1.0 - progress) / 0.15;
                }
            }
            SoundType::Laser => {
                let noise = self.lcg.next_f32();
                let freq = 320.0 - 260.0 * progress;
                let tone = (2.0 * PI * freq * self.time).sin() * 0.4;
                val = 0.6 * noise + tone;
                val *= (-18.0 * progress).exp();
            }
            SoundType::Missile => {
                let noise = self.lcg.next_f32();
                let freq = 200.0 + 120.0 * (PI * progress).sin();
                let tone = (2.0 * PI * freq * self.time).sin();
                val = 0.8 * noise + 0.2 * tone;
                val *= (PI * progress).sin();
            }
            SoundType::Explosion => {
                let noise = self.lcg.next_f32();
                let sub_freq = 30.0 * (1.0 - progress * 0.5);
                let sub_bass = (2.0 * PI * sub_freq * self.time).sin();
                let rumble = (2.0 * PI * sub_freq * 2.0 * self.time).sin();
                val = 0.55 * noise + 0.30 * sub_bass + 0.15 * rumble;
                val *= (-1.8 * progress).exp();
            }
            SoundType::Speedboat => {
                let freq = 220.0 + 15.0 * (2.0 * PI * 8.0 * self.time).sin();
                let engine = (2.0 * PI * freq * self.time).sin() * 0.4;
                let mut slap = 0.0;
                if ((self.time * 12.0) as i32) % 2 == 0 {
                    let noise = self.lcg.next_f32();
                    slap = noise * 0.4 * (-30.0 * (self.time % (1.0 / 12.0))).exp();
                }
                val = engine + slap;
                val *= 0.5;
            }
        }

        self.time += 1.0 / self.sample_rate as f32;
        self.current_sample += 1;
        Some(val * self.volume)
    }
}

impl Source for SynthSound {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> std::num::NonZeroU16 {
        std::num::NonZeroU16::new(1).expect("channels must be non-zero")
    }

    fn sample_rate(&self) -> std::num::NonZeroU32 {
        std::num::NonZeroU32::new(self.sample_rate).expect("sample rate must be non-zero")
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.duration)
    }
}
