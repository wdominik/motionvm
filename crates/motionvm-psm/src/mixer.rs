//! The software mixer of `DMABLAST.DRV`: effect channels and a song's
//! digital voices summed into one 8-bit DMA buffer.
//!
//! File offsets in this module are `DMABLAST.DRV`'s (its code-segment
//! offsets), and the bracketed ones name the driver's own cells.
//!
//! **Channels.** Installed with `n` channels (entry 0, clamped to 1–8 at
//! `0x0e5f`–`0x0e6f`), the driver keeps one table row per channel: channel
//! 0, which nothing here reaches; channels 1 to `n` for effects, each
//! switched on with a priority rising to `0xff` (`0x0e72`–`0x0e95`); and
//! behind them, from channel `n + 1` on, one channel per digital voice of
//! the song that plays.
//!
//! **A block** (`0x18e3`) is 128 output bytes: an accumulator of 128 words
//! cleared, every switched-on channel from the last down to 0 running its
//! routine into it — the sample routine `0x196b` or the idle one `0x1965` —
//! and the sum turned into bytes (`0x04b0`): each word plus `0x80`, its low
//! byte where the high one is 0, `0xff` above, `0x00` below, the word's
//! sixteen bits wrapping as the driver's do. The buffer is two halves of
//! 128, written in turn. A block with no channel playing and no song
//! running is the end: the half gets the mid-point (`0x0500`) and the
//! driver marks itself stopped (`[1596h] = 0`), so the interrupt that
//! comes next plays nothing further.
//!
//! **The sample routine** (`0x196b`) works on copies of the channel's
//! volume, remaining length, read position, 16-bit fraction and held
//! contribution, and writes them back at the end of the block (`0x19bc`);
//! when the sample ends inside the block it writes nothing back, and the
//! rest of the block is empty for that channel. A byte's contribution is
//! `(byte − 0x80) × volume`, signed, bits 8 to 23 of the product (`imul`,
//! `0x19e8`). The sample's period `P` (the header's `+8`, PIT cycles) is
//! stepped against the mixer's own `M.m` ([`DmaDriver::period`]):
//!
//! - `M < P`, a sample slower than the mixer, is held: the step is `M.m /
//!   P` in 16 bits, and an output fetches the next byte only when adding
//!   the step to the fraction carries, adding the held contribution
//!   otherwise (`0x19a2`–`0x19fd`). The fraction starts at `0xffff`, so
//!   byte 0 is fetched for output 0; the length counts down per fetch, and
//!   the fetch that empties it ends the sample — **the last byte counts
//!   for one output only**.
//! - `M ≥ P`, a sample faster than the mixer, takes one byte per output
//!   and moves on one or two: `M / P = 1` steps by `((M mod P).m) / P`,
//!   a larger quotient by `0xffff` (`0x1a03`–`0x1a3b`, `0x1a5e`), and the
//!   carry of the step is the extra byte — **it skips zero or one per
//!   output, never more**, so a sample above twice the mixer's rate plays
//!   slow. With the fraction at `0xffff` the first step carries: byte 1 is
//!   skipped. The length counts down by the move, and reaching zero or
//!   passing it ends the sample.
//!
//! **What is not taken.** The play entry's mode argument is the number of
//! further passes; every play here is a single pass, the mode 0 a host
//! passes, and the repeat branch is not built — its restart on the held
//! path (`0x1a83` → `0x19e3`) fetches without counting the output it
//! fetches for, so a channel writes a 129th word past the accumulator, onto
//! the play cursor at `0x1180`. A period of 0 sends the routine through
//! `0x1a52` without the `pop bx` its frame needs, and a length of 0 counts
//! down past zero; samples with either are refused. A header volume of 0
//! or below takes the direct path (`0x2028`), as does every play with one
//! channel installed while the mixer stands; that path is
//! [`Voice`](crate::Voice)'s, and the mixer refuses such a play. The
//! mixer's overload handling (`0x1b41`) never acts here: it needs an
//! interrupt to arrive while a block is still being mixed (`0x07f2`,
//! `0x1b35`), and a block here is mixed whole before time moves on. The
//! rate therefore stays the file's.

use crate::dma::DmaDriver;
use crate::format::{MDH_VOICES, Mdh, Sample};
use crate::num;
use std::sync::Arc;

/// Output bytes in a block: one single-cycle transfer of the DSP, and one
/// half of the DMA buffer.
pub const BLOCK: usize = 128;

/// The most effect channels the install takes (`0x0e6a`).
pub const MAX_EFFECTS: u16 = 8;

/// Channel rows the rebuild keeps: channel 0, the effect channels, and a
/// song's voices behind them.
const CHANNELS: usize = 1 + 8 + MDH_VOICES;

