//! The HMI sequencer, rebuilt from the one linked into `ENGINE.EXE`.
//!
//! The original's service routine is `0x8FC82`, installed on a timer at the
//! rate in the song header (`0x8FE43` reads `+0xD4`; every shipped song asks
//! for 120 Hz). One call to [`Sequencer::tick`] is one of those timer ticks,
//! and it does what the original does, in the original's order:
//!
//! 1. **Age the pending notes** (`0x8FCD6`). The format has no note-offs — a
//!    note-on carries its own length — so the ends are held in a queue and
//!    counted down here, before anything else. That order matters: a note that
//!    ends on the same tick a new one starts ends first.
//! 2. Run the song's fade service (`0x8FD34`). Nothing in the game touches it.
//! 3. **Walk the tracks** (`0x8FD3F`). Each track's tick advances, its pending
//!    delta is decremented, and an event whose delta reads **zero before the
//!    decrement** falls due. After dispatching, the next delta is read and the
//!    test repeats, so a delta of zero chains any number of events into the
//!    same tick.
//!
//! The two places where a faithful rebuild differs from the obvious one:
//!
//! - **The channel is handed out, not read.** Every send site reads
//!   `track+0x7B` (`0x98AEF` and the rest), and the nibble in the stream is
//!   ignored. `+0x7B` is the device channel the track got when its
//!   controller 105 asked for one ([`Sequencer`]'s channel records); until
//!   then, and again after its controller 105 gives it back, the track
//!   plays nothing.
//! - **A loop is global.** `FE 15` sends *every* track of the song to its own
//!   `FE 10` marker with the same id (`0x90714`), and a branch is more than a
//!   seek: the engine kills the track's sounding notes, replays the marker's
//!   controller snapshot, and resets the track's tick from the marker.

use crate::num;
use motionvm_motion_formats::m32::hmi::{Event, Song};

/// What the sequencer emits. One MIDI message, on the channel the track owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Message {
    /// The MIDI channel, 0…15, taken from the track rather than the event.
    pub channel: u8,
    /// What happened on it.
    pub kind: Kind,
}

/// The MIDI events this sequencer produces.
///
/// Only the ones the game's songs actually contain. A note-on here always ends
/// itself: HMI carries a duration with the note rather than a matching note-off,
/// which is why [`Kind::NoteOff`] is emitted by the sequencer's own countdown
/// and never decoded from the stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Key down.
    NoteOn {
        /// MIDI note number.
        note: u8,
        /// Velocity, 0…127.
        velocity: u8,
    },
    /// Key up, raised when the note's duration has run out.
    NoteOff {
        /// The note that ends.
        note: u8,
    },
    /// A controller change. 7 is volume and 10 is pan; the driver reads both.
    Control {
        /// Controller number.
        controller: u8,
        /// Its new value.
        value: u8,
    },
    /// Instrument change: an index into the melodic bank.
    Program(u8),
    /// Pitch bend, as its two seven-bit halves.
    PitchBend {
        /// Low seven bits.
        lsb: u8,
        /// High seven bits.
        msb: u8,
    },
    /// Per-note aftertouch. Decoded because it occurs; the driver ignores it.
    PolyPressure {
        /// The note it applies to.
        note: u8,
        /// The pressure value.
        value: u8,
    },
    /// Channel-wide aftertouch. Likewise decoded and ignored.
    ChannelPressure(u8),
}

/// A note waiting to end.
#[derive(Debug, Clone, Copy)]
struct Pending {
    channel: u8,
    note: u8,
    /// Ticks left, counted down once a tick. The original stores the note-on's
    /// duration here and fires when the value it reads is already zero
    /// (`0x8FCE0`), so the end lands one tick later than the bare number.
    left: u32,
}

