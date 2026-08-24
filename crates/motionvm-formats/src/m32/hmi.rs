//! HMI songs — `HMI-MIDISONG061595`, the game's 26 music blocks.
//!
//! Everything here was read out of the HMI sequencer, which is **statically
//! linked into `ENGINE.EXE`** rather than living in the `.386` drivers: the
//! parser is at `0x8F880`, the per-tick service at `0x8FC82`, the event
//! dispatcher at `0x98983`, and the variable-length reader at `0x994B2`. The
//! addresses in the comments below are that image.
//!
//! # The one thing to get right about time
//!
//! **A delta is `1/song.tick_hz` of a second, not a PPQN tick.** The sequencer
//! installs its timer at the rate in the header (`0x8FE43` reads `+0xD4` and
//! hands it to the timer installer) and decrements each track's pending delta
//! once per timer tick (`0x8FD53`). Every shipped song asks for **120 Hz**.
//!
//! The division at `+0xD2` and the tempo map at `+0xD6` exist, but the
//! sequencer never reads them — the only consumer is `0x9CA6D`, which converts
//! a tick count into bars and beats for display. They are documentation, and
//! taking them for the clock puts the music at the wrong speed.
//!
//! # The stream
//!
//! `VLQ delta`, event, `VLQ delta`, event, … ending at `FF 2F`. The cursor
//! points at the status byte; a byte below `0x80` means running status.
//!
//! | Status | Bytes after it |
//! |---|---|
//! | `8n` note off | 2 — **never appears**; the engine would stall the track |
//! | `9n` note on | 2 **plus a VLQ duration** — there are no note-offs |
//! | `An` poly pressure | 2 |
//! | `Bn` control change | 2 |
//! | `Cn` program | 1 |
//! | `Dn` channel pressure | 1 |
//! | `En` pitch bend | 2 |
//! | `F0` sysex | `u32` length (little-endian, **not** a VLQ) then that many bytes |
//! | `FE` | an HMI extended event, see [`Event`] |
//! | `FF 2F` | end of track — the only meta the engine implements |
//!
//! Three traps, all of them things the code does and a reasonable guess would
//! not:
//!
//! - **Running status is clobbered by `FE` and `FF`.** `0x8FD6C` stores *any*
//!   byte ≥ 0x80 into `track+0x42`, extended events included.
//! - **The channel nibble of the status byte is ignored.** Output goes to the
//!   channel in the track header (`track+0x7B`, used at `0x98AEF` and
//!   everywhere else). `TEST.HMI`'s first track is channel 4 and its first
//!   status byte is `B0`.
//! - **A note-on with velocity 0 is not a note-off.** `0x98A89` scales
//!   velocity only on channel 9 and sends the message either way.

use crate::{Error, Result, reserve, u16le, u32le};

/// A parsed song.
#[derive(Debug, Clone)]
pub struct Song {
    /// `+0xD2` — ticks per quarter note. Musical bookkeeping only.
    pub division: i16,
    /// `+0xD4` — the timer rate in Hz, and so the unit of every delta.
    pub tick_hz: i16,
    /// `+0xE2` — how many notes may sound at once; the engine sizes its
    /// pending-note table from it (`0x8FA12`) and never checks the bound again.
    pub max_notes: u16,
    /// The tracks, in file order.
    pub tracks: Vec<Track>,
}

/// One track: a channel and a list of events at absolute ticks.
#[derive(Debug, Clone)]
pub struct Track {
    /// `+0x7B` — the channel every message of this track is sent on.
    pub channel: u16,
    /// `+0x77` — voice priority, also settable by controller 107.
    pub priority: u16,
    /// The branch-point table at `+0x63`, by id and stream offset.
    pub branch_points: Vec<BranchPoint>,
    /// The events, in stream order, with their absolute ticks worked out.
    pub events: Vec<Timed>,
}

/// A branch target, from the table at `track+0x63`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchPoint {
    /// The id a `Loop` event names to send tracks here.
    pub id: i16,
    /// Offset from the start of the track record.
    pub offset: u32,
}

/// An event, where it sits and when it falls due.
#[derive(Debug, Clone, PartialEq)]
pub struct Timed {
    /// Offset of the event's status byte from the start of the track record.
    /// The branch table counts from there, so a sequencer needs it to turn a
    /// branch target into a position in this list.
    pub at: u32,
    /// The delta that preceded it, in the song's ticks. This is what the
    /// sequencer counts down; `tick` is the running sum, kept for readers.
    pub delta: u32,
    /// Absolute tick from the start of the track.
    pub tick: u32,
    /// What happens at that tick.
    pub event: Event,
}