/// One row of the driver's channel tables.
#[derive(Debug, Clone, Default)]
struct Channel {
    /// Switched on (`[15C6h]`, low byte): only these are mixed.
    switch: u8,
    /// Priority (`[15C7h]`), read only by the overload handling.
    priority: u8,
    /// "Touched" (`[1606h]`): set by whatever starts or stops the channel,
    /// cleared when the block routine runs it.
    touched: u8,
    /// Volume (`[1646h]`).
    volume: u16,
    /// The routine (`[1686h]`): the sample routine `0x196b` when set, the
    /// idle `0x1965` otherwise.
    playing: bool,
    /// The sample header (`[16C6h]`), whose period the routine reads.
    sample: Option<Arc<Sample>>,
    /// Bytes left (`[1746h]`).
    rest: u16,
    /// The read position, counted from the first PCM byte (`[1786h]`).
    read: u16,
    /// The 16-bit fraction (`[17C6h]`).
    fraction: u16,
    /// Passes left (`[1806h]`).
    count: u16,
    /// The held contribution (`[1846h]`).
    held: u16,
}

/// One channel row as the driver holds it, for a caller holding the
/// rebuild against the original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelState {
    /// Switched on (`[15C6h]`).
    pub switch: u8,
    /// Priority (`[15C7h]`).
    pub priority: u8,
    /// Touched (`[1606h]`).
    pub touched: u8,
    /// Volume (`[1646h]`).
    pub volume: u16,
    /// Whether the routine is the sample routine (`[1686h]` = `0x196b`).
    pub playing: bool,
    /// Bytes left (`[1746h]`).
    pub rest: u16,
    /// Read position from the first PCM byte (`[1786h]` less the header
    /// pointer and 10).
    pub read: u16,
    /// Fraction (`[17C6h]`).
    pub fraction: u16,
    /// Passes left (`[1806h]`).
    pub count: u16,
    /// Held contribution (`[1846h]`).
    pub held: u16,
}

/// One digital voice's state (`[21A0h]`–`[21E3h]`, four bytes a voice per
/// table).
#[derive(Debug, Clone, Default)]
struct VoiceState {
    /// The four levels from the voice record (`[21A0h]`).
    levels: [u8; 4],
    /// The sample (`[21C0h]`).
    sample: Option<Arc<Sample>>,
    /// 2 once a start found the voice a sample, 0 before (`[21E0h]`).
    state: u8,
    /// The level the last event chose (`[21E1h]`).
    level: u8,
    /// The song's volume scale for the voice (`[21E2h]`).
    scale: u16,
}

/// A song's digital part as a host hands it to entry 21: the `MDH` chunk and
/// the song's sample table, which the voice records index.
#[derive(Debug, Clone)]
pub struct Music {
    mdh: Mdh,
    samples: Vec<Option<Arc<Sample>>>,
}

impl Music {
    /// The chunk and its sample table — `None` for an empty slot, which
    /// leaves its voice switched off. A voice naming an entry past the table
    /// is refused (the driver would read past it, `0x2579`), and so is a
    /// sample the mixer refuses to play.
    pub fn new(mdh: Mdh, samples: Vec<Option<Arc<Sample>>>) -> crate::Result<Self> {
        for voice in &mdh.voices {
            let Some(slot) = samples.get(usize::from(voice.sample)) else {
                return Err(crate::Error::Corrupt {
                    what: "PSM 2 MDH",
                    detail: format!("a voice names sample {} of {}", voice.sample, samples.len()),
                });
            };
            if slot.as_deref().is_some_and(|s| !playable(s)) {
                return Err(crate::Error::Corrupt {
                    what: "PSM 2 sample",
                    detail: "a period of 0 or a length of 0 or past a word".into(),
                });
            }
        }
        Ok(Self { mdh, samples })
    }
}

/// What the interrupt at the end of a block does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Irq {
    /// The next half plays, and the one after it has been mixed.
    Next,
    /// A start that waited for this interrupt: the buffer starts over at
    /// its first half, freshly mixed.
    Restart,
    /// The transfer ends: nothing further plays.
    End,
}

/// Whether the mixer plays a sample: its period and its length are what the
/// sample routine can step through.
fn playable(s: &Sample) -> bool {
    s.period != 0 && !s.pcm.is_empty() && u16::try_from(s.pcm.len()).is_ok()
}

/// `(byte − 0x80) × volume`, signed, bits 8 to 23 of the product (`0x19e5`–
/// `0x19ee`).
fn contribution(byte: u8, volume: u16) -> u16 {
    let s = i32::from(byte) - 0x80;
    num::word((s * i32::from(num::signed(volume))) >> 8)
}

/// An accumulator word as a DMA byte (`0x04e0`–`0x04fe`): plus `0x80`, the
/// low byte where the high one is 0, `0x00` where the word's sign is set,
/// `0xff` otherwise — the addition wrapping in sixteen bits.
fn clip(w: u16) -> u8 {
    let [lo, hi] = w.wrapping_add(0x80).to_le_bytes();
    if hi == 0 {
        lo
    } else if hi & 0x80 != 0 {
        0x00
    } else {
        0xff
    }
}

/// A digital voice's channel volume: level times scale, bits 8 to 23 of the
/// unsigned product, shifted by the scale shift (`0x23b0`–`0x23be`,
/// `0x24a0`–`0x24ad`).
fn voice_volume(level: u8, scale: u16, shift: u8) -> u16 {
    let v = num::low((u32::from(level) * u32::from(scale)) >> 8);
    v.checked_shl(u32::from(shift)).unwrap_or(0)
}

