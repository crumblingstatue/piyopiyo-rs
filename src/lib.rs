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

pub use crate::{
    player::Player,
    song::{LoadError, Song},
    track::{
        DRUM_SAMPLES, Event, MelodyTrack, N_KEYS, PercussionTrack, PianoKey, Track, piano_keys,
    },
};

mod player;
mod read_cursor;
mod song;
mod track;

/// 16 bit little endian integer sample
pub type Sample = i16;
/// A stereo pair of samples
pub type StereoSample = [Sample; 2];
