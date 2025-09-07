use crate::StereoSample;

pub struct Track {
    pub waveform: [i8; 0x100],
    pub envelope: [u8; 0x40],
    pub octave: u8,
    pub len: u32,
    // Some tracks seem to have over 255 volume, so this can't be u8
    pub vol: u16,
    active_note: Note,
    vol_left: f32,
    vol_right: f32,
    vol_mix: f32,
    vol_mix_low: f32,
    timers: [f32; N_KEYS as usize],
    f_phases: [f32; N_KEYS as usize],
    phases: [u32; N_KEYS as usize],
    pub notes: Box<[Note]>,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            waveform: [0; _],
            envelope: [0; _],
            octave: 0,
            len: 0,
            vol: 0,
            active_note: Note(0),
            vol_left: 1.0,
            vol_right: 1.0,
            vol_mix: 1.0,
            vol_mix_low: 1.0,
            timers: Default::default(),
            f_phases: Default::default(),
            phases: Default::default(),
            notes: Box::default(),
        }
    }
}

impl Track {
    pub fn tick<const PERCUSSION: bool>(&mut self, note_idx: usize) {
        self.active_note = self.notes[note_idx];
        for key in keys() {
            // Sample and track lengths are small enough to fit f32
            #[expect(clippy::cast_precision_loss)]
            if self.active_note.key_down(key) {
                self.timers[usize::from(key)] = if PERCUSSION {
                    PERCUSSION_SAMPLES[usize::from(key)].len() as f32
                } else {
                    self.len as f32
                };
                self.phases[usize::from(key)] = 0;
                self.f_phases[usize::from(key)] = 0.;
            }
        }
        if PERCUSSION {
            let vol = f32::from((i16::try_from(self.vol).unwrap() - 300) * 8);
            self.vol_mix = 10.0f32.powf(vol / 2000.0);
            let vol = f32::from((((7 * i16::try_from(self.vol).unwrap()) / 10) - 300) * 8);
            self.vol_mix_low = 10.0f32.powf(vol / 2000.0);
        } else {
            let vol = f32::from((i16::try_from(self.vol).unwrap() - 300) * 8);
            self.vol_mix = 10.0f32.powf(vol / 2000.0);
        }
        if let Some(pan) = self.active_note.pan() {
            self.vol_left = 10.0f32.powf(f32::from(pan.min(0)) / 2000.0);
            self.vol_right = 10.0f32.powf(f32::from((-pan).min(0)) / 2000.0);
        }
    }
    pub fn render<const PERCUSSION: bool>(&mut self, sample: &mut StereoSample, samp_phase: f32) {
        for key in keys() {
            if self.timers[usize::from(key)] <= 0.0 {
                continue;
            }
            self.timers[usize::from(key)] -= samp_phase;

            if PERCUSSION {
                self.render_percussion(sample, samp_phase, key);
            } else {
                self.render_melody(sample, samp_phase, key);
            }
        }
    }
    fn render_melody(&mut self, [l, r]: &mut StereoSample, samp_phase: f32, key: Key) {
        let key = usize::from(key);
        // Since we use the timer as an index here, truncation is expected.
        // We ignore any fractional part.
        // Also, we expect the timer to remain positive at all times, so there shouldn't be
        // any sign loss
        debug_assert!(self.timers[key] >= 0.0);
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mut idx = 64 * (self.len as usize - self.timers[key] as usize) / self.len as usize;
        if idx >= 64 {
            idx = 63;
        }
        let envelope = 2 * i16::from(self.envelope[idx]);
        let oct_shift: u8 = 1 << self.octave;
        let phase = (f32::from(oct_shift)
            * (if key < 12 {
                f32::from(FREQ_TBL[key]) / 16.0
            } else {
                f32::from(FREQ_TBL[key - 12]) / 8.0
            }))
            * samp_phase;
        // We intentionally convert the phase into an integer here, so truncation is expected.
        // Moreover, we assume that phase is never negative, so no sign loss can occur.
        debug_assert!(phase >= 0.0);
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            self.phases[key] += phase as u32;
        }
        let tp = self.phases[key] / 256;

        let s0 = i16::from(self.waveform[(tp & 0xff) as usize]);
        let s = s0 * envelope;

        // We are converting floating point samples to integer samples.
        // There really isn't anything we can do about the truncation.
        #[expect(clippy::cast_possible_truncation)]
        {
            *l = l.saturating_add((f32::from(s) * self.vol_mix * self.vol_left) as i16);
            *r = r.saturating_add((f32::from(s) * self.vol_mix * self.vol_right) as i16);
        }
    }

    fn render_percussion(&mut self, [l, r]: &mut StereoSample, samp_phase: f32, key: Key) {
        let key = usize::from(key);
        self.f_phases[key] += samp_phase;
        // Since we use the phase as an index, truncation is expected.
        // We also assume that the phase can never be negative, so sign loss cannot occur.
        debug_assert!(self.f_phases[key] >= 0.0);
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ph = self.f_phases[key] as usize;
        let ph2 = ph + usize::from(ph + 1 != PERCUSSION_SAMPLES[key].len());
        let ph_fract = self.f_phases[key].fract();
        if ph >= PERCUSSION_SAMPLES[key].len() {
            return;
        }
        let v0 = f32::from(i16::from(PERCUSSION_SAMPLES[key][ph]) - 0x80);
        let v1 = f32::from(i16::from(PERCUSSION_SAMPLES[key][ph2]) - 0x80);
        let p = ph_fract.mul_add(v1 - v0, v0)
            * 256.0
            * (if (key & 1) != 0 {
                self.vol_mix_low
            } else {
                self.vol_mix
            });
        // We assume that the sample can fit within i16 range, and we don't care about
        // the fractional part.
        #[expect(clippy::cast_possible_truncation)]
        {
            *l = l.saturating_add((p * self.vol_left) as i16);
            *r = r.saturating_add((p * self.vol_right) as i16);
        }
    }
}

const FREQ_TBL: [i16; 12] = [
    1551, 1652, 1747, 1848, 1955, 2074, 2205, 2324, 2461, 2616, 2770, 2938,
];

const PAN_TBL: [i16; 8] = [2560, 1600, 760, 320, 0, -320, -760, -1640];

#[repr(transparent)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Note(u32);

impl Note {
    pub const fn key_down(self, key: u8) -> bool {
        self.0 & (1 << key) != 0
    }
    pub fn pan(self) -> Option<i16> {
        (self.0 & 0xff00_0000 != 0).then(|| PAN_TBL[(self.0 >> 24) as usize])
    }
}

const BASS1: &[u8] = include_bytes!("../wav/bass1.bin");
const BASS2: &[u8] = include_bytes!("../wav/bass2.bin");
const SNARE: &[u8] = include_bytes!("../wav/snare.bin");
const HAT1: &[u8] = include_bytes!("../wav/hat1.bin");
const HAT2: &[u8] = include_bytes!("../wav/hat2.bin");
const CYMBAL: &[u8] = include_bytes!("../wav/cymbal.bin");

const PERCUSSION_SAMPLES: [&[u8]; N_KEYS as usize] = [
    BASS1, BASS1, BASS2, BASS2, SNARE, SNARE, SNARE, SNARE, HAT1, HAT1, HAT2, HAT2, CYMBAL, CYMBAL,
    CYMBAL, CYMBAL, CYMBAL, CYMBAL, CYMBAL, CYMBAL, CYMBAL, CYMBAL, CYMBAL, CYMBAL,
];

const N_KEYS: u8 = 24;

type Key = u8;

const fn keys() -> std::ops::Range<Key> {
    0..N_KEYS
}
