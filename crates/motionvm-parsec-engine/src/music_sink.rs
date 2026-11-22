//! Where the music goes: the seam between the engine, which knows when the
//! game asks its Ad Lib driver for something, and whatever plays it.
//!
//! The calls are the driver's own entries (`MUSADL.DRV`, loaded out of
//! `PSMDRV.DAT` resource 5): `SetSong` with `Play` or `PlayFade`,
//! `FadeOut`, `Stop` — in the order the program makes them and when it
//! makes them. A song crosses as the bytes the program hands the driver,
//! never parsed: the bare `PLX` section of a module of `PSMP2L.DAT`. The
//! sink owns the codec and the engine never learns it, so the engine has no
//! audio dependency and a test can see what was asked for without a sound
//! card in the room. `Send`, because a host may step the game on any
//! thread and the sink goes where the game goes.

/// Where the game's music goes.
pub trait MusicSink: Send {
    /// `SetSong` then `Play(loops)`: `song` from its first row at full
    /// volume, `loops` times more, `-1` for endlessly.
    fn play(&mut self, song: &[u8], loops: i16);
    /// `SetSong` then `PlayFade(ms, loops)`: the same from volume 0,
    /// rising to full over `ms` milliseconds.
    fn play_fade(&mut self, song: &[u8], loops: i16, ms: u16);
    /// `FadeOut(ms)`: the volume falling to 0 over `ms` milliseconds while
    /// the song plays on.
    fn fade_out(&mut self, ms: u16);
    /// `Stop`: every note keyed off, and what still sounds left to die
    /// away.
    fn stop(&mut self);
    /// The driver taken down with one program and installed again by the
    /// next: the title program ends, and the game's own installs its copy
    /// of the same file.
    fn reinstall(&mut self);
}
