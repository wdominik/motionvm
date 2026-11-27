//! The title programs' animation: a frame's deltas laid on the screen.

use motionvm_render::Framebuffer;

/// Captain Zins's `0221h:00A0h` (`0x22B0`), *Bi-Fi 2*'s `0215h:009Eh`
/// (`0x21EE`), the same code: a frame's deltas onto the screen. After the
/// width and height (unused), the offset on the screen and the count of
/// runs; each run a byte to go on by after it and a byte of how many to
/// copy (`lodsw`, `xchg ah, cl`), then those bytes.
pub(crate) fn apply_delta(screen: &mut Framebuffer, d: &[u8]) {
    let word = |at: usize| {
        d.get(at..at + 2)
            .map_or(0, |w| u16::from_le_bytes([w[0], w[1]]))
    };
    let mut di = usize::from(word(4));
    let runs = word(6);
    let mut si = 8;
    for _ in 0..runs {
        let (Some(&skip), Some(&count)) = (d.get(si), d.get(si + 1)) else {
            return;
        };
        si += 2;
        let n = usize::from(count);
        let (Some(src), Some(dst)) = (d.get(si..si + n), screen.pixels.get_mut(di..di + n)) else {
            return;
        };
        dst.copy_from_slice(src);
        si += n;
        di += n + usize::from(skip);
    }
}
