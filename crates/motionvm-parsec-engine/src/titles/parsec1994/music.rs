//! The music and the sound effects: the library routines of segment
//! `0590h` that the logic and the visit call, what they hand the two sound
//! drivers, and the effect slots of segment `1458h`.
//!
//! The sound is two drivers out of `PSMDRV.DAT`, installed as the
//! program's init installs them: `MUSADL.DRV` (resource 5, `0x5B7B`, which
//! sets `[20Ch]`) on the OPL, and beside it `DMABLAST.DRV` (resource 1,
//! `0x5BEC`, which sets `[210h]`) with six effect channels at its file's
//! time constant `D0h`, its default volume `50h` (`0x5C15`) and then `3Ch`
//! (`0x4E85`). Both words stand whenever the music library could be read —
//! never by whether a host listens, so the game runs the same with sound
//! and without.
//!
//! A song is loaded by `1197h:000Dh(n)` (`0x1197D`). The loader
//! `13ECh:028Fh(n, 0, 0)` takes, for a card without a DAC, the bare `PLX`
//! section of module `n` out of `PSMP2L.DAT` (`0x1435A`); with a DAC it
//! assembles the whole module (`0x14186`) — the head, the same `PLX`
//! behind it, and the song's samples — and the play routine hands the
//! module's digital part to the digital driver's entry 21 before the FM
//! driver's `SetSong` (`0x6080`, `0x6180`). The far pointer to the block is
//! what the logic keeps and passes back. Here the pointer is a handle —
//! [`SONG_HANDLE`] and the module's number — and the bytes are taken out of
//! the library when a driver is handed them; the logic only ever tests the
//! pointer against null and passes it on.
//!
//! What reaches the drivers goes over a [`MusicSink`], as their own
//! entries: `SetSong` with `Play` or `PlayFade`, `FadeOut`, `Stop` of the
//! FM driver; play (17), `StopAll` (18), the song's digital part (21) and
//! the default volume (22) of the digital one. The routines themselves,
//! with their words in `DGROUP`, are here:
//!
//! | routine | what |
//! |---|---|
//! | `0590h:06A9h(song, loops)` `0x5FA9` | play: `[214h]` := loops, `[216h:218h]` := the song when it loops endlessly, else null; with the switch `[212h]` on, the digital part (with a DAC), `SetSong` and `Play` |
//! | `0590h:07A5h(song, loops, ms)` `0x60A5` | the same with `PlayFade(ms, loops)` (`0x619D`) |
//! | `0590h:08A9h(ms)` `0x61A9` | with the switch on, `FadeOut(ms)`; `[216h:218h]` := null either way (`0x61EC`) |
//! | `0590h:08FAh` `0x61FA` | `Stop`, then entry 21 with nothing (`0x6207`), `[216h:218h]` := null |
//! | `0590h:091Ch(sample, 0)` `0x621C` | with the switch and a DAC, entry 17: the effect played |
//! | `0590h:0948h` `0x6248` | with the switch and a DAC, entry 18: every effect and voice cut |
//!
//! **The effects** are 21 slots of far pointers at `DS:1EF4h`, one per
//! effect, which `0EB9h:1949h(k)` plays from (`0x104D9`). Slot `k` holds the
//! row of the library's table A that the byte at `2140h:0034h + k` names
//! (`0x14640`), fetched one by one (`13ECh:0210h`) and only with a DAC
//! (`0x14634`). Slots 0 to 11 are filled at the start (`1458h:01C1h`,
//! `0x14741`, 11 down to 0) and stay; slots 12 to 20 belong to the room:
//! the room loader fills the ones the slot's mask at `2140h:0000h + 2·slot`
//! names, bit `k − 12` for slot `k`, into slots still empty (`1458h:00F7h`,
//! `0x14677`), and the slot unload empties them again (`1458h:017Ch`,
//! `0x146FC`) after the digital driver's `StopAll` (`0x85DA`) — both
//! skipped between two forest rows whose map keeps bit 2 of its flags
//! (`0x85D4`).
//!
//! **A call made while a screen effect plays waits for it.** The effects
//! of `10B8h` are synchronous in the original, so a music call the program
//! makes after one is made when the effect is over: the stop that follows
//! the transition out of a location (`0x0ED55`) lands a transition later
//! than the fade before it (`0x0ED39`). The effect queue carries such a
//! call behind the effect, as it carries the DAC writes — a sound effect
//! and a `StopAll` too, with what they found when they were called.

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
    /// The segment of the effects' two tables (`mov ax, 2140h`, `0x1463B`,
    /// `0x14692`).
    pub effect_seg: u16,
    /// `2140h:0034h`: the row of table A each effect slot holds, a byte a
    /// slot (`0x14640`).
    pub effect_rows: u16,
    /// `2140h:0000h`: each room slot's mask of effect slots 12 to 20, a
    /// word a slot (`0x14697`).
    pub effect_masks: u16,
}

