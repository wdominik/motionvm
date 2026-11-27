//! Stateful panel DAC selection, read from `XP01`'s `0C1C:0202`
//! (`0x0C3C2`, which loads a variant only when it differs from the one at
//! `[4E1h]`; called at `0x0C4A1`, `0x0C97A`, `0x0C9D3`) and
//! `0C3F:005C/0581/05DA/0659`.

use crate::num;
use motionvm_render::Palette;

use crate::error::Result;
use crate::exe::Image;

/// Three channel curves for each of the two transformed panel variants.
/// Each curve is `(start, divisor)` for `0724:0360`.
pub(crate) type PanelCurves = [[[u8; 2]; 3]; 2];

/// Panel palette selection and the masks controlling its delayed release.
/// RGB writes here deliberately do not update the caption flatten lookup:
/// Dunkle Schatten's selector calls `0590:09E2`, not the resource loader
/// `0590:0A1F`.
#[derive(Debug, Clone)]
pub(crate) struct PanelColors {
    /// The bank's first DAC register.
    start: u8,
    /// Tinted, dimmed, and original RGB, indexed as `DS:1CC4h` pointers.
    variants: [Vec<u8>; 3],
    /// Current pointer index (`DS:04E1h`).
    selected: usize,
    /// `DS:04ECh`: owners requesting the tinted panel and flattened band.
    pub mask: u16,
    /// `DS:04EEh`: pending releases, applied on refresh expiry.
    release: u16,
    /// `DS:04F0h`: owners suppressing a panel redraw.
    held: u16,
    /// `DS:04F2h`: pending releases of redraw suppression.
    release_held: u16,
    /// `DS:04F4h`: owners freezing refresh, as a choice menu does.
    frozen: u16,
    /// `DS:04F6h`: panel refresh calls remaining, not elapsed PIT ticks.
    refresh: u16,
    /// Whether the variants go to the upper half of the palette, where the
    /// band held dark shows them, and the refresh never selects the
    /// original back ([`PanelPlace::Upper`]).
    upper: bool,
}

/// Where a build keeps the panel's variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelPlace {
    /// Over the panel's own colours, the original selected back when the
    /// owners let go (*Dunkle Schatten*'s `0x0C4A1`).
    Over,
    /// `80h` past them, the half the band's bit 7 shows: the loader puts
    /// variant 0 there (*Bi-Fi 2*'s `add ax, 80h`, `0x0744A`, and
    /// `0x074D2`), and the refresh selects nothing (`0x075A9`–`0x075C5`).
    Upper,
}

impl PanelColors {
    /// Builds the two transformed buffers that `0x0C32E..0x0C36A`
    /// derives from the panel resource, retaining the original as variant 2.
    pub(crate) fn new(bank: &[u8], curves: PanelCurves, place: PanelPlace) -> Result<Self> {
        let upper = place == PanelPlace::Upper;
        let shift = if upper { 0x80 } else { 0 };
        if bank.len() < 2
            || bank[1] == 0
            || bank.len() != 2 + usize::from(bank[1]) * 3
            || usize::from(bank[0]) + usize::from(bank[1]) + shift > 256
            || bank[2..].iter().any(|&c| c > 63)
        {
            return Err("invalid panel palette bank".into());
        }
        let mut variants = [Vec::new(), Vec::new(), bank[2..].to_vec()];
        for (variant, channels) in curves.into_iter().enumerate() {
            let tables = channels.map(|[start, divisor]| channel_curve(start, divisor));
            let [r, g, b] = tables;
            let tables = [r?, g?, b?];
            variants[variant] = bank[2..]
                .iter()
                .enumerate()
                .map(|(i, &c)| tables[i % 3][usize::from(c)])
                .collect();
        }
        Ok(Self {
            start: bank[0].wrapping_add(num::byte(shift)),
            variants,
            selected: if upper { 0 } else { 2 },
            mask: 0,
            release: 0,
            held: 0,
            release_held: 0,
            frozen: 0,
            refresh: 0,
            upper,
        })
    }

