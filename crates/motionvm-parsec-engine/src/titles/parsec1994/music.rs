//! The music: the library routines of segment `0590h` that the logic and
//! the visit call, and what they hand the Ad Lib driver.
//!
//! The program installs `MUSADL.DRV` (`PSMDRV.DAT` resource 5) when the
//! sound setup's `PSMCFG4.DAT` names an Ad Lib card (`0x5B7B`, which sets
//! `[20Ch]`), and a digital driver beside it when the file names a DAC
//! (`0x5BEC`, `[210h]`). This engine runs as the setup's Ad Lib answer
//! with no DAC: there is a card when the music library could be read, and
//! never a digital channel — so every call below takes its Ad Lib path, the
//! digital driver's entries (`DMA` 17, 21) are the stubs of a driver that
//! is not installed, and the sound effects (`0590h:091Ch`, which needs
//! `[210h]`) play nothing, as they play nothing on such a machine.
//!
//! A song is loaded by `1197h:000Dh(n)` (`0x1197D`): the loader
//! `13ECh:028Fh(n, 0, 0)` takes, for a card without a DAC, the bare `PLX`
//! section of module `n` out of `PSMP2L.DAT` (`0x1435A`), and the far
//! pointer to that block is what the logic keeps and passes back. Here the
//! pointer is a handle — [`SONG_HANDLE`] and the module's number — and the
//! bytes are taken out of the library when the driver is handed them; the
//! logic only ever tests the pointer against null and passes it on.
//!
//! What reaches the driver goes over a [`MusicSink`], as the driver's own
//! entries: `SetSong` with `Play` or `PlayFade`, `FadeOut`, `Stop`. The
//! routines themselves, with their words in `DGROUP`, are here:
//!
//! | routine | what |
//! |---|---|
//! | `0590h:06A9h(song, loops)` `0x5FA9` | play: `[214h]` := loops, `[216h:218h]` := the song when it loops endlessly, else null; with the switch `[212h]` on, `SetSong` and `Play` |
//! | `0590h:07A5h(song, loops, ms)` `0x60A5` | the same with `PlayFade(ms, loops)` (`0x619D`) |
//! | `0590h:08A9h(ms)` `0x61A9` | with the switch on, `FadeOut(ms)`; `[216h:218h]` := null either way (`0x61EC`) |
//! | `0590h:08FAh` `0x61FA` | `Stop`, `[216h:218h]` := null |
//!
//! **A call made while a screen effect plays waits for it.** The effects
//! of `10B8h` are synchronous in the original, so a music call the program
//! makes after one is made when the effect is over: the stop that follows
//! the transition out of a location (`0x0ED55`) lands a transition later
//! than the fade before it (`0x0ED39`). The effect queue carries such a
//! call behind the effect, as it carries the DAC writes.

use super::Adventure;
use super::api::{Api, Far};
use crate::music_sink::MusicSink;
use motionvm_parsec_formats::P2l;

/// The segment word of a song handle; the offset word is the module's
/// number. Like the object and person handles, a value no far pointer the
/// program makes can have, which the logic only passes on.
pub(crate) const SONG_HANDLE: u16 = 0xF0B3;

/// Where one build keeps the music's words in `DGROUP`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MusicMap {
    /// `[212h]`: the music switch, 1 in the image, flipped by the options.
    pub switch: u16,
    /// `[214h]`: the loop count the last play was asked for.
    pub loops: u16,
    /// `[216h]`, `[218h]`: the song playing endlessly, kept for the switch
    /// to start again.
    pub kept: u16,
    /// `[285Dh]`, `[285Fh]`: the song the room loader loaded for the
    /// location (`0x7814`).
    pub room_song: u16,
    /// `DS:13E8h`: the song of each room slot below 30, `-1` for none
    /// (`1197h:0035h`, `0x119A5`).
    pub slot_songs: u16,
}