/// The effect slots at `DS:1EF4h`: as many as table A's bound lets the
/// fetch take (`[1EA6h]`, 21), the count the free loops run to (`mov si,
/// 15h`, `0x146FD`).
pub(crate) const EFFECT_SLOTS: usize = 21;

/// The first of the room's effect slots: the loader and the free run from 20
/// down to it (`cmp ax, 0Ch`, `0x146F5`, `0x1473A`).
const ROOM_EFFECTS: usize = 12;

/// What the drivers are asked, in their own entries.
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
    /// The drivers taken down with one program and installed by the next.
    Reinstall,
    /// The digital driver's entry 21: module `piece`'s digital part for the
    /// song about to start, or none.
    Digital(Option<u16>),
    /// The digital driver's entry 17 with mode 0: row `row` of table A.
    Effect { row: u16 },
    /// The digital driver's entry 18: every effect and voice cut.
    StopEffects,
    /// The digital driver's entry 22: the effects' default volume.
    EffectVolume(u16),
    /// The digital driver installed (entry 0) with this many effect
    /// channels, by a program's init.
    Install { effects: u16 },
}

impl Request {
    /// Whether the request is the Ad Lib driver's (or both drivers', for a
    /// reinstall) rather than the digital driver's alone.
    #[cfg(test)]
    pub(crate) fn is_fm(&self) -> bool {
        matches!(
            self,
            Request::Play { .. }
                | Request::PlayFade { .. }
                | Request::FadeOut(_)
                | Request::Stop
                | Request::Reinstall
        )
    }
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
    /// `0590h:091Ch(sample, 0)` with the gate passed when it was called:
    /// the row of table A the slot held.
    Effect(u16),
    /// `0590h:0948h` with the gate passed when it was called.
    StopEffects,
}

/// The sound side of the runtime: the library, the effect slots,
/// and where the drivers' calls go.
pub(crate) struct Music {
    /// `PSMP2L.DAT`, or why it could not be read.
    library: Result<P2l, String>,
    /// `[210h]`: a digital driver installed.
    pub(crate) digital: bool,
    /// The slots at `DS:1EF4h`: the row of table A each holds, `None` for a
    /// null pointer.
    pub(crate) slots: [Option<u16>; EFFECT_SLOTS],
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
            .field("digital", &self.digital)
            .field("slots", &self.slots)
            .field("sink", &self.sink.is_some())
            .finish_non_exhaustive()
    }
}

impl Music {
    /// The library read from `PSMP2L.DAT`'s bytes, or the reason there are
    /// none.
    pub(crate) fn new(library: Result<P2l, String>, map: MusicMap) -> Self {
        Self {
            digital: library.is_ok(),
            library,
            slots: [None; EFFECT_SLOTS],
            sink: None,
            map,
            #[cfg(test)]
            sent: Vec::new(),
        }
    }

    /// Whether the Ad Lib driver is installed: `[20Ch]` (`0x5B93`), with the
    /// library opened.
    pub(crate) fn card(&self) -> bool {
        self.library.is_ok()
    }