/// One message of a track's stream.
///
/// The ordinary MIDI statuses plus HMI's own `FE` family, which is where the
/// looping lives. Every variant here was read out of a shipped song or out of
/// the engine's handler for it; the ones no song contains say so.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// `9n note velocity <VLQ duration>` — the note-off is scheduled by the
    /// sequencer, `duration` ticks later (`0x98A12` reads it, `0x8FCE0` ages
    /// it, and the note ends on the tick the counter is already zero).
    NoteOn {
        /// MIDI note number.
        note: u8,
        /// How hard, 0 to 127.
        velocity: u8,
        /// Ticks until the sequencer sends the note-off itself.
        duration: u32,
    },
    /// `8n` — present for completeness; no shipped song contains one, and the
    /// engine's handler returns without advancing the cursor, which stalls the
    /// track for good (`0x98B07`).
    NoteOff {
        /// MIDI note number.
        note: u8,
        /// Release velocity.
        velocity: u8,
    },
    /// `An` — aftertouch on one note.
    PolyPressure {
        /// MIDI note number.
        note: u8,
        /// Pressure.
        value: u8,
    },
    /// `Bn` — a controller change.
    Control {
        /// Controller number.
        controller: u8,
        /// Its new value.
        value: u8,
    },
    /// `Cn` — patch change.
    Program(u8),
    /// `Dn` — aftertouch across the channel.
    ChannelPressure(u8),
    /// `En` — pitch bend, fourteen bits across two bytes.
    PitchBend {
        /// Low seven bits.
        lsb: u8,
        /// High seven bits.
        msb: u8,
    },
    /// `F0` — a system-exclusive block, carried but not interpreted.
    SysEx(Vec<u8>),
    /// `FE 10 <u16 id> <u8 len> <len bytes> <u32 tick>` — a loop target. The
    /// bytes are a controller snapshot the engine replays when it branches
    /// here (`0x99977`), and the `u32` is the tick position the track is
    /// reset to.
    BranchPoint {
        /// The id a `Loop` names to come here.
        id: i16,
        /// The controller snapshot replayed on arrival.
        snapshot: Vec<u8>,
        /// The tick position the track is reset to.
        tick: u32,
    },
    /// `FE 12` / `FE 14 <live> <reset>` — the loop counter. The engine copies
    /// `reset` over `live` **in the stream itself** (`0x991B8`), so a buffer
    /// that has been played once no longer matches the file. `0xFF` means
    /// "forever", and every shipped song carries `fe 14 ff ff`.
    LoopCounter {
        /// The count as it stands, decremented in the buffer itself.
        live: u8,
        /// What it is set back to. `0xFF` means forever.
        reset: u8,
    },
    /// `FE 15 <u16 id> <u32 counter offset>` — the loop the game relies on.
    /// It sends **every** track of the song to its own branch point with that
    /// id (`0x90714`), not just this one.
    Loop {
        /// Which branch point every track jumps to.
        id: i16,
        /// Offset of the counter this loop decrements.
        counter_offset: u32,
    },
    /// `FE 11` / `FE 13` / `FE 16` — conditional and counted branches, gated on
    /// callbacks the game never installs. No shipped song contains one.
    Branch {
        /// `0x11`, `0x13` or `0x16`.
        kind: u8,
        /// The rest of the message, carried unread.
        body: Vec<u8>,
    },
    /// `FF 2F`. The engine reads no length byte; the `00` that follows in every
    /// file is padding nothing looks at.
    EndOfTrack,
}

impl Song {
    /// The eighteen bytes every song starts with.
    pub const MAGIC: &'static [u8] = b"HMI-MIDISONG061595";
    /// The thirteen bytes every track record starts with.
    pub const TRACK_MAGIC: &'static [u8] = b"HMI-MIDITRACK";

    /// Reads a whole song: the header, then every track's event stream with
    /// its deltas summed into absolute ticks.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let magic = data.get(..Self::MAGIC.len()).ok_or(Error::Truncated {
            off: 0,
            need: Self::MAGIC.len(),
            have: data.len(),
        })?;
        if magic != Self::MAGIC {
            let mut found = [0u8; 18];
            found.copy_from_slice(magic);
            return Err(Error::MissingSongMagic { found });
        }
        let division = u16le(data, 0xd2)? as i16;
        let tick_hz = u16le(data, 0xd4)? as i16;
        let max_notes = u16le(data, 0xe2)?;
        let count = u32le(data, 0xe4)? as usize;
        let table = u32le(data, 0xe8)? as usize;

        // Four bytes per entry in the offset table at `table`.
        let mut tracks = reserve(count, data.len(), 4);
        for i in 0..count {
            let at = u32le(data, table + i * 4)? as usize;
            tracks.push(Track::parse(data, at)?);
        }
        Ok(Self {
            division,
            tick_hz,
            max_notes,
            tracks,
        })
    }
}

