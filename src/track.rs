pub use self::{melody::MelodyTrack, percussion::PercussionTrack};

use crate::StereoSample;

mod melody;
mod percussion;

pub struct TrackBase {
    // Seems to be in the range 0..=300
    pub vol: u16,
    vol_left: f32,
    vol_right: f32,
    vol_mix: f32,
    timers: [f32; N_KEYS as usize],
    phases: [f32; N_KEYS as usize],
    pub events: Box<[Event]>,
}

impl Default for TrackBase {
    fn default() -> Self {
        Self {
            vol: 0,
            vol_left: 1.0,
            vol_right: 1.0,
            vol_mix: 0.0,
            timers: Default::default(),
            phases: Default::default(),
            events: Box::default(),
        }
    }
}

pub trait Track {
    fn note_duration(&self, key: PianoKey) -> f32;
    fn sample_of_key(&mut self, key: PianoKey, samp_phase: f32) -> StereoSample;
    fn base(&mut self) -> &mut TrackBase;
    fn tick(&mut self, event_idx: usize) {
        let event = self.base().events[event_idx];
        for key in piano_keys() {
            if event.key_down(key) {
                self.base().timers[usize::from(key)] = self.note_duration(key);
                self.base().phases[usize::from(key)] = 0.;
            }
        }
        let vol = f32::from((i16::try_from(self.base().vol).unwrap() - 300) * 8);
        self.base().vol_mix = 10.0f32.powf(vol / 2000.0);
        if let Some(pan) = event.pan() {
            self.base().vol_left = 10.0f32.powf(f32::from(pan.min(0)) / 2000.0);
            self.base().vol_right = 10.0f32.powf(f32::from((-pan).min(0)) / 2000.0);
        }
        self.post_tick();
    }
    fn post_tick(&mut self) {}
    fn render(&mut self, [out_l, out_r]: &mut StereoSample, samp_phase: f32) {
        for key in piano_keys() {
            if self.base().timers[usize::from(key)] <= 0.0 {
                continue;
            }
            self.base().timers[usize::from(key)] -= samp_phase;

            let [l, r] = self.sample_of_key(key, samp_phase);
            *out_l = out_l.saturating_add(l);
            *out_r = out_r.saturating_add(r);
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Event(u32);

impl Event {
    pub const fn key_down(self, key: PianoKey) -> bool {
        self.0 & (1 << key) != 0
    }
    pub const fn set_key_down(&mut self, key: PianoKey) {
        self.0 |= 1 << key;
    }
    pub const fn unset_key_down(&mut self, key: PianoKey) {
        self.0 &= !(1 << key);
    }
    pub fn pan(self) -> Option<i16> {
        let pan_table = [2560, 1600, 760, 320, 0, -320, -760, -1640];
        (self.0 & 0xff00_0000 != 0).then(|| pan_table[(self.0 >> 24) as usize])
    }
}

/// Number of piano keys
pub const N_KEYS: PianoKey = 24;

/// Piano key in range 0..=23
pub type PianoKey = u8;

/// Returns a range of all piano keys
#[must_use]
pub const fn piano_keys() -> std::ops::Range<PianoKey> {
    0..N_KEYS
}
