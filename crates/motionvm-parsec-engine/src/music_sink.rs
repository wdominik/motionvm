//! Where the sound goes: the seam between the engine, which knows when the
//! game asks its sound drivers for something, and whatever plays it.
//!
//! The calls are the drivers' own entries, in the order the program makes
//! them and when it makes them: the Ad Lib driver's (`MUSADL.DRV`, loaded
//! out of `PSMDRV.DAT` resource 5) `SetSong` with `Play` or `PlayFade`,
//! `FadeOut`, `Stop`; and the digital
//! driver's (`DMABLAST.DRV`, resource 1) install, play, `StopAll`, default
//! volume and a song's digital part. A song crosses as the bytes the
//! program hands the driver, never parsed: the bare `PLX` section of a
//! module of `PSMP2L.DAT`, its assembled module for the digital part, and
//! an effect's `SM8` block. The sink owns the codec and the engine never
//! learns it, so the engine has no audio dependency and a test can see
//! what was asked for without a sound card in the room. `Send`, because a
//! host may step the game on any thread and the sink goes where the game
//! goes.

/// Where the game's sound goes.
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
    /// The drivers taken down with one program and the Ad Lib driver
    /// installed again by the next: the title program ends, and the game's
    /// own installs its copy of the same file. The digital driver goes with
    /// the first program, and the next installs it with
    /// [`MusicSink::install_digital`].
    fn reinstall(&mut self);
    /// The digital driver's install (entry 0) with `effects` mixer
    /// channels: its mixer standing, the Ad Lib driver's callbacks reaching
    /// it.
    fn install_digital(&mut self, effects: u16);
    /// Entry 22: the default volume an effect plays at.
    fn effect_volume(&mut self, volume: u16);
    /// Entry 17: `sample`, the `SM8` block of row `row` of the library's
    /// table A, played on a mixer channel, and then `repeats` times more —
    /// the entry's argument, 0 at every call but one.
    fn play_effect(&mut self, row: u16, sample: &[u8], repeats: u16);
    /// Entry 18: every effect and every digital voice cut, the mixer
    /// stopped.
    fn stop_effects(&mut self);
    /// Entry 21: the digital part of the song about to start — `module`,
    /// module `piece` of the library as the DAC path assembles it — or none.
    fn set_digital(&mut self, module: Option<(u16, &[u8])>);
}
