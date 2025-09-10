use crate::{
    StereoSample,
    track::{Key, N_KEYS, Track, TrackBase},
};

pub struct PercussionTrack {
    pub base: TrackBase,
    vol_mix_low: f32,
}

impl Default for PercussionTrack {
    fn default() -> Self {
        Self {
            base: TrackBase::default(),
            vol_mix_low: 1.0,
        }
    }
}

impl Track for PercussionTrack {
    fn note_duration(&self, key: Key) -> f32 {
        // Percussion samples are short enough to fit into f32 without problem.
        #[expect(clippy::cast_precision_loss)]
        (PERCUSSION_SAMPLES[usize::from(key)].len() as f32)
    }
    fn post_tick(&mut self) {
        let vol = f32::from((((7 * i16::try_from(self.base.vol).unwrap()) / 10) - 300) * 8);
        self.vol_mix_low = 10.0f32.powf(vol / 2000.0);
    }
    fn sample_of_key(&mut self, key: Key, samp_phase: f32) -> StereoSample {
        let key = usize::from(key);
        self.base.phases[key] += samp_phase;
        // Since we use the phase as an index, truncation is expected.
        // We also assume that the phase can never be negative, so sign loss cannot occur.
        debug_assert!(self.base.phases[key] >= 0.0);
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ph = self.base.phases[key] as usize;
        if ph >= PERCUSSION_SAMPLES[key].len() {
            return [0, 0];
        }
        let ph2 = ph + usize::from(ph + 1 != PERCUSSION_SAMPLES[key].len());
        let ph_fract = self.base.phases[key].fract();
        let v0 = f32::from(i16::from(PERCUSSION_SAMPLES[key][ph]) - 0x80);
        let v1 = f32::from(i16::from(PERCUSSION_SAMPLES[key][ph2]) - 0x80);
        let p = ph_fract.mul_add(v1 - v0, v0)
            * 256.0
            * (if (key & 1) != 0 {
                self.vol_mix_low
            } else {
                self.base.vol_mix
            });
        // We assume that the sample can fit within i16 range, and we don't care about
        // the fractional part.
        #[expect(clippy::cast_possible_truncation)]
        [
            (p * self.base.vol_left) as i16,
            (p * self.base.vol_right) as i16,
        ]
    }

    fn base(&mut self) -> &mut TrackBase {
        &mut self.base
    }
}

const BASS1: &[u8] = include_bytes!("../../wav/bass1.bin");
const BASS2: &[u8] = include_bytes!("../../wav/bass2.bin");
const SNARE: &[u8] = include_bytes!("../../wav/snare.bin");
const HAT1: &[u8] = include_bytes!("../../wav/hat1.bin");
const HAT2: &[u8] = include_bytes!("../../wav/hat2.bin");
const CYMBAL: &[u8] = include_bytes!("../../wav/cymbal.bin");

const PERCUSSION_SAMPLES: [&[u8]; N_KEYS as usize] = [
    BASS1, BASS1, BASS2, BASS2, SNARE, SNARE, SNARE, SNARE, HAT1, HAT1, HAT2, HAT2, CYMBAL, CYMBAL,
    CYMBAL, CYMBAL, CYMBAL, CYMBAL, CYMBAL, CYMBAL, CYMBAL, CYMBAL, CYMBAL, CYMBAL,
];
