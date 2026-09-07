//! The 32-bit engine's digital samples: the five words Checker 2000 reaches
//! for, read from `ENGINE.EXE` V0.04.15/R78 and cross-read on R109, which
//! carries the same code under Dunkle Schatten 2 and never runs it.
//!
//! The sound layer is HMI's SOS. A sample is a node of `0x2e` bytes on a
//! list at `0xcb308` (R78): the SOS handle at `+0`, a kind-3 timer at `+4`
//! opened and set to zero as the sample starts, the next node at `+8`, a
//! playing flag at `+0xc` that the layer's end-of-sample callback clears
//! (`0x6a020`), a stream bit `0x80` at `+0x25`, and at `+0x2a` a duration in
//! the timer's 200 Hz unit. Two words make one, one drops one, one reads
//! one, and one sets the music beside them:
//!
//! - `STARTSAMPLE ( block loops -- handle )` (`0x6ad10`) loads block
//!   `NNN.blk` whole, hands the bytes to the layer's start (`0x6f18c`), sets
//!   the volume to **`0x1fff`** of `0x7fff` (`0x6ae8f`), and answers the
//!   handle — 0 when sound is off or the block is missing. The duration it
//!   writes is `bytes × 200 / rate`, the whole block's bytes over the rate at
//!   `+0x18` of it (`0x6ae51`) — twice the truth for a 16-bit block, which
//!   every shipped one is.
//! - `->STARTSAMPLE ( name$ loops -- handle )` (`0x6a870`) plays the file
//!   `<smppath>\<name>` at **`0x7fff`**. A file over 128 KB streams through a
//!   ring buffer when streaming is on and no stream runs (`0x6a919`–`0x6a934`),
//!   and its duration is the data bytes over the rate, ×200 for 8-bit and
//!   ×100 for anything else (`0x6aa63`–`0x6aa94`); a smaller file loads whole
//!   and goes the block path's way, duration and volume apart. A file that
//!   is not there is asked for again without end (`0x6a8e0`–`0x6a8fe`) — the
//!   CD prompt of a game whose speech lived on the disc.
//! - **The layer's start** (`0x6f18c` → `0x6ee60`) takes the first of 34
//!   sample slots at `0xbb1e4` that is free or whose sample the driver
//!   reports done, and refuses only when none is — samples sound together,
//!   and the mixer sums them. The `loops` argument goes to `+0x30` of the
//!   slot's record (`0x6eedb`), where the mixer reads it as the data runs
//!   out (`0x82d28`): −1 rewinds for ever, 0 ends the sample, any other
//!   count rewinds and counts down. Checker 2000 passes 0 nearly everywhere,
//!   and −1 for the ambience of two scenes (block 17 in module 212, a file
//!   in module 205), which `STOPSAMPLE` ends. The stream path builds its own
//!   record (`0x70990`) and takes no count: a streamed file plays once.
//! - `STOPSAMPLE ( handle -- )` (`0x6aee0`) stops the layer's sample, drops
//!   the node and, for the stream, the stream's globals.
//! - `?STIME ( handle -- t | -1 )` (`0x6b0b0`) answers the node's timer in
//!   200 Hz ticks while the node is found and playing and the timer has not
//!   passed **1.3 times** the duration (`0x6b175`–`0x6b197`); past that it
//!   marks the node finished and answers −1, as it does for a node the end
//!   callback has already finished, and for a handle it cannot find.
//! - `MUSVOLUME ( f -- )` (`0x6b380`) sets every sequence's volume to
//!   `0x3800` for zero and `0x7fff` otherwise — the music ducked under
//!   speech — which the sequencer turns into a controller 7 on every channel
//!   (`0x70590` → `0x7afad`); that part is the audio crate's.
//!
//! What the engine keeps is the bookkeeping the words read back: the node,
//! its timer, its duration, whether it has ended. The end is the DAC running
//! dry, which here is the frames of the file, as many passes as the loop
//! count says, over its rate on the master counter — deterministic, like
//! every other clock in this crate — and never, for a count below zero. The
//! bytes go over the [`crate::MusicSink`] whole with the count, and the sink
//! owns the format.

use crate::Engine;
use motionvm_motion_formats::m32::Wav;

