//! The scene graph: what the engine draws, and what it remembers about each item.
//!
//! A descriptor is the original's 0x5c-byte record. It carries one kind of
//! content — a sprite, a block, a text, or nothing — plus a place, a level, a
//! wait and a callback. The per-frame walk visits them in level order, and the
//! drawer paints onto a surface that persists: a descriptor is drawn when it
//! is marked, and switching one off marks its neighbours rather than erasing
//! it. What erases is `SDAUTOBUF` — see [`Descriptor::auto_buffer`].
//!
//! The savegame codecs live here too: the numbers they write are these enums
//! and nothing else uses them.

use crate::{Engine, Error, Result};
use std::collections::BTreeMap;

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
            .selected
            .and_then(|i| self.descriptors.get(i))
            .map(|d| (d.screen, d.handle));
        self.descriptors.retain(|d| !doomed(d));
        self.selected = selected.and_then(|(screen, h)| {
            self.descriptors
                .iter()
                .position(|d| d.handle == h && (!self.per_screen_descriptors || d.screen == screen))
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
        self.selected_handle = Some(handle);
        if self.per_screen_descriptors {
            // The 16-bit `ACTDESC` stores the number alone; the screen it
            // counts on is whichever is active when the descriptor is next
            // touched, so `ACTSCR` re-resolves it ([`Engine::select_screen`]).
            // A number no descriptor of the screen has yet leaves nothing
            // selected — a slot past the count, which the original would write
            // into without complaint — so that the next `SD…` says so.
            let screen = self.display.current.unwrap_or(0);
            self.selected = self
                .descriptors
                .iter()
                .position(|d| d.screen == screen && d.handle == handle);
            return;
        }
        if let Some(i) = self.descriptors.iter().position(|d| d.handle == handle) {
            self.selected = Some(i);
        }
    }

    /// The handle a descriptor made now gets: the next free number, or — in
    /// the per-screen scheme — the active screen's count.
    pub(crate) fn next_handle(&mut self) -> u32 {
        if self.per_screen_descriptors {
            let screen = self.display.current.unwrap_or(0);
            return self
                .descriptors
                .iter()
                .filter(|d| d.screen == screen)
                .count() as u32;
        }
        let handle = self.next_descriptor;
        self.next_descriptor += 1;
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
        let i = self.descriptors.iter().position(|d| d.handle == handle)?;
        self.tick_at(i)
    }

    /// The same for the descriptor `handle` of `screen` — the pair that names
    /// one descriptor on either machine.
    pub(crate) fn tick_descriptor_on(&mut self, screen: u32, handle: u32) -> Option<i32> {
        let i = self
            .descriptors
            .iter()
            .position(|d| d.screen == screen && d.handle == handle)?;
        self.tick_at(i)
    }

    fn tick_at(&mut self, i: usize) -> Option<i32> {
        let need_active = self.callbacks_need_active;
        let d = self.descriptors.get_mut(i)?;
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
        let Some(d) = self.descriptors.get(index).cloned() else {
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
        let d = &mut self.descriptors[index];
        d.dirty = true;
        d.changed = true;
    }

    /// The same for whatever `ACTDESC` last selected, which is what the words
    /// have in hand.
    pub(crate) fn touch_current(&mut self) {
        if let Some(i) = self.selected {
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
        if d.kind == DescriptorKind::Text {
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
        for d in self.descriptors.iter_mut().filter(|d| d.screen == screen) {
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
        self.set_field("SD%SHR", v);
        self.set_field("SDH%SHR", v);
        self.set_field("SDV%SHR", v);
    }

    /// The next chain stamp — see [`Descriptor::stamp`].
    pub(crate) fn next_stamp(&mut self) -> u64 {
        self.level_stamp += 1;
        self.level_stamp
    }

    /// A screen's surface has been wiped, so nothing owes it a rebuild.
    ///
    /// `FADEOUT` fills the surface with colour 0 before its bands start
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
    pub(crate) fn require_descriptor(&mut self, word: &str) -> Result<&mut Descriptor> {
        let current = self.selected;
        self.descriptor_mut().ok_or_else(|| Error::Unimplemented {
            ordinal: 0,
            name: format!("{word} with no current descriptor (ACTDESC selected {current:?})"),
            at: motionvm_forth::Address(0),
        })
    }

    /// The selected descriptor, or `None` when nothing is selected.
    ///
    /// The quiet counterpart to [`Engine::require_descriptor`]: for the words
    /// whose handlers do nothing rather than fail when the lookup comes back
    /// empty. Which of the two a word uses is the original's choice, not ours.
    pub(crate) fn descriptor_mut(&mut self) -> Option<&mut Descriptor> {
        let i = self.selected?;
        self.descriptors.get_mut(i)
    }
}

/// The `SD…` words as methods, acting on whatever `ACTDESC` last selected.
///
/// The kernel words are the interface the *bytecode* uses; these are the same
/// behaviors for callers who already know which descriptor and which value
/// they mean. `words/descriptors.rs` keeps the words themselves and is now only
/// the stack ABI over these: pop, then call.
///
/// **The selection is not saved and restored.** It is engine state that the
/// game's own bytecode shares — the original selects a descriptor and writes to
/// it several words later — so a method that tidied up after itself would be a
/// divergence, not an improvement.
///
/// Two families, and the difference is behavior rather than style: the ones
/// returning [`Result`] go through `require_descriptor` and **fail** when
/// nothing is selected, the ones returning `()` go through `descriptor_mut`
/// and quietly do nothing. That split is inherited from the original's own
/// handlers and is preserved word for word.
impl Engine {
    /// `SDACTIVE` and `SDINACTIVE`: whether the selected descriptor is drawn.
    ///
    /// Silent when nothing is selected, and silent when the bit is already
    /// what it is asked for — both handlers test it first (0x71ff9, 0x7204c)
    /// and do nothing at all when there is nothing to change.
    ///
    /// The order of the two lines is theirs: `SDACTIVE` sets the bit and then
    /// marks (0x72002, 0x72025), `SDINACTIVE` marks and then clears it
    /// (0x72055, 0x7205d). Both therefore mark while the descriptor still has
    /// the shape the mark is about.
    pub(crate) fn set_active(&mut self, active: bool) {
        let Some(i) = self.selected else {
            return;
        };
        if self.descriptors[i].active == active {
            return;
        }
        if active {
            self.descriptors[i].active = true;
            self.touch(i);
        } else {
            self.touch(i);
            self.descriptors[i].active = false;
        }
    }

    /// Marks around a change: the rectangle it leaves and the one it takes.
    ///
    /// `SDX` runs 0x6ab6e twice, before the store and after it (0x7112f,
    /// 0x7114e), because a descriptor that moves damages both places. Every
    /// setter here does the same, and every one of them leaves early when the
    /// value is the one already there — as the 32-bit `SDX` (0x7111a) and
    /// `SDSPR` (0x71715) do. The 16-bit setters skip nothing: each writes
    /// and sets the dirty bit whatever the value — read on `SDX`
    /// (`05f1:0df2`), `SDFNT` (`05f1:1038`), `SDLEV`, `SDNORM` — which is
    /// what makes the intro's `.DRAWNEW` (`SMDESC GDX SDX`, a value written
    /// over itself) force a redraw. [`Engine::sd_marks_always`] carries the
    /// difference.
    fn changing(&mut self, word: &'static str, change: impl FnOnce(&mut Descriptor)) -> Result<()> {
        self.require_descriptor(word)?;
        let i = self.selected.expect("require_descriptor found one");
        self.touch(i);
        change(&mut self.descriptors[i]);
        self.touch(i);
        Ok(())
    }

    /// `SDX`, `SDCX`/`SDCEN`, `SDOX`: the horizontal coordinate and what it means.
    ///
    /// The three words differ only in the [`Placement`] they store beside the
    /// number, which is exactly how the original encodes them.
    pub(crate) fn place_x(&mut self, v: i32, mode: Placement) -> Result<()> {
        let always = self.sd_marks_always;
        let d = self.require_descriptor("SDX")?;
        if !always && (d.x, d.x_mode) == (v, mode) {
            return Ok(());
        }
        self.changing("SDX", |d| (d.x, d.x_mode) = (v, mode))
    }

    /// `SDY`, `SDCY`/`SDVCEN`, `SDOY`: the vertical coordinate and what it means.
    ///
    /// `SDOX` is `SDOY` turned sideways: 0x71212 tests the *x* at +4 and 0x6bd97
    /// the horizontal mode, where 0x71419 and 0x6bdc8 take the y and the
    /// vertical one. Both then place against the far edge — `SDOX` is a
    /// *placement*, not a field to store under its own name. Recording it
    /// without acting on it leaves the dialogue's right margin unplaced.
    pub(crate) fn place_y(&mut self, v: i32, mode: Placement) -> Result<()> {
        let always = self.sd_marks_always;
        let d = self.require_descriptor("SDY")?;
        if !always && (d.y, d.y_mode) == (v, mode) {
            return Ok(());
        }
        self.changing("SDY", |d| (d.y, d.y_mode) = (v, mode))
    }

    /// `SDLEV`, `SDLV`, `SDZ`: the level the per-frame walk sorts by.
    pub(crate) fn set_level(&mut self, v: i32) -> Result<()> {
        if !self.level_chain && self.require_descriptor("SDLEV")?.level == v {
            return Ok(());
        }
        // The 16-bit handler re-inserts into the level chain and sets the
        // dirty bit whatever the value (`05f1:1018`), so an unchanged level
        // still moves the descriptor after its equals — see
        // [`Descriptor::stamp`].
        let stamp = self.next_stamp();
        // Marked twice over for a second reason here: the two marks go down at
        // the old level and the new one, so both depths are repainted.
        self.changing("SDLEV", |d| {
            d.level = v;
            d.stamp = stamp;
        })
    }

    /// `SDSPR`: the sprite to draw. Negative clears it.
    pub(crate) fn set_sprite(&mut self, v: i32) -> Result<()> {
        let want = (v >= 0).then_some(v as u32);
        if !self.sd_marks_always && self.require_descriptor("SDSPR")?.sprite == want {
            return Ok(());
        }
        self.changing("SDSPR", |d| d.sprite = want)
    }

    /// `SDBL`: the block to draw. Negative clears it.
    ///
    /// The same graphics pool as [`Engine::set_sprite`] — the two differ in
    /// descriptor *type*, not in where the picture comes from.
    pub(crate) fn set_block(&mut self, v: i32) -> Result<()> {
        let want = (v >= 0).then_some(v as u32);
        if !self.sd_marks_always && self.require_descriptor("SDBL")?.block == want {
            return Ok(());
        }
        self.changing("SDBL", |d| d.block = want)
    }

    /// `SDTXT`: the text entry to show, which makes this a text descriptor.
    pub(crate) fn set_text(&mut self, v: i32) -> Result<()> {
        let always = self.sd_marks_always;
        let d = self.require_descriptor("SDTXT")?;
        if !always && d.text == Some(v) && d.kind == DescriptorKind::Text {
            return Ok(());
        }
        self.changing("SDTXT", |d| {
            d.text = Some(v);
            d.kind = DescriptorKind::Text;
        })
    }

    /// `SDTB`: the text table to read from, which makes this a text descriptor.
    pub(crate) fn set_text_table(&mut self, v: i32) -> Result<()> {
        let always = self.sd_marks_always;
        let d = self.require_descriptor("SDTB")?;
        if !always && d.table == Some(v) && d.kind == DescriptorKind::Text {
            return Ok(());
        }
        self.changing("SDTB", |d| {
            d.table = Some(v);
            d.kind = DescriptorKind::Text;
        })
    }

    /// `SDCOL`: the composite color, backing bit and all.
    ///
    /// Stored undivided. The drawer tests it at 0x69f50 with `cmpl $0x100` and
    /// runs a backing pass for anything from 256 up; the low byte is the
    /// palette index. Clamping it here threw both halves away.
    pub(crate) fn set_color(&mut self, v: i32) -> Result<()> {
        if !self.sd_marks_always && self.require_descriptor("SDCOL")?.color == v {
            return Ok(());
        }
        self.changing("SDCOL", |d| d.color = v)
    }

    /// `SDFNT`: which registered font the text draws in.
    pub(crate) fn set_font(&mut self, v: i32) -> Result<()> {
        if !self.sd_marks_always && self.require_descriptor("SDFNT")?.font == Some(v) {
            return Ok(());
        }
        self.changing("SDFNT", |d| d.font = Some(v))
    }

    /// `SDTDT`: the text template, which names the outline font and the gaps.
    ///
    /// The 16-bit handler takes only 1..=20 (`05f1:0c78`: `cmp $1` / `jl`,
    /// `cmp $0x14` / `jg` skip the store and the dirty mark alike), so
    /// `0 SDTDT` cannot clear a template — the descriptor keeps the one it
    /// has. `SAYDAVID` runs on that before any `SETSAY` has filled
    /// `_SxTDT`. The 32-bit handler has no such gate; there a stored 0
    /// simply makes the drawer skip the outline pass (`cmpl $0,8(%eax)`,
    /// `jle` at `0x6a0c6`), which storing an unmatched id reproduces.
    pub(crate) fn set_template(&mut self, v: i32) -> Result<()> {
        if self.text16 && !(1..=20).contains(&v) {
            return Ok(());
        }
        if !self.sd_marks_always && self.require_descriptor("SDTDT")?.template == Some(v) {
            return Ok(());
        }
        self.changing("SDTDT", |d| d.template = Some(v))
    }

    /// `SDWAIT`: how long the descriptor waits before it is taken down.
    pub(crate) fn set_wait(&mut self, v: i32) -> Result<()> {
        self.require_descriptor("SDWAIT")?.wait = v;
        Ok(())
    }

    /// `SDWORD`: the callback word, already screened.
    ///
    /// `SDWORD` screens its argument exactly as `NEWSETDESC` does — 0 and -1
    /// clear the field (0x72bf4-0x72c03), and the same three checks stand
    /// between anything else and the store at 0x72c3a. The screening is
    /// the machine's, [`motionvm_forth::AddressSpace::callable`], and stays
    /// at the call site, because it needs the machine's memory to decide and
    /// this does not.
    pub(crate) fn set_callback(&mut self, v: i32) -> Result<()> {
        self.require_descriptor("SDWORD")?.callback = v;
        Ok(())
    }

    /// One of the setters kept by name rather than modeled.
    ///
    /// `key` must come from [`DESCRIPTOR_SETTERS`], because that is what the
    /// field map is keyed on. Silent when nothing is selected.
    pub(crate) fn set_field(&mut self, key: &'static str, v: i32) {
        let Some(i) = self.selected else {
            return;
        };
        if !self.sd_marks_always && self.descriptors[i].fields.get(key) == Some(&v) {
            return;
        }
        // Through the same marking as the modelled setters, because two of
        // these — `SD%SHR` and its per-axis pair — change how large the
        // descriptor draws, and a size that changes without a mark leaves the
        // difference standing on the surface.
        self.touch(i);
        self.descriptors[i].fields.insert(key, v);
        self.touch(i);
    }

    /// What every `GD…` word starts from: the selected descriptor and the
    /// corner it is actually drawn at.
    ///
    /// Nothing selected answers with a default descriptor rather than failing,
    /// which is what the getters have always done — they are read by bytecode
    /// that tests the answer, not by anything that could handle an error.
    ///
    /// `&mut self` because measuring a text may have to load its font — see
    /// [`Engine::extent`](crate::Engine::extent) for why that borrow is real
    /// and not a wart.
    fn selected_with_corner(&mut self) -> (Descriptor, i32, i32) {
        let d = self.descriptor_mut().cloned().unwrap_or_default();
        let (cx, cy) = self.corner(&d);
        (d, cx, cy)
    }

    /// `GDX`: the drawn left edge.
    ///
    /// The *drawn* corner, not the number that was stored. `SETT1` and `TSC`
    /// both test a text for overhang by comparing against the screen origin,
    /// and a centered descriptor answering with its center never overhangs —
    /// which is why Gaby's line ran off the left of the screen with the clamp
    /// sitting right there, never firing.
    pub(crate) fn descriptor_x(&mut self) -> i32 {
        self.selected_with_corner().1
    }

    /// `GDY`: the drawn top edge. See [`Engine::descriptor_x`].
    pub(crate) fn descriptor_y(&mut self) -> i32 {
        self.selected_with_corner().2
    }

    /// `GDLEV`, `GDLV`, `GDZ`: the level.
    pub(crate) fn descriptor_level(&mut self) -> i32 {
        self.selected_with_corner().0.level
    }

    /// `GDACTIVE`: whether the descriptor is drawn.
    pub(crate) fn descriptor_active(&mut self) -> i32 {
        i32::from(self.selected_with_corner().0.active)
    }

    /// `GDSPR`: the sprite id, or -1.
    pub(crate) fn descriptor_sprite(&mut self) -> i32 {
        self.selected_with_corner()
            .0
            .sprite
            .map_or(-1, |s| s as i32)
    }

    /// `GDBL`: the block id, or -1.
    pub(crate) fn descriptor_block(&mut self) -> i32 {
        self.selected_with_corner().0.block.map_or(-1, |b| b as i32)
    }

    /// `GDTXT`: the text entry.
    pub(crate) fn descriptor_text_entry(&mut self) -> i32 {
        self.selected_with_corner().0.text.unwrap_or(0)
    }

    /// `GDTB`: the text table.
    pub(crate) fn descriptor_table(&mut self) -> i32 {
        self.selected_with_corner().0.table.unwrap_or(0)
    }

    /// `GDCOL`: the composite color `SDCOL` stored, undivided.
    ///
    /// Reaches the text record through the same accessor `SDCOL` writes
    /// through and reads the same field (0x6a5e7 then +0xC at 0x731a0, against
    /// 0x730d4/0x730df in the setter). So a backed text answers 421, not 165 —
    /// the backing bit is part of the value and comes back out with it.
    pub(crate) fn descriptor_color(&mut self) -> i32 {
        self.selected_with_corner().0.color
    }

    /// The corner and the **stored** size, which is what the derived getters
    /// below are built from.
    ///
    /// The size the handlers use is the stored one: 0x6b190 reads +0x2e and
    /// 0x6b1b6 reads +0x32, which for a text is the measured extent plus four.
    /// Deriving these from the bare measurement instead put every center and
    /// every far edge two pixels off.
    ///
    /// Earlier still, `GDCX`, `GDCY`, `GDWIDTH` and `GDHEIGHT` were answered
    /// out of whatever a *setter* had filed away under the same name. They are
    /// geometry, not stored values, and nothing ever set those fields — so they
    /// answered zero, and `TSC` had nothing to clamp with.
    fn selected_box(&mut self) -> (i32, i32, i32, i32) {
        let (d, cx, cy) = self.selected_with_corner();
        let (w, h) = self.stored_extent(&d);
        (cx, cy, w, h)
    }

    /// `GDCX`: the horizontal center. 0x712a9 halves the stored width and adds
    /// the corner.
    pub(crate) fn descriptor_center_x(&mut self) -> i32 {
        let (cx, _, w, _) = self.selected_box();
        cx + w / 2
    }

    /// `GDCY`: the vertical center. See [`Engine::descriptor_center_x`].
    pub(crate) fn descriptor_center_y(&mut self) -> i32 {
        let (_, cy, _, h) = self.selected_box();
        cy + h / 2
    }

    /// `GDWIDTH` and `GDXLEN`: the stored width.
    ///
    /// Two names apiece, one handler: `GDWIDTH` is table 2 index 67 and
    /// `GDXLEN` index 65, both 0x72c46. They are synonyms, not neighbors.
    pub(crate) fn descriptor_width(&mut self) -> i32 {
        self.selected_box().2
    }

    /// `GDHEIGHT` and `GDYLEN`: the stored height, both 0x72c77.
    pub(crate) fn descriptor_height(&mut self) -> i32 {
        self.selected_box().3
    }

    /// `GDOX`: the far right edge, corner plus size — added at 0x712ff.
    pub(crate) fn descriptor_far_x(&mut self) -> i32 {
        let (cx, _, w, _) = self.selected_box();
        cx + w
    }

    /// `GDOY`: the far bottom edge, added at 0x71506.
    pub(crate) fn descriptor_far_y(&mut self) -> i32 {
        let (_, cy, _, h) = self.selected_box();
        cy + h
    }
}

/// What a coordinate on a descriptor means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// The value is the left or top edge. `NEWSETDESC` starts here.
    #[default]
    Edge,
    /// The value is the center, so the edge is it minus half the drawn size.
    ///
    /// Verified by arithmetic against the frame captured from the original:
    /// sprite 86 is 320x200, `2000 SD%SHR` makes it 640x400, and the title
    /// macro sets the center to (320, 240). Minus half the size that is
    /// exactly (0, 40) — the position the pixel comparison already agreed on.
    Center,
    /// The value is the far edge — the right one across, the bottom one down.
    ///
    /// Read from the pair of routines that work out a descriptor's rectangle,
    /// `0x6beec` across and `0x6c57c` down. Both dispatch on the same mode and
    /// differ only in which field they take:
    ///
    /// ```text
    /// mode 1:  edge   = value
    /// mode 2:  edge   = value - size/2
    /// mode 3:  edge   = value - size
    /// ```
    ///
    /// (The routines subtract a further 4 to 6 pixels, but that is the margin
    /// of the area they mark for repainting, not part of the position.)
    ///
    /// For `SDOY` down this is the figure's feet, which is what a walking
    /// character is placed by.
    FarEdge,
}

/// What a descriptor draws.
///
/// The original keeps this in a two-byte field at offset 2 and switches on it:
/// `SDSPR` writes 2 and allocates eight bytes, `SDBL` writes 3 with four, and
/// `SDTXT` and `SDTB` both write 4 with 0x5c. A descriptor is therefore one of
/// these and not several — which matters, because a descriptor that was once
/// given a text and later a sprite would otherwise draw both.
///
/// It was called `Kind2` for a while, after a collision with
/// [`motionvm_formats::m32::Kind`] — a name that recorded the accident rather than
/// the thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DescriptorKind {
    /// Nothing chosen yet. `NEWSETDESC` leaves a descriptor here when it is
    /// given no graphic.
    #[default]
    Empty,
    /// A sprite, from `SDSPR`.
    Sprite,
    /// A background block, from `SDBL`.
    Block,
    /// Text, from `SDTXT` or `SDTB`.
    Text,
}

/// One entry of the engine's scene graph.
///
/// The original packs these into a 0x3E-byte struct; the field offsets that are
/// known (`+4` x, `+6` y, `+8` level) came out of the `NEWSETDESC` and `SDX`
/// handlers. Only the behavior is reproduced here, not the layout.
#[derive(Debug, Clone, Default)]
pub struct Descriptor {
    /// What `NEWSETDESC` answered with, and what `ACTDESC` selects by.
    pub handle: u32,
    /// The screen that was current when the descriptor was made.
    pub screen: u32,
    /// The horizontal coordinate; what it means is [`Descriptor::x_mode`].
    pub x: i32,
    /// The vertical coordinate; what it means is [`Descriptor::y_mode`].
    pub y: i32,
    /// `SDLEV`, `SDLV`, `SDZ`: the order the per-frame walk draws in, low
    /// first. Ties keep the order they were made in.
    pub level: i32,
    /// `SDSPR`: the sprite to draw.
    pub sprite: Option<u32>,
    /// `SDBL`: a background picture. Indexes the same graphics pool as
    /// `sprite`; the two differ in descriptor type, not in where the picture
    /// comes from.
    pub block: Option<u32>,
    /// `SDTXT`: index into a text table.
    pub text: Option<i32>,
    /// `SDTB`: the descriptor's text buffer or table id.
    pub table: Option<i32>,
    /// `SDFNT`: which registered font the text draws in. Unset falls back to
    /// the system font, not to whatever was registered first.
    pub font: Option<i32>,
    /// `SDCOL`: field +0x0C of the text record, composite and unmasked.
    ///
    /// A plain value rather than an option, because the original has no
    /// "unset" to model. The text record does not exist until `SDTXT`
    /// (0x71b4f) or `SDTB` (0x71d45) allocates it, and both allocate through
    /// 0x203F3 → 0x823E0, which is a *zeroing* allocator (`xor %al,%al;
    /// rep stos`). `SDTB` then writes +0x00, +0x04, +0x08, +0x10 and +0x18
    /// explicitly and leaves +0x0C to the allocator — so a text nobody gave a
    /// color is deterministically **0**, black in 54 of the 60 shipped
    /// palettes, and `GDCOL` (0x73177) answers 0 for it.
    ///
    /// Modeling it as an option invites two readers to pick different
    /// defaults — 255 for the drawer, 0 for `GDCOL` — and nothing compares
    /// them. 255 is magenta in 42 of the 60 palettes, and it is the help pages
    /// that show it: every one of their texts runs on the default, because
    /// `SHOW_DOC` contains no `SDCOL` at all and `XYLTITEM.` (module 2,
    /// 0x01364) never gives them one.
    pub color: i32,
    /// `SDTDT`: the text template, which names the outline font and the gaps.
    pub template: Option<i32>,
    /// `SDWAIT`: field +0x18, a plain count of frames.
    ///
    /// The original's per-frame walk over the descriptors (0x68c64, driven by
    /// `ANIMPLAY`) counts this down and runs [`Descriptor::callback`] when it
    /// reaches zero. Neither the walk nor the callbacks happen here yet, which
    /// is why a text shown with a wait never finishes.
    pub wait: i32,
    /// `SDWORD`: field +0x14, the address of a bytecode word.
    ///
    /// Not text word-wrapping, which is what this field was called until the
    /// handler was read: `SDWORD` stores a word address that the frame walk
    /// executes once the wait above expires, and clears the field for 0 or -1.
    /// The game uses it to end a displayed text —
    /// `_IINFO @ ACTDESC … SDACTIVE  0x53370 SDWORD  0 SDWAIT` in module 5 —
    /// and `?TEXTREADY` waits for exactly that to have happened.
    pub callback: i32,
    /// Where the level chain last put it among its equals.
    ///
    /// The 16-bit engine keeps its descriptors on a doubly linked chain
    /// sorted by level, and `SDLEV` (`05f1:1018`) always unlinks and
    /// re-inserts — `0362:2007` walks to the first node whose level is
    /// *greater* and splices in before it, so the freshest set lands
    /// **after** every equal and draws on top of them (the drawer walks the
    /// chain head first, `016a:0a4f`). This stamp is that position: paint
    /// order is `(level, stamp)`. On the 32-bit machine nothing bumps it
    /// after creation, so the order stays what it always was — creation
    /// order among equals; its chain handler is unread. Not saved: a loaded
    /// game restamps in list order, and the next `SDLEV` puts a moving
    /// figure back where the chain would have it.
    pub stamp: u64,
    /// How `x` and `y` are to be read, one mode per axis.
    ///
    /// The original keeps both in the low nibble of the word at +0x12 — bits
    /// 0-1 across, bits 2-3 down — and each setter word writes its own:
    /// `SDX`/`SDY` mean the edge, `SDCX`/`SDCY`/`SDCEN`/`SDVCEN` the center,
    /// `SDOY` an offset. `NEWSETDESC` sets both to the edge, which is why
    /// ignoring the whole thing worked until a figure came along that is placed
    /// by its center.
    pub x_mode: Placement,
    /// What [`Descriptor::y`] means: an edge, a center, or the far edge.
    pub y_mode: Placement,
    /// Everything else a `SD*` word can set, kept by name.
    ///
    /// The descriptor struct in the original is 0x3E bytes of fields whose
    /// meanings are not all known yet. Recording the rest by name keeps the
    /// values — they are needed to reproduce a frame — without inventing a
    /// meaning for each one before it has been measured.
    pub fields: BTreeMap<&'static str, i32>,
    /// Which of the three kinds this is, as the type field at offset 2 records.
    pub kind: DescriptorKind,
    /// Whether the drawer visits it at all — bit 0x80 of the flag byte at
    /// +0x13, set and cleared only by `SDACTIVE`/`SDINACTIVE`.
    ///
    /// **Clearing it erases nothing.** `SDINACTIVE` (0x72033) marks the
    /// descriptor's rectangle on its *own* level (0x6ab6e → 0x6e701), so
    /// everything above it is repainted and nothing below is — and the
    /// descriptor itself is no longer drawn. Its pixels therefore stay on the
    /// surface until something paints over them, unless it carries a
    /// [`Descriptor::auto_buffer`].
    pub active: bool,
    /// Bit 0x40: the descriptor has to be drawn on the next pass.
    ///
    /// A new descriptor starts with it (`NEWSETDESC` writes the flag word
    /// 0xD000 at 0x70d07); every change sets it (0x6ab6e); the damage map sets
    /// it on the neighbours (0x6e8c8); the drawer clears it once it has drawn
    /// (0x69659). Without it a descriptor is skipped even while active
    /// (0x694ed, 0x69680).
    pub dirty: bool,
    /// Bit 0x10: this descriptor is what changed, as opposed to a neighbour.
    ///
    /// Set beside [`Descriptor::dirty`] by 0x6ab6e and cleared by the drawer
    /// with it. In the original it picks between two blitters — 0x27765 /
    /// 0x29ae9 against 0x273e8 / 0x299e1, whose destination is the display's
    /// software surface at 0xE7D7C (0x69331, 0x697cd). motionvm has one
    /// drawing path, onto the screen's own surface, and that path is the one
    /// checked pixel for pixel against the original; the bit is carried so the
    /// distinction is not lost, and what it is for is an open question.
    pub changed: bool,
    /// `SDBUF`, on the 16-bit machine: the off-screen buffer this descriptor
    /// draws through, by the number `SETBUF` allocated it under. What the
    /// 16-bit kernel does with it is unread — see [`crate::Buffers`]; the
    /// 32-bit game keeps `SDBUF` as a by-name field and never reads it.
    pub buffer: Option<i32>,
    /// `SDAUTOBUF`: whether taking this descriptor away puts the picture back.
    ///
    /// `SDAUTOBUF` (0x72104) hands the descriptor a buffer number at +0x1C and
    /// sets flag 0x08, and the original keeps a copy of the surface under it:
    /// the drawer saves the area before every blit (0x696ed → 0x6b936 →
    /// 0x2825d) and 0x6ab6e pastes the copy back when the descriptor is hidden
    /// or moves (0x6ac33). That copy is what makes something disappear — and
    /// without one, nothing does.
    ///
    /// **motionvm rebuilds the area instead of remembering it.** When a
    /// descriptor with this flag goes away, its rectangle is cleared and every
    /// active descriptor across it is drawn again (`Engine::draw_screens`), so
    /// what comes back is what the scene graph says belongs there. The two
    /// agree wherever the picture under the descriptor came from descriptors,
    /// which is everywhere the game puts one of these: `INITANI` gives every
    /// animation the flag (module 6, 0x007dc), as do the captions, the menu
    /// sprites and the mailbox's bars. They differ only for pixels no
    /// descriptor owns — the mailbox's own hidden terminal rows, say, which a
    /// remembered copy would put back and a rebuild leaves out. A remembered
    /// copy also goes stale: it holds the surface as it was when it was taken,
    /// and every redraw underneath it since is news the copy does not have.
    pub auto_buffer: bool,
}

/// A text template defined by `DEFTDT`, which takes seventeen arguments.
#[derive(Debug, Clone)]
pub struct TextTemplate {
    /// The template number `DEFTDT` gave it, which `SDTDT` names.
    pub id: i32,
    /// The arguments as given, deepest first. What each one means is still open;
    /// recording them keeps the information until it is needed.
    pub args: Vec<i32>,
}

/// The two small enums a savegame has to carry, as numbers and back.
///
/// Written out by hand rather than derived, so that adding a variant makes the
/// mapping a decision instead of silently renumbering every saved file.
pub(crate) fn placement_code(p: Placement) -> u8 {
    match p {
        Placement::Edge => 0,
        Placement::Center => 1,
        Placement::FarEdge => 2,
    }
}

pub(crate) fn placement_of(code: u8) -> std::result::Result<Placement, String> {
    match code {
        0 => Ok(Placement::Edge),
        1 => Ok(Placement::Center),
        2 => Ok(Placement::FarEdge),
        n => Err(format!(
            "savegame has placement mode {n}, which this build does not know"
        )),
    }
}

pub(crate) fn kind_code(k: DescriptorKind) -> u8 {
    match k {
        DescriptorKind::Empty => 0,
        DescriptorKind::Sprite => 1,
        DescriptorKind::Block => 2,
        DescriptorKind::Text => 3,
    }
}

pub(crate) fn kind_of(code: u8) -> std::result::Result<DescriptorKind, String> {
    match code {
        0 => Ok(DescriptorKind::Empty),
        1 => Ok(DescriptorKind::Sprite),
        2 => Ok(DescriptorKind::Block),
        3 => Ok(DescriptorKind::Text),
        n => Err(format!(
            "savegame has descriptor kind {n}, which this build does not know"
        )),
    }
}

/// The one-argument descriptor setters, which are also the field keys they
/// write.
///
/// Kept as a table rather than as a list of match arms so that the arm and the
/// key cannot drift apart. Written out twice — once in a pattern and once in
/// [`DESCRIPTOR_FIELDS`] — they would need an `expect` between them saying
/// "arm and table list the same names", which is a promise nothing would be
/// asking the compiler to keep.
pub(crate) const DESCRIPTOR_SETTERS: &[&str] = &[
    "SD%SHR",
    "SDV%SHR",
    "SDH%SHR",
    "SDBUF",
    "SDSTARTLINE",
    "SDALINES",
    "SDTRANS",
    "SDSHADE",
];

/// Descriptor fields kept by name rather than modeled.
///
/// A superset of [`DESCRIPTOR_SETTERS`]: `INSERT` is a field too, but it is
/// written by `SDINSERT`, which takes three arguments and is not one of these.
/// A savegame may name any of them.
pub(crate) const DESCRIPTOR_FIELDS: &[&str] = &[
    "SD%SHR",
    "SDV%SHR",
    "SDH%SHR",
    "SDBUF",
    "SDSTARTLINE",
    "SDALINES",
    "SDTRANS",
    "SDSHADE",
    "INSERT",
];
