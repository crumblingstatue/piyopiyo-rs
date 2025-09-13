use crate::{
    StereoSample,
    read_cursor::ReadCursor,
    song::LoadError,
    track::{PianoKey, Track, TrackBase},
};

/// A melody track based on a waveform and envelope
pub struct MelodyTrack {
    /// Track data common to melody/percussion tracks
    pub base: TrackBase,
    /// The waveform, or in other words, the instrument we're playing
    pub waveform: [i8; 0x100],
    /// The envelope (volume variation over time) of the waveform
    pub envelope: [u8; 0x40],
    /// Octave shift applied when playing the instrument
    pub octave: u8,
    /// How long a note "holds" after being hit
    pub len: u16,
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
    pub(crate) fn read(&mut self, cur: &mut ReadCursor) -> Result<(), LoadError> {
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
}

impl Track for MelodyTrack {
    fn note_duration(&self, _key: PianoKey) -> f64 {
        f64::from(self.len)
    }
    fn sample_of_key(&mut self, key: PianoKey, samp_phase: f64) -> StereoSample {
        let key = usize::from(key);
        // If the timer is below 0 due to whatever reason, clamp it back to 0 for sanity's sake.
        if self.base.timers[key] < 0.0 {
            self.base.timers[key] = 0.0;
        }
        // Since we use the timer as an index here, truncation is expected.
        // We ignore any fractional part.
        // Also, we expect the timer to remain positive at all times, so there shouldn't be
        // any sign loss
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mut idx = (64 * ((self.len as usize).saturating_sub(self.base.timers[key] as usize)))
            .checked_div(self.len as usize)
            .unwrap_or(0);
        if idx >= 64 {
            idx = 63;
        }
        let envelope = 2 * i16::from(self.envelope[idx]);
        let oct_shift: u8 = 1 << self.octave;
        let freq_table = [
            1551., 1652., 1747., 1848., 1955., 2074., 2205., 2324., 2461., 2616., 2770., 2938.,
        ];
        let phase = (f64::from(oct_shift)
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

    fn base(&mut self) -> &mut TrackBase {
        &mut self.base
    }
}