/// One sample the game has started and not yet stopped: the node at
/// `0xcb308`'s list, as the words read it.
#[derive(Debug, Clone)]
pub struct SampleNode {
    /// The handle the start word answered — `+0`.
    pub handle: i32,
    /// The master count the node's kind-3 timer was set to zero at — `+4`.
    pub started: u64,
    /// The duration the start word computed, in 200 Hz ticks — `+0x2a`.
    pub duration: i32,
    /// The master count at which the DAC runs dry and the layer's end
    /// callback would clear the playing flag.
    pub ends: u64,
    /// `+0xc`: cleared by the end callback, by `?STIME`'s 1.3× rule, and by
    /// nothing else.
    pub playing: bool,
    /// `+0x25 & 0x80`: the one node `->STARTSAMPLE` streams.
    pub stream: bool,
    /// The loop count the layer's record holds at `+0x30`: 0 once, a
    /// positive count that many passes more, a negative one for ever. A
    /// stream's is 0 whatever the word was handed.
    pub loops: i32,
}

/// The SOS volume `STARTSAMPLE` sets (`0x6ae8f`): a quarter of full scale.
pub(crate) const BLOCK_VOLUME: u16 = 0x1fff;
/// The SOS volume `->STARTSAMPLE` sets (`0x6ab48`, `0x6ac7c`): full scale.
pub(crate) const FILE_VOLUME: u16 = 0x7fff;
/// Files longer than this stream rather than load (`0x6a919`).
const STREAM_FROM: usize = 0x20000;
/// The sample slots the layer's start walks for a free one (`0x6ee6e`,
/// 34 of them at `0xbb1e4`): how many samples can sound at once.
const SLOTS: usize = 34;

impl Engine {
    /// Whether the node is still sounding: its flag is up and the DAC has
    /// not run dry. The end callback is the layer's interrupt; here it is
    /// the master count passing the end.
    fn node_playing(&self, node: &SampleNode) -> bool {
        node.playing && self.master_ticks < node.ends
    }

    /// Whether every slot of the layer is taken by a sample still sounding,
    /// which is the one thing the layer's start refuses on (`0x6ef97`–
    /// `0x6efa5`): no shipped script comes near.
    fn slots_full(&self) -> bool {
        self.sound
            .samples
            .iter()
            .filter(|n| self.node_playing(n))
            .count()
            >= SLOTS
    }

    /// The end of a sample on the master counter: its frames, once and once
    /// more for every loop, over its rate, rounded up to the tick the last
    /// one is still sounding in — and never, for a count below zero.
    fn dac_runs_dry_at(&self, wav: &Wav, loops: i32) -> u64 {
        let Ok(loops) = u64::try_from(loops) else {
            return u64::MAX;
        };
        let frames = u64::try_from(wav.frames()).unwrap_or(u64::MAX);
        let raw = frames
            .saturating_mul(loops.saturating_add(1))
            .saturating_mul(u64::from(Self::RAW_TICKS_PER_SECOND))
            .div_ceil(u64::from(wav.rate.max(1)));
        self.master_ticks.saturating_add(raw)
    }

    /// The node for a started sample, and the bytes to the sink.
    fn start_node(
        &mut self,
        wav: &Wav,
        bytes: &[u8],
        duration: i32,
        stream: bool,
        volume: u16,
        loops: i32,
    ) -> i32 {
        let handle = self.sound.next_handle;
        self.sound.next_handle += 1;
        let ends = self.dac_runs_dry_at(wav, loops);
        self.sound.samples.push(SampleNode {
            handle,
            started: self.master_ticks,
            duration,
            ends,
            playing: true,
            stream,
            loops,
        });
        if let Some(sink) = self.sound.sink.as_mut() {
            sink.start_sample(handle, bytes, volume, loops);
        }
        handle
    }