#[derive(Debug, Clone)]
struct TrackState {
    /// The device channel the track holds — `track+0x7B` once its record
    /// is set at `+0x81` — or `None` while the track waits for one, which
    /// is the flag `0x10` at `+0x20` that every send site tests first.
    channel: Option<u8>,
    /// The track's group (`0x9887C`): the tracks that ask for the same
    /// channel share a record and one set of remembered controllers.
    group: usize,
    /// Index of the next event in the track's decoded list.
    next: usize,
    /// The delta the original keeps in `track+0x53`.
    delta: u32,
    /// `track+0x4F`, reset from a branch marker.
    tick: u32,
    done: bool,
}

/// How many events one track may fire inside a single tick before it is taken
/// to be looping on zero deltas.
///
/// Real music does not come close: the densest tick in the shipped songs fires
/// ten note-ons. The number is a runaway backstop, not a limit anyone should
/// reach.
const RUNAWAY_TRACK: u32 = 10_000;

/// How many channel records the OPL3 has (`0xB781C` indexed by the device
/// id, `0xDF840` = 16), and the device channel behind each
/// (`0xDFA88`: 0…8, then `-1`, then 10…15).
const RECORDS: usize = 16;
/// The record whose device channel is `-1`: it is built taken and locked
/// (`0x8F55D` writes `0x8400`), so nothing allocates it, and the drum
/// channel's tracks point at it without asking (`0x9952C`).
const DRUMS: usize = 9;

/// One of the device's channel records (`0xDFB08`, `0x24` bytes each).
#[derive(Debug, Clone, Copy, Default)]
struct Record {
    /// Flag `0x80` at `+1`.
    taken: bool,
    /// `+4` — how many of the group's playing tracks hold the record; the
    /// release frees it when the count comes to zero (`0x9980B`).
    users: u32,
}

/// One song being played: where each track has got to, and what is still ringing.
#[derive(Debug)]
pub struct Sequencer {
    song: Song,
    tracks: Vec<TrackState>,
    pending: Vec<Pending>,
    /// The device's channel records, all free when a song opens: the stop
    /// gives back every channel the last song held (`0x99B74`).
    records: [Record; RECORDS],
    /// Each group's remembered controllers, one byte per slot of the
    /// song's table (`Song::slots`), `0xFF` for nothing kept. The group's
    /// first track lends its area to the rest (`0x9891F`).
    remembered: Vec<Vec<u8>>,
    /// Set when a track's `FE 15` asked for a branch; applied to every track.
    branch_to: Option<i16>,
    finished: bool,
    /// The device channels' volumes; see [`ChannelVolumes`].
    volume: ChannelVolumes,
    /// The sound layer's master volume, `0xbe9d0` in R78 (`0xbe9d0` set at
    /// `0x7b1bc`): what every controller 7 is scaled by on its way to the
    /// device, `chan+0x1c = value × master / 127` at `0x8371e` (R78), and
    /// what the drum fold reads (`0x98aa8` in R109 reads `+0x1c`). `0x7f`
    /// until `MUSVOLUME` ducks it to `0x38`.
    master: u8,
    /// The track that was stopped for running away, if one was.
    ///
    /// Recorded rather than reported on the spot because there is nowhere to
    /// report to from the audio thread. [`Sequencer::runaway`] is how the game
    /// side finds out, and it is the same "counted and reported, never silent"
    /// shape the VM uses for its stray reads.
    runaway: Option<usize>,
}

/// The volumes in the device's channel records, carried from one song to the
/// next: the records are the device's, not the song's — a track's pointer
/// to its record is worked out of the device's own buffer (`0x99515`) — so
/// a channel keeps what the last song gave it.
///
/// Each record keeps two: the last controller 7 at `+0x14`, and its scaled
/// form at `+0x1C`, which is what goes to the device and what channel 9's
/// notes are folded by. The device's install builds the records zeroed
/// (`0x8F538`, `0x82C50` with a fill of 0) and then writes `+0x14 = 0x7F`
/// (`0x8F575`) but not `+0x1C`. So before any song has set a channel, a
/// note on channel 9 is folded to velocity 0, a note-off to the driver:
/// song 1 opens with a drum note at velocity 1 ahead of its first
/// controller 7, and in a recording of the original it takes no voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelVolumes {
    raw: [u8; 16],
    scaled: [u8; 16],
}

