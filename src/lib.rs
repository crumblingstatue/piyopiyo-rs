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

use crate::{read_cursor::ReadCursor, track::Track as _};

mod read_cursor;
mod track;

/// PMD music player
pub struct Player {
    sample_rate: u16,
    millis_per_tick: u32,
    repeat_tick: u32,
    end_tick: u32,
    /// The melody tracks of the song
    pub melody_tracks: [MelodyTrack; 3],
    /// The percussion track of the song
    pub percussion_track: PercussionTrack,
    curr_tick: u32,
    /// Index of event to process next
    pub event_cursor: u32,
}

/// Error that can happen when loading a PMD file
#[derive(Debug)]
pub enum LoadError {
    /// Invalid magic (not `PMD`)
    InvalidMagic,
    /// End of file was reached prematurely
    PrematureEof,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::InvalidMagic => f.write_str("Invalid magic (expected PMD)"),
            LoadError::PrematureEof => f.write_str("End of file reached prematurely"),
        }
    }
}

impl std::error::Error for LoadError {}

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
    pub fn new(data: &[u8]) -> Result<Self, LoadError> {
        let mut cur = ReadCursor(data);
        let magic = cur.next_bytes();
        if magic != Some(b"PMD") {
            return Err(LoadError::InvalidMagic);
        }
        cur.skip(5);
        let millis_per_tick = cur.next_u32_le().ok_or(LoadError::PrematureEof)?;
        let repeat_tick = cur.next_u32_le().ok_or(LoadError::PrematureEof)?;
        let end_tick = cur.next_u32_le().ok_or(LoadError::PrematureEof)?;
        let n_events = cur.next_u32_le().ok_or(LoadError::PrematureEof)? as usize;

        let mut melody_tracks = std::array::from_fn(|_| MelodyTrack::default());

        for track in &mut melody_tracks {
            track.read(&mut cur)?;
        }

        let mut percussion_track = PercussionTrack::default();

        percussion_track.base.vol = cur
            .next_u32_le()
            .ok_or(LoadError::PrematureEof)?
            .try_into()
            .unwrap();

        for track in &mut melody_tracks {
            track.base.events = cur.next_n(n_events).into();
        }
        percussion_track.base.events = cur.next_n(n_events).into();
        Ok(Self {
            sample_rate: 44_100,
            millis_per_tick,
            repeat_tick,
            end_tick,
            melody_tracks,
            percussion_track,
            curr_tick: 0,
            event_cursor: 0,
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
            let samples_per_tick = u32::from(self.sample_rate) * self.millis_per_tick / 1000;
            self.curr_tick = samples_per_tick;

            for track in &mut self.melody_tracks {
                track.tick(self.event_cursor as usize);
            }
            self.percussion_track.tick(self.event_cursor as usize);
            self.event_cursor += 1;
            if self.event_cursor >= self.end_tick {
                self.event_cursor = self.repeat_tick;
            }
        }
    }

    fn next_sample(&mut self) -> StereoSample {
        let mut sample = [0; 2];
        let samp_phase = 22_050. / f32::from(self.sample_rate);
        for track in &mut self.melody_tracks {
            track.render(&mut sample, samp_phase);
        }
        self.percussion_track.render(&mut sample, samp_phase);
        sample
    }

    /// Returns number of events in the song
    #[must_use]
    pub fn n_events(&self) -> usize {
        // Each track has the same length, we just use the percussion track for simplicity
        self.percussion_track.base.events.len()
    }
}