/// The mixer of `DMABLAST.DRV`, installed.
#[derive(Debug, Clone)]
pub struct Mixer {
    channels: [Channel; CHANNELS],
    voices: [VoiceState; MDH_VOICES],
    /// Effect channels (`[1182h]` / 2).
    effects: usize,
    /// The first voice channel (`[15C4h]`), `effects + 1`.
    first_voice: usize,
    /// Channels mixed (`[15C2h]`): the effects and channel 0, and the
    /// song's voices while one plays.
    top: usize,
    /// The channel the next play tries first (`[1180h]` / 2).
    cursor: usize,
    /// The default volume (`[1888h]`).
    default_volume: u16,
    /// The scale shift (`[284h]`).
    shift: u8,
    /// The mixer's period, 16.16 (`[49Eh]:[49Ch]`).
    period: u32,
    /// The volume curve (`0x2212`).
    curve: [u8; 256],
    /// The song's digital part (`[2200h]`, `[2204h]`).
    music: Option<Arc<Music>>,
    /// The stream position (`[220Ch]`).
    stream: usize,
    /// The stream's state (`[2210h]`): 0 none, 1 running, 2 an event
    /// waiting behind a one-row pause.
    streaming: u8,
    /// Skip voice events (`[2211h]`).
    skip: u8,
    /// The mixer runs (`[1596h]` = `0xff`): the next interrupt plays the
    /// next block.
    running: bool,
    /// What the last interrupt saw (`[1592h]`).
    acknowledged: bool,
    /// Counts the block's playing channels and a running stream (`[188Ah]`).
    live: u8,
    /// The first block after a start (`[188Bh]`): it goes to the first half.
    fresh: bool,
    /// A start that waits for the interrupt (`0x20f1`, `0x25d8`).
    pending: bool,
    /// A start from rest happened since the last look.
    started: bool,
    /// The DMA buffer, two halves (`0x028a`).
    buffer: [u8; 2 * BLOCK],
    /// The half the last block went to (`[48Ch]`).
    half: usize,
}

impl Mixer {
    /// The driver installed with `effects` effect channels (entry 0,
    /// `0x0e5f`–`0x0eb1`): clamped to 1–8, channel 0 switched off at
    /// priority `0xff`, each effect channel switched on with priorities
    /// stepping by `0x1fe / 2n` up to `0xff` for the last, and the buffer
    /// at the mid-point.
    pub fn new(driver: &DmaDriver, effects: u16) -> Self {
        let n = effects.clamp(1, MAX_EFFECTS);
        let effects = usize::from(n);
        let mut channels: [Channel; CHANNELS] = Default::default();
        let step = num::byte(0x1fe / (2 * u32::from(n)));
        let mut priority = 0xffu8;
        for ch in channels[1..=effects].iter_mut().rev() {
            ch.switch = 2;
            ch.priority = priority;
            priority = priority.wrapping_sub(step);
        }
        channels[0].priority = 0xff;
        Self {
            channels,
            voices: Default::default(),
            effects,
            first_voice: effects + 1,
            top: effects + 1,
            cursor: usize::from(driver.cursor / 2),
            default_volume: driver.volume,
            shift: driver.shift,
            period: driver.period,
            curve: driver.curve,
            music: None,
            stream: 0,
            streaming: 0,
            skip: 0,
            running: false,
            acknowledged: false,
            live: 0,
            fresh: true,
            pending: false,
            started: false,
            buffer: [0x80; 2 * BLOCK],
            half: 0,
        }
    }

    /// Entry 22 (`0x1f37`): the default volume, the argument shifted by the
    /// scale shift and taken when it lies in 1 to `0x2001`; answers the old
    /// one, shifted back.
    pub fn set_volume(&mut self, volume: u16) -> u16 {
        let old = self
            .default_volume
            .checked_shr(u32::from(self.shift))
            .unwrap_or(0);
        let v = volume.checked_shl(u32::from(self.shift)).unwrap_or(0);
        if v.wrapping_sub(1) <= 0x2000 {
            self.default_volume = v;
        }
        old
    }

    /// Entry 17 with mode 0 (`0x200c`): the sample on an effect channel, and
    /// the mixer started if it stands. Answers the channel, or `None` where
    /// the driver would take a path the rebuild does not build — the
    /// direct path, or a sample the routine cannot step (the module's
    /// words on what is not taken).
    ///
    /// The channel: from the cursor round, the first switched-on channel
    /// whose routine is idle (`0x2045`–`0x206a`); with none, the one at the
    /// cursor is taken from whatever plays on it, or channel 1 if the
    /// cursor's is switched off (`0x206c`–`0x207c`). Only the effect
    /// channels are searched. The cursor moves past the channel taken
    /// (`0x207e`–`0x208b`). The channel gets the default volume where the
    /// header says `0x0100`, and `0x0100` for any other positive word — the
    /// entry loads `0x100` (`0x2090`) and replaces it only with the default
    /// (`0x2098`), never with the word ([`Sample::volume`]); one pass;
    /// nothing held; the fraction at
    /// `0xffff` (`0x2090`–`0x20e0`).
    pub fn play(&mut self, sample: &Arc<Sample>) -> Option<usize> {
        if !playable(sample) || num::signed(sample.volume) <= 0 {
            return None;
        }
        if !self.running && self.effects == 1 {
            return None;
        }
        let effects = self.effects;
        let next = |k: usize| if k < effects { k + 1 } else { 1 };
        let start = self.cursor;
        let mut k = start;
        let mut free = None;
        loop {
            let ch = &self.channels[k];
            if ch.switch != 0 && !ch.playing {
                free = Some(k);
                break;
            }
            k = next(k);
            if k == start {
                break;
            }
        }
        let k = match free {
            Some(k) => k,
            None if self.channels[start].switch != 0 => start,
            None if self.channels[1].switch != 0 => 1,
            None => return None,
        };
        self.cursor = next(k);
        let volume = if sample.volume == 0x100 {
            self.default_volume
        } else {
            0x100
        };
        let ch = &mut self.channels[k];
        ch.volume = volume;
        ch.count = 1;
        ch.held = 0;
        ch.fraction = 0xffff;
        ch.touched = 0xff;
        ch.sample = Some(Arc::clone(sample));
        ch.rest = u16::try_from(sample.pcm.len()).unwrap_or(u16::MAX);
        ch.read = 0;
        ch.playing = true;
        self.live = 1;
        self.start_or_wait();
        Some(k)
    }