impl Default for ChannelVolumes {
    /// As the device's install leaves them.
    fn default() -> Self {
        Self {
            raw: [0x7f; 16],
            scaled: [0; 16],
        }
    }
}

/// Whether a track that names `designations` plays on `device`, as the song
/// open decides it (`0x9c8a6`): the track goes to the first installed device
/// one of its names matches, where `0xA000` also matches `0xA001` and
/// `0xA008`, and `0xA002` also matches `0xA009`, the OPL3. With one device
/// installed that is a yes or a no.
pub fn plays_on(designations: &[u16], device: u32) -> bool {
    designations.iter().any(|&named| match named {
        0xa000 => matches!(device, 0xa000 | 0xa001 | 0xa008),
        0xa002 => matches!(device, 0xa002 | 0xa009),
        other => u32::from(other) == device,
    })
}

impl Sequencer {
    /// Starts a song at tick zero on `device`, with every track the song
    /// designates for that device at its first event.
    ///
    /// The other tracks are not loaded at all. The original's song open
    /// (`0x9c8a6`) gives each track to a device by the list at `track+0x99`
    /// and drops one that no installed device serves; Dunkle Schatten 2's
    /// opening tune carries a bass and a second guitar for other devices,
    /// and on the OPL3 they are silent, which is what a recording of the
    /// original shows.
    pub fn new(song: Song, device: u32, master: u8) -> Self {
        let song = Song {
            tracks: song
                .tracks
                .into_iter()
                .filter(|t| plays_on(&t.devices, device))
                .collect(),
            ..song
        };
        // The song-start pass (`0x9887C`): tracks asking for the same
        // channel form a group, in track order, the first one's area of
        // remembered values serving them all.
        let mut leaders: Vec<u16> = Vec::new();
        let mut remembered = Vec::new();
        let groups: Vec<usize> = song
            .tracks
            .iter()
            .map(|t| {
                leaders
                    .iter()
                    .position(|&c| c == t.requested)
                    .unwrap_or_else(|| {
                        leaders.push(t.requested);
                        remembered.push(t.remembered.clone());
                        leaders.len() - 1
                    })
            })
            .collect();
        let mut records = [Record::default(); RECORDS];
        records[DRUMS].taken = true;
        let tracks = song
            .tracks
            .iter()
            .zip(groups)
            .map(|(t, group)| TrackState {
                channel: None,
                group,
                next: 0,
                // The track initializer reads the first delta before anything
                // plays (`0x9872B`), which is exactly the first event's.
                delta: t.events.first().map_or(0, |e| e.delta),
                tick: 0,
                done: t.events.is_empty(),
            })
            .collect();
        Self {
            song,
            tracks,
            pending: Vec::new(),
            records,
            remembered,
            branch_to: None,
            finished: false,
            volume: ChannelVolumes::default(),
            master,
            runaway: None,
        }
    }

    /// The channel volumes the device's records hold when this song opens:
    /// what the song before it left there.
    #[must_use]
    pub fn with_channel_volumes(mut self, volume: ChannelVolumes) -> Self {
        self.volume = volume;
        self
    }

    /// The channel volumes as this song leaves them, for the next.
    pub fn channel_volumes(&self) -> ChannelVolumes {
        self.volume
    }

    /// The master volume a sequence starts under: full.
    pub const FULL_VOLUME: u8 = 0x7f;

    /// A channel's volume as it reaches the device: the last controller 7
    /// scaled by the master, `value × master / 127` truncating (`0x8371e`
    /// in R78; the sequence's own volume and the channel's factor at `+0x18`
    /// are both 127 in every song and leave the product alone).
    fn scale(&self, raw: u8) -> u8 {
        num::byte(u32::from(raw) * u32::from(self.master) / 127)
    }