impl MusicMap {
    /// Dunkle Schatten's.
    pub(crate) const DS1: MusicMap = MusicMap {
        switch: 0x0212,
        loops: 0x0214,
        kept: 0x0216,
        room_song: 0x285D,
        slot_songs: 0x13E8,
    };
}

/// What the driver is asked, in its own entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Request {
    /// `SetSong` then `Play(loops)`: module `piece`'s song.
    Play {
        /// The module's number in the library.
        piece: u16,
        /// The loop count, `-1` endless.
        loops: i16,
    },
    /// `SetSong` then `PlayFade(ms, loops)`.
    PlayFade {
        /// The module's number in the library.
        piece: u16,
        /// The loop count, `-1` endless.
        loops: i16,
        /// The fade's length in milliseconds.
        ms: u16,
    },
    /// `FadeOut(ms)`.
    FadeOut(u16),
    /// `Stop`.
    Stop,
    /// The driver taken down with one program and installed by the next.
    Reinstall,
}

/// One library routine called, as the effect queue holds it until the
/// effect before it is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Call {
    /// `0590h:06A9h(song, loops)`.
    Play { song: Far, loops: i16 },
    /// `0590h:07A5h(song, loops, ms)`.
    PlayFade { song: Far, loops: i16, ms: u16 },
    /// `0590h:08A9h(ms)`.
    Fade(i16),
    /// `0590h:08FAh`.
    Stop,
}

/// The music side of the runtime: the library, and where the driver's
/// calls go.
pub(crate) struct Music {
    /// `PSMP2L.DAT`, or why it could not be read.
    library: Result<P2l, String>,
    /// Where the driver's calls go, once the front door has given one.
    sink: Option<Box<dyn MusicSink>>,
    /// The build's words.
    pub(crate) map: MusicMap,
    /// What went to the driver, for the tests.
    #[cfg(test)]
    pub(crate) sent: Vec<Request>,
}

impl std::fmt::Debug for Music {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Music")
            .field("library", &self.library.is_ok())
            .field("sink", &self.sink.is_some())
            .finish_non_exhaustive()
    }
}

impl Music {
    /// The library read from `PSMP2L.DAT`'s bytes, or the reason there are
    /// none.
    pub(crate) fn new(library: Result<P2l, String>, map: MusicMap) -> Self {
        Self {
            library,
            sink: None,
            map,
            #[cfg(test)]
            sent: Vec::new(),
        }
    }

    /// Whether there is a card: `[20Ch]`, which the program sets when the
    /// sound setup names one and the library opened.
    pub(crate) fn card(&self) -> bool {
        self.library.is_ok()
    }

    /// Why there is no music, when there is none.
    pub(crate) fn missing(&self) -> Option<&str> {
        self.library.as_ref().err().map(String::as_str)
    }

    /// Where the driver's calls go from now on.
    pub(crate) fn set_sink(&mut self, sink: Box<dyn MusicSink>) {
        self.sink = Some(sink);
    }

    /// Module `n`'s handle, as `1197h:000Dh(n)` answers it: null without
    /// a card (`0x1199F`), and null for a number the library does not
    /// hold, where the original ends the program with "P2L: Ungültiger
    /// Index!" (`0x14376`).
    pub(crate) fn load(&self, n: u16) -> Far {
        match &self.library {
            Ok(p) if usize::from(n) < p.bounds[2] && usize::from(n) < p.modules.len() => Far {
                off: n,
                seg: SONG_HANDLE,
            },
            _ => Far::NULL,
        }
    }

