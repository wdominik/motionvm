//! One digital sample on the sample clock: the direct-DMA path of the
//! `DMA*.DRV` drivers, as the games install them.
//!
//! `STERN.EXE` installs its digital driver with one channel — the sixth
//! install argument is what the sound manager's trampoline leaves in that
//! slot, the caller's `DI`, which is 1 at the one call site (`1058:01bd`) —
//! and with one channel the driver's play entry takes the single-sample path
//! (`DMABLAST.DRV` `0x2028` → `0x0611`): the header's length word becomes the
//! DMA count, its period word — clamped to 255 (`0x0653`) — indexes the table
//! at `0x60` for the DSP's time constant (`0x065b`), and the bytes go to the
//! DAC as they are, unsigned 8-bit, once, at full scale — no volume, no
//! mixing. When the transfer ends the interrupt handler writes `0x80` to the
//! DAC (`0x0a06`), the mid-point, and the output is silent.
//!
//! The clock the DAC then runs on is the DSP's, not the PIT's. A Sound
//! Blaster DSP plays one sample every `256 − constant` microseconds, and the
//! table is the header's period — PIT cycles — in whole microseconds at
//! 1.193 cycles each, [`time_constant`] below. So a period of 149, which is
//! 8008 Hz on the PIT, plays at 8000 Hz; a recording of the original under
//! DOSBox-X runs 2.3 ms ahead of the PIT reading after 2.3 s, and exactly
//! on the DSP's.
//!
//! The rebuild does the DAC's work at the device rate: every output frame
//! advances the DSP clock by a second's worth of microseconds against the
//! sample's period, the byte under the clock is held until the next is due
//! — a zero-order hold, which is what a DAC fed by DMA is — and its value,
//! centred and widened to sixteen bits, is added to both channels of the
//! frame the OPL has already filled. Full scale for the eight bits is a
//! choice the analog mixer of the card made and no file records; it is the
//! level DOSBox-X gives the DAC, and the same recording peaks where the
//! rebuild does.

use motionvm_motion_formats::m16::psm::Sample;

/// The DSP's clock: its time constant counts microseconds.
const MICROS_PER_SECOND: u64 = 1_000_000;

/// The digital driver's table at `0x60`: the DSP time constant for a header
/// period, `256 − round(period · 1000 / 1193)` — the period in whole
/// microseconds at 1.193 PIT cycles each, from the top of the byte. Every one
/// of the 256 entries is this formula, in all four `DMA*.DRV` drivers alike;
/// the constant is 1.193 and not the clock's 1.193182, which at periods 139,
/// 207 and 244 rounds the other way. The driver clamps the period to the
/// table's last entry before the lookup (`0x0653`), and entry 0 is `0xff`:
/// no sample is shorter than a microsecond.
#[must_use]
pub fn time_constant(period: u16) -> u8 {
    let period = u32::from(period.min(255));
    let micros = u8::try_from((period * 1000 + 1193 / 2) / 1193).unwrap_or(u8::MAX);
    255 - micros.saturating_sub(1)
}

/// A sample on its way to the DAC.
#[derive(Debug)]
pub struct Voice {
    /// The PCM, unsigned 8-bit, as the block holds it.
    pcm: Vec<u8>,
    /// One source sample's length in microseconds times the output rate —
    /// the unit the clock below counts in, so that every side stays whole.
    unit: u64,
    /// Microseconds times rate accumulated toward the next source sample.
    acc: u64,
    /// The byte under the clock.
    pos: usize,
}

impl Voice {
    /// A sample about to play at output rate `rate`, on the DSP's clock:
    /// one byte every `256 − time_constant` microseconds.
    pub fn new(sample: Sample, rate: u32) -> Self {
        let micros = 256 - u64::from(time_constant(sample.period));
        Self {
            pcm: sample.pcm,
            unit: micros * u64::from(rate),
            acc: 0,
            pos: 0,
        }
    }

