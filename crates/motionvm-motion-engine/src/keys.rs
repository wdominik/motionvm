//! The keyboard, as `?KEY` answers it.
//!
//! `?KEY` (handler `0x6258c`) and its blocking sister `KEY` (`0x62501`) both
//! call one native translator, `0x2379b`, and that translator does not hand the
//! script a character. It reads the keyboard through the real-mode trampoline
//! table at `0x88f9c`: `0x83e78` is INT 16h AH=01 (is a key waiting), `0x83e9c`
//! is AH=00 (take it — AL the character, AH the scan code) and `0x83eb8` is
//! AH=02 (the shift state, folded to 2/4/8 by `0x23a03`). What it returns is a
//! character *or* a scan code, with the modifiers in the high byte:
//!
//! | bit | meaning | where it is set |
//! |---|---|---|
//! | `0x100` | the low byte is a scan code, not a character | `0x23894` |
//! | `0x200` | Shift | `0x23874` |
//! | `0x400` | Ctrl | `0x2388e`, `0x238bb` |
//! | `0x800` | Alt | `0x23825`, `0x23851` |
//!
//! The game reads both halves of that. Module 216 — the mailbox — dispatches on
//! `328`, `336`, `331` and `333` at `0x0c21c`, which are `0x100` over the scan
//! codes of the four cursor keys, and module 4's debug layer tests `315`, `316`
//! and `323` at `0x027a0` and `0x052e0`, the same rule over F1, F2 and F9.
//!
//! Two of the original's steps are folded away here, because they cancel. The
//! BIOS gives Shift+F1 scan code `0x54` and Ctrl+F1 `0x5e`, and `0x23860` and
//! `0x2387a` subtract `0x19` and `0x23` to get F1's own `0x3b` back; a host
//! that already knows which key was struck can set the bit and skip the detour.
//! The same holds for Ctrl and a letter: the BIOS answers `0x01` for Ctrl+A and
//! `0x238b8` adds `0x40` straight back, so what reaches the script is the
//! upper-case letter with the Ctrl bit on.
//!
//! What arrives here is the contract's [`KeyPress`] — the physical key, the
//! layout's character, the modifiers — which is exactly what the translation
//! consumes, delivered in a type a test can build.

use motionvm_playable::{Key, KeyPress};

/// Slots in the keyboard buffer: the BIOS type-ahead buffer `?KEY` reads
/// through INT 16h holds fifteen keystrokes, and a sixteenth was dropped on
/// the original too.
pub(crate) const SLOTS: usize = 15;

/// The low byte is a scan code and not a character (`0x23894`).
const EXTENDED: i32 = 0x100;
/// Shift was held (`0x23a29`, through `0x23874`).
const SHIFT: i32 = 0x200;
/// Ctrl was held (`0x23a49`, through `0x2388e` and `0x238bb`).
const CTRL: i32 = 0x400;
/// Alt was held (`0x23a59`, through `0x23825` and `0x23851`).
const ALT: i32 = 0x800;

/// Alt and a letter, as the table at `0xd667c` maps it: a scan code and the
/// upper-case character to answer with, walked in order until a scan code
/// matches or a zero ends the table (`0x237ed`…`0x2382d`).
///
/// It is copied byte for byte, and that includes its one mistake. The entry for
/// `Z` names scan code `0x18`, which is `O`'s and stands eight rows earlier, so
/// the walk finds `O` first and `Z` is never reached — Alt+Z falls through to
/// the scan-code answer below instead. The table is also evidence in its own
/// right: those 26 scan codes are the US set-1 codes exactly, which is what
/// says the rest of this file may read from the same set.
const ALT_LETTERS: [(u8, u8); 26] = [
    (0x1e, b'A'),
    (0x30, b'B'),
    (0x2e, b'C'),
    (0x20, b'D'),
    (0x12, b'E'),
    (0x21, b'F'),
    (0x22, b'G'),
    (0x23, b'H'),
    (0x17, b'I'),
    (0x24, b'J'),
    (0x25, b'K'),
    (0x26, b'L'),
    (0x32, b'M'),
    (0x31, b'N'),
    (0x18, b'O'),
    (0x19, b'P'),
    (0x10, b'Q'),
    (0x13, b'R'),
    (0x1f, b'S'),
    (0x14, b'T'),
    (0x16, b'U'),
    (0x2f, b'V'),
    (0x11, b'W'),
    (0x2d, b'X'),
    (0x15, b'Y'),
    (0x18, b'Z'),
];