    /// One request to the driver: a song's bytes taken out of the library
    /// for it, the bare `PLX` the Ad Lib path hands over.
    pub(crate) fn send(&mut self, request: Request) {
        #[cfg(test)]
        self.sent.push(request);
        let Some(sink) = &mut self.sink else { return };
        let song = |piece: u16| {
            self.library
                .as_ref()
                .ok()
                .and_then(|p| p.song(usize::from(piece)).ok())
        };
        match request {
            Request::Play { piece, loops } => {
                if let Some(bytes) = song(piece) {
                    sink.play(&bytes, loops);
                }
            }
            Request::PlayFade { piece, loops, ms } => {
                if let Some(bytes) = song(piece) {
                    sink.play_fade(&bytes, loops, ms);
                }
            }
            Request::FadeOut(ms) => sink.fade_out(ms),
            Request::Stop => sink.stop(),
            Request::Reinstall => sink.reinstall(),
        }
    }
}

/// The module a handle names, if it is one.
fn piece(song: Far) -> Option<u16> {
    (song.seg == SONG_HANDLE).then_some(song.off)
}

impl Adventure {
    /// A library routine called: at once, or behind the screen effect
    /// that is playing.
    pub(crate) fn music_call(&mut self, call: Call) {
        if self.effects.busy() {
            self.effects.music(call);
        } else {
            self.music_run(call);
        }
    }

    /// A library routine, run.
    pub(crate) fn music_run(&mut self, call: Call) {
        match call {
            Call::Play { song, loops } => self.music_play_06a9(song, loops, None),
            Call::PlayFade { song, loops, ms } => self.music_play_06a9(song, loops, Some(ms)),
            Call::Fade(ms) => self.music_fade_08a9(ms),
            Call::Stop => self.music_stop_08fa(),
        }
    }

    /// `0590h:06A9h` and its twin `07A5h` (`0x5FA9`, `0x60A5`): without a
    /// card or a song nothing (`0x5FB1`–`0x5FC3`); the count and the
    /// endless song kept (`0x5FC8`–`0x5FE0`); then, with the switch on, the
    /// song's tag tested — a bare `PLX`, which is all the Ad Lib loader
    /// gives out (`0x5FF9`) — and `SetSong` with `Play` (`0x609C`) or
    /// `PlayFade` (`0x619D`).
    fn music_play_06a9(&mut self, song: Far, loops: i16, fade: Option<u16>) {
        if !self.music.card() || song == Far::NULL {
            return;
        }
        let m = self.music.map;
        self.set_word(m.loops, crate::num::unsigned(loops));
        let kept = if loops == -1 { song } else { Far::NULL };
        self.set_word(m.kept, kept.off);
        self.set_word(m.kept.wrapping_add(2), kept.seg);
        if self.word(m.switch) == 0 {
            return;
        }
        let Some(piece) = piece(song) else { return };
        self.music.send(match fade {
            None => Request::Play { piece, loops },
            Some(ms) => Request::PlayFade { piece, loops, ms },
        });
    }

    /// `0590h:08A9h(ms)` (`0x61A9`): with the switch on and a card, the
    /// fade-out begun and the routine back at once; the endless song
    /// forgotten whatever the switch (`0x61EC`).
    ///
    /// A negative length asks the routine to wait for the fade and stop
    /// the song itself (`0x61CA`–`0x61E9`). Nothing in Dunkle Schatten
    /// passes one, and the wait is not built: such a call fades and is
    /// recorded as an event.
    fn music_fade_08a9(&mut self, ms: i16) {
        let m = self.music.map;
        if self.word(m.switch) != 0 && self.music.card() {
            if ms < 0 {
                self.events.push(super::Event::Unbuilt(
                    "0590h:08A9h",
                    u32::from(crate::num::unsigned(ms)),
                ));
            }
            self.music.send(Request::FadeOut(ms.unsigned_abs()));
        }
        self.set_word(m.kept, 0);
        self.set_word(m.kept.wrapping_add(2), 0);
    }

    /// `0590h:08FAh` (`0x61FA`): `Stop`, whatever the switch, and the
    /// endless song forgotten.
    fn music_stop_08fa(&mut self) {
        self.music.send(Request::Stop);
        let m = self.music.map;
        self.set_word(m.kept, 0);
        self.set_word(m.kept.wrapping_add(2), 0);
    }

