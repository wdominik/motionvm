//! What a verb does to an object: the generic rules, the location's own,
//! the inventory, and the text a scene number resolves to.

use super::profile::StripRange;
use super::{Adventure, Result};
use crate::num;

impl Adventure {
    /// Installs a title's initial carried list without sorting it or
    /// running acquisition side effects. `XP01.EXE`'s `0x0CD67`/`0x0CD71` read
    /// initialized executable data, including an item already carried.
    pub(crate) fn initialize_inventory(&mut self, items: &[u8], scroll: usize) -> Result<()> {
        if items
            .iter()
            .any(|&id| id == 0 || usize::from(id) > self.item_names.len())
            || scroll > items.len().saturating_sub(1)
        {
            return Err("invalid initial inventory or scroll position".into());
        }
        self.items = items.to_vec();
        self.scroll = scroll;
        self.inv_lo = 0;
        self.inv_hi = self.items.len();
        self.strip_range();
        Ok(())
    }

    /// The strip's loader on the carried list (`0C1Ch:000Ch`, `0x0C1CC`):
    /// the range is the whole list — or, in a build that splits it
    /// ([`super::Profile::strip`]), the items below the split for the
    /// first strip file and the rest for the second. Then the first
    /// visible slot is put back to the range's start when it lies outside
    /// it (`0x0C1E0`–`0x0C1F5`). A build whose loader keeps no range has
    /// the whole list and leaves the slot where it is.
    pub(crate) fn strip_range(&mut self) {
        let count = self.items.len();
        match self.profile.strip {
            StripRange::Whole => {
                self.inv_lo = 0;
                self.inv_hi = count;
            }
            StripRange::Unranged => {
                self.inv_lo = 0;
                self.inv_hi = count;
                return;
            }
            StripRange::Split(split) => {
                let below = self.items.iter().take_while(|&&id| id < split).count();
                if self.figure_digit != 0 {
                    self.inv_lo = below;
                    self.inv_hi = count;
                } else {
                    self.inv_lo = 0;
                    self.inv_hi = below;
                }
            }
        }
        if self.inv_lo > self.scroll || self.inv_hi <= self.scroll {
            self.scroll = self.inv_lo;
        }
    }

    /// An arrow clicked (`0C3Fh:0677h`, the arms at `0x0CAF0` and
    /// `0x0CB1E` in Dunkle Schatten): back only while the first visible
    /// item is past the range's start, on only while a row more is still
    /// below its end. The first button moves one item; the second (event 6)
    /// a row, held inside the range — on, no further than a row before its
    /// end and never before its start. Then the slots are drawn again
    /// (`or word [4E8h], 0FFh`), which the strip does from the list anyway.
    ///
    /// A build whose arrows move a row on every click (*Bi-Fi 2*'s
    /// `0x07C0E`–`0x07C39`) moves back while the first visible item is not
    /// the first, and on while the list reaches `n` past it, in bytes as
    /// the original adds them.
    pub(crate) fn scroll_strip(&mut self, way: crate::titles::panel::Scroll, by_row: bool) {
        use crate::titles::panel::Scroll;
        let row = usize::from(self.panel_widgets.arrows.row);
        let (lo, hi) = (self.inv_lo, self.inv_hi);
        if let Some(n) = self.panel_widgets.arrows.every_click {
            let scroll = num::byte(self.scroll);
            let step = self.panel_widgets.arrows.row;
            match way {
                Scroll::Up if scroll != 0 => self.scroll = usize::from(scroll.wrapping_sub(step)),
                Scroll::Down if usize::from(scroll) + usize::from(n) < self.items.len() => {
                    self.scroll = usize::from(scroll.wrapping_add(step));
                }
                _ => {}
            }
            return;
        }
        match way {
            Scroll::Up if self.scroll > lo => {
                self.scroll = if by_row {
                    self.scroll.saturating_sub(row).max(lo)
                } else {
                    self.scroll - 1
                };
            }
            Scroll::Down if self.scroll + row < hi => {
                self.scroll = if by_row {
                    (self.scroll + row).min(hi - row).max(lo)
                } else {
                    self.scroll + 1
                };
            }
            _ => {}
        }
    }

    /// The first id past the items: the item row's names and one. An id
    /// below it is an item, at or above it a room's object. The program
    /// compares with it as an immediate wherever it tells the two apart —
    /// in *Dunkle Schatten* `17h` for its 22 names (`0x0CD4F`, `0x0F2DC`,
    /// `0x0F5AC`, `0x0F892`, `0x1031E`), which the title's tests hold
    /// against the image.
    pub(crate) fn items_end(&self) -> u16 {
        num::word(self.item_names.len() + 1)
    }

    /// `0C3Fh:0B06h` (`0x0CEF6`): is `id` in the player's hands?
    ///
    /// The routine walks the carried list downwards from `[15FEh]` and
    /// compares each byte at `[1602h] + i`. This engine keeps the same
    /// list, so the walk is a search.
    pub(crate) fn carried(&self, id: u8) -> bool {
        self.items.contains(&id)
    }