    /// `STARTSAMPLE`'s body: the block whole, a quarter of full scale, as
    /// many passes as `loops` says.
    ///
    /// 0 where the original answers 0: no sound layer, no such block, a
    /// block that is not a WAV file the layer could play, or every slot
    /// taken.
    pub(crate) fn start_block_sample(&mut self, block: i32, loops: i32) -> i32 {
        if self.sound.sink.is_none() {
            return 0;
        }
        let Some(bytes) = self.resources.as_ref().and_then(|r| r.block(block)) else {
            return 0;
        };
        let Ok(wav) = Wav::parse(&bytes) else {
            return 0;
        };
        if self.slots_full() {
            return 0;
        }
        // `bytes × 200 / rate` over the whole block (`0x6ae51`).
        let duration = ticks_of(bytes.len(), 200, wav.rate);
        self.start_node(&wav, &bytes, duration, false, BLOCK_VOLUME, loops)
    }

    /// `->STARTSAMPLE`'s body: the file under the sample directory, at full
    /// scale, streamed when it is long and no stream runs — once, then;
    /// otherwise as many passes as `loops` says.
    ///
    /// A file that is not there answers 0 and is counted, where the original
    /// asks for it forever — see the departures ledger.
    pub(crate) fn start_file_sample(&mut self, name: &str, loops: i32) -> i32 {
        if self.sound.sink.is_none() {
            return 0;
        }
        let Some(bytes) = self.sample_file(name) else {
            self.note_unhandled(
                crate::words::Word::TO_STARTSAMPLE,
                Some(format!("{name} missing")),
            );
            return 0;
        };
        let Ok(wav) = Wav::parse(&bytes) else {
            return 0;
        };
        let stream_busy = self
            .sound
            .samples
            .iter()
            .any(|n| n.stream && self.node_playing(n));
        let streams = bytes.len() > STREAM_FROM && !stream_busy;
        if self.slots_full() {
            return 0;
        }
        let (duration, loops) = if streams {
            // The data bytes over the rate: ×200 for 8-bit, ×100 for the
            // rest (`0x6aa63`–`0x6aa94`); the stream's record takes no count.
            let data = bytes
                .len()
                .saturating_sub(motionvm_motion_formats::m32::wav::HEADER_LEN);
            (
                ticks_of(data, if wav.bits == 8 { 200 } else { 100 }, wav.rate),
                0,
            )
        } else {
            (ticks_of(bytes.len(), 200, wav.rate), loops)
        };
        self.start_node(&wav, &bytes, duration, streams, FILE_VOLUME, loops)
    }

    /// `STOPSAMPLE`: the node goes, and the sink's voice with it. A handle
    /// nothing started is nothing to stop.
    pub(crate) fn stop_sample(&mut self, handle: i32) {
        let Some(at) = self.sound.samples.iter().position(|n| n.handle == handle) else {
            return;
        };
        self.sound.samples.remove(at);
        if let Some(sink) = self.sound.sink.as_mut() {
            sink.stop_sample(handle);
        }
    }

    /// `?STIME`: the node's timer, or −1.
    pub(crate) fn sample_time(&mut self, handle: i32) -> i32 {
        let Some(at) = self.sound.samples.iter().position(|n| n.handle == handle) else {
            return -1;
        };
        let node = self.sound.samples[at].clone();
        if !self.node_playing(&node) {
            return -1;
        }
        let elapsed = self.unit3_since(node.started);
        // `duration × 130 / 100 < elapsed` marks the node finished
        // (`0x6b175`–`0x6b1b1`); at or below it, the elapsed time answers.
        let limit = i64::from(node.duration) * 130 / 100;
        if limit < i64::from(elapsed) {
            self.sound.samples[at].playing = false;
            return -1;
        }
        elapsed
    }

    /// `MUSVOLUME`: the music's volume, full or ducked.
    pub(crate) fn music_volume(&mut self, full: bool) {
        if let Some(sink) = self.sound.sink.as_mut() {
            sink.music_volume(if full { 0x7fff } else { 0x3800 });
        }
    }

    /// The samples still on the list — what a test reads.
    pub fn samples(&self) -> &[SampleNode] {
        &self.sound.samples
    }
}

