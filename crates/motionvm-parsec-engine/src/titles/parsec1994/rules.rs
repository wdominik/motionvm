//! What a verb does to an object: the generic rules, the location's own,
//! the inventory, and the text a scene number resolves to.

use super::{Adventure, Result, objects};

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
        Ok(())
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
            let place = usize::from(crate::num::unsigned(n));
            let Some(bank) = self.own_bank().filter(|b| place < b.len()) else {
                return self.scene_text(!0x13);
            };
            let line = &bank.get(place)?.lines.first()?.text;
            return Some(motionvm_parsec_formats::to_cp437(line));
        }
        let place = !crate::num::unsigned(n);
        let (bank, place) = if place >= 0x4000 {
            (
                &self.dialogues.1,
                usize::from(place - 0x4000).checked_sub(1)?,
            )
        } else {
            (&self.dialogues.0, usize::from(place))
        };
        let line = &bank.get(place)?.lines.first()?.text;
        Some(motionvm_parsec_formats::to_cp437(line))
    }

    /// Where the live object with this id sits in the current location's
    /// overlay list, if there is one (`0BB6h:052Bh`, then `+2` bit 0).
    pub(super) fn live_object(&self, id: u8) -> Option<usize> {
        self.objects
            .iter()
            .position(|o| o.id == id && o.flags & objects::LIVE != 0)
    }

    /// What is in slot `n`, which is the item `scroll + n` of what is
    /// carried (`0x0CAC0`).
    pub(super) fn item_in_slot(&self, n: usize) -> Option<u8> {
        self.items.get(self.scroll + n).copied()
    }

    /// What an item is called. Ids count from one.
    pub(super) fn item_name(&self, id: u8) -> Option<&str> {
        self.item_names
            .get(usize::from(id).checked_sub(1)?)
            .map(String::as_str)
    }

    /// Puts an item in the player's hands, in the order the strip keeps
    /// them: `0C3Fh:095Ch` walks the list from its end and shifts every
    /// item above the new one up (`0x0CD6F`), so the list stays sorted.
    pub(crate) fn give(&mut self, id: u8) {
        let at = self
            .items
            .iter()
            .position(|&i| i > id)
            .unwrap_or(self.items.len());
        self.items.insert(at, id);
    }

    /// `0C3Fh:09E1h(id)`: an item out of the player's hands, the ones above
    /// it shifted down (`0x0CDFA`). Nothing happens for one not carried.
    pub(crate) fn take_item(&mut self, id: u8) {
        if let Some(at) = self.items.iter().position(|&i| i == id) {
            self.items.remove(at);
            if self.scroll > 0 && self.scroll + 8 > self.items.len() {
                self.scroll = self.items.len().saturating_sub(8);
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
