//! The descriptor table itself: which one is selected, which handle comes
//! next, what order a frame visits them in, and what a change to one marks
//! for redrawing.
//!
//! None of this is a kernel word. It is the housekeeping the words in
//! [`super::words`] and the drawer both stand on — the selection `ACTDESC`
//! moves, the stamp that orders equal levels, and `touch`, which is where
//! the engine's whole erase model lives: a descriptor is never rubbed out,
//! its neighbours are marked instead.

use super::Descriptor;
use crate::{Engine, Field};
use motionvm_motion_forth::Error;
use motionvm_motion_forth::Result;
use motionvm_motion_forth::cell;
impl Engine {
    /// Removes every descriptor the test picks out, keeping the selection on
    /// whatever it was pointing at.
    ///
    /// `current` is an index, so anything that shortens the list moves it. The
    /// original keeps a resolved pointer in `0xf2af0` and a handle in `0xdb4c0`
    /// and neither is touched by a kill, so the selection survives — unless the
    /// descriptor it named is the one that went, and then there is nothing left
    /// to point at.
    ///
    /// The test is called once per descriptor, in list order, which is what
    /// lets `KILLNDESC` decide by position.
    pub(crate) fn forget_descriptors(&mut self, mut doomed: impl FnMut(&Descriptor) -> bool) {
        let selected = self
            .scene
            .selected
            .and_then(|i| self.scene.descriptors.get(i))
            .map(|d| (d.screen, d.handle));
        self.scene.descriptors.retain(|d| !doomed(d));
        self.scene.selected = selected.and_then(|(screen, h)| {
            self.scene.descriptors.iter().position(|d| {
                d.handle == h && (!self.profile.per_screen_descriptors || d.screen == screen)
            })
        });
    }

    /// Makes a descriptor the current one, the way `ACTDESC` does.
    ///
    /// A handle that resolves to nothing changes nothing. The original looks the
    /// record up first (0x71618) and jumps clear of *both* stores when the
    /// lookup comes back empty (0x71624 → 0x71636), so the descriptor that was
    /// selected stays selected. Clearing the selection instead would silently
    /// disarm every `SD…` that follows, which is a worse failure than acting on
    /// the wrong descriptor: nothing reports it.
    pub(crate) fn select_descriptor(&mut self, handle: u32) {
        self.scene.selected_handle = Some(handle);
        if self.profile.per_screen_descriptors {
            // The 16-bit `ACTDESC` stores the number alone; the screen it
            // counts on is whichever is active when the descriptor is next
            // touched, so `ACTSCR` re-resolves it ([`Engine::select_screen`]).
            // A number no descriptor of the screen has yet leaves nothing
            // selected — a slot past the count, which the original would write
            // into without complaint — so that the next `SD…` says so.
            let screen = self.display.current.unwrap_or(0);
            self.scene.selected = self
                .scene
                .descriptors
                .iter()
                .position(|d| d.screen == screen && d.handle == handle);
            return;
        }
        if let Some(i) = self
            .scene
            .descriptors
            .iter()
            .position(|d| d.handle == handle)
        {
            self.scene.selected = Some(i);
        }
    }

    /// The handle a descriptor made now gets: the next free number, or — in
    /// the per-screen scheme — the active screen's count.
    pub(crate) fn next_handle(&mut self) -> u32 {
        if self.profile.per_screen_descriptors {
            let screen = self.display.current.unwrap_or(0);
            return cell::narrow(
                self.scene
                    .descriptors
                    .iter()
                    .filter(|d| d.screen == screen)
                    .count(),
            );
        }
        let handle = self.scene.next_descriptor;
        self.scene.next_descriptor += 1;
        handle
    }

    /// The descriptors in the order the original's per-frame walk visits them.
    ///
    /// That walk (0x68c64) follows the sibling chain at +0x1E, and the chain is
    /// built by the insertion at 0x6a648, which advances `while cur != sentinel
    /// && cur.level <= new.level` before linking. So a new descriptor lands
    /// after every entry of the same level or lower: the chain is exactly a
    /// **stable sort by level**, and reproducing the order needs no links of
    /// their own — which is why none are stored here.
    ///
    /// Descriptors belonging to a frozen screen are left out, as the walk
    /// returns on `screen.flags & 4` before touching them. `FREEZESCR` sets
    /// that bit; the two were read apart and belong together.
    pub(crate) fn frame_order(&self) -> Vec<(u32, u32)> {
        let mut live: Vec<&Descriptor> = self
            .scene
            .descriptors
            .iter()
            .filter(|d| !self.display.frozen(d.screen))
            .collect();
        live.sort_by_key(|d| d.level);
        live.iter().map(|d| (d.screen, d.handle)).collect()
    }