    /// A controller 7 reaching the record of `channel`: the value is kept at
    /// `+0x14` and its scaled form at `+0x1C` (`0x98C44`, `0x98C4D`), and
    /// the scaled form is what goes to the device.
    fn set_volume(&mut self, channel: u8, value: u8) -> u8 {
        let c = usize::from(channel & 0x0f);
        let scaled = self.scale(value);
        self.volume.raw[c] = value;
        self.volume.scaled[c] = scaled;
        scaled
    }

    /// `MUSVOLUME`'s path (`0x70590` → `0x7b1b9` → `0x7afad`): the master
    /// changes, and every channel the song holds is sent a controller 7 at
    /// its new effective level — channel 9 included (`0x7b053` tests only
    /// that the channel is mapped).
    pub fn set_master_volume(&mut self, master: u8, out: &mut Vec<Message>) {
        self.master = master;
        let mut channels: Vec<u8> = self.tracks.iter().filter_map(|t| t.channel).collect();
        channels.sort_unstable();
        channels.dedup();
        for channel in channels {
            out.push(Message {
                channel,
                kind: Kind::Control {
                    controller: 7,
                    value: self.rescale(channel),
                },
            });
        }
    }

    /// The master path's work on one channel: `+0x1C` worked out again from
    /// the kept `+0x14`.
    fn rescale(&mut self, channel: u8) -> u8 {
        let c = usize::from(channel & 0x0f);
        let scaled = self.scale(self.volume.raw[c]);
        self.volume.scaled[c] = scaled;
        scaled
    }

    /// The song's tick rate in Hz — the rate its deltas are counted at.
    pub fn tick_hz(&self) -> i16 {
        self.song.tick_hz
    }

    /// True once every track has reached its end marker and no loop brought
    /// one back. The game's songs all loop, so this stays false for them.
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// The track that was stopped for chaining events without end, if any.
    ///
    /// Answers `None` for every song the game ships. A `Some` means the data
    /// was damaged in a way that would otherwise have spun the audio thread.
    pub fn runaway(&self) -> Option<usize> {
        self.runaway
    }

    /// One timer tick.
    pub fn tick(&mut self, out: &mut Vec<Message>) {
        self.age_pending(out);

        for i in 0..self.tracks.len() {
            self.tracks[i].tick = self.tracks[i].tick.wrapping_add(1);
            self.run_track(i, out);
        }

        // A branch is collective, so it is applied after the walk rather than
        // in the middle of it: the original reaches every track from the one
        // that asked (`0x90714`), and doing it here keeps the tracks that have
        // already run this tick from being visited twice.
        if let Some(id) = self.branch_to.take() {
            for i in 0..self.tracks.len() {
                self.branch(i, id, out);
            }
        }

        self.finished = self.tracks.iter().all(|t| t.done);
    }

    /// Step 1: the note-off queue.
    ///
    /// **The order matters and it is the order the notes were started in.** The
    /// original keeps the sounding notes as a singly linked list off the song
    /// record (`song+0xFC`), walks it from the front, and unlinks a node where
    /// it stands (`0x8FCD6`) — so notes that end on the same tick end in the
    /// order they began. Removing by swapping the last entry into the gap is
    /// cheaper and gives a different order; against a recording of the original
    /// that shows up immediately as three note-offs in reverse.
    fn age_pending(&mut self, out: &mut Vec<Message>) {
        // Unlinked where it stands, like the original's list, and without
        // building a second one: this runs 120 times a second on the audio
        // thread, where an allocation is a thing to avoid.
        self.pending.retain_mut(|p| {
            if p.left == 0 {
                out.push(Message {
                    channel: p.channel,
                    kind: Kind::NoteOff { note: p.note },
                });
                false
            } else {
                p.left -= 1;
                true
            }
        });
    }