impl Track {
    fn parse(data: &[u8], at: usize) -> Result<Self> {
        let magic = data
            .get(at..at + Song::TRACK_MAGIC.len())
            .ok_or(Error::Truncated {
                off: at,
                need: Song::TRACK_MAGIC.len(),
                have: data.len(),
            })?;
        if magic != Song::TRACK_MAGIC {
            let mut found = [0u8; 13];
            found.copy_from_slice(magic);
            return Err(Error::MissingTrackMagic { at, found });
        }
        let channel = u16le(data, at + 0x7b)?;
        let priority = u16le(data, at + 0x77)?;
        let branch_points = branch_table(data, at)?;

        // `+0x57` is where the events start, as a track-relative offset that
        // the engine relocates in place (`0x986DE`). The field at `+0x0C` holds
        // 75 in every track of every song and nothing in the image reads it.
        let start = at + u32le(data, at + 0x57)? as usize;
        let events = decode(data, start, at)?;
        Ok(Self {
            channel,
            priority,
            branch_points,
            events,
        })
    }
}

/// `track+0x63` → `<u8 count> { i16 id; u32 offset }*` — or nothing at all.
fn branch_table(data: &[u8], at: usize) -> Result<Vec<BranchPoint>> {
    let rel = u32le(data, at + 0x63)? as usize;
    if rel == 0 {
        return Ok(Vec::new());
    }
    let table = at + rel;
    let count = *data.get(table).ok_or(Error::Truncated {
        off: table,
        need: 1,
        have: data.len(),
    })? as usize;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = table + 1 + i * 6;
        out.push(BranchPoint {
            id: u16le(data, o)? as i16,
            offset: u32le(data, o + 2)?,
        });
    }
    Ok(out)
}

/// The variable-length quantity of `0x994B2`: seven bits a byte, ending at the
/// first byte **without** the top bit — the same shape as standard MIDI.
/// Five bytes is the most that can fit in the `u32` this returns; a sixth would
/// shift the first one out and silently answer something else. No real quantity
/// is anywhere near that long — a tick delta of four bytes is already longer
/// than any song — so a longer run is a stream that has come off its rails, and
/// saying so beats returning a plausible wrong number.
const VLQ_MAX_BYTES: usize = 5;

fn vlq(data: &[u8], at: usize) -> Result<(u32, usize)> {
    let mut value = 0u32;
    let mut used = 0usize;
    loop {
        let b = byte(data, at + used)?;
        used += 1;
        value = (value << 7) | u32::from(b & 0x7f);
        if b & 0x80 == 0 {
            return Ok((value, used));
        }
        if used == VLQ_MAX_BYTES {
            return Err(Error::Corrupt {
                what: "song",
                detail: format!(
                    "a variable-length quantity at {at:#x} runs past {VLQ_MAX_BYTES} bytes"
                ),
            });
        }
    }
}

fn byte(data: &[u8], at: usize) -> Result<u8> {
    data.get(at).copied().ok_or(Error::Truncated {
        off: at,
        need: 1,
        have: data.len(),
    })
}