    /// What the walk does to one descriptor: count its wait down, and when it
    /// runs out hand back the word to run.
    ///
    /// Returns the callback only on the frame the wait reaches zero, because
    /// that is when the handler executes it — a descriptor with no callback
    /// (`+0x14` clear, which is what `SDWORD 0` and `SDWORD -1` leave) is
    /// skipped entirely, wait and all.
    pub fn tick_descriptor(&mut self, handle: u32) -> Option<i32> {
        let i = self
            .scene
            .descriptors
            .iter()
            .position(|d| d.handle == handle)?;
        self.tick_at(i)
    }

    /// The same for the descriptor `handle` of `screen` — the pair that names
    /// one descriptor on either machine.
    pub(crate) fn tick_descriptor_on(&mut self, screen: u32, handle: u32) -> Option<i32> {
        let i = self
            .scene
            .descriptors
            .iter()
            .position(|d| d.screen == screen && d.handle == handle)?;
        self.tick_at(i)
    }

    fn tick_at(&mut self, i: usize) -> Option<i32> {
        let need_active = self.profile.callbacks_need_active;
        let d = self.scene.descriptors.get_mut(i)?;
        if d.callback <= 0 {
            return None;
        }
        // The 16-bit walk (`016a:05d6`) skips a descriptor whose active
        // bit is off — and with it the select that would displace the
        // controller's own `SMDESC` choice.
        if need_active && !d.active {
            return None;
        }
        if d.wait == 0 {
            return Some(d.callback);
        }
        if d.wait > 0 {
            d.wait -= 1;
        }
        None
    }

    /// `0x6ab6e`: this descriptor has changed, so its rectangle wants
    /// redrawing — and whatever its save-under is holding goes back first.
    ///
    /// The one routine every mutating `SD…` word runs through, which is why it
    /// lives here beside the setters rather than in `words/`: the engine's own
    /// code — the walk, the menu, the conversation — reaches the same setters,
    /// and a change that skipped this would freeze that part of the picture
    /// until something else happened to overlap it.
    ///
    /// The mark is on the descriptor's **own** level, so the repaint reaches
    /// everything above it and nothing below. That is the whole reason hiding
    /// something does not erase it.
    pub(crate) fn touch(&mut self, index: usize) {
        let Some(d) = self.scene.descriptors.get(index).cloned() else {
            return;
        };
        // Measured before anything is borrowed, because measuring can load.
        let (x, y, w, h) = self.drawn_rect(&d);
        if let Some(s) = self.display.screen_mut(d.screen) {
            s.mark(x, y, w, h, d.level);
        }
        // 0x6ac33: a descriptor that carries a buffer also owes the picture
        // that was under it. The next pass over this screen rebuilds the place
        // it is leaving — see `Engine::draw_screens`.
        //
        // On the 16-bit machine a descriptor with an `SDBUF` buffer owes the
        // same: the intro's motifs slide across the screen with buffers 2–6
        // and its text stands on buffer 1, and without this they leave their
        // trail standing — which is what settles `SDBUF` as the save-under
        // the 32-bit `SDAUTOBUF` is, and not a compositing layer. A reading
        // of the picture, not of the handler.
        if d.auto_buffer || d.buffer.is_some() {
            self.rebuild.push((d.screen, (x, y, w, h)));
        }
        let d = &mut self.scene.descriptors[index];
        d.dirty = true;
        d.changed = true;
    }

    /// The same for whatever `ACTDESC` last selected, which is what the words
    /// have in hand.
    pub(crate) fn touch_current(&mut self) {
        if let Some(i) = self.scene.selected {
            self.touch(i);
        }
    }