    /// What stopping a song does: every sounding note ends.
    ///
    /// `ENDTUNE` reaches the sound layer's stop (`0x858F4`), which reaches the
    /// sequencer's (`0x8FE86`), which calls `0x99B11` — and that walks the
    /// song's channel list and sends each of them **controller 108**, the
    /// sequencer's own all-notes-off. The controller never reaches the device;
    /// what it does is drop that channel's entries from the pending queue, note
    /// off. Since every channel of the song gets one, the whole queue goes.
    ///
    /// Without this a location change would leave the old tune's notes hanging
    /// while the new one starts over the top of them.
    /// The same pass gives back each channel (`0x99B74`), which is why a
    /// song always opens on free records; nothing is left to give back
    /// here, since a sequencer is not reused.
    pub fn all_notes_off(&mut self, out: &mut Vec<Message>) {
        for p in self.pending.drain(..) {
            out.push(Message {
                channel: p.channel,
                kind: Kind::NoteOff { note: p.note },
            });
        }
    }

    /// Step 3 for one track: count the delta down and fire what is due.
    fn run_track(&mut self, i: usize, out: &mut Vec<Message>) {
        let mut guard = 0;
        loop {
            if self.tracks[i].done || self.branch_to.is_some() {
                return;
            }
            // The engine reads the delta, decrements it, and acts when the
            // value it read was zero.
            let due = self.tracks[i].delta == 0;
            self.tracks[i].delta = self.tracks[i].delta.wrapping_sub(1);
            if !due {
                return;
            }

            let index = self.tracks[i].next;
            let Some(timed) = self.song.tracks[i].events.get(index) else {
                self.tracks[i].done = true;
                return;
            };
            let event = timed.event.clone();
            self.tracks[i].next = index + 1;
            self.dispatch(i, &event, out);

            // Whatever the event was, the next delta is the following event's.
            self.tracks[i].delta = self.song.tracks[i]
                .events
                .get(self.tracks[i].next)
                .map_or(0, |e| e.delta);

            guard += 1;
            if guard >= RUNAWAY_TRACK {
                // Stop the track rather than the process. This runs inside the
                // audio callback, where an `assert!` is not a failed test but a
                // dead program — and the cause would be a song with a cycle of
                // zero-delta events, which is damaged data rather than a bug
                // here. The track is marked done, so the rest of the song plays
                // on without it and the silence says which one gave up.
                self.tracks[i].done = true;
                self.runaway = Some(i);
                return;
            }
        }
    }