    /// Whether the digital driver is installed: `[210h]` (`0x5C1B`).
    pub(crate) fn dac(&self) -> bool {
        self.digital
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

    /// One request to the drivers, as the library routine makes it: with a
    /// DAC a song's play is preceded by its digital part (`0x6080`,
    /// `0x6180`) and a stop followed by entry 21 with nothing (`0x6207`,
    /// and after the switch's stop `0x12338`). Without a digital driver its
    /// entries are the stubs of a driver that is not installed, and reach
    /// nothing.
    pub(crate) fn send(&mut self, request: Request) {
        let digital = matches!(
            request,
            Request::Digital(_)
                | Request::Effect { .. }
                | Request::StopEffects
                | Request::EffectVolume(_)
                | Request::Install { .. }
        );
        if digital && !self.dac() {
            return;
        }
        match request {
            Request::Play { piece, .. } | Request::PlayFade { piece, .. } if self.dac() => {
                self.deliver(Request::Digital(Some(piece)));
            }
            _ => {}
        }
        self.deliver(request);
        if request == Request::Stop && self.dac() {
            self.deliver(Request::Digital(None));
        }
    }

    /// One request, to the sink: a song's bytes taken out of the library
    /// for it, the bare `PLX` both paths hand the FM driver; a module
    /// assembled as the DAC path does for its digital part; an effect's
    /// `SM8` block.
    fn deliver(&mut self, request: Request) {
        #[cfg(test)]
        self.sent.push(request);
        let Some(sink) = &mut self.sink else { return };
        let library = self.library.as_ref().ok();
        let song = |piece: u16| library.and_then(|p| p.song(usize::from(piece)).ok());
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
            Request::Digital(None) => sink.set_digital(None),
            Request::Digital(Some(piece)) => {
                match library.and_then(|p| p.module(usize::from(piece)).ok()) {
                    Some(bytes) => sink.set_digital(Some((piece, &bytes))),
                    None => sink.set_digital(None),
                }
            }
            Request::Effect { row } => {
                if let Some(bytes) = library.and_then(|p| p.sample(usize::from(row)).ok()) {
                    sink.play_effect(row, &bytes);
                }
            }
            Request::StopEffects => sink.stop_effects(),
            Request::EffectVolume(v) => sink.effect_volume(v),
            Request::Install { effects } => sink.install_digital(effects),
        }
    }

    /// Whether row `row` of table A is one a fetch takes: below the bound
    /// (`0x140E6`) and readable.
    fn fetchable(&self, row: u16) -> bool {
        self.library
            .as_ref()
            .is_ok_and(|p| usize::from(row) < p.bounds[0] && usize::from(row) < p.samples.len())
    }