    /// Adds the sample's next `out.len() / 2` frames into `out`, interleaved
    /// stereo, and answers whether any of the sample is left to play.
    ///
    /// The clock: a frame is a million microseconds at the output rate, a
    /// source sample `256 − time_constant` of them, both multiplied through
    /// by the rate so the comparison is exact. The byte is held until the
    /// accumulator has run past a whole source sample, then the next byte is
    /// under the clock.
    pub fn mix(&mut self, out: &mut [i16]) -> bool {
        let step = MICROS_PER_SECOND;
        for frame in out.as_chunks_mut::<2>().0 {
            let Some(&byte) = self.pcm.get(self.pos) else {
                return false;
            };
            let value = (i16::from(byte) - 128) << 8;
            for channel in frame {
                *channel = channel.saturating_add(value);
            }
            self.acc += step;
            while self.acc >= self.unit {
                self.acc -= self.unit;
                self.pos += 1;
            }
        }
        self.pos < self.pcm.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{MICROS_PER_SECOND, Voice, time_constant};
    use motionvm_motion_formats::m16::psm::Sample;

    fn sample(period: u16, pcm: &[u8]) -> Sample {
        Sample {
            version: 0x0100,
            period,
            pcm: pcm.to_vec(),
        }
    }

    /// The driver's table, entry for entry where it matters: the ends, the
    /// clamp past the last entry, the shipped range's extremes, and the three
    /// periods where 1.193 and 1.193182 round apart.
    #[test]
    fn the_time_constant_is_the_drivers_table() {
        for (period, constant) in [
            (0, 0xff),
            (1, 0xff),
            (2, 0xfe),
            (56, 0xd1),
            (59, 0xcf),
            (83, 0xba),
            (139, 0x8b),
            (149, 0x83),
            (179, 0x6a),
            (207, 0x52),
            (244, 0x33),
            (255, 0x2a),
            (256, 0x2a),
            (u16::MAX, 0x2a),
        ] {
            assert_eq!(time_constant(period), constant, "period {period}");
        }
    }

    /// Each byte lasts `(256 − constant) × rate / 1 000 000` frames, held:
    /// a two-byte sample at a period of 59 — constant `0xcf`, 49 µs — is
    /// 2 × 2.16 frames at 44 100 Hz: the first byte under three frames, the
    /// second under the next two (the clock carries the remainder over), and
    /// the fifth frame is the end.
    #[test]
    fn a_byte_is_held_for_its_period() {
        let mut v = Voice::new(sample(59, &[0x80 + 64, 0x80 - 64]), 44_100);
        let mut out = [0i16; 14];
        assert!(!v.mix(&mut out[..10]), "the fifth frame is the last");
        assert_eq!(&out[..6], &[16384; 6]);
        assert_eq!(&out[6..10], &[-16384; 4]);
        assert!(!v.mix(&mut out[10..]), "and nothing follows");
        assert_eq!(&out[10..], &[0, 0, 0, 0]);
    }

    /// The whole sample lasts `len × (256 − constant)` microseconds whatever
    /// the output rate: 1000 bytes at a period of 149 — constant `0x83`,
    /// 125 µs, where the PIT would say 124.9 — are 125 ms, 5512.5 frames at
    /// 44 100 Hz, so 5513 are touched, and 2757 at 22 050.
    #[test]
    fn the_length_in_frames_follows_the_period() {
        for (rate, frames) in [(44_100u32, 5513usize), (22_050, 2757)] {
            let mut v = Voice::new(sample(149, &[0x80; 1000]), rate);
            let mut count = 0;
            let mut out = [0i16; 2];
            while v.mix(&mut out) {
                count += 1;
            }
            let expected = 1000 * 125 * u64::from(rate) / MICROS_PER_SECOND;
            assert_eq!(frames, count + 1, "at {rate} Hz");
            assert!(
                (u64::try_from(count + 1).unwrap()).abs_diff(expected) <= 1,
                "{count} frames at {rate} Hz against {expected}"
            );
        }
    }

    /// The mid-point byte adds nothing, and a loud byte saturates rather than
    /// wrapping over what the OPL already put in the frame.
    #[test]
    fn the_sample_is_added_and_saturates() {
        let mut v = Voice::new(sample(100, &[0x80, 0xff]), 8_000);
        let mut out = [30_000i16, -30_000, 30_000, -30_000];
        v.mix(&mut out);
        assert_eq!(out, [30_000, -30_000, 32_767, 2_512]);
    }
}