/// What `?KEY` answers for one press, or nothing for a key the PC has no answer
/// for — a modifier on its own, a media key, anything the 1996 keyboard did not
/// have. A press that answers nothing never reaches the buffer, exactly as it
/// never reached the BIOS's.
pub(crate) fn code(press: &KeyPress) -> Option<i32> {
    // Alt suppresses the character: the BIOS answers AL = 0 for Alt and a
    // letter, so `0x237cf` takes the scan-code path for it too.
    if press.alt {
        return alt(press.key);
    }

    // A key that carries no character at all — the cursor block, the function
    // keys — is the same path without the Alt bit.
    if let Some(scan) = scancode(press.key) {
        return Some(extended(scan, press));
    }

    let ascii = ascii(press)?;
    // The byte the BIOS would leave in AL. Ctrl and a letter is a control code
    // there, which is the half of the round trip a host does not get for free.
    let al = if press.ctrl && ascii.is_ascii_alphabetic() {
        ascii.to_ascii_uppercase() - 0x40
    } else {
        ascii
    };
    // `0x2389d`: the character path, and the fold back at `0x238ad`. The bound
    // is the handler's own `jbe 0x1a`, which is why Escape — `0x1b` — passes
    // through as 27 even with Ctrl down.
    Some(if press.ctrl && al <= 0x1a {
        CTRL | (al as i32 + 0x40)
    } else {
        al as i32
    })
}

/// A key with no character of its own, as `0x23894` marks it.
fn extended(scan: u8, press: &KeyPress) -> i32 {
    let code = EXTENDED | scan as i32;
    // Only the function keys carry a modifier here, because only they have
    // shifted scan codes for `0x23860` and `0x2387a` to fold: `0x54`…`0x5d`
    // for Shift and `0x5e`…`0x67` for Ctrl. The BIOS answers the Ctrl code
    // when both are down, so Ctrl is asked first.
    //
    // What the BIOS reports for a *shifted cursor key* is settled by neither
    // the binary nor by any script — it is a property of the keyboard's own
    // translation — so the plain code goes through rather than an invented one.
    if (0x3b..=0x44).contains(&scan) {
        if press.ctrl {
            return code | CTRL;
        }
        if press.shift {
            return code | SHIFT;
        }
    }
    code
}

/// Alt and a key, the walk at `0x237ed` and the fall-through at `0x2383d`.
fn alt(key: Key) -> Option<i32> {
    let scan = letter_scancode(key).or_else(|| scancode(key))?;
    if let Some(&(_, ascii)) = ALT_LETTERS.iter().find(|(code, _)| *code == scan) {
        return Some(ALT | ascii as i32);
    }
    // `0x2383d` maps Alt+F1…F10 — scan codes `0x68`…`0x71` — back onto the
    // plain function keys and sets both bits, which is the same value this
    // reaches directly. Alt and a cursor key is a divergence and a small one:
    // the BIOS gives those their own scan codes, nothing here or in the game
    // pins them down, and no module reads an Alt code at all.
    Some(ALT | EXTENDED | scan as i32)
}

