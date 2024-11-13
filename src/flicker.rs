use core::f32;
use std::ops::{Range, Rem};

use rand_distr::Distribution;

pub type DurationMillis = f32;
pub type InstantMillis = f32;
/// In percent from 0.0-1.0 inclusive.
pub type Brightness = f32;

fn lerp<T: num_traits::Float>(low: T, high: T, x: T) -> T {
    low + x * (high - low)
}

/// Flickering is simulated by rendering a sine wave of brightness with random
/// "blinks" which drop to a constant brightness for a random amount of time.
/// Graphically this looks like a sine wave which drops to `min_brightness`
/// for varying durations.
#[derive(Debug, Clone)]
pub struct FlickerParams {
    /// How long to wait before progressing to the next brightness level.
    time_step: DurationMillis,
    /// Sine wave period.
    wave_period: DurationMillis,
    /// Minimum brightness of the sine wave.
    min_brightness: Brightness,
    /// Maximum brightness of the sine wave.
    max_brightness: Brightness,
    /// The probability of a blink happening during a time window. This is actually
    /// sampled _after_ a delta timestep occurs: if the sampled duration is shorter
    /// than how long we just waited, we start a blink.
    blink_probability: rand_distr::Exp<DurationMillis>,
    // The length of a blink (rounded to nearest `time_step`).
    blink_duration: rand_distr::Normal<DurationMillis>,
}
impl FlickerParams {
    fn new(
        time_step: DurationMillis,
        wave_period: DurationMillis,
        min_brightness: Brightness,
        max_brightness: Brightness,
        blink_probability: rand_distr::Exp<DurationMillis>,
        blink_duration: rand_distr::Normal<DurationMillis>,
    ) -> Self {
        assert!(time_step > 0.0);
        assert!(wave_period > 0.0);
        assert!(time_step < wave_period);
        assert!((0.0..=1.0).contains(&min_brightness));
        assert!((0.0..=1.0).contains(&max_brightness));
        assert!(min_brightness <= max_brightness);

        FlickerParams {
            time_step,
            wave_period,
            min_brightness,
            max_brightness,
            blink_probability,
            blink_duration,
        }
    }

    /// Domain-specific `sin(t)`, scaled to `wave_period`, `min_brightness`, `max_brightness`.
    fn scaled_sin(&self, t: InstantMillis) -> Brightness {
        let magnitude = f32::sin(2.0 * f32::consts::PI * (self.wave_period - t));
        lerp(self.min_brightness, self.max_brightness, magnitude)
    }
}

pub struct FlickerSequence<'r, R: rand::Rng + ?Sized> {
    params: FlickerParams,
    rng: &'r mut R,
    time: InstantMillis,
    blink: Option<Range<InstantMillis>>,
}
impl<'r, R: rand::Rng + ?Sized> FlickerSequence<'r, R> {
    pub fn new(params: FlickerParams, rng: &'r mut R) -> Self {
        FlickerSequence {
            params,
            rng,
            time: 0.0,
            blink: None,
        }
    }

    fn step(&mut self, delta_time: DurationMillis) -> Brightness {
        assert!(delta_time > 0.0);
        if self.time + delta_time > self.params.wave_period {
            self.time = (self.time + delta_time).rem(self.params.wave_period);
        }

        if self.blink.as_ref().is_some_and(|r| r.end < self.time) {
            self.blink = None
        }
        if self.blink.is_none() && self.params.blink_probability.sample(self.rng) <= delta_time {
            let duration = self.params.blink_duration.sample(self.rng);
            self.blink = Some(self.time..(self.time + duration));
        }
        if self.blink.as_ref().is_some_and(|r| r.contains(&self.time)) {
            return self.params.min_brightness;
        }

        self.params.scaled_sin(self.time)
        //f32::sin(self)
    }
}
//impl Iterator for FlickerSequence {
//    type Item = Brightness;
//
//    fn next(&mut self) -> Option<Self::Item> {
//        Some(self.step())
//    }
//}