    /// Entry 18 (`0x2121` → `0x2104`, `0x0794`): every channel mixed set
    /// idle, the song's voices with them, and the mixer marked stopped —
    /// the block playing ends, and the one already mixed after it does not
    /// play. A song's stream runs on; its voices stay silent until a play or
    /// a song start runs the mixer again. The entry's argument makes the
    /// caller wait for that interrupt (`0x2139`–`0x2145`), which is the
    /// caller's to do.
    pub fn stop_all(&mut self) {
        for ch in &mut self.channels[..self.top] {
            ch.playing = false;
            ch.touched = 1;
        }
        self.running = false;
    }

    /// Entry 21 (`0x25ea`): the voices of a song that played are let go,
    /// and the song's digital part — or none — is what the next start
    /// callback reads. The chunk was checked when it was read.
    pub fn set_music(&mut self, music: Option<Arc<Music>>) {
        self.voices_off();
        self.music = music;
    }

    /// The callback with `bx = 0` (`0x2325` → `0x2444`): the song's voices
    /// cut, from the next block on, and the stream over.
    pub fn music_stop(&mut self) {
        self.voices_off();
    }

    /// The callback with `bx = 2` (`0x2329` → `0x2506`): the song's digital
    /// part starts at `level`, its voices placed on the channels behind the
    /// effects, each with its record's priority and levels and the level
    /// as its scale, taken as it is; a voice whose sample slot is empty
    /// stays switched off. The stream starts, and the mixer with it if it
    /// stands. Answers 1, or 0 without a song (and then no voice plays).
    /// `skip`, the fast-forward flag, makes later steps pass over voice
    /// events.
    pub fn music_start(&mut self, skip: u8, level: u16) -> u16 {
        self.skip = skip;
        self.streaming = 0;
        self.voices_off();
        let Some(music) = self.music.clone() else {
            self.top = self.first_voice;
            return 0;
        };
        self.top = self.first_voice + music.mdh.voices.len();
        for (v, record) in music.mdh.voices.iter().enumerate().rev() {
            let ch = &mut self.channels[self.first_voice + v];
            ch.playing = false;
            ch.switch = 0;
            ch.touched = 1;
            ch.priority = record.priority;
            let voice = &mut self.voices[v];
            voice.levels = record.levels;
            voice.scale = level;
            if let Some(Some(sample)) = music.samples.get(usize::from(record.sample)) {
                ch.sample = Some(Arc::clone(sample));
                voice.sample = Some(Arc::clone(sample));
                voice.state = 2;
                ch.switch = 0x0a;
            }
        }
        self.stream = 0;
        self.streaming = 1;
        self.start_or_wait();
        1
    }

    /// The callback with `bx = 4` (`0x2342`): the stream's events up to the
    /// next pause, answering the pause in rows — 0 when the stream is over
    /// or there is none.
    ///
    /// A voice event sets the voice's channel volume from the level it names
    /// and the voice's scale, and starts its sample from the beginning
    /// (`0x24c8`): nothing held, the fraction at `0xffff`, one pass. A voice
    /// without a sample goes idle instead (`0x24c1`). An event does not
    /// start the mixer.
    pub fn music_step(&mut self, skip: u8) -> u16 {
        self.skip = skip;
        if self.streaming == 0 {
            return 0;
        }
        let Some(music) = self.music.clone() else {
            return 0;
        };
        let byte = |at: usize| music.mdh.events.get(at).copied();
        let mut at = self.stream;
        loop {
            let Some(b) = byte(at) else {
                // Past the stream: `Mdh::parse` refuses a stream that gets
                // here, so a song read through it never does.
                self.voices_off();
                return 0;
            };
            at += 1;
            let event = if self.streaming != 1 {
                // `0x2387`: the event behind a one-row pause.
                self.streaming = 1;
                (b << 1) & 0x7e
            } else if b & 0x80 != 0 {
                let rows = if b & 0x40 != 0 {
                    let low = byte(at).unwrap_or(0);
                    at += 1;
                    (u16::from(b & 0x3f) << 8) | u16::from(low)
                } else {
                    u16::from(b & 0x3f) * 2
                };
                self.stream = at;
                return rows;
            } else if b & 0x40 != 0 {
                // `0x2365`: one row, and the byte read again after it.
                self.stream = at - 1;
                self.streaming = 2;
                return 1;
            } else {
                b << 1
            };
            if event >= 0x40 {
                // `20 00` (`0x23e9` → `0x2500`): the voices let go, and the
                // stream, read on, is over.
                self.voices_off();
                return 0;
            }
            if self.skip != 0 {
                continue;
            }
            let b = event >> 1;
            self.voice_event(usize::from(b >> 2), usize::from(b & 3));
        }
    }

