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
    pub notes: Box<[Note]>,
}

impl Default for TrackBase {
    fn default() -> Self {
        Self {
            vol: 0,
            vol_left: 1.0,
            vol_right: 1.0,
            vol_mix: 1.0,
            timers: Default::default(),
            phases: Default::default(),
            notes: Box::default(),
        }
    }
}

pub trait Track {
    fn sample_of_key(&mut self, key: Key, samp_phase: f32) -> StereoSample;
    fn timers(&mut self) -> &mut [f32; N_KEYS as usize];
    fn render(&mut self, [out_l, out_r]: &mut StereoSample, samp_phase: f32) {
        for key in keys() {
            if self.timers()[usize::from(key)] <= 0.0 {
                continue;
            }
            self.timers()[usize::from(key)] -= samp_phase;

            let [l, r] = self.sample_of_key(key, samp_phase);
            *out_l = out_l.saturating_add(l);
            *out_r = out_r.saturating_add(r);
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Note(u32);

impl Note {
    pub const fn key_down(self, key: Key) -> bool {
        self.0 & (1 << key) != 0
    }
    pub fn pan(self) -> Option<i16> {
        let pan_table = [2560, 1600, 760, 320, 0, -320, -760, -1640];
        (self.0 & 0xff00_0000 != 0).then(|| pan_table[(self.0 >> 24) as usize])
    }
}

const N_KEYS: u8 = 24;

type Key = u8;

const fn keys() -> std::ops::Range<Key> {
    0..N_KEYS
}