    /// The rectangle to mark, or `None` when the size cannot be had yet.
    ///
    /// Measuring loads — a sprite's pixels, a text's table — and the *first*
    /// sprite a run loads is also the one that decides the palette (see
    /// [`Engine::sprite`](crate::Engine::sprite)). This runs whenever a field
    /// is set, long before anything is drawn, so measuring an unloaded sprite
    /// here would change which one that is. A picture nobody has loaded is
    /// therefore not measured; the caller marks the whole screen instead, which
    /// repaints more than the original would and never less.
    /// The rectangle the descriptor covers on its screen's surface.
    ///
    /// What every mark on the damage map is measured against, and what the
    /// save-under copies. Measuring loads — a sprite's pixels, a text's table —
    /// but only through [`Engine::load_sprite`](crate::Engine::load_sprite),
    /// which is the load without the palette that comes with drawing one. That
    /// is what lets this be called from the setters, where it runs long before
    /// anything reaches the screen.
    pub(crate) fn drawn_rect(&mut self, d: &Descriptor) -> (i32, i32, i32, i32) {
        if d.is_text() {
            // A text covers its backing, which is a good deal larger than the
            // glyphs — and the original's buffer record grows by the template's
            // margin for exactly that reason (0x6baa7).
            return self.backing_rect(d);
        }
        let (x, y) = self.corner(d);
        let (w, h) = self.stored_extent(d);
        (x, y, w, h)
    }

    /// `0x6a8f9`: every descriptor on a screen has to be drawn again.
    ///
    /// The original's "all of it" switch, reached from `0x6b0fe` — which is
    /// what `FADEIN` calls before its single draw (0x74af1). Nothing else puts
    /// a whole picture back after `FADEOUT` has wiped the surface.
    pub fn repaint_screen(&mut self, screen: u32) {
        for d in self
            .scene
            .descriptors
            .iter_mut()
            .filter(|d| d.screen == screen)
        {
            d.dirty = true;
            d.changed = true;
        }
    }

    /// The walk's scale, put down the way the `SD%SHR` word puts it: the
    /// one value into the horizontal and the vertical factor both — the
    /// 32-bit handler writes the pair (`0x721b8`), the 16-bit one calls
    /// `SDH%SHR` and `SDV%SHR` with it (`0xb38b`). All three fields,
    /// because a per-axis value a script has set would otherwise stand,
    /// and the figure would walk the city at the living room's size.
    pub(crate) fn set_shrink(&mut self, v: i32) {
        self.set_field(Field::SD_PCT_SHR, v);
        self.set_field(Field::SDH_PCT_SHR, v);
        self.set_field(Field::SDV_PCT_SHR, v);
    }

    /// The next chain stamp — see [`Descriptor::stamp`].
    pub(crate) fn next_stamp(&mut self) -> u64 {
        self.scene.level_stamp += 1;
        self.scene.level_stamp
    }

    /// A screen's surface has been wiped, so nothing owes it a rebuild.
    ///
    /// `FADEOUT` fills the surface with color 0 before its bands start
    /// (0x74d44) and `ERASESCR` calls the same routine (0x74801). Whatever was
    /// waiting to be put back was on that surface.
    pub(crate) fn forget_rebuilds(&mut self, screen: u32) {
        self.rebuild.retain(|&(s, _)| s != screen);
    }

    /// The current descriptor, or an error naming the word that wanted it.
    ///
    /// Every `SD…` word acts on whatever `ACTDESC` last selected. When that
    /// lookup finds nothing — a stale handle, a descriptor that was never
    /// created — quietly doing nothing produces a picture that is wrong in a
    /// way nothing reports. Since a whole phase of the intro is `ACTDESC`
    /// followed by setters, a silent miss there would simply not draw, and the
    /// search would start at the renderer instead of here.
    pub(crate) fn require_descriptor(&mut self, word: &'static str) -> Result<&mut Descriptor> {
        let i = self.require_selected(word)?;
        Ok(&mut self.scene.descriptors[i])
    }

    /// The index of the selected descriptor, or the same refusal as
    /// [`Engine::require_descriptor`] — for a caller that has to mark around
    /// the descriptor and so wants the index rather than the borrow.
    pub(crate) fn require_selected(&self, word: &'static str) -> Result<usize> {
        let selected = self.scene.selected;
        selected
            .filter(|&i| i < self.scene.descriptors.len())
            .ok_or(Error::NoSelection {
                word,
                selected,
                at: None,
            })
    }

    /// The selected descriptor, or `None` when nothing is selected.
    ///
    /// The quiet counterpart to [`Engine::require_descriptor`]: for the words
    /// whose handlers do nothing rather than fail when the lookup comes back
    /// empty. Which of the two a word uses is the original's choice, not ours.
    pub(crate) fn descriptor_mut(&mut self) -> Option<&mut Descriptor> {
        let i = self.scene.selected?;
        self.scene.descriptors.get_mut(i)
    }
}