    /// The callback with `bx = 6` and `ax < 0` (`0x2405`, `0x242b`–
    /// `0x2440`): every voice's scale. Below `0x100` the level goes through
    /// the curve first; each voice that has a sample takes it as its scale
    /// and its channel volume follows at once; above `0x100` it is only a
    /// question. Answers voice 0's old scale (`0x100` if it has no sample,
    /// its scale if only asked). The callback's form for one voice, `ax ≥ 0`,
    /// which only `MUSADL.DRV`'s volume entry would send, is not built.
    pub fn music_volume(&mut self, level: u16) -> u16 {
        let cx = if level < 0x100 {
            u16::from(self.curve[usize::from(level)])
        } else {
            level
        };
        let mut answer = 0;
        for v in (0..MDH_VOICES).rev() {
            answer = self.voice_scale(v, cx);
        }
        answer
    }

    /// `0x2482`: one voice's scale.
    fn voice_scale(&mut self, v: usize, cx: u16) -> u16 {
        let voice = &mut self.voices[v];
        if voice.state == 0 {
            return 0x100;
        }
        if cx > 0x100 {
            return voice.scale;
        }
        let old = voice.scale;
        voice.scale = cx;
        self.channels[self.first_voice + v].volume = voice_volume(voice.level, cx, self.shift);
        old
    }

    /// A voice event (`0x239d`–`0x23e5`).
    fn voice_event(&mut self, v: usize, l: usize) {
        let voice = &mut self.voices[v];
        let level = voice.levels[l];
        voice.level = level;
        let volume = voice_volume(level, voice.scale, self.shift);
        let rest = voice
            .sample
            .as_deref()
            .map_or(0, |s| u16::try_from(s.pcm.len()).unwrap_or(u16::MAX));
        let on = voice.state == 2;
        let ch = &mut self.channels[self.first_voice + v];
        ch.volume = volume;
        if !on {
            ch.playing = false;
            return;
        }
        ch.held = 0;
        ch.fraction = 0xffff;
        ch.count = 1;
        ch.rest = rest;
        ch.read = 0;
        ch.playing = true;
        ch.touched = 1;
    }

    /// `0x2444`: the song's voices cut and the stream over. Each voice
    /// channel goes idle and switched off, its touched byte the voice count.
    fn voices_off(&mut self) {
        let count = self.top - self.first_voice;
        self.streaming = 0;
        if count == 0 {
            return;
        }
        self.top = self.first_voice;
        let touched = num::byte(u32::try_from(count).unwrap_or(0));
        for ch in &mut self.channels[self.first_voice..self.first_voice + count] {
            ch.touched = touched;
            ch.switch = 0;
            ch.playing = false;
        }
    }

    /// `0x20e6`–`0x20fa` and `0x25ce`–`0x25e4`: a mixer that runs goes on;
    /// one that stands starts now, from rest; one whose stop the interrupt
    /// has not yet seen starts at that interrupt.
    fn start_or_wait(&mut self) {
        if self.running {
            return;
        }
        if self.acknowledged {
            self.pending = true;
        } else {
            self.start();
            self.started = true;
        }
    }

    /// The start from rest (`0x1bc7`–`0x1bd6`): a block mixed into the
    /// first half, the transfer started on it (`0x1b10` → `0x06d8`), and the
    /// next block mixed behind it.
    fn start(&mut self) {
        self.live = 1;
        self.fresh = true;
        self.mix_block();
        self.mix_block();
    }

    /// The interrupt at the end of a block (`0x07a7`, `0x07e1`–`0x07ed`):
    /// it takes over the mixer's state; running, the next half plays and the
    /// block after it is mixed (`0x1b10`); stopped, nothing further plays
    /// (`0x085e`) — unless a start was waiting for it.
    pub fn irq(&mut self) -> Irq {
        self.acknowledged = self.running;
        if self.acknowledged {
            self.mix_block();
            return Irq::Next;
        }
        if self.pending {
            self.pending = false;
            self.start();
            return Irq::Restart;
        }
        Irq::End
    }

    /// Whether a play or a song start started the mixer from rest since the
    /// last call: the transfer begins on the first half now.
    pub fn take_start(&mut self) -> bool {
        std::mem::take(&mut self.started)
    }

    /// The DMA buffer, both halves.
    pub fn buffer(&self) -> &[u8; 2 * BLOCK] {
        &self.buffer
    }

    /// Where the last block went: 0 or [`BLOCK`].
    pub fn half(&self) -> usize {
        self.half
    }

