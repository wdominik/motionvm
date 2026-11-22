//! The original's scaled sampling.
//!
//! The object blitter (`0BB6h:000Ah`) sizes an object's box from its two
//! scale words and hands the cel to the scaler to be resampled into a
//! scratch buffer before the blit. In `XP01` the scaler is segment `1750h`
//! (`0x17500`): the routine at `0x17930` sizes the picture and calls into
//! its core at `cs:0038h` (`0x179BF`). `XP00` carries the same core as
//! segment `089Eh`: the 430h bytes from `0x89E0` of its image are those
//! from `0x17500` of `XP01`'s, byte for byte, the dispatch tables at
//! `cs:18h` and `cs:28h` included. Its own entry `089Eh:0430h`
//! (`0x8E10`) stores its arguments into the scratch and calls the same
//! `cs:0038h` (`0x8E3E`), so the title program's wave columns sample
//! through the same [`columns`] and [`rows`]. It is hand-written assembly with a
//! self-modifying scratch at the start of its own segment; the arithmetic
//! is what this module reproduces.
//!
//! The scaler picks one of four cases per axis, by the ratio of source to
//! destination, and dispatches through two jump tables at `cs:18h`
//! (columns) and `cs:28h` (rows):
//!
//! | Case | Step it computes | Table entry |
//! |---|---|---|
//! | `src == dst` | none | `rep movsb`, `add si,srcW` |
//! | `src < dst` | `(src << 16) / dst`, a 0.16 fraction | magnify |
//! | `src < 2*dst` | `(src << 16) / (2*dst)`, then doubled | minify below 2 |
//! | otherwise | `(src << 8) / dst`, 8.8, `FFFFh` on overflow | minify from 2 |
//!
//! Every accumulator but one starts at **half a step** — `shr dx,1` in the
//! magnifier, `stc` + `rcr dx,1` in the near minifier (which rebuilds
//! `r/2` from the stored fraction), `shr dl,1` on the integer part in the
//! far minifier. That is a sample at the center of the destination pixel,
//! not at its left edge, and it is what separates this from a plain
//! `dst * src / dstLen`. The exception is the far minifier's **row** loop,
//! which sets its accumulator byte to 0 and starts on row 0.
//!
//! Mirroring is not part of the sampling: the mirrored table entries walk
//! the destination backwards (`std`, `add di,dstW`) over the same source
//! order, and the vertical flip does the same with rows.

use crate::num;

/// One axis of the **scaler's** own box: `(src * scale + 7Fh) >> 8`.
///
/// `add ax,7Fh` / `adc dx,0` inside the scaler, once per axis. The
/// `+7Fh` rounds, and `or dh,dh` / `stc` fails the whole call when the
/// product does not fit in 24 bits, which is why anything that wide
/// returns 0 here.
///
/// This is the size of the picture the scaler hands back — **not** the
/// box a figure is aligned on. That is [`aligned_extent`], and the two
/// differ by a row or a column wherever the fraction reaches a half.
pub(crate) fn scaled_extent(src: u16, scale: u16) -> u16 {
    let product = u32::from(src) * u32::from(scale) + 0x7F;
    if product >> 24 != 0 {
        return 0;
    }
    let v = product >> 8;
    if v >= 0x100 { 0xFF } else { num::low_word(v) }
}

/// One axis of the box a figure is **aligned** on: `(src * scale) >> 8`,
/// clamped to `0FFh`, with no rounding term.
///
/// Where the fraction reaches a half, the picture [`scaled_extent`] hands
/// back is one row taller than this box, so a figure aligned on it with
/// its whole height above its feet ends on its node's own row: at scale
/// `84h` a 50-row cel aligns on 25 rows and draws 26.
pub(crate) fn aligned_extent(src: u16, scale: u16) -> u16 {
    let v = (u32::from(src) * u32::from(scale)) >> 8;
    if v >= 0x100 { 0xFF } else { num::low_word(v) }
}

/// `(src << 8) / dst` as the far minifier computes it, `FFFFh` when the
/// division would not fit.
fn step_8_8(src: u16, dst: u16) -> u16 {
    if u32::from(src >> 8) >= u32::from(dst) {
        return 0xFFFF;
    }
    num::low_word((u32::from(src) << 8) / u32::from(dst))
}

