//! Stateful panel DAC selection, read from `XP01`'s `0C1C:0202`
//! (`0x0C3C2`, which loads a variant only when it differs from the one at
//! `[4E1h]`; called at `0x0C4A1`, `0x0C97A`, `0x0C9D3`) and
//! `0C3F:005C/0581/05DA/0659`.

use crate::num;
use motionvm_render::Palette;

use crate::error::Result;

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
}

impl PanelColors {
    /// Builds the two transformed buffers that `0x0C32E..0x0C36A`
    /// derives from the panel resource, retaining the original as variant 2.
    pub(crate) fn new(bank: &[u8], curves: PanelCurves) -> Result<Self> {
        if bank.len() < 2
            || bank[1] == 0
            || bank.len() != 2 + usize::from(bank[1]) * 3
            || usize::from(bank[0]) + usize::from(bank[1]) > 256
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
            start: bank[0],
            variants,
            selected: 2,
            mask: 0,
            release: 0,
            held: 0,
            release_held: 0,
            frozen: 0,
            refresh: 0,
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
            if self.held == 0 {
                self.selected = if self.mask == 0 { 2 } else { 0 };
            }
        }
    }

    /// `0x0C3E8`: replace only this bank's RGB, preserving other registers.
    pub(crate) fn apply(&self, dac: &mut Palette) {
        dac.load(self.start, &self.variants[self.selected]);
    }
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