    /// The digital driver installed by a program's init with `effects`
    /// channels — six by `XP01` (`0x5BD3`), three by `XP00` (XP00 `0x9A2`) —
    /// and entry 22 with `50h` right after it (`0x5C15`; XP00 `0x9E4`).
    pub(crate) fn install(&mut self, effects: u16) {
        self.send(Request::Install { effects });
        self.send(Request::EffectVolume(0x50));
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
            Call::Effect(row) => self.music.send(Request::Effect { row }),
            Call::StopEffects => self.music.send(Request::StopEffects),
        }
    }

    /// `0EB9h:1949h(k)` (`0x104D9`): the effect in slot `k` played through
    /// `0590h:091Ch(slot, 0)`, which plays only with the switch `[212h]` on,
    /// a digital driver `[210h]` and a slot that holds a sample
    /// (`0x621F`–`0x6233`). Every caller names a slot of the 21; past them
    /// the word `[1EF4h + 4k]` is other data, which no call reaches, and is
    /// taken as null here.
    pub(crate) fn effect_1949(&mut self, k: u8) {
        self.events.push(super::Event::Sample(k));
        if self.word(self.music.map.switch) == 0 || !self.music.dac() {
            return;
        }
        let Some(Some(row)) = self.music.slots.get(usize::from(k)).copied() else {
            return;
        };
        self.music_call(Call::Effect(row));
    }

    /// `0590h:0948h` (`0x6248`): the digital driver's `StopAll` with its
    /// wait (argument 1), with the switch on and a digital driver.
    pub(crate) fn effects_stop_0948(&mut self) {
        if self.word(self.music.map.switch) != 0 && self.music.dac() {
            self.music_call(Call::StopEffects);
        }
    }

    /// `1458h:00ADh(k)` (`0x1462D`): the sample of slot `k` fetched, with a
    /// digital driver: the row the table at `2140h:0034h` names for it
    /// (`0x14640`), held to table A's bound (`13ECh:0210h`, `0x140E6`, where
    /// one past it ends the program). The developer's path that reads
    /// `SMP\DS%%.SMP` instead (`[1EDCh]` bit 15, `0x14649`) is not the
    /// shipped game's.
    fn effect_fetch(&self, k: usize) -> Option<u16> {
        if !self.music.dac() {
            return None;
        }
        let m = self.music.map;
        let at = usize::from(m.effect_seg) * 16 + usize::from(m.effect_rows) + k;
        let row = u16::from(*self.image.bytes().get(at)?);
        self.music.fetchable(row).then_some(row)
    }

    /// `1458h:01C1h` (`0x14741`): slots 11 down to 0 fetched, at the start.
    pub(crate) fn effects_preload(&mut self) {
        for k in (0..ROOM_EFFECTS).rev() {
            self.music.slots[k] = self.effect_fetch(k);
        }
    }

    /// `1458h:00F7h(slot)` (`0x14677`): with a digital driver and a room
    /// slot `0 < slot < 26` (`0x14685`, `0x14689`: `cmp ax, 1Ah`) whose
    /// mask is not zero, slots 20 down to 12
    /// fetched where the mask names them and they are still empty.
    fn effects_load(&mut self, slot: u16) {
        if !self.music.dac() || slot == 0 || slot >= 26 {
            return;
        }
        let m = self.music.map;
        let at =
            usize::from(m.effect_seg) * 16 + usize::from(m.effect_masks) + 2 * usize::from(slot);
        let mask = self.image.word(at).unwrap_or(0);
        for k in (ROOM_EFFECTS..EFFECT_SLOTS).rev() {
            if u32::from(mask) & (1 << (k - ROOM_EFFECTS)) != 0 && self.music.slots[k].is_none() {
                self.music.slots[k] = self.effect_fetch(k);
            }
        }
    }

    /// `1458h:017Ch` (`0x146FC`): slots 20 down to 12 freed.
    fn effects_release(&mut self) {
        for slot in &mut self.music.slots[ROOM_EFFECTS..] {
            *slot = None;
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
            let slot = self.room_slot(loc);
            let song = self.slot_song(slot);
            self.set_word(at, song.off);
            self.set_word(at.wrapping_add(2), song.seg);
            // `0x781C`: the slot's effects.
            self.effects_load(slot);
        }
        let kept = Far {
            off: self.word(at),
            seg: self.word(at.wrapping_add(2)),
        };
        let Some(map) = &mut self.bound_map else {
            return;
        };
        map.song = kept;
        let flags = map.flags;
        if flags & 8 == 0 {
            let song = self.slot_song(loc);
            self.set_word(at, song.off);
            self.set_word(at.wrapping_add(2), song.seg);
        }
        // `0x8355`–`0x8361`: the row's own effects, with bit 2 clear.
        if flags & 4 == 0 {
            self.effects_load(loc);
        }
    }

    /// The slot unload's sound (`0766h:0D34h`, `0x85D4`–`0x8626`): with a
    /// room song loaded it is stopped and its block freed (`0x85F3`,
    /// `0x8600`), and the word cleared — or, when the loader is about to
    /// keep the room, two forest rows in a row (`0x83A3`–`0x83FB`), given
    /// back the song the map kept (`0x860C`). Between two forest rows the
    /// kept room's map decides first: with bit 3 of its flags set
    /// (`0x8416`, `0x85ED`) nothing is stopped and the word holds, and the
    /// music plays on — the leaving and the entry skip it there too.
    pub(crate) fn room_music_unload(&mut self) {
        // `0x85D4`–`0x85DF`: unless the kept room's map holds bit 2, the
        // digital driver's `StopAll` and the room's effects freed.
        let keeps_effects =
            self.forest_kept() && self.bound_map.as_ref().is_some_and(|m| m.flags & 4 != 0);
        if !keeps_effects {
            self.effects_stop_0948();
            self.effects_release();
        }
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
    /// both forest rows — two different locations of the table (in *Dunkle
    /// Schatten* 0 to 60, `cmp …, 3Dh`) of the forest's kind — so the loader
    /// will keep the slot.
    pub(crate) fn forest_kept(&self) -> bool {
        let (a, b) = (self.here_word, self.location_word);
        let kind = |n: u16| self.locations.kind(&self.image, usize::from(n));
        let rows = crate::num::word(self.locations.count);
        a != b
            && crate::num::signed(a) >= 0
            && a < rows
            && crate::num::signed(b) >= 0
            && b < rows
            && kind(a) == Some(self.kinds.forest)
            && kind(b) == Some(self.kinds.forest)
    }

    /// The music switch flipped by the options (`0x12304`): off, the
    /// driver's `Stop` and, with a DAC, entry 21 with nothing when an
    /// endless song is kept and there is a card (`0x12321`, `0x12338`), the
    /// song still kept; on, the kept song played again with its count
    /// (`0x1235B`).
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
        let sent = &a.music.sent;
        let play = sent
            .iter()
            .position(|r| matches!(r, Request::Play { .. }))
            .expect("the Zimmer's song");
        assert_eq!(
            sent[play - 1..=play],
            [
                Request::Digital(Some(1)),
                Request::Play {
                    piece: 1,
                    loops: -1
                }
            ]
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
        assert_eq!(a.music.sent, [Request::Stop, Request::Digital(None)]);
        assert_eq!(a.word(m.kept.wrapping_add(2)), SONG_HANDLE);
        a.music.sent.clear();
        a.set_word(m.switch, 1);
        a.music_switched();
        assert_eq!(
            a.music.sent,
            [
                Request::Digital(Some(1)),
                Request::Play {
                    piece: 1,
                    loops: -1
                }
            ]
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
                Request::Digital(Some(10)),
                Request::PlayFade {
                    piece: 10,
                    loops: -1,
                    ms: 2000
                },
                Request::Digital(Some(5)),
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

    /// The slots the Zimmer holds: the twelve of the start (`0x14741`), rows
    /// 0 to 11 as `2140h:0034h` names them, and the Zimmer's mask 3 — slots
    /// 12 and 13 (`0x14677`).
    #[test]
    fn the_zimmer_holds_the_start_s_effects_and_two_of_its_own() {
        let Some(a) = zimmer() else { return };
        let want: Vec<Option<u16>> = (0..EFFECT_SLOTS)
            .map(|k| (k < 14).then(|| u16::try_from(k).unwrap()))
            .collect();
        assert_eq!(a.music.slots.to_vec(), want);
    }

    /// The slot unload frees the room's effects after a `StopAll`
    /// (`0x85DA`, `0x85DF`), and the next room fills its own: the
    /// Wohnzimmer's mask `84h`, slots 14 and 19.
    #[test]
    fn a_room_s_effects_go_with_it() {
        let Some(mut a) = zimmer() else { return };
        a.slot_release();
        assert_eq!(
            a.music.sent,
            [Request::StopEffects, Request::Stop, Request::Digital(None)]
        );
        assert!(a.music.slots[ROOM_EFFECTS..].iter().all(Option::is_none));
        assert!(a.music.slots[..ROOM_EFFECTS].iter().all(Option::is_some));
        a.location_word = 4;
        a.picture_kept = false;
        a.effects_load(4);
        let room: Vec<usize> = (ROOM_EFFECTS..EFFECT_SLOTS)
            .filter(|&k| a.music.slots[k].is_some())
            .collect();
        assert_eq!(room, [14, 19]);
    }

    /// An effect plays only with the switch on and a filled slot
    /// (`0x621F`–`0x6233`); the call is an event either way.
    #[test]
    fn an_effect_needs_the_switch_and_its_slot() {
        let Some(mut a) = zimmer() else { return };
        a.events.clear();
        a.effect_1949(13);
        a.effect_1949(15);
        a.set_word(a.music.map.switch, 0);
        a.effect_1949(13);
        a.effects_stop_0948();
        assert_eq!(a.music.sent, [Request::Effect { row: 13 }]);
        let asked: Vec<_> = a
            .events
            .iter()
            .filter(|e| matches!(e, super::super::Event::Sample(_)))
            .cloned()
            .collect();
        assert_eq!(
            asked,
            [13, 15, 13].map(super::super::Event::Sample).to_vec()
        );
    }

    /// Without a digital driver (`[210h]` clear) nothing is fetched and no
    /// digital entry is reached: the effects are the calls alone, and a song
    /// plays without its digital part — the gates the routines test.
    #[test]
    fn without_a_digital_driver_there_are_no_effects_and_no_digital_part() {
        let Some(mut a) = zimmer() else { return };
        a.music.digital = false;
        a.music.slots = [None; EFFECT_SLOTS];
        a.effects_preload();
        a.effects_load(2);
        assert!(a.music.slots.iter().all(Option::is_none));
        a.music.install(6);
        a.effect_1949(0);
        a.effects_stop_0948();
        a.music_play(a.music.load(1), -1);
        assert_eq!(
            a.music.sent,
            [Request::Play {
                piece: 1,
                loops: -1
            }]
        );
    }
}
