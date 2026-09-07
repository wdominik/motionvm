//! One digital sample of the 32-bit engine on the output clock: the WAV a
//! sample block or a speech file holds, played at the sound layer's volume
//! over the music, as many times as its loop count says.
//!
//! The 32-bit engine's digital layer is HMI's SOS driving the card's DSP,
//! and its mixer is in `ENGINE.EXE` (R78 `0x82b70`–`0x82f01`), read there:
//!
//! - **The driver runs at 22 050 Hz, 16-bit signed stereo.** The layer's
//!   init parses `HMISET.CFG` to a rate of 11 025 (`0x6f7ed`), and the
//!   sample layer's init at once asks for 100 % more (`0x6a406` →
//!   `0x6fdc0`), which re-initializes the driver at 22 050 (`0x6feb3`); the
//!   SB16 driver starts the DSP with `0xb4`, 16-bit auto-init output, and
//!   mode `0x30`, signed stereo. Every file and block Checker 2000 ships is
//!   22 050 Hz 16-bit mono, so the mixer never resamples them — its
//!   resampler, a 16.16 fixed-point step of `rate / 22 050` holding each
//!   source frame (`0x82c74`–`0x82cc7`), is reached by no shipped data.
//! - **A sample starts at a volume on the layer's `0x7fff` scale** —
//!   `0x1fff` for a block (`STARTSAMPLE`, `0x6ae8f`), `0x7fff` for
//!   a file (`->STARTSAMPLE`, `0x6ab48`) — which `sosDIGISetSampleVolume`
//!   (`0x76381`) files as one dword at `+0x2c` of the sample's slot, left in
//!   the high word and right in the low, the engine passing the same value
//!   for both; the pan at `+0x44` stays at its data default `0x8000`, the
//!   center, so the two volumes are used as given (`0x82bc6`–`0x82bec`).
//! - **The mix** (routines out of the table at `0x877d1`, indexed by the
//!   source's format, the output's and whether the volume is full): a mono
//!   frame goes into both of two 32-bit accumulators, **unchanged when both
//!   volumes are `0x7ff0` or more** (`0x87c9c`), and otherwise as
//!   `2 × ⌊frame × volume / 65 536⌋` — `imulw` by the volume, the product's
//!   high word doubled (`0x881c6`). An 8-bit source is widened by its byte
//!   into the high half, unsigned ones through `xor 0x8000`.
//! - **Every sounding sample goes into the same accumulators** — the layer
//!   keeps 34 sample slots (`0x6ee60` takes the first free or finished one)
//!   and the mixer walks all of them — which **go out as 16-bit** (routines
//!   at `0x89715`), clipped to the range only when more than one sample was
//!   mixed (`0x89d73`), and plainly otherwise — one sample cannot overflow.
//! - **When a sample's data runs out** (`0x82d28`–`0x82d3c`) the mixer reads
//!   the loop count at `+0x30` of its record, the start words' second
//!   argument: −1 rewinds for ever, 0 ends the sample, any other count
//!   rewinds and counts down.
//!
//! What the card then does is analog: the DSP's output and the OPL3's meet
//! in the SB16's mixer, at the levels its registers hold, which nothing in
//! the drivers changes. Here each voice accumulates into a 32-bit frame the
//! player holds — every output frame moves a source clock on by the file's
//! rate against the device's and holds the frame under the clock, the DAC's
//! zero-order hold, straight to the device rate, which for a 22 050 Hz file
//! is the one conversion the original's chain has too — scaled as the mixer
//! scales it; the player clips the sum to sixteen bits and adds it to the
//! frame the OPL has already filled, with saturation, the sum of the two
//! paths as a DAC of the device would clip it.

use motionvm_motion_formats::m32::Wav;

/// What the game hands the player: the file, the level, how often, and the
/// handle `STOPSAMPLE` will name it by.
#[derive(Debug, Clone)]
pub struct Sample {
    /// The engine's handle for the sample, the token a stop names.
    pub handle: i32,
    /// The WAV, header read.
    pub wav: Wav,
    /// The sound layer's volume, `0`…`0x7fff`.
    pub volume: u16,
    /// The loop count at `+0x30` of the layer's record: 0 plays the sample
    /// once, a positive count that many times more, a negative one for ever.
    pub loops: i32,
}