/// The scan code of a key that carries no character.
///
/// The four cursor keys are read off the game's own comparisons in module 216
/// (`0x0c21c`: 328, 336, 331, 333 over `0x100`). F1 is fixed three times over
/// inside `0x2379b` — `0x54 − 0x19`, `0x5e − 0x23` and `0x68 − 0x2d` all give
/// `0x3b` — and F2 and F9 are confirmed against module 4, which tests 316 and
/// 323. The navigation block comes from the same US set-1 codes the Alt table
/// at `0xd667c` is written in; no module reads one, so the game cannot confirm
/// them and cannot be disturbed by them either.
///
/// F11 and F12 answer nothing. The original's folds stop at F10, which is the
/// keyboard MOTION was written for, and F12 belongs to the window here anyway.
fn scancode(key: Key) -> Option<u8> {
    Some(match key {
        Key::Up => 0x48,
        Key::Left => 0x4b,
        Key::Right => 0x4d,
        Key::Down => 0x50,
        Key::Function(n @ 1..=10) => 0x3a + n,
        Key::Home => 0x47,
        Key::PageUp => 0x49,
        Key::End => 0x4f,
        Key::PageDown => 0x51,
        Key::Insert => 0x52,
        Key::Delete => 0x53,
        _ => return None,
    })
}

/// The scan code of a letter key, which only Alt ever needs.
///
/// Twenty-five of these are read straight out of [`ALT_LETTERS`]; `Z` is the
/// one the table gets wrong, and `0x2c` is its US set-1 code.
fn letter_scancode(key: Key) -> Option<u8> {
    let Key::Letter(letter) = key else {
        return None;
    };
    if letter == b'Z' {
        return Some(0x2c);
    }
    ALT_LETTERS
        .iter()
        .find(|&&(_, l)| l == letter)
        .map(|&(scan, _)| scan)
}

/// The character byte the BIOS leaves in AL.
///
/// The named five are the control codes a PC keyboard puts there; everything
/// else comes from the layout. Only a byte can be reported, so a character
/// above `0xff` is dropped — and the bytes above `0x7f` are CP437 in the
/// original, which is a difference no module can see: every comparison the game
/// makes against a character is against an ASCII one.
fn ascii(press: &KeyPress) -> Option<u8> {
    match press.key {
        Key::Backspace => Some(8),
        Key::Tab => Some(9),
        Key::Enter => Some(13),
        Key::Escape => Some(27),
        Key::Space => Some(32),
        _ => {
            let c = press.text? as u32;
            (1..=0xff).contains(&c).then_some(c as u8)
        }
    }
}

#[cfg(test)]
mod tests {
    //! Every expected value here is read out of the original rather than
    //! chosen: the four cursor codes and the three function-key codes are the
    //! game's own comparisons (module 216 at `0x0c21c`, module 4 at `0x027a0`
    //! and `0x052e0`), and the modifier bits are the ones `0x2379b` sets.

    use super::*;
    use crate::Engine;

    /// A press of a key that carries no character, with nothing held.
    fn plain(key: Key) -> Option<i32> {
        code(&press(key, None, [false, false, false]))
    }

    /// A press of a key that carries a character.
    fn typed(key: Key, text: char, mods: [bool; 3]) -> Option<i32> {
        code(&press(key, Some(text), mods))
    }

    fn press(key: Key, text: Option<char>, [shift, ctrl, alt]: [bool; 3]) -> KeyPress {
        KeyPress {
            key,
            text,
            shift,
            ctrl,
            alt,
        }
    }

    const SHIFT: [bool; 3] = [true, false, false];
    const CTRL: [bool; 3] = [false, true, false];
    const ALT: [bool; 3] = [false, false, true];
    const NONE: [bool; 3] = [false, false, false];

    #[test]
    fn the_cursor_keys_answer_what_the_mailbox_dispatches_on() {
        assert_eq!(plain(Key::Up), Some(328));
        assert_eq!(plain(Key::Down), Some(336));
        assert_eq!(plain(Key::Left), Some(331));
        assert_eq!(plain(Key::Right), Some(333));
    }

    #[test]
    fn the_function_keys_answer_what_the_debug_layer_tests() {
        assert_eq!(plain(Key::Function(1)), Some(315));
        assert_eq!(plain(Key::Function(2)), Some(316));
        assert_eq!(plain(Key::Function(9)), Some(323));
        assert_eq!(plain(Key::Function(10)), Some(324));
    }