    /// `1197h:0035h(slot)` (`0x119A5`): a room slot's song — the table's
    /// entry for a slot below 30 that is not negative — loaded.
    fn slot_song(&self, slot: u16) -> Far {
        if slot >= 30 {
            return Far::NULL;
        }
        let n = self.word(self.music.map.slot_songs.wrapping_add(slot * 2));
        if crate::num::signed(n) < 0 {
            return Far::NULL;
        }
        self.music.load(n)
    }

    /// The room loader's music (`0766h:010Ah`, `0x77DA`–`0x7818`): the
    /// song of the slot the loader takes ([`Adventure::room_slot`]) goes
    /// into `[285Dh]` unless the loader keeps the last room — two forest
    /// rows in a row, `14DDh:08F1h` answering a kind (`0x7802`), where
    /// nothing is loaded and the word holds what it held. For any other
    /// row the loader drops that answer (`0x7800`). The program's unload
    /// and loader read the same two words, so the room is kept for both
    /// or for neither; here the loader goes by what the unload kept
    /// ([`Adventure::picture_kept`]).
    ///
    /// A forest row's map keeps the word as the binding finds it
    /// (`0x7C50`), and with bit 3 of its flags clear the row's own song
    /// takes its place (`0x8339`–`0x834F`). Every map Dunkle Schatten
    /// ships has the bit set: the forest plays the picture's song.
    pub(crate) fn room_music_load(&mut self) {
        let loc = self.location_word;
        let at = self.music.map.room_song;
        if !self.picture_kept {
            let song = self.slot_song(self.room_slot(loc));
            self.set_word(at, song.off);
            self.set_word(at.wrapping_add(2), song.seg);
        }
        let kept = Far {
            off: self.word(at),
            seg: self.word(at.wrapping_add(2)),
        };
        let Some(map) = &mut self.bound_map else {
            return;
        };
        map.song = kept;
        if map.flags & 8 == 0 {
            let song = self.slot_song(loc);
            self.set_word(at, song.off);
            self.set_word(at.wrapping_add(2), song.seg);
        }
    }

    /// The slot unload's music (`0766h:0D34h`, `0x85E4`–`0x8626`): with a
    /// room song loaded it is stopped and its block freed (`0x85F3`,
    /// `0x8600`), and the word cleared — or, when the loader is about to
    /// keep the room, two forest rows in a row (`0x83A3`–`0x83FB`), given
    /// back the song the map kept (`0x860C`). Between two forest rows the
    /// kept room's map decides first: with bit 3 of its flags set
    /// (`0x8416`, `0x85ED`) nothing is stopped and the word holds, and the
    /// music plays on — the leaving and the entry skip it there too.
    pub(crate) fn room_music_unload(&mut self) {
        let kept = self
            .forest_kept()
            .then_some(self.bound_map.as_ref())
            .flatten();
        if kept.is_some_and(|m| m.flags & 8 != 0) {
            return;
        }
        let back = kept.map_or(Far::NULL, |m| m.song);
        let at = self.music.map.room_song;
        if self.word(at) | self.word(at.wrapping_add(2)) != 0 {
            self.music_call(Call::Stop);
        }
        self.set_word(at, back.off);
        self.set_word(at.wrapping_add(2), back.seg);
    }

    /// `0x8394`–`0x83FB`: whether the place left and the place ahead are
    /// both forest rows — two different locations in 0..=60 whose kind is
    /// 3 — so the loader will keep the slot.
    pub(crate) fn forest_kept(&self) -> bool {
        let (a, b) = (self.here_word, self.location_word);
        let kind = |n: u16| self.locations.kind(&self.image, usize::from(n));
        a != b
            && crate::num::signed(a) >= 0
            && a < 0x3D
            && crate::num::signed(b) >= 0
            && b < 0x3D
            && kind(a) == Some(3)
            && kind(b) == Some(3)
    }