    /// `0x0C971`: tint immediately, cancel this owner's pending release,
    /// and thaw its menu freeze only on the branch without redraw holds.
    pub(crate) fn tint(&mut self, mask: u16) {
        self.selected = 0;
        self.release &= !mask;
        self.mask |= mask;
        if self.held != 0 {
            self.release_held |= mask;
            self.refresh = 3;
        } else if self.frozen & mask != 0 {
            self.frozen &= !mask;
            self.refresh = 3;
        } else {
            self.refresh = 0;
        }
    }

    /// `0x0C9CA`: select dimmed colors immediately and freeze refresh.
    pub(crate) fn dim(&mut self, mask: u16) {
        self.selected = 1;
        self.release &= !mask;
        self.mask |= mask;
        self.frozen |= mask;
        if self.held != 0 && self.held & !mask != 0 {
            self.release_held |= mask;
            self.refresh = 1;
        } else {
            self.held = 0;
            self.refresh = 0;
        }
    }

    /// `0x0CA29` (*Bi-Fi 2*'s `0x07B45`): the owner holds the redraw, its
    /// pending release of the hold is cancelled, and the countdown stops
    /// (`mov word [4F6h], 0`). The variant shown stays as it is.
    pub(crate) fn hold_redraw(&mut self, mask: u16) {
        self.held |= mask;
        self.release_held &= !mask;
        self.refresh = 0;
    }

    /// `0x0CA49`: release is deferred by three panel refresh calls.
    pub(crate) fn release(&mut self, mask: u16) {
        self.frozen &= !mask;
        self.release_held |= mask;
        self.release |= mask;
        self.refresh = 3;
    }

    /// `0x0C451..0x0C4A1`: a frozen menu stops even the countdown.
    pub(crate) fn frame(&mut self) {
        if self.frozen != 0 || self.refresh == 0 {
            return;
        }
        self.refresh -= 1;
        if self.refresh == 0 {
            self.held &= !self.release_held;
            self.release_held = 0;
            self.mask &= !self.release;
            self.release = 0;
            if self.held == 0 && !self.upper {
                self.selected = if self.mask == 0 { 2 } else { 0 };
            }
        }
    }

    /// `0x0C3E8`: replace only this bank's RGB, preserving other registers.
    pub(crate) fn apply(&self, dac: &mut Palette) {
        dac.load(self.start, &self.variants[self.selected]);
    }
}

/// The panel's two variants as the program builds them: channel rows 1
/// to 3, each by `push divisor ; push start ; push row ; call far` at
/// `channel_rows` (the far call row zero's at `row_zero` makes, eight
/// bytes on), and the variants' `push b ; push g ; push r` of row numbers
/// at `variants`. Captain Zins's rows are at `0x010F1`, `0x010FF`,
/// `0x0110D`, its variants at `0x077EF`, `0x07811`.
pub(crate) fn read_panel_curves(
    img: &Image,
    row_zero: usize,
    channel_rows: [(u8, usize); 3],
    variants: [usize; 2],
) -> Result<PanelCurves> {
    let push = |at: usize| -> Result<u8> {
        let b = img
            .bytes()
            .get(at..at + 2)
            .ok_or("truncated palette argument")?;
        if b[0] != 0x6A || b[1] >= 0x80 {
            return Err(format!("expected positive push imm8 at {at:#x}").into());
        }
        Ok(b[1])
    };
    let builder = img.bytes().get(row_zero + 8..row_zero + 13);
    let mut rows = [[0; 2]; 4];
    for (slot, at) in channel_rows {
        if push(at + 4)? != slot || img.bytes().get(at + 6..at + 11) != builder {
            return Err("unexpected panel channel initialization".into());
        }
        rows[usize::from(slot)] = [push(at + 2)?, push(at)?];
    }
    let mut out = [[[0; 2]; 3]; 2];
    for (variant, at) in variants.into_iter().enumerate() {
        for (channel, curve) in out[variant].iter_mut().enumerate() {
            let slot = usize::from(push(at + (2 - channel) * 2)?);
            if !(1..=3).contains(&slot) {
                return Err("unread panel channel row".into());
            }
            *curve = rows[slot];
        }
    }
    Ok(out)
}