    /// One event of track `i`. Every send site tests the track's channel
    /// first and sends nothing while it has none (`0x98A22`, `0x98AE7`,
    /// `0x98BFF`, `0x98E58`, `0x98EA9` …); the controllers, program, bend
    /// and pressures are remembered either way.
    fn dispatch(&mut self, i: usize, event: &Event, out: &mut Vec<Message>) {
        let channel = self.tracks[i].channel;
        match *event {
            Event::NoteOn {
                note,
                velocity,
                duration,
            } => {
                // A note of a track without a channel is skipped whole: no
                // message and no entry in the note-off queue (`0x98A25`).
                let Some(channel) = channel else { return };
                // On channel 9 — and only there — the engine folds the
                // channel's volume into the velocity before sending the note
                // (`0x98A89`). The controller still goes out as well, so the
                // drums are attenuated twice: once here and once by the
                // driver's own volume. That is not a reading of the code, it is
                // what the recording of the original shows, and it is worth
                // one number: tune 25's hi-hat, velocity 80 at controller 7 =
                // 105, reaches the chip as attenuation 0x10. Only 80·105/127 =
                // 66 followed by the driver's (105·66)>>7 = 54 gives that.
                let velocity = if channel == 9 {
                    num::byte(u32::from(velocity) * u32::from(self.volume.scaled[9]) / 127)
                } else {
                    velocity
                };
                out.push(Message {
                    channel,
                    kind: Kind::NoteOn { note, velocity },
                });
                self.pending.push(Pending {
                    channel,
                    note,
                    left: duration,
                });
            }
            Event::NoteOff { note, .. } => {
                if let Some(channel) = channel {
                    out.push(Message {
                        channel,
                        kind: Kind::NoteOff { note },
                    });
                }
            }
            Event::Control { controller, value } => self.control(i, controller, value, out),
            Event::Program(p) => {
                self.remember(i, Self::PROGRAM, p);
                self.send(channel, Kind::Program(p), out);
            }
            Event::PitchBend { lsb, msb } => {
                self.remember(i, Self::BEND, msb);
                self.send(channel, Kind::PitchBend { lsb, msb }, out);
            }
            Event::PolyPressure { note, value } => {
                // The first data byte is what is kept (`0x98F68`).
                self.remember(i, Self::POLY, note);
                self.send(channel, Kind::PolyPressure { note, value }, out);
            }
            Event::ChannelPressure(v) => {
                self.remember(i, Self::PRESSURE, v);
                self.send(channel, Kind::ChannelPressure(v), out);
            }
            Event::Loop { id, .. } => {
                // The counter lives in the stream and every shipped song sets
                // it to 0xFF, which the original reads as "forever"
                // (`0x9929D`). Counted loops are left for the day a song uses
                // one.
                self.branch_to = Some(id);
            }
            Event::EndOfTrack => self.tracks[i].done = true,
            // Markers, counters, the branches the game never takes, and sysex
            // are carried by the decoder but say nothing to a device.
            Event::BranchPoint { .. }
            | Event::LoopCounter { .. }
            | Event::Branch { .. }
            | Event::SysEx(_) => {}
        }
    }

    /// The keys the slot table keeps program, pitch bend, poly pressure and
    /// channel pressure under (`song+0x8E`…`+0x91`, read at `0x98ECE`,
    /// `0x98F10`, `0x98F60` and after `0x98F8E`).
    const PROGRAM: u8 = 104;
    const BEND: u8 = 105;
    const POLY: u8 = 106;
    const PRESSURE: u8 = 107;

    /// A message on `channel`, if the track holds one.
    fn send(&self, channel: Option<u8>, kind: Kind, out: &mut Vec<Message>) {
        if let Some(channel) = channel {
            out.push(Message { channel, kind });
        }
    }

    /// Stores `value` in the group's slot for `key`. The original stores at
    /// the slot byte even when it reads `0xFF`, one past any slot a song
    /// has; nothing reads that byte back, so it is not kept here.
    fn remember(&mut self, i: usize, key: u8, value: u8) {
        let Some(&slot) = self.song.slots.get(usize::from(key)) else {
            return;
        };
        if let Some(v) = self.remembered[self.tracks[i].group].get_mut(usize::from(slot)) {
            *v = value;
        }
    }

    /// Controller changes, dispatched at `0x98B0B`.
    ///
    /// Controller 7 is cached on the way through (`0x98C4D`). The engine's
    /// cache is not the raw value but `value · song · master · channel / 127³`
    /// (`0x98C05`…`0x98C3C`); nothing in the game moves the song's or the
    /// channel's factor, so the cache is the value scaled by the master.
    /// `0x98C56` withholds the controller from the device when the
    /// record's device channel reads 9; the OPL3's list has `-1` there
    /// (`0xDFA88`), so on this device it always goes out.
    ///
    /// 105 asks for a channel (any value but 0) or gives it back (0); 108
    /// ends the track's sounding notes (`0x98B56`); 103, 104, 106 and 107
    /// keep the sequencer's books — a mask, a lock, a priority — that only
    /// matter once records run out, which no shipped song comes near; and
    /// 119 is a callback the game never installs. None of them reaches the
    /// device.
    fn control(&mut self, i: usize, controller: u8, value: u8, out: &mut Vec<Message>) {
        let channel = self.tracks[i].channel;
        match controller {
            7 => {
                if let Some(c) = channel {
                    let value = self.set_volume(c, value);
                    out.push(Message {
                        channel: c,
                        kind: Kind::Control { controller, value },
                    });
                }
                self.remember(i, controller, value);
            }
            105 => {
                if value != 0 {
                    self.allocate(i, out);
                } else if channel.is_some() {
                    self.release(i);
                }
            }
            108 => {
                if let Some(c) = channel {
                    self.notes_off(c, out);
                }
            }
            103 | 104 | 106 | 107 | 119 => {}
            _ => {
                self.send(channel, Kind::Control { controller, value }, out);
                self.remember(i, controller, value);
            }
        }
    }