/// Walks one event stream to its `FF 2F`.
fn decode(data: &[u8], start: usize, base: usize) -> Result<Vec<Timed>> {
    let mut out = Vec::new();
    let mut p = start;
    let mut tick = 0u32;
    let mut status = 0u8;
    loop {
        let (delta, used) = vlq(data, p)?;
        p += used;
        // Saturating rather than wrapping: a tick count that has run away is
        // already meaningless, and a debug build would otherwise panic on data
        // rather than report it. The end-of-track check below is what stops the
        // walk; this only keeps the arithmetic from being the thing that fails.
        tick = tick.saturating_add(delta);
        let at = (p - base) as u32;

        // Running status: the cursor sits on the status byte only when the
        // byte there has the top bit set. The engine keeps one byte and lets
        // `FE` and `FF` overwrite it too (`0x8FD6C`).
        let here = byte(data, p)?;
        if here >= 0x80 {
            status = here;
            p += 1;
        }

        let event = match status {
            0xff => {
                let kind = byte(data, p)?;
                if kind != 0x2f {
                    return Err(Error::UnknownSongEvent {
                        at: p,
                        status,
                        sub: kind,
                    });
                }
                out.push(Timed {
                    at,
                    delta,
                    tick,
                    event: Event::EndOfTrack,
                });
                return Ok(out);
            }
            0xfe => {
                let sub = byte(data, p)?;
                let (event, size) = extended(data, p, sub)?;
                p += size;
                out.push(Timed {
                    at,
                    delta,
                    tick,
                    event,
                });
                continue;
            }
            0xf0 => {
                let len = u32le(data, p)? as usize;
                let body = data.get(p + 4..p + 4 + len).ok_or(Error::Truncated {
                    off: p + 4,
                    need: len,
                    have: data.len(),
                })?;
                p += 4 + len;
                out.push(Timed {
                    at,
                    delta,
                    tick,
                    event: Event::SysEx(body.to_vec()),
                });
                continue;
            }
            _ => {
                let (event, size) = channel_event(data, p, status)?;
                p += size;
                event
            }
        };
        out.push(Timed {
            at,
            delta,
            tick,
            event,
        });
    }
}

/// The `8n`–`En` classes. Returns the event and how many bytes it consumed
/// after the status byte.
fn channel_event(data: &[u8], p: usize, status: u8) -> Result<(Event, usize)> {
    Ok(match status & 0xf0 {
        0x80 => (
            Event::NoteOff {
                note: byte(data, p)?,
                velocity: byte(data, p + 1)?,
            },
            2,
        ),
        0x90 => {
            // The duration is what makes this format its own: it comes after
            // the velocity, as a second variable-length value (`0x98A12`).
            let (duration, used) = vlq(data, p + 2)?;
            (
                Event::NoteOn {
                    note: byte(data, p)?,
                    velocity: byte(data, p + 1)?,
                    duration,
                },
                2 + used,
            )
        }
        0xa0 => (
            Event::PolyPressure {
                note: byte(data, p)?,
                value: byte(data, p + 1)?,
            },
            2,
        ),
        0xb0 => (
            Event::Control {
                controller: byte(data, p)?,
                value: byte(data, p + 1)?,
            },
            2,
        ),
        0xc0 => (Event::Program(byte(data, p)?), 1),
        0xd0 => (Event::ChannelPressure(byte(data, p)?), 1),
        0xe0 => (
            Event::PitchBend {
                lsb: byte(data, p)?,
                msb: byte(data, p + 1)?,
            },
            2,
        ),
        _ => {
            return Err(Error::UnknownSongEvent {
                at: p,
                status,
                sub: 0,
            });
        }
    })
}

/// The `FE` family. Sizes come from the jump table at `0x98967`; only `FE 10`
/// is variable, and its length byte sits at `+4`.
fn extended(data: &[u8], p: usize, sub: u8) -> Result<(Event, usize)> {
    Ok(match sub {
        0x10 => {
            let id = u16le(data, p + 1)? as i16;
            let len = byte(data, p + 3)? as usize;
            let snapshot = data.get(p + 4..p + 4 + len).ok_or(Error::Truncated {
                off: p + 4,
                need: len,
                have: data.len(),
            })?;
            let tick = u32le(data, p + 4 + len)?;
            (
                Event::BranchPoint {
                    id,
                    snapshot: snapshot.to_vec(),
                    tick,
                },
                8 + len,
            )
        }
        0x12 | 0x14 => (
            Event::LoopCounter {
                live: byte(data, p + 1)?,
                reset: byte(data, p + 2)?,
            },
            3,
        ),
        0x15 => (
            Event::Loop {
                id: u16le(data, p + 1)? as i16,
                counter_offset: u32le(data, p + 3)?,
            },
            7,
        ),
        0x11 => (
            Event::Branch {
                kind: sub,
                body: slice(data, p + 1, 6)?,
            },
            7,
        ),
        0x13 => (
            Event::Branch {
                kind: sub,
                body: slice(data, p + 1, 10)?,
            },
            11,
        ),
        0x16 => (
            Event::Branch {
                kind: sub,
                body: slice(data, p + 1, 2)?,
            },
            3,
        ),
        _ => {
            return Err(Error::UnknownSongEvent {
                at: p,
                status: 0xfe,
                sub,
            });
        }
    })
}

fn slice(data: &[u8], at: usize, len: usize) -> Result<Vec<u8>> {
    data.get(at..at + len)
        .map(<[u8]>::to_vec)
        .ok_or(Error::Truncated {
            off: at,
            need: len,
            have: data.len(),
        })
}
