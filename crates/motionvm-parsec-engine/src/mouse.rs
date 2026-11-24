//! The mouse buttons, as a PARSEC program reads them: in its frame loop.
//!
//! The original does not act on a button where the driver sees it. Its
//! frame loop polls the pointer library (`179Eh:00DBh`, inside
//! `179Eh:01B0h`), and the library hands the program button events —
//! event 4 to 7 in the table at `cs:1479h`, events 5, 6 and 7 at `cs:0C86h`
//! in the options — which the dispatcher then takes. A press between two
//! passes is therefore read by the next pass, at the pointer as it stands
//! then. So a press here is latched, and the runtime hands the game the
//! press at the start of the next step, never between two.
//!
//! The latch keeps the level as well as the press: a press is a button
//! going down that was up, which is what the driver counts, so a second
//! `down` with no release between is not a second press. Two presses
//! between two steps are one; how the original's library folds presses
//! that come faster than its passes is not read.

use motionvm_playable::Button;

/// The two buttons' level, and whether each went down since the last
/// step took it.
///
/// A button has two sources: the mouse, and the two keys the program's
/// keyboard hook makes buttons of (`titles::parsec1994::keyboard`). The
/// library takes the key bits as the buttons' level beside the mouse's
/// (`0x187BE`), so a button is down while either holds it, and a press is
/// the two together going from up to down.
#[derive(Debug, Default)]
pub(crate) struct Latch {
    /// Down on the mouse now, left then right.
    held: [bool; 2],
    /// Down on the keyboard now, left then right.
    keys: [bool; 2],
    /// Went down since the last [`Latch::take`], left then right.
    pressed: [bool; 2],
}

/// The latch's slot for a button, or `None` for one the game does not read.
fn slot(which: Button) -> Option<usize> {
    match which {
        Button::Left => Some(0),
        Button::Right => Some(1),
        _ => None,
    }
}

impl Latch {
    /// One transition, as the platform saw it.
    pub(crate) fn button(&mut self, which: Button, down: bool) {
        let Some(i) = slot(which) else {
            return;
        };
        if down && !self.held[i] && !self.keys[i] {
            self.pressed[i] = true;
        }
        self.held[i] = down;
    }

    /// One transition of a button key.
    pub(crate) fn key(&mut self, which: Button, down: bool) {
        let Some(i) = slot(which) else {
            return;
        };
        if down && !self.held[i] && !self.keys[i] {
            self.pressed[i] = true;
        }
        self.keys[i] = down;
    }

    /// The presses since the last call, left first, taken out of the
    /// latch: `(left, right)`.
    pub(crate) fn take(&mut self) -> (bool, bool) {
        let [left, right] = std::mem::take(&mut self.pressed);
        (left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_is_read_once_and_a_release_is_not_a_press() {
        let mut l = Latch::default();
        l.button(Button::Left, true);
        assert_eq!(l.take(), (true, false));
        assert_eq!(l.take(), (false, false), "taken, not read again");
        l.button(Button::Left, false);
        assert_eq!(l.take(), (false, false), "a release presses nothing");
    }

    /// A held button is down once, however many steps it is held over —
    /// which is what keeps a held button from racing through the game.
    #[test]
    fn a_held_button_is_not_pressed_again() {
        let mut l = Latch::default();
        l.button(Button::Right, true);
        l.button(Button::Right, true);
        assert_eq!(l.take(), (false, true));
        l.button(Button::Right, true);
        assert_eq!(l.take(), (false, false), "still held from before");
    }

    #[test]
    fn a_press_and_release_between_steps_still_lands() {
        let mut l = Latch::default();
        l.button(Button::Left, true);
        l.button(Button::Left, false);
        l.button(Button::Left, true);
        l.button(Button::Left, false);
        assert_eq!(l.take(), (true, false), "two presses in one step are one");
    }
}