    /// Controller 108's walk (`0x98B56`): every pending note on `channel`
    /// ends now, in the order it was queued.
    fn notes_off(&mut self, channel: u8, out: &mut Vec<Message>) {
        self.pending.retain(|p| {
            if p.channel == channel {
                out.push(Message {
                    channel,
                    kind: Kind::NoteOff { note: p.note },
                });
                false
            } else {
                true
            }
        });
    }

    /// Controller 105 asking for a channel (`0x994EA`).
    ///
    /// A track asking for channel 9 points at the drum record and takes
    /// channel 9, and nothing is sent. Any other takes the first free
    /// record, in order, for its whole group: every track of the group
    /// gets the record's device channel, the record counts the group's
    /// playing tracks as its users, and then the channel is restored
    /// (`0x9981E`): controller 121 goes out on it, and every controller
    /// the group remembers is replayed. A track that already holds a
    /// channel asks for nothing.
    ///
    /// With no free record the original takes one from a group of lower
    /// priority and queues that group to wait (`0x99624`…). The OPL3 has
    /// fifteen records to hand out, and the largest shipped songs (blocks
    /// 12 and 16) ask for exactly fifteen besides the drums, so that path
    /// is not built; a track that finds none stays silent.
    fn allocate(&mut self, i: usize, out: &mut Vec<Message>) {
        if self.song.tracks[i].requested == 9 {
            self.tracks[i].channel = Some(9);
            return;
        }
        if self.tracks[i].channel.is_some() {
            return;
        }
        let Some((r, channel)) = self
            .records
            .iter()
            .position(|r| !r.taken)
            .and_then(|r| Some((r, u8::try_from(r).ok()?)))
        else {
            return;
        };
        let group = self.tracks[i].group;
        let mut users = 0;
        for t in self.tracks.iter_mut().filter(|t| t.group == group) {
            t.channel = Some(channel);
            if !t.done {
                users += 1;
            }
        }
        self.records[r] = Record { taken: true, users };

        out.push(Message {
            channel,
            kind: Kind::Control {
                controller: 121,
                value: 0,
            },
        });
        // The replay walks the controller numbers in order and the slots
        // with them, one slot per number the table keeps (`0x9985D`).
        let slots = self.song.slots;
        let kept = slots.iter().enumerate().filter(|&(_, &s)| s != 0xff);
        for (slot, (key, _)) in kept.enumerate() {
            let Ok(key) = u8::try_from(key) else { break };
            let Some(&value) = self.remembered[group].get(slot) else {
                break;
            };
            if value == 0xff {
                continue;
            }
            let kind = match key {
                Self::PROGRAM => Kind::Program(value),
                Self::BEND => Kind::PitchBend { lsb: 0, msb: value },
                Self::POLY => Kind::PolyPressure {
                    note: value,
                    value: 0,
                },
                Self::PRESSURE => Kind::ChannelPressure(value),
                7 => Kind::Control {
                    controller: 7,
                    value: self.set_volume(channel, value),
                },
                controller => Kind::Control { controller, value },
            };
            out.push(Message { channel, kind });
        }
    }

