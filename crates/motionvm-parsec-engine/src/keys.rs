//! The keyboard, as a PARSEC program reads it: one keystroke at a time out
//! of the BIOS's type-ahead buffer.
//!
//! What arrives is the contract's [`KeyPress`] — the physical key, the
//! layout's character, the modifiers — and what the game is handed is the
//! one integer its module reads: the character where the
//! key types one, and `100h` plus the scan code where it does not. That is
//! the two halves of the word INT 16h AH=00 answers (AL the character, AH
//! the scan code), folded into one the way a Turbo-C program of its
//! day folds them. Escape is 27, Enter 13, Tab 9 and Backspace 8 —
//! the characters the BIOS gives those keys — and Shift-Tab, which the BIOS
//! reports as scan code `0Fh` without a character, is `10Fh`. What the game
//! makes of the integer is its library's reading: Dunkle Schatten's
//! `179Eh:0611h` turns Escape into `100h`, Enter into `101h`, Tab into `105h`
//! and Shift-Tab into `106h`.
//!
//! The scan codes are the US set-1 codes, the set every PC BIOS of the time
//! answers with: `48h`, `50h`, `4Bh` and `4Dh` for the four arrows.
//!
//! A key that types nothing and has no code here — a function key, Home, a
//! bare modifier — is not delivered: the game reads none, and inventing one
//! would put a way through a game that the game does not know about.
//!
//! **The buffer.** The BIOS keeps fifteen keystrokes in its ring at
//! `0040h:001Eh` and drops a sixteenth; a program takes them out one call
//! at a time. A press here goes into such a ring and the runtime hands the
//! game one keystroke a step, before the step runs, so a press between two
//! steps is read by the next one. How many keystrokes the original's
//! frame pass takes in one pass when more are waiting is not read; one a
//! step is this runtime's choice, and keys typed faster than the steps run
//! land in later steps instead of being lost.

use crate::num;
use std::collections::VecDeque;

use motionvm_playable::{Key, KeyPress};

/// Keystrokes the BIOS type-ahead buffer holds; a sixteenth is dropped.
pub(crate) const SLOTS: usize = 15;

/// The integer the game is handed for `press`, or `None` for a press the
/// game does not read.
pub(crate) fn code(press: &KeyPress) -> Option<i32> {
    let scan = |s: i32| Some(0x100 + s);
    match press.key {
        Key::Escape => Some(27),
        Key::Tab if press.shift => scan(0x0F),
        Key::Tab => Some(9),
        Key::Enter => Some(13),
        Key::Space => Some(i32::from(b' ')),
        Key::Backspace => Some(8),
        Key::Up => scan(0x48),
        Key::Down => scan(0x50),
        Key::Left => scan(0x4B),
        Key::Right => scan(0x4D),
        _ => press.text.map(|c| num::signed32(u32::from(c))),
    }
}

/// The type-ahead buffer: keystrokes waiting for the step that reads them.
#[derive(Debug, Default)]
pub(crate) struct Buffer {
    /// Oldest first.
    waiting: VecDeque<i32>,
}

impl Buffer {
    /// Takes a press, translated; one the game does not read, or one
    /// arriving on a full buffer, is dropped as the BIOS dropped it.
    pub(crate) fn press(&mut self, press: &KeyPress) {
        if let Some(c) = code(press)
            && self.waiting.len() < SLOTS
        {
            self.waiting.push_back(c);
        }
    }

    /// The oldest keystroke waiting, taken out.
    pub(crate) fn take(&mut self) -> Option<i32> {
        self.waiting.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: Key, text: Option<char>, shift: bool) -> KeyPress {
        KeyPress {
            key,
            text,
            shift,
            ctrl: false,
            alt: false,
        }
    }

    /// The four keys the parsec1994 generation's library maps by their full
    /// word arrive as the codes it compares against.
    #[test]
    fn escape_enter_tab_and_shift_tab_keep_their_codes() {
        assert_eq!(code(&press(Key::Escape, None, false)), Some(27));
        assert_eq!(code(&press(Key::Enter, None, false)), Some(13));
        assert_eq!(code(&press(Key::Tab, None, false)), Some(9));
        assert_eq!(code(&press(Key::Tab, None, true)), Some(0x10F));
    }

    #[test]
    fn a_key_without_a_character_is_its_scan_code_over_100h() {
        assert_eq!(code(&press(Key::Up, None, false)), Some(0x148));
        assert_eq!(code(&press(Key::Down, None, false)), Some(0x150));
        assert_eq!(code(&press(Key::Left, None, false)), Some(0x14B));
        assert_eq!(code(&press(Key::Right, None, false)), Some(0x14D));
    }

    /// The character is what the layout typed, not the key's US position:
    /// a German `y` on the key in Z's place is `y`.
    #[test]
    fn a_typing_key_is_its_character() {
        assert_eq!(
            code(&press(Key::Letter(b'Z'), Some('y'), false)),
            Some(i32::from(b'y'))
        );
        assert_eq!(code(&press(Key::Space, Some(' '), false)), Some(0x20));
        // A character past ASCII is handed on as its Unicode value, as the
        // window's translation always did; what a BIOS of the time would
        // answer — CP437, `81h` for `ü` — is not modelled, and the game
        // reads none.
        assert_eq!(code(&press(Key::Other, Some('ü'), false)), Some(0xFC));
        assert_eq!(code(&press(Key::Function(1), None, false)), None);
    }

    #[test]
    fn the_buffer_holds_fifteen_and_hands_them_out_in_order() {
        let mut b = Buffer::default();
        for c in 'a'..='q' {
            b.press(&press(Key::Other, Some(c), false));
        }
        let got: Vec<i32> = std::iter::from_fn(|| b.take()).collect();
        let want: Vec<i32> = ('a'..='o')
            .map(|c| i32::try_from(u32::from(c)).unwrap())
            .collect();
        assert_eq!(got, want, "the sixteenth and seventeenth are dropped");
    }
}
