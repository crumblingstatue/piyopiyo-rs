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

pub use crate::track::{MelodyTrack, N_KEYS, PercussionTrack, PianoKey, piano_keys};

use crate::{
    song::{LoadError, Song},
    track::Track as _,
};

mod read_cursor;
mod song;
mod track;

/// PMD music player
pub struct Player {
    sample_rate: u16,
    curr_tick: u32,
    /// Index of event to process next
    pub event_cursor: u32,
    /// The currently loaded song
    pub song: Song,
}

impl std::error::Error for LoadError {}

type StereoSample = [i16; 2];

impl Player {
    /// Create a new `Player` with a song loaded from `data`.
    ///
    /// # Panics
    /// - If the file is too short
    ///
    /// # Errors
    ///
    /// - If the file doesn't have the proper magic marker (`PMD`)
    pub fn new(data: &[u8]) -> Result<Self, LoadError> {
        Ok(Self {
            sample_rate: 44_100,
            curr_tick: 0,
            event_cursor: 0,
            song: Song::load(data)?,
        })
    }
    /// Advances playback and renders samples into `buf`.
    pub fn render_next(&mut self, buf: &mut [i16]) {
        for sample in buf.as_chunks_mut().0 {
            self.tick();
            *sample = self.next_sample();
        }
    }

    fn tick(&mut self) {
        let curr_tick = self.curr_tick;
        self.curr_tick = self.curr_tick.wrapping_sub(1);
        if curr_tick == 0 {
            let samples_per_tick = u32::from(self.sample_rate) * self.song.millis_per_tick / 1000;
            self.curr_tick = samples_per_tick;

            for track in &mut self.song.melody_tracks {
                track.tick(self.event_cursor as usize);
            }
            self.song.percussion_track.tick(self.event_cursor as usize);
            self.event_cursor += 1;
            if self.event_cursor >= self.song.end_tick {
                self.event_cursor = self.song.repeat_tick;
            }
        }
    }

    fn next_sample(&mut self) -> StereoSample {
        let mut sample = [0; 2];
        let samp_phase = 22_050. / f32::from(self.sample_rate);
        for track in &mut self.song.melody_tracks {
            track.render(&mut sample, samp_phase);
        }
        self.song.percussion_track.render(&mut sample, samp_phase);
        sample
    }

    /// Returns number of events in the song
    #[must_use]
    pub fn n_events(&self) -> usize {
        // Each track has the same length, we just use the percussion track for simplicity
        self.song.percussion_track.base.events.len()
    }
}