    /// Controller 105 giving the channel back (`0x997F0`): the track waits
    /// again, and the record is free once its last user has gone. The drum
    /// record is never given back — its device channel reads `-1`
    /// (`0x997FA`). A freed record would go to the first group queued for
    /// one, and none is, since nothing here takes a record away.
    fn release(&mut self, i: usize) {
        let Some(channel) = self.tracks[i].channel else {
            return;
        };
        let r = usize::from(channel);
        if r == DRUMS {
            return;
        }
        self.tracks[i].channel = None;
        let Some(record) = self.records.get_mut(r) else {
            return;
        };
        record.users = record.users.wrapping_sub(1);
        if record.users == 0 {
            record.taken = false;
        }
    }

    /// Sends one track to its branch point, the way `0x907BC` does it, and
    /// brings it back if it had ended (`0x90760`).
    fn branch(&mut self, i: usize, id: i16, out: &mut Vec<Message>) {
        let track = &self.song.tracks[i];
        let Some(point) = track.branch_points.iter().find(|b| b.id == id) else {
            return;
        };
        let Some(index) = track.events.iter().position(|e| e.at == point.offset) else {
            return;
        };
        let snapshot = match &track.events[index].event {
            Event::BranchPoint { snapshot, tick, .. } => {
                self.tracks[i].tick = *tick;
                snapshot.clone()
            }
            _ => Vec::new(),
        };
        self.tracks[i].next = index + 1;
        self.tracks[i].delta = track.events.get(index + 1).map_or(0, |e| e.delta);

        // Controller 108 through the dispatcher: the track's sounding notes
        // end. Then, if it holds a channel, the marker's snapshot is
        // replayed (`0x99977`) — each value remembered as well as sent.
        // Its keys are the slot table's: 104 a program change, 105 a pitch
        // bend, 107 channel pressure (`0x99A11`, `0x99A2C`, `0x99A6C`), 7
        // the channel volume, anything else the controller of that number.
        if let Some(channel) = self.tracks[i].channel {
            self.notes_off(channel, out);
            for pair in snapshot.as_chunks::<2>().0 {
                let (key, value) = (pair[0], pair[1]);
                self.remember(i, key, value);
                let kind = match key {
                    Self::PROGRAM => Kind::Program(value),
                    Self::BEND => Kind::PitchBend { lsb: 0, msb: value },
                    Self::PRESSURE => Kind::ChannelPressure(value),
                    7 => Kind::Control {
                        controller: 7,
                        value: self.set_volume(channel, value),
                    },
                    _ => Kind::Control {
                        controller: key,
                        value,
                    },
                };
                out.push(Message { channel, kind });
            }
        }

        // A track that had ended goes back into the walk and asks for a
        // channel again, as if its stream said controller 105 = 1.
        if self.tracks[i].done {
            self.tracks[i].done = false;
            self.allocate(i, out);
        }
    }
}

#[cfg(test)]
mod device_tests {
    use super::plays_on;

    /// The matching `0x9c8a6` does, with its two families: a track for
    /// `0xA000` plays on `0xA001` and `0xA008` too, one for `0xA002` on the
    /// OPL3, and any other name only on the device it names.
    #[test]
    fn a_track_plays_on_the_devices_its_list_names() {
        assert!(plays_on(&[0xa000, 0xa004, 0xa00a, 0xa002], 0xa009));
        assert!(!plays_on(&[0xa000, 0xa004, 0xa00a], 0xa009), "no OPL3 name");
        assert!(plays_on(&[0xa000], 0xa001));
        assert!(plays_on(&[0xa000], 0xa008));
        assert!(!plays_on(&[0xa000], 0xa009));
        assert!(plays_on(&[0xa00a], 0xa00a));
        assert!(!plays_on(&[], 0xa009), "a track for nothing plays nowhere");
    }
}