    #[test]
    fn shift_and_ctrl_fold_onto_the_function_key_they_were_struck_with() {
        assert_eq!(code(&press(Key::Function(1), None, SHIFT)), Some(0x33b));
        assert_eq!(code(&press(Key::Function(1), None, CTRL)), Some(0x53b));
    }

    /// A cursor key keeps its plain code under Shift: what the BIOS reports for
    /// a shifted cursor key is settled neither by the binary nor by any module,
    /// so it is not invented.
    #[test]
    fn a_shifted_cursor_key_stays_the_plain_code() {
        assert_eq!(code(&press(Key::Up, None, SHIFT)), Some(328));
    }

    #[test]
    fn a_character_answers_its_own_byte() {
        assert_eq!(typed(Key::Letter(b'A'), 'a', NONE), Some(97));
        assert_eq!(typed(Key::Other, '8', NONE), Some(56));
        assert_eq!(code(&press(Key::Enter, None, NONE)), Some(13));
        assert_eq!(code(&press(Key::Escape, None, NONE)), Some(27));
        assert_eq!(code(&press(Key::Space, None, NONE)), Some(32));
        assert_eq!(code(&press(Key::Backspace, None, NONE)), Some(8));
    }

    /// The BIOS control code and `0x238b8`'s `+ 0x40` cancel, so Ctrl+A is `A`
    /// with the Ctrl bit — and Escape sits above the handler's `jbe 0x1a`, so it
    /// does not fold even with Ctrl down.
    #[test]
    fn ctrl_and_a_letter_answer_the_upper_case_letter() {
        assert_eq!(typed(Key::Letter(b'A'), 'a', CTRL), Some(0x441));
        assert_eq!(code(&press(Key::Backspace, None, CTRL)), Some(0x448));
        assert_eq!(code(&press(Key::Escape, None, CTRL)), Some(27));
    }

    /// Alt+O finds the table; Alt+Z cannot, because the table's entry for `Z`
    /// names `O`'s scan code, so it answers a scan code instead. Alt+F1 lands on
    /// the same value `0x2383d` computes from `0x68`.
    #[test]
    fn alt_reads_the_table_at_0xd667c_including_its_mistake() {
        assert_eq!(typed(Key::Letter(b'O'), 'o', ALT), Some(0x84f));
        assert_eq!(typed(Key::Letter(b'Z'), 'z', ALT), Some(0x92c));
        assert_eq!(code(&press(Key::Function(1), None, ALT)), Some(0x93b));
    }

    #[test]
    fn a_key_the_pc_has_no_answer_for_answers_nothing() {
        assert_eq!(plain(Key::Function(13)), None);
        assert_eq!(plain(Key::Other), None);
    }

    #[test]
    fn the_buffer_holds_fifteen_and_drops_the_sixteenth() {
        let mut e = Engine::with_display(640, 480);
        for _ in 0..20 {
            e.push_key(&press(Key::Enter, None, NONE));
        }
        let mut taken = 0;
        while e.pop_key() != 0 {
            taken += 1;
        }
        assert_eq!(taken, SLOTS, "the buffer caps at the BIOS's fifteen");
    }

    #[test]
    fn an_untranslatable_press_never_reaches_the_buffer() {
        let mut e = Engine::with_display(640, 480);
        e.push_key(&press(Key::Other, None, SHIFT));
        assert_eq!(e.pop_key(), 0);
    }

    #[test]
    fn presses_come_back_oldest_first_one_per_take() {
        let mut e = Engine::with_display(640, 480);
        e.push_key(&press(Key::Letter(b'A'), Some('a'), NONE));
        e.push_key(&press(Key::Letter(b'B'), Some('b'), NONE));
        assert_eq!(e.pop_key(), 97);
        assert_eq!(e.pop_key(), 98);
        assert_eq!(e.pop_key(), 0, "an empty buffer answers ?KEY's own 0");
    }
}
