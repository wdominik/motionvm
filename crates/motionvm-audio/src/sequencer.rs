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
//! - **The channel comes from the track header**, not from the status byte
//!   (`0x98AEF` and every other send site read `track+0x7B`). The nibble in the
//!   stream is ignored.
//! - **A loop is global.** `FE 15` sends *every* track of the song to its own
//!   `FE 10` marker with the same id (`0x90714`), and a branch is more than a
//!   seek: the engine kills the track's sounding notes, replays the marker's
//!   controller snapshot, and resets the track's tick from the marker.

use motionvm_formats::hmi::{Event, Song};

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

/// One song being played: where each track has got to, and what is still ringing.
pub struct Sequencer {
    song: Song,
    tracks: Vec<TrackState>,
    pending: Vec<Pending>,
    /// Set when a track's `FE 15` asked for a branch; applied to every track.
    branch_to: Option<i16>,
    finished: bool,
    /// The engine's own controller-7 cache, `chan+0x1C` (`0x98C4D`). Every song
    /// sets it on every channel before its first note; 127 is what an
    /// untouched channel would be worth.
    volume: [u8; 16],
    /// The track that was stopped for running away, if one was.
    ///
    /// Recorded rather than reported on the spot because there is nowhere to
    /// report to from the audio thread. [`Sequencer::runaway`] is how the game
    /// side finds out, and it is the same "counted and reported, never silent"
    /// shape the VM uses for its stray reads.
    runaway: Option<usize>,
}

impl Sequencer {
    /// Starts a song at tick zero, with every track at its first event.
    pub fn new(song: Song) -> Self {
        let tracks = song
            .tracks
            .iter()
            .map(|t| TrackState {
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
            branch_to: None,
            finished: false,
            volume: [0x7f; 16],
            runaway: None,
        }
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
            let channel = self.song.tracks[i].channel as u8 & 0x0f;
            let event = timed.event.clone();
            self.tracks[i].next = index + 1;
            self.dispatch(i, channel, &event, out);

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

    fn dispatch(&mut self, i: usize, channel: u8, event: &Event, out: &mut Vec<Message>) {
        match *event {
            Event::NoteOn {
                note,
                velocity,
                duration,
            } => {
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
                    (velocity as u32 * self.volume[9] as u32 / 127) as u8
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
                out.push(Message {
                    channel,
                    kind: Kind::NoteOff { note },
                });
            }
            Event::Control { controller, value } => self.control(channel, controller, value, out),
            Event::Program(p) => out.push(Message {
                channel,
                kind: Kind::Program(p),
            }),
            Event::PitchBend { lsb, msb } => out.push(Message {
                channel,
                kind: Kind::PitchBend { lsb, msb },
            }),
            Event::PolyPressure { note, value } => out.push(Message {
                channel,
                kind: Kind::PolyPressure { note, value },
            }),
            Event::ChannelPressure(v) => out.push(Message {
                channel,
                kind: Kind::ChannelPressure(v),
            }),
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

    /// Controller changes.
    ///
    /// Controller 7 is cached on the way through (`0x98C4D`). The engine's
    /// cache is not the raw value but `value · song · master · channel / 127³`
    /// (`0x98C05`…`0x98C3C`); nothing in the game moves those three, and the
    /// recording pins them down — any value but the maximum would change the
    /// drum levels, and they match to the byte. So the cache is the value.
    ///
    /// The code at `0x98C56` withholds the controller from the device when a
    /// field of the channel record reads 9. What that field is has not been
    /// established, and in the recording the controller plainly does reach the
    /// driver on channel 9: the drums come out attenuated by the channel volume
    /// twice over, which only happens if the driver has it too.
    fn control(&mut self, channel: u8, controller: u8, value: u8, out: &mut Vec<Message>) {
        if controller == 7 {
            self.volume[(channel & 0x0f) as usize] = value;
        }
        match controller {
            // 103, 104, 106 and 107 are the sequencer's own bookkeeping, 105
            // turns the track's voice on and off, 108 is its internal
            // all-notes-off, and 119 is a callback the game never installs.
            // None of them is a message for a device.
            103..=108 | 119 => {}
            _ => out.push(Message {
                channel,
                kind: Kind::Control { controller, value },
            }),
        }
    }

    /// Sends one track to its branch point, the way `0x907BC` does it.
    fn branch(&mut self, i: usize, id: i16, out: &mut Vec<Message>) {
        let track = &self.song.tracks[i];
        let Some(point) = track.branch_points.iter().find(|b| b.id == id) else {
            return;
        };
        let Some(index) = track.events.iter().position(|e| e.at == point.offset) else {
            return;
        };
        let channel = track.channel as u8 & 0x0f;

        // Controller 108: every note this track has sounding is killed. The
        // original walks the pending list and drops the entries whose channel
        // matches (`0x98B56`).
        let mut still = Vec::with_capacity(self.pending.len());
        for p in std::mem::take(&mut self.pending) {
            if p.channel == channel {
                out.push(Message {
                    channel,
                    kind: Kind::NoteOff { note: p.note },
                });
            } else {
                still.push(p);
            }
        }
        self.pending = still;

        // Then the marker's snapshot is replayed. Its keys are the engine's
        // cache slots, not plain controller numbers: 104 is a program change
        // and 105 a pitch bend (`0x99A1B` and `0x99A38`), everything else is
        // the controller of that number.
        if let Event::BranchPoint { snapshot, tick, .. } = &track.events[index].event {
            for pair in snapshot.as_chunks::<2>().0 {
                let (key, value) = (pair[0], pair[1]);
                let kind = match key {
                    104 => Kind::Program(value),
                    105 => Kind::PitchBend { lsb: 0, msb: value },
                    107 => Kind::ChannelPressure(value),
                    _ => Kind::Control {
                        controller: key,
                        value,
                    },
                };
                out.push(Message { channel, kind });
            }
            self.tracks[i].tick = *tick;
        }

        self.tracks[i].next = index + 1;
        self.tracks[i].delta = track.events.get(index + 1).map_or(0, |e| e.delta);
        self.tracks[i].done = false;
    }
}
