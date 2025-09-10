use crate::{
    LoadError, StereoSample,
    read_cursor::ReadCursor,
    track::{Key, TrackBase, keys},
};

pub struct MelodyTrack {
    pub base: TrackBase,
    waveform: [i8; 0x100],
    envelope: [u8; 0x40],
    octave: u8,
    // How long a note "holds" after being hit
    len: u16,
}

impl Default for MelodyTrack {
    fn default() -> Self {
        Self {
            base: TrackBase::default(),
            waveform: [0; _],
            envelope: [0; _],
            octave: 0,
            len: 0,
        }
    }
}

impl MelodyTrack {
    pub fn render(&mut self, [out_l, out_r]: &mut StereoSample, samp_phase: f32) {
        for key in keys() {
            if self.base.timers[usize::from(key)] <= 0.0 {
                continue;
            }
            self.base.timers[usize::from(key)] -= samp_phase;

            let [l, r] = self.sample_of_key(key, samp_phase);
            *out_l = out_l.saturating_add(l);
            *out_r = out_r.saturating_add(r);
        }
    }
    pub fn read(&mut self, cur: &mut ReadCursor) -> Result<(), LoadError> {
        self.octave = cur.next_u8().ok_or(LoadError::PrematureEof)?;
        cur.skip(3);
        self.len = cur
            .next_u32_le()
            .ok_or(LoadError::PrematureEof)?
            .try_into()
            .unwrap();
        self.base.vol = cur
            .next_u32_le()
            .ok_or(LoadError::PrematureEof)?
            .try_into()
            .unwrap();
        cur.skip(8);
        self.waveform =
            *bytemuck::cast_ref(cur.next_bytes::<256>().ok_or(LoadError::PrematureEof)?);
        self.envelope = *cur.next_bytes().ok_or(LoadError::PrematureEof)?;
        Ok(())
    }
    pub fn tick(&mut self, note_idx: usize) {
        let note = self.base.notes[note_idx];
        for key in keys() {
            if note.key_down(key) {
                self.base.timers[usize::from(key)] = f32::from(self.len);
                self.base.phases[usize::from(key)] = 0.;
            }
        }

        let vol = f32::from((i16::try_from(self.base.vol).unwrap() - 300) * 8);
        self.base.vol_mix = 10.0f32.powf(vol / 2000.0);
        if let Some(pan) = note.pan() {
            self.base.vol_left = 10.0f32.powf(f32::from(pan.min(0)) / 2000.0);
            self.base.vol_right = 10.0f32.powf(f32::from((-pan).min(0)) / 2000.0);
        }
    }
    fn sample_of_key(&mut self, key: Key, samp_phase: f32) -> StereoSample {
        let key = usize::from(key);
        // Since we use the timer as an index here, truncation is expected.
        // We ignore any fractional part.
        // Also, we expect the timer to remain positive at all times, so there shouldn't be
        // any sign loss
        debug_assert!(self.base.timers[key] >= 0.0);
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mut idx = 64 * (self.len as usize - self.base.timers[key] as usize) / self.len as usize;
        if idx >= 64 {
            idx = 63;
        }
        let envelope = 2 * i16::from(self.envelope[idx]);
        let oct_shift: u8 = 1 << self.octave;
        let freq_table = [
            1551., 1652., 1747., 1848., 1955., 2074., 2205., 2324., 2461., 2616., 2770., 2938.,
        ];
        let phase = (f32::from(oct_shift)
            * (if key < 12 {
                freq_table[key] / 16.0
            } else {
                freq_table[key - 12] / 8.0
            }))
            * samp_phase;
        self.base.phases[key] += phase;
        // We intentionally convert the phase into an index here, so truncation is expected.
        // Moreover, we assume that phase is never negative, so no sign loss can occur.
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let tp = self.base.phases[key] as usize / 256;
        let s0 = i16::from(self.waveform[tp & 0xff]);
        let s = s0 * envelope;

        // We are converting floating point samples to integer samples.
        // There really isn't anything we can do about the truncation.
        #[expect(clippy::cast_possible_truncation)]
        [
            (f32::from(s) * self.base.vol_mix * self.base.vol_left) as i16,
            (f32::from(s) * self.base.vol_mix * self.base.vol_right) as i16,
        ]
    }
}
