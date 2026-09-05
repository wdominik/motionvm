//! Where the engine's music goes: the family-internal seam between the game
//! runtime and whoever owns the codecs.
//!
//! The engine hands a song over as the whole undecoded block, exactly as the
//! original hands the file to its MIDI layer, and never learns the format —
//! the seam is what lets this crate go without an audio dependency, with the
//! family's front door pairing the two sides.

/// Where a game's music goes.
///
/// A trait rather than a concrete player because its callers want opposite
/// things: the front door hands the commands on to an audio thread, and a
/// test wants to see what was asked for without a sound card in the room. A
/// game with no sink installed plays silently. `Send`, because the game that
/// holds the sink may itself be opened on a worker thread.
pub trait MusicSink: Send {
    /// Starts a song. `handle` is the token the game will later stop it by;
    /// `tune` names the piece the game asked for; `song` is the whole file,
    /// undecoded — the sink owns the codec, and the engine never learns it.
    fn start(&mut self, handle: i32, tune: i32, looping: bool, song: &[u8]);
    /// Stops the song that `handle` was started under.
    fn stop(&mut self, handle: i32);
    /// What the sink has to say about the songs it was handed — a tune that
    /// would not decode, and nothing else so far.
    ///
    /// It goes back to the caller rather than to stderr for two reasons: a
    /// library that prints has decided both where the report goes and when,
    /// and on a windowed build the answer to the first is nowhere. The
    /// engine collects this into [`crate::Engine::diagnostics`].
    ///
    /// The default body has nothing to say, which is right for the sinks a
    /// test installs to watch what was asked for.
    fn diagnostics(&self) -> Vec<String> {
        Vec::new()
    }
}
