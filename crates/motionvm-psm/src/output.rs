//! The DSP's side of the mixer: the DMA buffer played a half at a time on
//! the DSP's clock, and held from that clock to the device's.
//!
//! File offsets in this module are `DMABLAST.DRV`'s.
//!
//! Every block goes to the DSP as one single-cycle transfer of 128 bytes
//! (`0x06d8`: command `0x14`, length `0x7f`), at the time constant of the
//! file (`0x40` + `[4A0h]`) — `0xd0`, a byte every 48 microseconds, 20 833⅓
//! a second, mono, 8-bit unsigned. The interrupt at the end of a block
//! starts the next, already mixed, and then mixes the one after it
//! (`0x07a7` → `0x1b10`). So while one block plays the next is ready: what a
//! host or a song does during a block is heard from the block after the
//! next one — 128 to 256 bytes later, always from the first byte of a
//! block. A start from rest mixes its first block and plays it at once
//! (`0x1bc7`), so a play into a quiet mixer is heard from the call.
//!
//! What the rebuild decides that the driver does not:
//!
//! - **Where the mixer's clock stands against the music's.** The music
//!   ticks on the PIT, the blocks on the DSP; the two meet only where a
//!   tick's callback writes channel state that the next block reads. The
//!   rebuild starts the DSP's clock at the frame a start from rest happens
//!   on and keeps both clocks exact from there; at a frame where a tick and
//!   a block's end fall together, the block ends first.
//! - **No gap between blocks.** Each block is its own transfer, and the
//!   interrupt takes some time before the next begins; how long is the
//!   machine's. The rebuild plays the blocks back to back.
//! - **The level against the OPL.** The card mixed its DAC and its OPL in
//!   analog, and no file records the balance. The rebuild adds each byte
//!   as the one-channel [`Voice`](crate::Voice) does — `(byte − 0x80) <<
//!   8`, saturating, onto both channels of the frame the OPL filled.
//! - **The DAC after the last block.** The interrupt that ends the
//!   transfer (`0x085e`) writes nothing to the DAC; the rebuild adds
//!   nothing after the last byte played.

use crate::mixer::{BLOCK, Irq, Mixer};

/// The DSP's clock counts microseconds.
const MICROS_PER_SECOND: u64 = 1_000_000;

/// The mixer and the transfer it feeds.
#[derive(Debug)]
pub(crate) struct Output {
    mixer: Mixer,
    /// One DMA byte's length in microseconds times the output rate.
    unit: u64,
    /// Whether a transfer runs.
    playing: bool,
    /// The byte under the DSP's clock, 0 to 255 in the buffer.
    at: usize,
    /// Microseconds times rate accumulated toward the next byte.
    clock: u64,
}

impl Output {
    /// The mixer, standing, for an output rate of `rate`.
    pub(crate) fn new(mixer: Mixer, time_constant: u8, rate: u32) -> Self {
        let micros = 256 - u64::from(time_constant);
        Self {
            mixer,
            unit: micros * u64::from(rate),
            playing: false,
            at: 0,
            clock: 0,
        }
    }

    /// The mixer, to call its entries through [`Output::with`].
    pub(crate) fn mixer(&self) -> &Mixer {
        &self.mixer
    }

    /// Runs one of the mixer's entries, and starts the transfer if the entry
    /// started the mixer from rest: on the first half, at this frame.
    pub(crate) fn with<R>(&mut self, entry: impl FnOnce(&mut Mixer) -> R) -> R {
        let r = entry(&mut self.mixer);
        if self.mixer.take_start() {
            self.playing = true;
            self.at = 0;
            self.clock = 0;
        }
        r
    }

    /// Whether a transfer runs.
    pub(crate) fn playing(&self) -> bool {
        self.playing
    }

    /// Adds the next `out.len() / 2` frames of the transfer into `out`,
    /// interleaved stereo; a block's end is the interrupt that plays the
    /// next.
    pub(crate) fn render(&mut self, out: &mut [i16]) {
        for frame in out.as_chunks_mut::<2>().0 {
            if !self.playing {
                return;
            }
            let value = (i16::from(self.mixer.buffer()[self.at]) - 128) << 8;
            for channel in frame {
                *channel = channel.saturating_add(value);
            }
            self.clock += MICROS_PER_SECOND;
            while self.playing && self.clock >= self.unit {
                self.clock -= self.unit;
                self.advance();
            }
        }
    }