    /// The music switch flipped by the options (`0x12304`): off, the
    /// driver's `Stop` when an endless song is kept and there is a card
    /// (`0x12321`), the song still kept; on, the kept song played again
    /// with its count (`0x1235B`).
    pub(crate) fn music_switched(&mut self) {
        let m = self.music.map;
        let kept = Far {
            off: self.word(m.kept),
            seg: self.word(m.kept.wrapping_add(2)),
        };
        if self.word(m.switch) == 0 {
            if kept != Far::NULL && self.music.card() {
                self.music.send(Request::Stop);
            }
        } else if kept != Far::NULL {
            let loops = crate::num::signed(self.word(m.loops));
            self.music_run(Call::Play { song: kept, loops });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh game in the Zimmer, its entry effect played and with it the
    /// room's song, and what reached the driver so far cleared.
    fn zimmer() -> Option<Adventure> {
        let mut a = crate::titles::ds1::adventure_for_test()?;
        while a.effects.busy() {
            a.interrupt();
            a.effects_tick();
        }
        assert_eq!(
            a.music.sent.last(),
            Some(&Request::Play {
                piece: 1,
                loops: -1
            })
        );
        a.music.sent.clear();
        Some(a)
    }

    /// The options' switch off stops the kept song and keeps it; on plays
    /// it again with its count (`0x12304`–`0x1235B`).
    #[test]
    fn the_switch_stops_the_kept_song_and_starts_it_again() {
        let Some(mut a) = zimmer() else { return };
        let m = a.music.map;
        assert_eq!(a.word(m.kept), 1, "the Zimmer's song is kept");
        a.set_word(m.switch, 0);
        a.music_switched();
        assert_eq!(a.music.sent, [Request::Stop]);
        assert_eq!(a.word(m.kept.wrapping_add(2)), SONG_HANDLE);
        a.music.sent.clear();
        a.set_word(m.switch, 1);
        a.music_switched();
        assert_eq!(
            a.music.sent,
            [Request::Play {
                piece: 1,
                loops: -1
            }]
        );
    }

    /// With the switch off a play reaches no driver but is kept, so the
    /// switch turned on starts it (`0x5FE3`); a fade-out forgets it
    /// whatever the switch (`0x61EC`).
    #[test]
    fn a_play_with_the_switch_off_is_only_kept() {
        let Some(mut a) = zimmer() else { return };
        let m = a.music.map;
        a.set_word(m.switch, 0);
        let song = a.music.load(11);
        a.music_play(song, -1);
        assert!(a.music.sent.is_empty());
        assert_eq!((a.word(m.kept), a.word(m.loops)), (11, 0xFFFF));
        a.music_fade(0x07D0);
        assert!(a.music.sent.is_empty());
        assert_eq!(a.word(m.kept), 0);
    }

    /// `0590h:07A5h`, as Jörg's event 13 calls it (`0x1AB9`): the song
    /// faded in over 2000 ms; a count other than endless is not kept.
    #[test]
    fn a_fade_in_goes_to_the_driver_as_one() {
        let Some(mut a) = zimmer() else { return };
        let song = a.music.load(10);
        a.music_play_fade(song, -1, 0x07D0);
        a.music_play(a.music.load(5), 0);
        assert_eq!(
            a.music.sent,
            [
                Request::PlayFade {
                    piece: 10,
                    loops: -1,
                    ms: 2000
                },
                Request::Play { piece: 5, loops: 0 }
            ]
        );
        assert_eq!(a.word(a.music.map.kept), 0, "a single pass is not kept");
    }

    /// The loader answers null for a module the library does not hold.
    #[test]
    fn a_module_past_the_library_loads_nothing() {
        let Some(a) = zimmer() else { return };
        assert_eq!(a.music.load(16), Far::NULL);
        assert_ne!(a.music.load(15), Far::NULL);
    }
}