/// A sample on its way to the DAC.
#[derive(Debug)]
pub struct Voice {
    /// The engine's handle, which a stop names.
    handle: i32,
    /// The PCM as signed 16-bit frames, left and right — a mono file on both.
    frames: Vec<[i16; 2]>,
    /// The file's rate, in frames per second.
    rate: u64,
    /// The device's rate.
    out: u64,
    /// Source frames accumulated toward the next, scaled by the device rate.
    acc: u64,
    /// The frame under the clock.
    pos: usize,
    /// The volume as a fraction of `0x7fff`.
    volume: i32,
    /// Passes still to come after this one; negative for ever.
    loops: i32,
}

impl Voice {
    /// A sample about to play at output rate `rate`.
    pub fn new(sample: Sample, rate: u32) -> Self {
        let Sample {
            handle,
            wav,
            volume,
            loops,
        } = sample;
        let channels = usize::from(wav.channels.max(1));
        let wide = wav.bits == 16;
        let per = if wide { 2 } else { 1 };
        let frames = wav
            .pcm
            .chunks_exact(channels * per)
            .map(|frame| {
                let value = |c: usize| -> i16 {
                    let at = c.min(channels - 1) * per;
                    if wide {
                        i16::from_le_bytes([frame[at], frame[at + 1]])
                    } else {
                        // Unsigned 8-bit, centred and widened, as the DAC
                        // takes it.
                        (i16::from(frame[at]) - 128) << 8
                    }
                };
                [value(0), value(1)]
            })
            .collect();
        Self {
            handle,
            frames,
            rate: u64::from(wav.rate.max(1)),
            out: u64::from(rate.max(1)),
            acc: 0,
            pos: 0,
            volume: i32::from(volume.min(0x7fff)),
            loops,
        }
    }

    /// The handle the sample was started under.
    pub fn handle(&self) -> i32 {
        self.handle
    }

    /// Adds the sample's next `out.len() / 2` frames into the accumulators
    /// `out`, interleaved stereo, and answers whether any of the sample is
    /// left to play. When the data runs out the loop count decides, as the
    /// mixer's end check does: below zero the sample rewinds, at zero it is
    /// over, above it rewinds and counts the pass.
    pub fn mix(&mut self, out: &mut [i32]) -> bool {
        for frame in out.as_chunks_mut::<2>().0 {
            if self.pos >= self.frames.len() && !self.rewind() {
                return false;
            }
            let [left, right] = self.frames[self.pos];
            for (channel, value) in frame.iter_mut().zip([left, right]) {
                *channel += scale(value, self.volume);
            }
            self.acc += self.rate;
            while self.acc >= self.out {
                self.acc -= self.out;
                self.pos += 1;
            }
        }
        self.pos < self.frames.len() || self.loops != 0
    }

    /// The end of the data: another pass, or not.
    fn rewind(&mut self) -> bool {
        if self.frames.is_empty() || self.loops == 0 {
            return false;
        }
        if self.loops > 0 {
            self.loops -= 1;
        }
        self.pos = 0;
        true
    }
}

/// The layer's volume scaling on the way to the DAC.
///
/// The sound layer's volume is `0`…`0x7fff`. From `0x7ff0` up the mixer adds
/// the frame as it is (`0x82c2e`–`0x82c46` chooses the routine); below, it
/// takes the high word of the 16×16 signed product and doubles it — a
/// `>> 16` then `<< 1` on the product, arithmetic, so the result is even and
/// one below `frame × volume / 32 768` for the odd cases.
pub(crate) const FULL: i32 = 0x7ff0;

fn scale(frame: i16, volume: i32) -> i32 {
    if volume >= FULL {
        i32::from(frame)
    } else {
        ((i32::from(frame) * volume) >> 16) << 1
    }
}

#[cfg(test)]
mod tests {
    use super::{Sample, Voice, scale};
    use motionvm_motion_formats::m32::Wav;

    fn once(wav: Wav, volume: u16) -> Sample {
        Sample {
            handle: 1,
            wav,
            volume,
            loops: 0,
        }
    }

    fn mono16(rate: u32, samples: &[i16]) -> Wav {
        Wav {
            channels: 1,
            rate,
            bits: 16,
            pcm: samples.iter().flat_map(|s| s.to_le_bytes()).collect(),
        }
    }