    /// Whether the mixer runs (`[1596h]`).
    pub fn running(&self) -> bool {
        self.running
    }

    /// The channel the next play tries first.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// How many channels are mixed (`[15C2h]`).
    pub fn mixed(&self) -> usize {
        self.top
    }

    /// Channel `k`'s row, if the rebuild keeps one.
    pub fn channel(&self, k: usize) -> Option<ChannelState> {
        self.channels.get(k).map(|c| ChannelState {
            switch: c.switch,
            priority: c.priority,
            touched: c.touched,
            volume: c.volume,
            playing: c.playing,
            rest: c.rest,
            read: c.read,
            fraction: c.fraction,
            count: c.count,
            held: c.held,
        })
    }

    /// One block (`0x18e3`).
    fn mix_block(&mut self) {
        let mut acc = [0u16; BLOCK];
        self.live = self.streaming;
        for k in (0..self.top).rev() {
            if self.channels[k].switch == 0 {
                continue;
            }
            if self.channels[k].playing {
                self.run(k, &mut acc);
            } else {
                self.channels[k].touched = 0;
            }
        }
        if self.fresh {
            // `0x04b0`/`0x0500`: the first block after a start goes to the
            // first half.
            self.fresh = false;
            self.half = BLOCK;
        }
        self.half ^= BLOCK;
        let out = &mut self.buffer[self.half..self.half + BLOCK];
        if self.live == 0 {
            out.fill(0x80);
            self.running = false;
            return;
        }
        self.running = true;
        for (byte, &w) in out.iter_mut().zip(&acc) {
            *byte = clip(w);
        }
    }

    /// The sample routine (`0x196b`) for channel `k`, into the accumulator.
    fn run(&mut self, k: usize, acc: &mut [u16; BLOCK]) {
        self.live = self.live.wrapping_add(1);
        let [lo, hi] = num::split(self.period);
        let ch = &mut self.channels[k];
        ch.touched = 0;
        let Some(sample) = ch.sample.clone() else {
            ch.playing = false;
            return;
        };
        let p = sample.period;
        if p == 0 {
            ch.playing = false;
            return;
        }
        let pcm = &sample.pcm;
        let byte = |at: u16| pcm.get(usize::from(at)).copied().unwrap_or(0x80);
        let volume = ch.volume;
        let (mut rest, mut read, mut fraction, mut held) = (ch.rest, ch.read, ch.fraction, ch.held);
        if hi < p {
            // `0x19a2`: held.
            let step = num::low(((u32::from(hi) << 16) | u32::from(lo)) / u32::from(p));
            for slot in acc.iter_mut() {
                let (f, carry) = fraction.overflowing_add(step);
                fraction = f;
                if carry {
                    held = contribution(byte(read), volume);
                    read = read.wrapping_add(1);
                    *slot = slot.wrapping_add(held);
                    rest = rest.wrapping_sub(1);
                    if rest == 0 {
                        end(ch);
                        return;
                    }
                } else {
                    *slot = slot.wrapping_add(held);
                }
            }
        } else {
            // `0x1a03`: one or two bytes on.
            let step = if hi / p == 1 {
                num::low(((u32::from(hi % p) << 16) | u32::from(lo)) / u32::from(p))
            } else {
                0xffff
            };
            for slot in acc.iter_mut() {
                held = contribution(byte(read), volume);
                *slot = slot.wrapping_add(held);
                let (f, carry) = fraction.overflowing_add(step);
                fraction = f;
                let advance = 1 + u16::from(carry);
                read = read.wrapping_add(advance);
                let (left, borrow) = rest.overflowing_sub(advance);
                rest = left;
                if borrow || left == 0 {
                    end(ch);
                    return;
                }
            }
        }
        // `0x19bc`: the block is over and the channel's state goes back.
        ch.rest = rest;
        ch.read = read;
        ch.fraction = fraction;
        ch.held = held;
    }
}

