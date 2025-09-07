//! Player for Pixel's Piyo Piyo (PMD) music format.
//!
//! Based on <https://github.com/alula/piyoplay>

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    unused_qualifications,
    redundant_imports,
    trivial_casts,
    trivial_numeric_casts,
    clippy::pedantic,
    clippy::missing_const_for_fn,
    clippy::suboptimal_flops
)]

use crate::{read_cursor::ReadCursor, track::Track};

mod read_cursor;
mod track;

/// PMD music player
pub struct Player {
    sample_rate: u16,
    millis_per_tick: u32,
    repeat_tick: u32,
    end_tick: u32,
    tracks: [Track; 4],
    curr_tick: u32,
    note_ptr: u32,
    loaded: bool,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            sample_rate: 44_100,
            millis_per_tick: 0,
            repeat_tick: 0,
            end_tick: 0,
            tracks: std::array::from_fn(|_| Track::default()),
            curr_tick: 0,
            note_ptr: 0,
            loaded: false,
        }
    }
}

/// Error that can happen when loading a PMD file
#[derive(Debug)]
pub enum LoadError {
    /// Invalid magic (not `PMD`)
    InvalidMagic,
}

type StereoSample = [i16; 2];

impl Player {
    /// Load a PMD music file into the player
    ///
    /// # Panics
    /// - If the file is too short
    ///
    /// # Errors
    ///
    /// - If the file doesn't have the proper magic marker (`PMD`)
    pub fn load(&mut self, data: &[u8]) -> Result<(), LoadError> {
        self.loaded = false;
        let mut cur = ReadCursor(data);
        let magic = cur.next_bytes();
        if magic != Some(b"PMD") {
            return Err(LoadError::InvalidMagic);
        }
        cur.skip(5);
        self.millis_per_tick = cur.next_u32_le().unwrap();
        self.repeat_tick = cur.next_u32_le().unwrap();
        self.end_tick = cur.next_u32_le().unwrap();
        let n_notes = cur.next_u32_le().unwrap() as usize;

        for track in &mut self.tracks[..3] {
            track.octave = cur.next_u8().unwrap();
            cur.skip(3);
            track.len = cur.next_u32_le().unwrap();
            track.vol = cur.next_u32_le().unwrap().try_into().unwrap();
            cur.skip(8);
            track.waveform = *bytemuck::cast_ref(cur.next_bytes::<256>().unwrap());
            track.envelope = *cur.next_bytes().unwrap();
        }

        self.tracks[3].vol = cur.next_u32_le().unwrap().try_into().unwrap();

        for track in &mut self.tracks {
            track.notes = cur.next_n(n_notes).into();
        }
        self.loaded = true;
        Ok(())
    }
    /// Advances playback and renders samples into `buf`.
    pub fn render_next(&mut self, buf: &mut [i16]) {
        if !self.loaded {
            return;
        }
        for sample in buf.as_chunks_mut().0 {
            self.tick();
            *sample = self.next_sample();
        }
    }

    fn tick(&mut self) {
        let curr_tick = self.curr_tick;
        self.curr_tick = self.curr_tick.wrapping_sub(1);
        if curr_tick == 0 {
            let samples_per_tick = u32::from(self.sample_rate) * self.millis_per_tick / 1000;
            self.curr_tick = samples_per_tick;
            let (perc, melody) = self.tracks.split_last_mut().unwrap();

            for track in melody {
                track.tick::<false>(self.note_ptr as usize);
            }
            perc.tick::<true>(self.note_ptr as usize);
            self.note_ptr += 1;
            if self.note_ptr >= self.end_tick {
                self.note_ptr = self.repeat_tick;
            }
        }
    }

    fn next_sample(&mut self) -> StereoSample {
        let mut sample = [0; 2];
        let samp_phase = 22_050. / f32::from(self.sample_rate);
        let (perc, melody) = self.tracks.split_last_mut().unwrap();
        for track in melody {
            track.render::<false>(&mut sample, samp_phase);
        }
        perc.render::<true>(&mut sample, samp_phase);
        sample
    }
}