/// Which source column each destination column samples — the `cs:18h`
/// table, unmirrored.
pub(crate) fn columns(src: u16, dst: u16) -> Vec<u16> {
    let mut out = Vec::with_capacity(usize::from(dst));
    if dst == 0 || src == 0 {
        return out;
    }
    if src == dst {
        // One `rep movsb` and rewind.
        out.extend(0..dst);
        return out;
    }
    if src < dst {
        // `movsb` then `adc si,0FFFFh` — the column advances
        // only when the accumulator wraps.
        let step = num::low_word((u32::from(src) << 16) / u32::from(dst));
        let mut acc = step >> 1;
        let mut si = 0u16;
        for _ in 0..dst {
            out.push(si);
            let (next, carry) = acc.overflowing_add(step);
            acc = next;
            si = si.wrapping_add(u16::from(carry));
        }
        return out;
    }
    if u32::from(src) < 2 * u32::from(dst) {
        // `movsb` then `adc si,0` — one column always, a second
        // one on the wrap. `stc` + `rcr` seeds the accumulator with `r/2`,
        // which is exactly the halved step the divider produced.
        let half = num::low_word((u32::from(src) << 16) / (2 * u32::from(dst)));
        let step = half.wrapping_shl(1);
        let mut acc = half;
        let mut si = 0u16;
        for _ in 0..dst {
            out.push(si);
            let (next, carry) = acc.overflowing_add(step);
            acc = next;
            si = si.wrapping_add(1).wrapping_add(u16::from(carry));
        }
        return out;
    }
    // Destination column 0 is source column 0, then the column
    // jumps to half the integer step and the 8-bit accumulator is seeded
    // with that same half — `xchg bh,dl` puts it where the fraction goes.
    let step = step_8_8(src, dst);
    let int = step >> 8;
    let frac = num::low8(step);
    let mut si = 0u16;
    out.push(si);
    si = int >> 1;
    let mut acc = num::low8(int >> 1);
    for _ in 1..dst {
        out.push(si);
        let (next, carry) = acc.overflowing_add(frac);
        acc = next;
        si = si
            .wrapping_add(1)
            .wrapping_add(int.wrapping_sub(1))
            .wrapping_add(u16::from(carry));
    }
    out
}