/// `bytes × scale / rate`, as the start words compute a duration: unsigned
/// and truncating (`div`), saturated rather than wrapped where a file would
/// be absurdly long.
fn ticks_of(bytes: usize, scale: u64, rate: u32) -> i32 {
    let ticks = u64::try_from(bytes)
        .unwrap_or(u64::MAX)
        .saturating_mul(scale)
        / u64::from(rate.max(1));
    i32::try_from(ticks).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MusicSink, Profile};

    /// A canonical WAV of `frames` 16-bit mono frames at `rate`.
    fn wav(rate: u32, frames: usize) -> Vec<u8> {
        let pcm = vec![0u8; frames * 2];
        let mut f = Vec::new();
        f.extend_from_slice(b"RIFF");
        f.extend_from_slice(&u32::try_from(36 + pcm.len()).unwrap().to_le_bytes());
        f.extend_from_slice(b"WAVEfmt ");
        f.extend_from_slice(&16u32.to_le_bytes());
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&rate.to_le_bytes());
        f.extend_from_slice(&(rate * 2).to_le_bytes());
        f.extend_from_slice(&2u16.to_le_bytes());
        f.extend_from_slice(&16u16.to_le_bytes());
        f.extend_from_slice(b"data");
        f.extend_from_slice(&u32::try_from(pcm.len()).unwrap().to_le_bytes());
        f.extend_from_slice(&pcm);
        f
    }

    /// A sink that remembers what it was told.
    #[derive(Default)]
    struct Log(std::sync::Arc<std::sync::Mutex<Vec<String>>>);
    impl Log {
        fn note(&self, line: String) {
            self.0.lock().expect("the log").push(line);
        }
    }
    impl MusicSink for Log {
        fn start(&mut self, _: i32, _: i32, _: bool, _: &[u8]) {}
        fn stop(&mut self, _: i32) {}
        fn cut(&mut self, _: i32) {}
        fn sample(&mut self, _: i32, _: &[u8]) {}
        fn start_sample(&mut self, handle: i32, wav: &[u8], volume: u16, loops: i32) {
            self.note(format!("start {handle} {} {volume:#x} {loops}", wav.len()));
        }
        fn stop_sample(&mut self, handle: i32) {
            self.note(format!("stop {handle}"));
        }
        fn music_volume(&mut self, volume: u16) {
            self.note(format!("music {volume:#x}"));
        }
    }

    /// The arithmetic the words read back: a 22 050 Hz 16-bit sample of one
    /// second is 44 144 bytes, a duration of 400 ticks — twice its length —
    /// and it ends on the master after 1020 ticks; `?STIME` counts from
    /// zero while it plays and answers −1 once the DAC is dry. A second
    /// sample sounds beside the first.
    #[test]
    fn a_sample_runs_on_the_master_counter() {
        let mut e = Engine::new(Profile::motion32());
        let log = Log::default();
        let seen = std::sync::Arc::clone(&log.0);
        e.set_music(Box::new(log));
        let bytes = wav(22050, 22050);
        let w = Wav::parse(&bytes).unwrap();
        let duration = ticks_of(bytes.len(), 200, w.rate);
        assert_eq!(duration, 400);
        let h = e.start_node(&w, &bytes, duration, false, BLOCK_VOLUME, 0);
        assert_eq!(h, 1);
        assert_eq!(e.sample_time(h), 0);
        e.master_ticks += 510;
        assert_eq!(e.sample_time(h), 100, "half a second, in 200 Hz ticks");
        assert!(!e.slots_full());
        assert_eq!(
            e.start_node(&w, &bytes, duration, false, BLOCK_VOLUME, 0),
            2
        );
        e.master_ticks += 510;
        assert_eq!(e.sample_time(h), -1, "the first is dry");
        assert_eq!(e.sample_time(2), 100, "the second is half way");
        e.master_ticks += 510;
        assert_eq!(e.sample_time(2), -1, "both are dry");
        e.stop_sample(2);
        e.music_volume(false);
        assert_eq!(
            *seen.lock().expect("the log"),
            [
                "start 1 44144 0x1fff 0",
                "start 2 44144 0x1fff 0",
                "stop 2",
                "music 0x3800"
            ]
        );
        assert_eq!(e.sample_time(99), -1, "a handle nothing started");
    }

    /// The loop count on the master counter: two loops are three passes,
    /// 3060 ticks for the second-long sample, and a negative count never
    /// runs dry — the sample sounds until `STOPSAMPLE`, though `?STIME`'s
    /// 1.3× rule finishes the node all the same.
    #[test]
    fn the_loop_count_stretches_the_end() {
        let mut e = Engine::new(Profile::motion32());
        let log = Log::default();
        let seen = std::sync::Arc::clone(&log.0);
        e.set_music(Box::new(log));
        let bytes = wav(22050, 22050);
        let w = Wav::parse(&bytes).unwrap();
        e.start_node(&w, &bytes, 400, false, BLOCK_VOLUME, 2);
        let endless = e.start_node(&w, &bytes, 400, false, BLOCK_VOLUME, -1);
        assert_eq!(e.samples()[0].ends, 3060);
        assert_eq!(e.samples()[1].ends, u64::MAX);
        e.master_ticks += 3059;
        assert!(e.node_playing(&e.samples()[0].clone()), "the third pass");
        e.master_ticks += 1;
        assert!(
            !e.node_playing(&e.samples()[0].clone()),
            "three passes over"
        );
        assert!(
            e.node_playing(&e.samples()[1].clone()),
            "the endless one goes on"
        );
        assert_eq!(
            e.sample_time(endless),
            -1,
            "but its timer is past 1.3 durations"
        );
        e.stop_sample(endless);
        assert_eq!(
            *seen.lock().expect("the log"),
            [
                "start 1 44144 0x1fff 2",
                "start 2 44144 0x1fff -1",
                "stop 2"
            ]
        );
    }

    /// The layer's 34 slots: the thirty-fifth start while all sound is
    /// refused, and one running dry frees its slot.
    #[test]
    fn a_start_is_refused_only_with_every_slot_taken() {
        let mut e = Engine::new(Profile::motion32());
        e.set_music(Box::new(Log::default()));
        let bytes = wav(22050, 22050);
        let w = Wav::parse(&bytes).unwrap();
        for _ in 0..SLOTS - 1 {
            e.start_node(&w, &bytes, 400, false, BLOCK_VOLUME, 0);
        }
        assert!(!e.slots_full(), "one slot left");
        e.master_ticks += 500;
        e.start_node(&w, &bytes, 400, false, BLOCK_VOLUME, 0);
        assert!(e.slots_full(), "none left");
        e.master_ticks += 600;
        assert!(!e.slots_full(), "the first thirty-three ran dry");
    }

    /// `?SOUND` is the layer's digital status bit: 0 with no sound layer,
    /// 1 once a sink is attached — and the start words answer 0 without
    /// one, as the original's do with no card configured.
    #[test]
    fn q_sound_answers_for_the_sound_layer() {
        let mut mem = motionvm_motion_forth::m32::Memory::default();
        let mut e = Engine::new(Profile::motion32());
        let mut stack = Vec::new();
        e.plain_word32("?SOUND", &mut stack, &mut mem).unwrap();
        assert_eq!(stack, [0], "no layer, no digital driver");
        assert_eq!(e.start_block_sample(3, 0), 0, "and nothing starts");
        e.set_music(Box::new(Log::default()));
        stack.clear();
        e.plain_word32("?SOUND", &mut stack, &mut mem).unwrap();
        assert_eq!(stack, [1], "a sink is the layer with its driver up");
    }

    /// The 1.3× rule: a node whose timer has run past 1.3 durations is
    /// finished even while the DAC would still be playing — which only
    /// happens for a duration shorter than the sample, as the stream path
    /// computes for 16-bit files.
    #[test]
    fn the_thirty_percent_margin_finishes_a_node() {
        let mut e = Engine::new(Profile::motion32());
        let bytes = wav(22050, 22050);
        let w = Wav::parse(&bytes).unwrap();
        let h = e.start_node(&w, &bytes, 10, false, FILE_VOLUME, 0);
        // Thirteen 200 Hz ticks are 66.3 master ticks; 67 read as 13, the
        // limit itself, and six more read as 14.
        e.master_ticks += 67;
        assert_eq!(e.sample_time(h), 13, "at the limit, still counting");
        e.master_ticks += 6;
        assert_eq!(e.sample_time(h), -1, "past it, finished");
        assert_eq!(e.sample_time(h), -1, "and it stays finished");
    }
}