/// Literal countdown arithmetic of `XP01.EXE`'s `0x075A0..0x075EE`. A
/// linear brightness multiplier does not reproduce the repeated output values.
pub(crate) fn channel_curve(start: u8, divisor: u8) -> Result<[u8; 64]> {
    if start > 63 || !(1..=64).contains(&divisor) {
        return Err("invalid panel channel curve".into());
    }
    let mut table = [0; 64];
    let mut value = start;
    let mut next = start;
    let mut countdown = 0u8;
    for (input, out) in table.iter_mut().enumerate() {
        *out = value;
        countdown = countdown.wrapping_sub(1);
        if countdown & 0x80 != 0 {
            next += 1;
            countdown = num::byte(input) / divisor;
            value = next.min(63);
        }
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic RGB inputs, with Dunkle Schatten's original channel parameters.
    fn panel() -> PanelColors {
        PanelColors::new(
            &[160, 2, 0, 0, 0, 63, 63, 63],
            [[[3, 30], [1, 25], [1, 25]], [[0, 8]; 3]],
            PanelPlace::Over,
        )
        .unwrap()
    }

    #[test]
    fn panel_release_waits_three_frames_and_preserves_other_owners() {
        let mut p = panel();
        p.tint(1);
        p.tint(4);
        p.release(1);
        p.frame();
        p.frame();
        assert_eq!(p.mask, 5);
        assert_eq!(p.selected, 0);
        p.frame();
        assert_eq!(p.mask, 4);
        p.release(4);
        p.frame();
        p.frame();
        p.frame();
        assert_eq!((p.mask, p.selected), (0, 2));
    }

    #[test]
    fn choice_freeze_stops_pending_release_until_the_choice_is_taken() {
        let mut p = panel();
        p.tint(4);
        p.release(4);
        p.dim(1);
        for _ in 0..20 {
            p.frame();
        }
        assert_eq!((p.mask, p.selected), (5, 1));
        p.tint(1);
        p.frame();
        p.frame();
        assert_eq!(p.mask, 5);
        p.frame();
        assert_eq!((p.mask, p.selected), (1, 0));
        p.release(1);
        p.frame();
        p.frame();
        p.frame();
        assert_eq!((p.mask, p.selected), (0, 2));
    }

    #[test]
    fn reacquiring_an_owner_cancels_its_pending_release() {
        let mut p = panel();
        p.tint(1);
        p.release(1);
        p.frame();
        p.tint(1);
        for _ in 0..5 {
            p.frame();
        }
        assert_eq!((p.mask, p.selected), (1, 0));
    }

    #[test]
    fn channel_countdowns_match_the_original_steps_and_only_write_the_panel_bank() {
        let red = channel_curve(3, 30).unwrap();
        let green = channel_curve(1, 25).unwrap();
        assert_eq!(&red[29..36], &[32, 33, 34, 34, 35, 35, 36]);
        assert_eq!(&green[24..31], &[25, 26, 27, 27, 28, 28, 29]);
        let mut p = panel();
        let mut dac = Palette::from_6bit(&[17; 768]);
        p.tint(1);
        p.apply(&mut dac);
        assert_eq!(&dac.raw[480..486], &[3, 1, 1, 49, 43, 43]);
        assert!(dac.raw[..480].iter().all(|&c| c == 17));
        assert!(dac.raw[486..].iter().all(|&c| c == 17));
        p.dim(1);
        p.apply(&mut dac);
        assert_eq!(&dac.raw[480..483], &[0, 0, 0]);
        p.release(1);
        p.frame();
        p.frame();
        p.frame();
        p.apply(&mut dac);
        assert_eq!(&dac.raw[480..486], &[0, 0, 0, 63, 63, 63]);
    }
}
