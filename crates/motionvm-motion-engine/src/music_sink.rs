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
    /// Stops the song that `handle` was started under **without the fade**:
    /// the driver's Stop entry alone, landing where a stop's would, half a
    /// second on. The 16-bit `PLAYSAMPLE` stops a tune this way before it
    /// would play its sample (`STERN.EXE` `15e5:035d`); no 32-bit word asks
    /// for it.
    fn cut(&mut self, handle: i32);
    /// Plays a digital sample: `block` names it, `sample` is the whole block
    /// — tag, header and PCM — as the 16-bit `PLAYSAMPLE` copies it into the
    /// digital driver's buffer and hands it to the driver's play entry
    /// (`STERN.EXE` `15e5:0425`). The sink owns the format. One sample plays
    /// at a time: the word stops the one before, and so does the sink.
    fn sample(&mut self, block: i32, sample: &[u8]);
    /// Starts a 32-bit sample: `handle` is the token [`MusicSink::stop_sample`]
    /// will name it by, `wav` the whole WAV file as the block or the file
    /// holds it, `volume` the sound layer's level, `0x7fff` full scale — the
    /// quarter `STARTSAMPLE` sets or the whole `->STARTSAMPLE` does — and
    /// `loops` the start word's second argument, the layer's loop count: 0
    /// plays the sample once, a positive count that many times more, a
    /// negative one until it is stopped (see `crate::sample`). Samples sound
    /// together: the layer keeps 34 slots and mixes every one that is taken,
    /// and the engine refuses a start only when all of them are.
    fn start_sample(&mut self, handle: i32, wav: &[u8], volume: u16, loops: i32);
    /// Stops the sample started under `handle`, as `STOPSAMPLE` does, and
    /// leaves the others sounding.
    fn stop_sample(&mut self, handle: i32);
    /// Sets the music's volume on the sound layer's scale: `MUSVOLUME`'s
    /// `0x3800` to duck it under speech, `0x7fff` to restore it.
    fn music_volume(&mut self, volume: u16);
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