/// The sample is over inside a block (`0x1a3e`, `0x1aa0`): the pass is
/// counted and the routine goes idle — every play here is a single pass —
/// and nothing of the block's state is written back.
fn end(ch: &mut Channel) {
    ch.count = ch.count.wrapping_sub(1);
    ch.playing = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dma::tests::file;
    use crate::format::MdhVoice;

    fn driver() -> DmaDriver {
        DmaDriver::parse(&file()).expect("the driver")
    }

    fn sample(period: u16, pcm: &[u8]) -> Arc<Sample> {
        Arc::new(Sample {
            volume: 0x100,
            period,
            pcm: pcm.to_vec(),
        })
    }

    /// Six channels with priorities stepping by `0x1fe / 12` up to `0xff`,
    /// channel 0 off at `0xff`; the count clamped to 1–8.
    #[test]
    fn the_install_switches_on_the_effect_channels() {
        let m = Mixer::new(&driver(), 6);
        let pri: Vec<u8> = (0..8).map(|k| m.channel(k).unwrap().priority).collect();
        assert_eq!(pri, [0xff, 0x2d, 0x57, 0x81, 0xab, 0xd5, 0xff, 0]);
        let sw: Vec<u8> = (0..8).map(|k| m.channel(k).unwrap().switch).collect();
        assert_eq!(sw, [0, 2, 2, 2, 2, 2, 2, 0]);
        assert_eq!(m.mixed(), 7);
        assert_eq!(Mixer::new(&driver(), 0).mixed(), 2);
        assert_eq!(Mixer::new(&driver(), 20).mixed(), 9);
    }

    /// Entry 22 takes 1 to `0x2001` and answers the old value.
    #[test]
    fn the_default_volume_takes_its_range() {
        let mut m = Mixer::new(&driver(), 6);
        assert_eq!(m.set_volume(0x50), 0x40);
        assert_eq!(m.set_volume(0x3c), 0x50);
        assert_eq!(m.set_volume(0), 0x3c);
        assert_eq!(m.set_volume(0x2002), 0x3c);
        assert_eq!(m.set_volume(0x2001), 0x3c);
        assert_eq!(m.set_volume(0x3c), 0x2001);
    }

    /// Plays go round from the cursor to the first idle channel; with all
    /// six busy the seventh takes the cursor's, and the cursor wraps from
    /// the last channel to the first.
    #[test]
    fn channels_go_round_and_the_cursor_is_stolen() {
        let mut m = Mixer::new(&driver(), 6);
        let long = sample(100, &[0x90; 4000]);
        let taken: Vec<_> = (0..8).map(|_| m.play(&long)).collect();
        assert_eq!(
            taken,
            [1, 2, 3, 4, 5, 6, 1, 2].map(Some),
            "round, then stolen at the cursor"
        );
        assert_eq!(m.cursor(), 3);
        // Channel 3 ends early: the search from the cursor finds it idle.
        m.channels[4].playing = false;
        assert_eq!(m.play(&long), Some(4), "the first idle from the cursor");
        assert_eq!(m.cursor(), 5);
    }

    /// A sample slower than the mixer is held: byte 0 lands in output 0,
    /// each byte lasts while the fraction does not carry, the last byte
    /// counts once; and the volume `0x3c` scales `(b − 0x80)` by 60/256,
    /// rounding down.
    #[test]
    fn a_slow_sample_is_held_and_its_last_byte_counts_once() {
        let mut d = driver();
        // M.m = 57.25 against P = 115: the step is 3 751 936 / 115 = 32 625,
        // so the fraction from 0xffff carries at outputs 0, 2 and 4.
        d.period = (57 << 16) | 0x4000;
        let mut m = Mixer::new(&d, 6);
        m.set_volume(0x3c);
        let s = sample(115, &[0x80 + 64, 0x80 - 64, 0xff]);
        assert_eq!(m.play(&s), Some(1));
        let a = &m.buffer()[..BLOCK];
        // 64 × 60 >> 8 = 15; −64 × 60 >> 8 = −15; 127 × 60 >> 8 = 29.
        let (up, down) = (0x80 + 15, 0x80 - 15);
        assert_eq!(&a[..5], &[up, up, down, down, 0x80 + 29]);
        assert!(a[5..].iter().all(|&b| b == 0x80), "{:x?}", &a[..8]);
        let ch = m.channel(1).unwrap();
        assert!(!ch.playing);
        assert_eq!(ch.count, 0);
        assert_eq!(ch.rest, 3, "an end writes nothing back");
    }

    /// A sample faster than the mixer but not twice as fast takes a byte per
    /// output and skips one now and then — the first step carries, so byte
    /// 1 is the first skipped; `P` 54 steps by 3949 a byte.
    #[test]
    fn a_fast_sample_skips_at_most_one() {
        let mut m = Mixer::new(&driver(), 6);
        m.set_volume(0x100);
        let pcm: Vec<u8> = (1..=40).map(|i| 0x80 + i).collect();
        m.play(&sample(54, &pcm));
        let a = &m.buffer()[..BLOCK];
        assert_eq!(&a[..4], &[0x81, 0x83, 0x84, 0x85]);
        let played: Vec<u8> = a.iter().copied().take_while(|&b| b != 0x80).collect();
        // The fraction carries at outputs 0, 16 and 33: 40 bytes in 37
        // outputs.
        assert_eq!(played.len(), 37);
        assert!(played.windows(2).all(|w| matches!(w[1] - w[0], 1 | 2)));
        assert_eq!(played.windows(2).filter(|w| w[1] - w[0] == 2).count(), 3);
    }

    /// The clip: past `0x7f` above the mid-point is `0xff`, below it `0x00`,
    /// and a sum past the word wraps as the driver's sixteen bits do.
    #[test]
    fn the_sum_is_clipped_to_a_byte() {
        assert_eq!(clip(0), 0x80);
        assert_eq!(clip(0x7f), 0xff);
        assert_eq!(clip(0x80), 0xff);
        assert_eq!(clip(0xff80), 0x00);
        assert_eq!(clip(0xff7f), 0x00);
        assert_eq!(clip(0x7f90), 0x00, "0x7f90 + 0x80 is negative");
        let mut m = Mixer::new(&driver(), 8);
        m.set_volume(0x400);
        let loud = sample(200, &[0xff; 300]);
        m.play(&loud);
        // 127 × 0x400 >> 8 = 508.
        assert_eq!(m.buffer()[0], 0xff);
    }

    /// Play from rest mixes two blocks; an interrupt mixes the next; a block
    /// with nothing playing marks the end; `StopAll` ends at the next
    /// interrupt; a play while the stop waits starts at that interrupt.
    #[test]
    fn the_mixer_runs_from_start_to_stop() {
        let mut m = Mixer::new(&driver(), 6);
        // About 1.75 outputs a byte: 200 bytes end in the third block.
        let s = sample(100, &[0xa0; 200]);
        m.play(&s);
        assert!(m.take_start());
        assert!(m.running());
        assert_eq!(m.irq(), Irq::Next);
        assert!(m.running());
        assert_eq!(m.irq(), Irq::Next);
        assert!(!m.running(), "the fourth block is silent");
        assert_eq!(m.irq(), Irq::End);
        // Before the first interrupt after a start, a stop leaves nothing for
        // a play to wait for.
        m.play(&s);
        assert!(m.take_start());
        m.stop_all();
        m.play(&s);
        assert!(m.take_start());
        // Once an interrupt has seen it run, a play after a stop waits.
        assert_eq!(m.irq(), Irq::Next);
        let k = m.cursor();
        m.stop_all();
        assert!((1..=6).all(|k| !m.channel(k).unwrap().playing));
        assert_eq!(m.play(&s), Some(k));
        assert!(!m.take_start());
        assert_eq!(m.irq(), Irq::Restart);
        assert_eq!(m.half(), BLOCK, "the restart mixed both halves");
        assert_ne!(m.buffer()[0], 0x80);
    }

    fn music(stream: &[u8], samples: Vec<Option<Arc<Sample>>>) -> Arc<Music> {
        let mdh = Mdh {
            voices: vec![
                MdhVoice {
                    priority: 0x40,
                    sample: 0,
                    levels: [0x40, 0x80, 0xc0, 0xff],
                },
                MdhVoice {
                    priority: 0x50,
                    sample: 1,
                    levels: [0x10, 0x20, 0x30, 0x40],
                },
            ],
            events: stream.to_vec(),
        };
        Arc::new(Music::new(mdh, samples).expect("music"))
    }

    /// A song's voices sit behind the effect channels, its stream answers
    /// its pauses, an event sets the volume from level times scale and
    /// starts the sample, and the end lets the voices go.
    #[test]
    fn a_song_plays_its_voices_behind_the_effects() {
        let mut m = Mixer::new(&driver(), 6);
        let a = sample(100, &[0x90; 3000]);
        let stream = [0x03, 0x04, 0x86, 0x42, 0xc1, 0x05, 0x20, 0x00];
        m.set_music(Some(music(&stream, vec![Some(a), None])));
        assert_eq!(m.music_start(0, 0x80), 1);
        assert!(m.take_start());
        assert_eq!(m.mixed(), 9);
        let v0 = m.channel(7).unwrap();
        assert_eq!((v0.switch, v0.priority, v0.touched), (0x0a, 0x40, 0));
        assert_eq!(m.channel(8).unwrap().switch, 0, "an empty slot stays off");
        // Voice 0 at level 3 (0xff × 0x80 >> 8 = 0x7f), voice 1 at level 0;
        // then 12 rows.
        assert_eq!(m.music_step(0), 12);
        let v0 = m.channel(7).unwrap();
        assert_eq!(
            (v0.volume, v0.playing, v0.rest, v0.fraction),
            (0x7f, true, 3000, 0xffff)
        );
        assert_eq!(m.channel(8).unwrap().volume, 0x08, "0x10 × 0x80 >> 8");
        // `42`: a row, then the event `02` behind it.
        assert_eq!(m.music_step(0), 1);
        assert_eq!(m.music_step(0), 0x0105);
        assert_eq!(m.channel(7).unwrap().volume, 0x60, "0xc0 × 0x80 >> 8");
        // The volume through the curve (the identity here): 0x40.
        assert_eq!(m.music_volume(0x40), 0x80);
        assert_eq!(m.channel(7).unwrap().volume, 0x30);
        assert_eq!(m.music_volume(0x101), 0x40, "above 0x100 only asked");
        assert_eq!(m.music_step(0), 0, "the end");
        assert_eq!(m.mixed(), 7);
        assert_eq!(m.channel(7).unwrap().touched, 2, "the voice count");
        assert_eq!(m.music_step(0), 0);
    }

    /// Voice events pass while the mixer stands after `StopAll`; the next play
    /// brings the voices back with it.
    #[test]
    fn stop_all_silences_the_voices_until_the_mixer_runs() {
        let mut m = Mixer::new(&driver(), 6);
        let a = sample(100, &[0x90; 3000]);
        m.set_music(Some(music(
            &[0x00, 0x82, 0x00, 0x82, 0x20, 0x00],
            vec![Some(a.clone()), None],
        )));
        m.music_start(0, 0x100);
        m.take_start();
        m.music_step(0);
        m.stop_all();
        assert_eq!(m.irq(), Irq::End);
        assert_eq!(m.music_step(0), 4, "the stream runs on");
        assert!(!m.take_start(), "an event does not start the mixer");
        assert!(m.channel(7).unwrap().playing);
        m.play(&a);
        assert!(m.take_start());
        assert!(m.buffer()[..BLOCK].iter().any(|&b| b != 0x80));
    }
}