    /// One byte on; at a block's end, the interrupt.
    fn advance(&mut self) {
        self.at += 1;
        if !self.at.is_multiple_of(BLOCK) {
            return;
        }
        match self.mixer.irq() {
            Irq::Next => self.at %= 2 * BLOCK,
            Irq::Restart => self.at = 0,
            Irq::End => self.playing = false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dma::{DmaDriver, tests::file};
    use crate::format::Sample;
    use std::sync::Arc;

    /// An output at 20 833⅓ × 3 = 62 500 Hz: three frames a DMA byte.
    fn output() -> Output {
        let d = DmaDriver::parse(&file()).expect("the driver");
        let mut mixer = Mixer::new(&d, 6);
        mixer.set_volume(0x100);
        Output::new(mixer, d.time_constant, 62_500)
    }

    fn sample(len: usize) -> Arc<Sample> {
        Arc::new(Sample {
            volume: 0x100,
            period: 100,
            pcm: vec![0xc0; len],
        })
    }

    /// Frames until the first non-silent one, rendering `frames` in all.
    fn onset(o: &mut Output, frames: usize) -> Option<usize> {
        let mut buf = vec![0i16; frames * 2];
        o.render(&mut buf);
        buf.chunks(2).position(|f| f[0] != 0)
    }

    /// A play into a quiet mixer is heard at once.
    #[test]
    fn a_play_from_rest_is_heard_at_once() {
        let mut o = output();
        o.with(|m| m.play(&sample(4000), 0));
        assert!(o.playing());
        assert_eq!(onset(&mut o, 10), Some(0));
    }

    /// A play while a block plays is heard from the block after the next:
    /// 40 bytes into block 0, a play lands at byte 256, the start of block
    /// 2 — 216 bytes, 648 frames on.
    #[test]
    fn a_play_while_running_waits_for_the_block_after_next() {
        let mut o = output();
        // A quiet sample keeps the mixer running.
        let quiet = Arc::new(Sample {
            volume: 0x100,
            period: 100,
            pcm: vec![0x80; 4000],
        });
        o.with(|m| m.play(&quiet, 0));
        assert_eq!(onset(&mut o, 120), None);
        o.with(|m| m.play(&sample(4000), 0));
        assert_eq!(onset(&mut o, 1000), Some(648));
    }

    /// `StopAll`: the block playing ends and the one mixed after it is not
    /// played; a play then starts at once again.
    #[test]
    fn stop_all_ends_with_the_block_playing() {
        let mut o = output();
        o.with(|m| m.play(&sample(4000), 0));
        let mut buf = vec![0i16; 30 * 2];
        o.render(&mut buf);
        o.with(Mixer::stop_all);
        let mut buf = vec![0i16; 800 * 2];
        o.render(&mut buf);
        let heard = buf.chunks(2).take_while(|f| f[0] != 0).count();
        assert_eq!(heard, 128 * 3 - 30, "the rest of block 0");
        assert!(!o.playing());
        o.with(|m| m.play(&sample(4000), 0));
        assert_eq!(onset(&mut o, 4), Some(0));
    }

    /// The transfer plays every byte for its 48 µs whatever the device
    /// rate: a sample of 1000 bytes at a period of 100 against 57.25 lasts
    /// about 1747 DMA bytes, and the transfer ends with the block it ends in
    /// or the one after.
    #[test]
    fn the_transfer_lasts_its_blocks() {
        for rate in [44_100u32, 48_000, 62_500] {
            let d = DmaDriver::parse(&file()).expect("the driver");
            let mut o = Output::new(Mixer::new(&d, 6), d.time_constant, rate);
            o.with(|m| m.play(&sample(1000), 0));
            let mut frames = 0u64;
            let mut buf = [0i16; 2];
            while o.playing() {
                o.render(&mut buf);
                frames += 1;
            }
            let bytes = frames * 1_000_000 / (48 * u64::from(rate));
            assert!((14 * 128..=15 * 128).contains(&bytes), "{bytes} at {rate}");
        }
    }
}
