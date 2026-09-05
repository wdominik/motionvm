//! The `SD…` and `GD…` words as methods on whatever `ACTDESC` last selected.
//!
//! One file for the whole interface the bytecode writes a descriptor
//! through, because the two dozen of them are one subject: pop a value,
//! change one field, mark the neighbours. The table underneath is
//! [`super::table`]'s and the record they change is the parent module's.

use super::{Descriptor, Placement, Shows};
use crate::{Engine, Field};
use motionvm_motion_forth::Result;
use motionvm_motion_forth::cell;
/// The `SD…` words as methods, acting on whatever `ACTDESC` last selected.
///
/// The kernel words are the interface the *bytecode* uses; these are the same
/// behaviors for callers who already know which descriptor and which value
/// they mean. `words/descriptors.rs` keeps the words themselves and is only
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
        let Some(i) = self.scene.selected else {
            return;
        };
        if self.scene.descriptors[i].active == active {
            return;
        }
        if active {
            self.scene.descriptors[i].active = true;
            self.touch(i);
        } else {
            self.touch(i);
            self.scene.descriptors[i].active = false;
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
    /// over itself) force a redraw. [`crate::Profile::sd_marks_always`] carries the
    /// difference.
    fn changing(&mut self, word: &'static str, change: impl FnOnce(&mut Descriptor)) -> Result<()> {
        let i = self.require_selected(word)?;
        self.touch(i);
        change(&mut self.scene.descriptors[i]);
        self.touch(i);
        Ok(())
    }

    /// `SDX`, `SDCX`/`SDCEN`, `SDOX`: the horizontal coordinate and what it means.
    ///
    /// The three words differ only in the [`Placement`] they store beside the
    /// number, which is exactly how the original encodes them.
    pub(crate) fn place_x(&mut self, v: i32, mode: Placement) -> Result<()> {
        let always = self.profile.sd_marks_always;
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
        let always = self.profile.sd_marks_always;
        let d = self.require_descriptor("SDY")?;
        if !always && (d.y, d.y_mode) == (v, mode) {
            return Ok(());
        }
        self.changing("SDY", |d| (d.y, d.y_mode) = (v, mode))
    }

    /// `SDLEV`, `SDLV`, `SDZ`: the level the per-frame walk sorts by.
    pub(crate) fn set_level(&mut self, v: i32) -> Result<()> {
        if !self.profile.level_chain && self.require_descriptor("SDLEV")?.level == v {
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
    ///
    /// Writes the one field [`Shows`] is, with the sprite marker on — the
    /// original's `+0x10` with bit 15 set (`05f1:12b6`). Whatever the
    /// descriptor showed before is gone, because it is the same word.
    pub(crate) fn set_sprite(&mut self, v: i32) -> Result<()> {
        let want = if v >= 0 {
            Shows::Sprite(cell::unsigned(v))
        } else {
            Shows::Nothing
        };
        if !self.profile.sd_marks_always && self.require_descriptor("SDSPR")?.shows == want {
            return Ok(());
        }
        self.changing("SDSPR", |d| d.shows = want)
    }

    /// `SDBL`: the picture to draw. Negative clears it.
    ///
    /// The same field and the same graphics pool as [`Engine::set_sprite`],
    /// only without the marker (`05f1:11ee`): what tells a block from a sprite
    /// is bit 15, not where the picture comes from. And on a descriptor
    /// `SDTXT` has already made a text, the same word is that text's table —
    /// which is how Hilfe für Amajambere turns its diary pages
    /// (`16 SDBL 1 SDTXT`).
    pub(crate) fn set_block(&mut self, v: i32) -> Result<()> {
        let want = if v >= 0 {
            Shows::Picture(v)
        } else {
            Shows::Nothing
        };
        if !self.profile.sd_marks_always && self.require_descriptor("SDBL")?.shows == want {
            return Ok(());
        }
        self.changing("SDBL", |d| d.shows = want)
    }

    /// `SDTXT`: the text entry to show, which makes this a text descriptor.
    pub(crate) fn set_text(&mut self, v: i32) -> Result<()> {
        let always = self.profile.sd_marks_always;
        let d = self.require_descriptor("SDTXT")?;
        if !always && d.text == Some(v) {
            return Ok(());
        }
        self.changing("SDTXT", |d| d.text = Some(v))
    }

    /// `SDTB`: the text table to read from.
    ///
    /// The same store as `SDBL` — one word, one picture-or-table
    /// (`05f1:0d7b`). On the 16-bit machine it does *not* make the descriptor
    /// a text: only `SDTXT` does that, and until it runs the value is read as
    /// a block. On the 32-bit machine `SDTB` allocates the text record itself,
    /// so there it marks the descriptor as `SDTXT` would.
    pub(crate) fn set_text_table(&mut self, v: i32) -> Result<()> {
        let always = self.profile.sd_marks_always;
        let allocates = self.profile.sdtb_allocates_text;
        let d = self.require_descriptor("SDTB")?;
        let settled = d.shows == Shows::Picture(v) && (!allocates || d.text.is_some());
        if !always && settled {
            return Ok(());
        }
        self.changing("SDTB", |d| {
            d.shows = Shows::Picture(v);
            if allocates && d.text.is_none() {
                // The 32-bit engine allocates the text record here (0x71d45),
                // so a descriptor is a text from `SDTB` on even before
                // `SDTXT` names an entry. Entry 0 is what that record holds.
                d.text = Some(0);
            }
        })
    }

    /// `SDCOL`: the composite color, backing bit and all.
    ///
    /// Stored undivided. The drawer tests it at 0x69f50 with `cmpl $0x100` and
    /// runs a backing pass for anything from 256 up; the low byte is the
    /// palette index. Clamping it here threw both halves away.
    pub(crate) fn set_color(&mut self, v: i32) -> Result<()> {
        if !self.profile.sd_marks_always && self.require_descriptor("SDCOL")?.color == v {
            return Ok(());
        }
        self.changing("SDCOL", |d| d.color = v)
    }

    /// `SDFNT`: which registered font the text draws in.
    pub(crate) fn set_font(&mut self, v: i32) -> Result<()> {
        if !self.profile.sd_marks_always && self.require_descriptor("SDFNT")?.font == Some(v) {
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
        if self.profile.templates_gated && !(1..=20).contains(&v) {
            return Ok(());
        }
        if !self.profile.sd_marks_always && self.require_descriptor("SDTDT")?.template == Some(v) {
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
    /// the machine's, [`motionvm_motion_forth::AddressSpace::callable`], and stays
    /// at the call site, because it needs the machine's memory to decide and
    /// this does not.
    pub(crate) fn set_callback(&mut self, v: i32) -> Result<()> {
        self.require_descriptor("SDWORD")?.callback = v;
        Ok(())
    }

    /// One of the setters kept by name rather than modeled.
    ///
    /// `key` comes from [`crate::words::Word::descriptor_field`], which is the
    /// only thing that turns a word into one. Silent when nothing is selected.
    pub(crate) fn set_field(&mut self, key: Field, v: i32) {
        let Some(i) = self.scene.selected else {
            return;
        };
        if !self.profile.sd_marks_always && self.scene.descriptors[i].fields.get(key) == Some(v) {
            return;
        }
        // Through the same marking as the modelled setters, because two of
        // these — `SD%SHR` and its per-axis pair — change how large the
        // descriptor draws, and a size that changes without a mark leaves the
        // difference standing on the surface.
        self.touch(i);
        self.scene.descriptors[i].fields.set(key, v);
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

    /// `GDSPR`: the sprite id, or -1 when the field holds something else.
    ///
    /// `05f1:16bd` tests bit 15 and answers -1 without it. The verb menu and
    /// the walk cycle count on from what this returns as a frame number, so
    /// handing them a block id would animate it.
    pub(crate) fn descriptor_sprite(&mut self) -> i32 {
        match self.selected_with_corner().0.shows {
            Shows::Sprite(id) => cell::signed(id),
            _ => -1,
        }
    }

    /// `GDBL`: the block id, or -1 when the field holds a sprite.
    ///
    /// `05f1:168e`, the mirror of `GDSPR`.
    pub(crate) fn descriptor_block(&mut self) -> i32 {
        match self.selected_with_corner().0.shows {
            Shows::Picture(id) => id,
            _ => -1,
        }
    }

    /// `GDTXT`: the text entry.
    pub(crate) fn descriptor_text_entry(&mut self) -> i32 {
        self.selected_with_corner().0.text.unwrap_or(0)
    }

    /// `GDTB`: the field raw, whatever it holds.
    ///
    /// `05f1:0d5c` reads `+0x10` and masks nothing, so a sprite comes back
    /// with its bit 15 still on. Only the 16-bit machine keeps the marker in
    /// the value; the 32-bit one has a type field of its own.
    pub(crate) fn descriptor_table(&mut self) -> i32 {
        let marks = self.profile.table_marks_sprites;
        match self.selected_with_corner().0.shows {
            Shows::Picture(id) => id,
            Shows::Sprite(id) if marks => cell::signed(id) | 0x8000,
            Shows::Sprite(id) => cell::signed(id),
            Shows::Nothing => 0,
        }
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