    /// What a **negative** scene number names.
    ///
    /// `0CFDh:172Fh` takes a scene number, and the rule handlers hand it
    /// the ones-complement of a place rather than a scene: `push 0FFEFh`
    /// in the `Betrachte` rule is `~16`, and bank 999's place 16 is
    /// `Ich kann nichts Besonderes erkennen.` So the place is `!n`, and a
    /// place at `4000h` or above is bank 2's — the inventory — with the
    /// item's own number one higher than its place (`0x0D56E` undoes the
    /// `~(id + 4000h)` the handlers build).
    ///
    /// A number that is **not** negative is one of the location's own
    /// scenes — a place in the bank the room loader keeps at `[2855h]`.
    /// What this answers with is that place's first line, which is what a
    /// test reads a scene by; a place the bank leaves empty answers `None`,
    /// because what the original shows for one (`0x0D615`, the `NULLDLG`
    /// arm) is not reproduced here.
    #[cfg(test)]
    pub(super) fn scene_text(&self, n: i16) -> Option<Vec<u8>> {
        if n >= 0 {
            // The resolver indexes the bank at `[2855h]` only when the
            // number is inside the count at `[1EEAh]` (`0x0D5EA`); past
            // it the pointer stays null, and the routine then falls back
            // on bank 999's place `13h` — the programmers' own joke about
            // a dialogue they forgot to put in.
            let place = usize::from(num::unsigned(n));
            let Some(bank) = self.own_bank().filter(|b| place < b.len()) else {
                return self.scene_text(!0x13);
            };
            let line = &bank.get(place)?.lines.first()?.text;
            return Some(motionvm_parsec_formats::to_cp437(line));
        }
        let place = !num::unsigned(n);
        let (bank, place) = if place >= 0x4000 {
            (
                &self.dialogues.1,
                usize::from(place.checked_sub(self.profile.scenes.item_base)?),
            )
        } else {
            (&self.dialogues.0, usize::from(place))
        };
        let line = &bank.get(place)?.lines.first()?.text;
        Some(motionvm_parsec_formats::to_cp437(line))
    }

    /// Where the live object with this id sits in the current location's
    /// overlay list, if there is one (`0BB6h:052Bh`, then `+2` bit 0).
    #[cfg(test)]
    pub(crate) fn live_object(&self, id: u8) -> Option<usize> {
        self.objects
            .iter()
            .position(|o| o.id == id && o.flags & super::objects::LIVE != 0)
    }

    /// What is in slot `n`, which is the item `scroll + n` of what is
    /// carried (`0x0CAC0`).
    pub(crate) fn item_in_slot(&self, n: usize) -> Option<u8> {
        let i = self.scroll + n;
        if i >= self.inv_hi {
            return None;
        }
        self.items.get(i).copied()
    }

    /// What an item is called. Ids count from one.
    pub(crate) fn item_name(&self, id: u8) -> Option<&str> {
        self.item_names
            .get(usize::from(id).checked_sub(1)?)
            .map(String::as_str)
    }

    /// Puts an item in the player's hands, in the order the strip keeps
    /// them: `0C3Fh:095Ch` walks the list from its end and shifts every
    /// item above the new one up (`0x0CD6F`), so the list stays sorted.
    /// The count and the range's end go up by one (`0x0CD99`), and a new
    /// item outside the visible slots scrolls the strip to it
    /// (`0x0CDA9`).
    pub(crate) fn give(&mut self, id: u8) {
        let at = self
            .items
            .iter()
            .position(|&i| i > id)
            .unwrap_or(self.items.len());
        self.items.insert(at, id);
        self.inv_hi += 1;
        let last = usize::from(self.profile.inventory_window).saturating_sub(1);
        if self.scroll > at || self.scroll + last < at {
            // By rows (Bi-Fi 2's `0x07E7B`): the start of the row
            // before the item's, 0 for one in the first row.
            self.scroll = match self.profile.inventory_rows.map(usize::from) {
                Some(row) if at < row => 0,
                Some(row) => (at - row) & !(row - 1),
                None => at,
            };
        }
    }

    /// `0C3Fh:09E1h(id)`: an item out of the player's hands, the ones above
    /// it shifted down (`0x0CDFA`). Nothing happens for one not carried.
    pub(crate) fn take_item(&mut self, id: u8) {
        if let Some(at) = self.items.iter().position(|&i| i == id) {
            self.items.remove(at);
            // The range's end down by one (`0x0CE19`). An item past the
            // visible slots leaves the strip as it is (`0x0CE21`); else the
            // first visible slot steps back by one when it lay past the
            // item, or — past the range's end — a whole window, down to
            // the range's start (`0x0CE48`–`0x0CE7C`).
            self.inv_hi = self.inv_hi.saturating_sub(1);
            let window = usize::from(self.profile.inventory_window);
            if self.scroll + window <= at {
                return;
            }
            // By rows (Bi-Fi 2's `0x07F23`–`0x07F44`): a row back while
            // the first visible row lies past the count rounded down to rows.
            if let Some(row) = self.profile.inventory_rows.map(usize::from) {
                if self.scroll != 0 && self.scroll + row > self.items.len() & !(row - 1) {
                    self.scroll -= row;
                }
                return;
            }
            if self.scroll < self.inv_hi {
                if self.scroll > at {
                    self.scroll -= 1;
                }
            } else if self.scroll >= self.inv_lo + window {
                self.scroll -= window;
            } else {
                self.scroll = self.inv_lo;
            }
        }
    }

