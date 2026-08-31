//! From winit's keyboard to the contract's: which physical key was struck,
//! what the layout in force made of it, which modifiers were down.
//!
//! Nothing here knows what a game reads out of a press. Translation into
//! whatever an engine's scripts compare against happens behind the contract,
//! on the engine's side — this file only carries a winit event across into a
//! [`motionvm_playable::KeyPress`], which a test over there can build without a window.

use motionvm_playable::{Key, KeyPress};
use winit::keyboard::{Key as Logical, KeyCode, ModifiersState, PhysicalKey};

/// One press, carried across whole.
///
/// The two halves of a winit key event are taken apart rather than passed on,
/// because a `KeyEvent` carries a private field and cannot be built outside
/// winit — and the modifiers arrive in their own event besides.
pub(crate) fn press(
    physical_key: PhysicalKey,
    logical_key: &Logical,
    mods: ModifiersState,
) -> KeyPress {
    KeyPress {
        key: key(physical_key),
        text: text(logical_key),
        shift: mods.shift_key(),
        ctrl: mods.control_key(),
        alt: mods.alt_key(),
    }
}

/// The physical key, for every position the contract names; everything else
/// is [`Key::Other`] and reaches the game as its character alone.
fn key(physical: PhysicalKey) -> Key {
    let PhysicalKey::Code(code) = physical else {
        return Key::Other;
    };
    match code {
        KeyCode::KeyA => Key::Letter(b'A'),
        KeyCode::KeyB => Key::Letter(b'B'),
        KeyCode::KeyC => Key::Letter(b'C'),
        KeyCode::KeyD => Key::Letter(b'D'),
        KeyCode::KeyE => Key::Letter(b'E'),
        KeyCode::KeyF => Key::Letter(b'F'),
        KeyCode::KeyG => Key::Letter(b'G'),
        KeyCode::KeyH => Key::Letter(b'H'),
        KeyCode::KeyI => Key::Letter(b'I'),
        KeyCode::KeyJ => Key::Letter(b'J'),
        KeyCode::KeyK => Key::Letter(b'K'),
        KeyCode::KeyL => Key::Letter(b'L'),
        KeyCode::KeyM => Key::Letter(b'M'),
        KeyCode::KeyN => Key::Letter(b'N'),
        KeyCode::KeyO => Key::Letter(b'O'),
        KeyCode::KeyP => Key::Letter(b'P'),
        KeyCode::KeyQ => Key::Letter(b'Q'),
        KeyCode::KeyR => Key::Letter(b'R'),
        KeyCode::KeyS => Key::Letter(b'S'),
        KeyCode::KeyT => Key::Letter(b'T'),
        KeyCode::KeyU => Key::Letter(b'U'),
        KeyCode::KeyV => Key::Letter(b'V'),
        KeyCode::KeyW => Key::Letter(b'W'),
        KeyCode::KeyX => Key::Letter(b'X'),
        KeyCode::KeyY => Key::Letter(b'Y'),
        KeyCode::KeyZ => Key::Letter(b'Z'),
        KeyCode::F1 => Key::Function(1),
        KeyCode::F2 => Key::Function(2),
        KeyCode::F3 => Key::Function(3),
        KeyCode::F4 => Key::Function(4),
        KeyCode::F5 => Key::Function(5),
        KeyCode::F6 => Key::Function(6),
        KeyCode::F7 => Key::Function(7),
        KeyCode::F8 => Key::Function(8),
        KeyCode::F9 => Key::Function(9),
        KeyCode::F10 => Key::Function(10),
        KeyCode::F11 => Key::Function(11),
        KeyCode::F12 => Key::Function(12),
        KeyCode::Digit0 => Key::Digit(0),
        KeyCode::Digit1 => Key::Digit(1),
        KeyCode::Digit2 => Key::Digit(2),
        KeyCode::Digit3 => Key::Digit(3),
        KeyCode::Digit4 => Key::Digit(4),
        KeyCode::Digit5 => Key::Digit(5),
        KeyCode::Digit6 => Key::Digit(6),
        KeyCode::Digit7 => Key::Digit(7),
        KeyCode::Digit8 => Key::Digit(8),
        KeyCode::Digit9 => Key::Digit(9),
        KeyCode::ArrowUp => Key::Up,
        KeyCode::ArrowDown => Key::Down,
        KeyCode::ArrowLeft => Key::Left,
        KeyCode::ArrowRight => Key::Right,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Insert => Key::Insert,
        KeyCode::Delete => Key::Delete,
        KeyCode::Enter => Key::Enter,
        KeyCode::Escape => Key::Escape,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Tab => Key::Tab,
        KeyCode::Space => Key::Space,
        _ => Key::Other,
    }
}

/// The character the layout makes of the press, when it makes one.
fn text(logical: &Logical) -> Option<char> {
    match logical {
        Logical::Character(t) => t.chars().next(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::{NamedKey, SmolStr};

    #[test]
    fn a_letter_crosses_with_its_position_and_its_character() {
        let p = press(
            PhysicalKey::Code(KeyCode::KeyZ),
            &Logical::Character(SmolStr::new("y")),
            ModifiersState::empty(),
        );
        // A German layout types `y` on the key in Z's position; both halves
        // cross, and which one matters is the engine's decision.
        assert_eq!(p.key, Key::Letter(b'Z'));
        assert_eq!(p.text, Some('y'));
        assert!(!p.shift && !p.ctrl && !p.alt);
    }

    #[test]
    fn named_keys_cross_by_position_with_no_character() {
        let p = press(
            PhysicalKey::Code(KeyCode::Escape),
            &Logical::Named(NamedKey::Escape),
            ModifiersState::empty(),
        );
        assert_eq!(p.key, Key::Escape);
        assert_eq!(p.text, None);
        let p = press(
            PhysicalKey::Code(KeyCode::ArrowUp),
            &Logical::Named(NamedKey::ArrowUp),
            ModifiersState::CONTROL,
        );
        assert_eq!(p.key, Key::Up);
        assert!(p.ctrl && !p.shift);
    }

    #[test]
    fn a_digit_crosses_with_its_position_and_its_character() {
        // On AZERTY the unshifted top row types letters, so the position is
        // the half a family reading "the 1 key" needs.
        let p = press(
            PhysicalKey::Code(KeyCode::Digit8),
            &Logical::Character(SmolStr::new("8")),
            ModifiersState::empty(),
        );
        assert_eq!(p.key, Key::Digit(8));
        assert_eq!(p.text, Some('8'));
    }

    #[test]
    fn an_unnamed_key_still_carries_its_character() {
        let p = press(
            PhysicalKey::Code(KeyCode::Minus),
            &Logical::Character(SmolStr::new("-")),
            ModifiersState::empty(),
        );
        assert_eq!(p.key, Key::Other);
        assert_eq!(p.text, Some('-'));
    }
}
