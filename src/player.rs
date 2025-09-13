use crate::{
    StereoSample,
    song::{LoadError, Song},
    track::Track as _,
};

/// PMD music player
pub struct Player {
    sample_rate: u16,
    /// When it reaches zero, we execute the next event
    wait_timer: u32,
    /// Index of event to process next
    pub event_cursor: u32,
    /// The currently loaded song
    pub song: Song,
}

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
            wait_timer: 0,
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
        if self.wait_timer == 0 {
            let samples_per_tick = u32::from(self.sample_rate) * self.song.event_wait_ms / 1000;
            self.wait_timer = samples_per_tick;

            for track in &mut self.song.melody_tracks {
                track.tick(self.event_cursor as usize);
            }
            self.song.percussion_track.tick(self.event_cursor as usize);
            self.event_cursor += 1;
            if self.event_cursor >= self.song.repeat_range.end {
                self.event_cursor = self.song.repeat_range.start;
            }
        } else {
            self.wait_timer = self.wait_timer.wrapping_sub(1);
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
