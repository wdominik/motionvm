//! Where the game's music goes, and what it has been told.
//!
//! The engine owns no codec: a song leaves it as the undecoded block the
//! original hands its MIDI layer, over the [`crate::MusicSink`] the family's
//! front door installs. What is kept here is only what the *script* can ask
//! about — the handle a tune was started under, and whether one is playing,
//! which is what the 16-bit `ENDTUNE` reads before it decides to wait. A
//! sample goes the same way, whole, and nothing of it is kept here: no word
//! of the games asks after one.

use crate::MusicSink;

/// The music, from the script's side of the seam.
pub(crate) struct Sound {
    pub(crate) sink: Option<Box<dyn MusicSink>>,

    /// Handles are ours, counted from 1, the way descriptor handles are. The
    /// original answers with its SOS sequence handle; nothing in the game does
    /// anything with the number except hand it back to `ENDTUNE`.
    pub(crate) next_handle: i32,

    /// Whether a tune has been started and not yet ended — the 16-bit stop
    /// routine's own flag (`ENVIRO.EXE` `ds:18f4`, `LL.EXE` `ds:13dc`), set
    /// when a song starts and cleared by `ENDTUNE`, which does nothing at
    /// all — no fade, no wait — while it is clear; the 16-bit `PLAYSAMPLE`
    /// tests and clears the same flag before it would play a sample. Whether the driver clears
    /// it when a non-looping song plays out is unread; every stop the games
    /// ask for comes while a song is still playing, and Victor Loomes'
    /// jingle, played once, still gets its fade.
    pub(crate) playing: bool,
}

impl Default for Sound {
    /// No sink, nothing playing, and the first handle **1** — zero is what a
    /// script reads as "no tune".
    fn default() -> Self {
        Self {
            sink: None,
            next_handle: 1,
            playing: false,
        }
    }
}