    /// One place of bank 999 as the file has it, for a test to look at.
    ///
    /// Not through `scene_text`, which hands back CP437 bytes for the
    /// painter: a test wants to read the words.
    #[cfg(test)]
    pub(crate) fn bank999_place_for_test(&self, n: usize) -> Option<String> {
        Some(self.dialogues.0.get(n)?.lines.first()?.text.clone())
    }

    /// What a scene number resolves to, as words, for a test to look at:
    /// the first record of the place `0CFDh:059Eh` would bind.
    #[cfg(test)]
    pub(crate) fn scene_text_for_test(&self, n: i16) -> Option<String> {
        self.scene_text(n)
            .map(|t| motionvm_parsec_formats::cp437_to_string(&t))
    }

    /// Scrolls the strip, which is what the arrows do.
    #[cfg(test)]
    pub(crate) fn scroll_to(&mut self, n: usize) {
        self.scroll = n;
    }
}

#[cfg(test)]
mod tests {
    /// Giving and taking move the strip the way `0C3Fh:095Ch` and
    /// `0C3Fh:09E1h` do: a new item outside the eight visible slots scrolls
    /// to it, and taking one back past the range's end steps a whole window
    /// back, down to the range's start.
    #[test]
    fn the_strip_scrolls_as_giving_and_taking_move_it() {
        let Some(mut a) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        a.items.clear();
        a.scroll = 0;
        a.strip_range();
        for id in 1..=17 {
            a.give(id);
        }
        assert_eq!((a.inv_lo, a.inv_hi), (0, 17));
        // Item 17 went in at index 16, past the visible slots.
        assert_eq!(a.scroll, 16);
        // Index 16 taken: the first visible slot is past the range's end,
        // a window or more above its start: a whole window back.
        a.take_item(17);
        assert_eq!((a.inv_hi, a.scroll), (16, 8));
        // An item below the first visible slot: one step back.
        a.take_item(3);
        assert_eq!(a.scroll, 7);
        // An item past the visible slots leaves the strip as it is.
        a.scroll = 0;
        a.take_item(16);
        assert_eq!(a.scroll, 0);
    }

    /// The arrows scroll the carried items as `0C3Fh:0677h` does: one item
    /// with the first button, a row of four with the second, back no
    /// further than the range's start and on only while a row more is
    /// below its end.
    #[test]
    fn the_arrows_scroll_the_strip() {
        use crate::titles::panel::Scroll;
        let Some(mut a) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        assert_eq!(a.panel_widgets.arrows.row, 4);
        a.items = (1..=10).collect();
        a.scroll = 0;
        a.strip_range();
        a.scroll_strip(Scroll::Up, false);
        assert_eq!(a.scroll, 0, "nothing before the start");
        a.scroll_strip(Scroll::Down, false);
        assert_eq!(a.scroll, 1);
        a.scroll_strip(Scroll::Down, true);
        assert_eq!(a.scroll, 5);
        // 5 + 4 is not below 10: on no further.
        a.scroll_strip(Scroll::Down, true);
        a.scroll_strip(Scroll::Down, false);
        assert_eq!(a.scroll, 6);
        a.scroll_strip(Scroll::Down, false);
        assert_eq!(a.scroll, 6, "a row more must still be below the end");
        a.scroll_strip(Scroll::Up, true);
        assert_eq!(a.scroll, 2);
        a.scroll_strip(Scroll::Up, true);
        assert_eq!(a.scroll, 0, "held at the start");
    }

    /// Captain Zins's handlers hand `~(id + 4000h)` to look at an item
    /// (`0x10E7B`) and its lookup takes `4000h` off (`sub si, 4000h`,
    /// `0x08A70`): item 2, the Ring, is the inventory bank's place 2.
    #[test]
    fn captain_zins_names_an_item_by_its_own_place() {
        let Some(a) = crate::titles::capzins::adventure_for_test() else {
            return;
        };
        let ring = a.scene_text(!(2 + 0x4000)).expect("the Ring has a line");
        let ring = String::from_utf8_lossy(&ring).into_owned();
        assert!(ring.contains("Metallring"), "{ring}");
        assert_eq!(a.item_names.get(1).map(String::as_str), Some("Ring"));
    }
}
