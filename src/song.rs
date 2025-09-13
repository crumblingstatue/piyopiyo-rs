use crate::{MelodyTrack, PercussionTrack, read_cursor::ReadCursor};

/// A Piyo Piyo song
pub struct Song {
    pub millis_per_tick: u32,
    pub repeat_tick: u32,
    pub end_tick: u32,
    /// The melody tracks of the song
    pub melody_tracks: [MelodyTrack; 3],
    /// The percussion track of the song
    pub percussion_track: PercussionTrack,
}

impl Song {
    /// Load a PMD music file
    ///
    /// # Panics
    /// - If the file is too short
    ///
    /// # Errors
    ///
    /// - If the file doesn't have the proper magic marker (`PMD`)
    pub fn load(data: &[u8]) -> Result<Self, LoadError> {
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
            millis_per_tick,
            repeat_tick,
            end_tick,
            melody_tracks,
            percussion_track,
        })
    }
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
