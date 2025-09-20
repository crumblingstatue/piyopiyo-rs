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
    timers: [f64; N_KEYS as usize],
    phases: [f64; N_KEYS as usize],
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

/// A track defines how to interpret events and generate sound samples from them.
///
/// There are 3 melody tracks and one drum track.
pub trait Track {
    /// How long the note will last after being pressed
    fn note_duration(&self, key: PianoKey) -> f64;
    /// Generates a sample for a piano key being held down at index `key`
    fn sample_of_key(&mut self, key: PianoKey, samp_phase: f64) -> StereoSample;
    /// Returns the data shared between melody and trum tracks
    fn base(&mut self) -> &mut TrackBase;
    /// Processes the event at the provided event index in the track's own event data
    fn do_event_at_idx(&mut self, event_idx: usize) {
        let event = self.base().events[event_idx];
        self.do_event(event);
    }
    /// Processes the provided event
    fn do_event(&mut self, event: Event) {
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
        self.post_event();
    }
    /// Some tracks have to do some post-event handling
    fn post_event(&mut self) {}
    /// Render the next stereo sample according to the internal state of the track
    fn render_next(&mut self, [out_l, out_r]: &mut StereoSample, samp_phase: f64) {
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
    /// For each piano key, how much time there's after a keypress left until silence (0.0)
    ///
    /// Can be used for example to detect which keys are being held down currently
    fn timers(&mut self) -> [f64; N_KEYS as usize] {
        self.base().timers
    }
}

/// An event consisting of piano key down states and optional pan value
#[repr(transparent)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Event(u32);

impl Event {
    /// Whether a piano key at index `key` is down
    #[must_use]
    pub const fn key_down(self, key: PianoKey) -> bool {
        self.0 & (1 << key) != 0
    }
    /// Set piano key at index `key` to be down
    pub const fn set_key_down(&mut self, key: PianoKey) {
        self.0 |= 1 << key;
    }
    /// Set piano key at index `key` to be up
    pub const fn set_key_up(&mut self, key: PianoKey) {
        self.0 &= !(1 << key);
    }
    /// Return the pan value (if any) of this event
    #[must_use]
    pub fn pan(self) -> Option<i16> {
        let pan_table = [2560, 1600, 760, 320, 0, -320, -760, -1640];
        (self.0 & 0xff00_0000 != 0).then(|| pan_table[(self.0 >> 24) as usize])
    }
    /// Construct an event from an array of piano key down states
    #[must_use]
    pub fn from_keydown_array(arr: [bool; N_KEYS as usize]) -> Self {
        let mut ev = 0;
        for (i, down) in arr.into_iter().enumerate() {
            if down {
                ev |= 1 << i;
            }
        }
        Self(ev)
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