    /// The mixer's arithmetic: as is from `0x7ff0` up, and the doubled high
    /// word of the product below — even, and a step below the plain
    /// `>> 15` on odd cases, negative values rounding toward −∞.
    #[test]
    fn the_volume_scales_as_the_mixers_routines_do() {
        assert_eq!(scale(1000, 0x7fff), 1000);
        assert_eq!(scale(1000, 0x7ff0), 1000, "the routine table's threshold");
        assert_eq!(scale(1000, 0x7fef), 998, "just below it, scaled");
        assert_eq!(scale(4000, 0x1fff), 998);
        assert_eq!(scale(-4000, 0x1fff), -1000, "toward minus infinity");
        assert_eq!(scale(1, 0x3800), 0);
        assert_eq!(scale(i16::MAX, 0x3800), 14334);
        assert_eq!(scale(i16::MIN, 0x3800), -14336);
    }

    /// At the device's own rate every source frame is one output frame, on
    /// both channels, and a quarter volume is a quarter of the value.
    #[test]
    fn frames_land_one_to_one_at_the_same_rate() {
        let mut v = Voice::new(once(mono16(44100, &[1000, -2000, 3000]), 0x7fff), 44100);
        let mut out = [0i32; 8];
        assert!(!v.mix(&mut out), "three frames into four: the sample ends");
        assert_eq!(
            &out[..6],
            &[1000, 1000, -2000, -2000, 3000, 3000],
            "full volume adds the frame as it is"
        );
        assert_eq!(&out[6..], &[0, 0]);
        let mut quarter = Voice::new(once(mono16(44100, &[4000]), 0x1fff), 44100);
        let mut out = [0i32; 2];
        quarter.mix(&mut out);
        assert_eq!(
            out,
            [998, 998],
            "0x1fff of 0x7fff: 4000 × 8191 = 32 764 000, whose high word 499 is doubled"
        );
    }

    /// Half the device rate holds every frame for two output frames.
    #[test]
    fn a_slower_file_is_held() {
        let mut v = Voice::new(once(mono16(22050, &[100, 200]), 0x7fff), 44100);
        let mut out = [0i32; 8];
        assert!(!v.mix(&mut out));
        assert_eq!(out, [100, 100, 100, 100, 200, 200, 200, 200]);
    }

    /// The loop count as the mixer's end check reads it: a count of two is
    /// three passes, and a negative count never ends.
    #[test]
    fn the_loop_count_rewinds_the_data() {
        let mut v = Voice::new(
            Sample {
                loops: 2,
                ..once(mono16(44100, &[1, 2]), 0x7fff)
            },
            44100,
        );
        let mut out = [0i32; 16];
        assert!(
            !v.mix(&mut out),
            "two frames three times is six, into eight"
        );
        assert_eq!(&out[..12], &[1, 1, 2, 2, 1, 1, 2, 2, 1, 1, 2, 2]);
        assert_eq!(&out[12..], &[0, 0, 0, 0]);

        let mut endless = Voice::new(
            Sample {
                loops: -1,
                ..once(mono16(44100, &[7]), 0x7fff)
            },
            44100,
        );
        let mut out = [0i32; 6];
        assert!(endless.mix(&mut out), "still going");
        assert_eq!(out, [7; 6]);
        assert!(endless.mix(&mut out), "and going");
    }

    /// Two voices into the same accumulators sum, and nothing clips on the
    /// way — that is the player's, once, over the sum.
    #[test]
    fn voices_accumulate() {
        let mut a = Voice::new(once(mono16(44100, &[30000]), 0x7fff), 44100);
        let mut b = Voice::new(once(mono16(44100, &[30000]), 0x7fff), 44100);
        let mut out = [0i32; 2];
        a.mix(&mut out);
        b.mix(&mut out);
        assert_eq!(out, [60000, 60000]);
    }

    /// Eight-bit files come off the unsigned scale, centred.
    #[test]
    fn eight_bit_is_centred() {
        let wav = Wav {
            channels: 1,
            rate: 44100,
            bits: 8,
            pcm: vec![128, 255, 0],
        };
        let mut v = Voice::new(once(wav, 0x7fff), 44100);
        let mut out = [0i32; 6];
        v.mix(&mut out);
        assert_eq!(out, [0, 0, 32512, 32512, -32768, -32768]);
    }
}