/// Which source row each destination row samples — the `cs:28h` table,
/// unflipped.
pub(crate) fn rows(src: u16, dst: u16) -> Vec<u16> {
    let mut out = Vec::with_capacity(usize::from(dst));
    if dst == 0 || src == 0 {
        return out;
    }
    if src == dst {
        // `add si,srcW` per destination row.
        out.extend(0..dst);
        return out;
    }
    if src < dst {
        // The source row advances on the wrap; without one the
        // written row is copied to the next with `rep movsb`.
        let step = num::low_word((u32::from(src) << 16) / u32::from(dst));
        let mut acc = step >> 1;
        let mut si = 0u16;
        for _ in 0..dst {
            out.push(si);
            let (next, carry) = acc.overflowing_add(step);
            acc = next;
            si = si.wrapping_add(u16::from(carry));
        }
        return out;
    }
    if u32::from(src) < 2 * u32::from(dst) {
        // `add si,srcW`, and a second one on the wrap. The
        // divider's own result is `r/2`, stored as the seed and doubled
        // into the step.
        let half = num::low_word((u32::from(src) << 16) / (2 * u32::from(dst)));
        let step = half.wrapping_shl(1);
        let mut acc = half;
        let mut si = 0u16;
        for _ in 0..dst {
            out.push(si);
            let (next, carry) = acc.overflowing_add(step);
            acc = next;
            si = si.wrapping_add(1).wrapping_add(u16::from(carry));
        }
        return out;
    }
    // The one loop with **no** half-step. The accumulator byte
    // starts at 0, row 0 is drawn before it is touched, and each further
    // row adds the 8.8 step and advances by the integer that carries out.
    let step = step_8_8(src, dst);
    let mut acc = 0u8;
    let mut si = 0u16;
    out.push(si);
    for _ in 1..dst {
        let sum = u16::from(acc) + step;
        acc = num::low8(sum);
        si = si.wrapping_add(sum >> 8);
        out.push(si);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The scaler rounds the box up at half a source pixel and refuses to grow
    /// past `0FFh`.
    #[test]
    fn the_box_rounds_at_half_and_caps_at_255() {
        assert_eq!(scaled_extent(29, 0x100), 29, "scale 1 is the cel itself");
        // 29 * 0x7F = 3683; +7Fh = 3810; >>8 = 14. Truncation gives 14 too,
        // but 50 * 0x7F = 6350, +7Fh = 6477, >>8 = 25 against 24.
        assert_eq!(scaled_extent(29, 0x7F), 14);
        assert_eq!(scaled_extent(50, 0x7F), 25);
        assert_eq!((50 * 0x7F) >> 8, 24, "plain truncation loses a row");
        assert_eq!(scaled_extent(50, 0x110), 53);
        // 29 * 80h = 3712, which is 80h above a multiple of 100h: the
        // listed `add ax,7Fh` stops one short of the next whole pixel
        // where an `add ax,80h` would round up to it.
        assert_eq!(scaled_extent(29, 0x80), 14);
        assert_eq!(scaled_extent(1, 0x80), 0, "half of one pixel is none");
        assert_eq!(scaled_extent(200, 0x400), 0xFF, "capped, not 800");
    }

    /// Same length in and out: the identity, on both axes.
    #[test]
    fn equal_lengths_are_the_identity() {
        assert_eq!(columns(29, 29), (0..29).collect::<Vec<_>>());
        assert_eq!(rows(50, 50), (0..50).collect::<Vec<_>>());
    }

    /// Every index a sampler produces has to be inside the cel, and it may
    /// never go backwards.
    #[test]
    fn the_samplers_stay_inside_the_cel_and_never_go_back() {
        for src in [1u16, 2, 3, 7, 16, 29, 50, 64, 100, 255] {
            for scale in [0x40u16, 0x5D, 0x80, 0xA0, 0xC0, 0xFF, 0x100, 0x110, 0x200] {
                let dst = scaled_extent(src, scale);
                for (what, v) in [("columns", columns(src, dst)), ("rows", rows(src, dst))] {
                    assert_eq!(v.len(), usize::from(dst), "{what} {src}->{dst}");
                    for w in v.windows(2) {
                        assert!(w[1] >= w[0], "{what} {src}->{dst} went back: {v:?}");
                    }
                    if let Some(&last) = v.last() {
                        assert!(last < src, "{what} {src}->{dst} ran off: {v:?}");
                    }
                }
            }
        }
    }

    /// Half-step sampling is the point: a 2:1 magnify takes the first
    /// source pixel once, not twice, because the first destination pixel
    /// already sits half a step in.
    #[test]
    fn magnifying_samples_at_the_pixel_centre() {
        // 2 -> 4: step = 0.5, accumulator starts at 0.25 and wraps on the
        // second, fourth … destination pixel.
        assert_eq!(columns(2, 4), vec![0, 0, 1, 1]);
        assert_eq!(rows(2, 4), vec![0, 0, 1, 1]);
        // 4 -> 5: step 0.8, accumulator 0.4. The doubled column is the
        // second, not the first — a left-edge sampler would repeat column
        // 0 instead.
        assert_eq!(columns(4, 5), vec![0, 1, 1, 2, 3]);
        assert_eq!(
            (0..5).map(|d| d * 4 / 5).collect::<Vec<_>>(),
            vec![0, 0, 1, 2, 3],
            "the left-edge sampler this replaces"
        );
    }

    /// Minifying by less than two drops every other source pixel from the
    /// middle out; by two or more the row loop starts flat at 0 while the
    /// column loop starts half an integer step in.
    #[test]
    fn the_two_minifiers_differ_where_the_listing_differs() {
        assert_eq!(columns(4, 3), vec![0, 1, 3]);
        assert_eq!(rows(4, 3), vec![0, 1, 3]);
        // 8 -> 2: step 4.0. The rows start at 0, the columns at 0 then
        // jump to half the step.
        assert_eq!(rows(8, 2), vec![0, 4]);
        assert_eq!(columns(8, 2), vec![0, 2]);
        // 9 -> 2 has a half-pixel fraction, so a row accumulator seeded
        // anywhere but 0 would carry it into the second row.
        assert_eq!(rows(9, 2), vec![0, 4]);
        assert_eq!(columns(9, 2), vec![0, 2]);
    }
}
