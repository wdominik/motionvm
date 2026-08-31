//! Turning the descriptor list into pixels.
//!
//! **Nothing on this path may iterate a `HashMap`, read a clock, or walk a
//! directory unsorted.** A frame has to come out the same for the same input
//! state, every time, because rendering a scene before and after a change and
//! comparing the two is how the drawing code here is checked — and an
//! iteration order that varies between runs makes overlapping descriptors
//! land in a different order and quietly ruins that. `BTreeMap` and `Vec`
//! throughout is the whole of the discipline; there is no lint for it,
//! because the rule is about this path rather than about the workspace, where
//! a hash map off it is perfectly fine.
//!
//! Two steps, and the original keeps them apart: the **drawer** (`0x6915b`)
//! walks the descriptors in level order and fills each screen's surface, and
//! the **presenter** (`0x1457D`) copies what changed onto the visible screen.
//! A frame that only draws changes nothing anyone can see, which is why both
//! appear in the loop and why a test that forgets the second measures the frame
//! before it.
//!
//! **The drawer is incremental, and the surface persists.** It never clears a
//! screen buffer (0x6915b has no fill); it empties the screen's damage map
//! (0x69248), works out from that map which descriptors have to be drawn again
//! (0x6e8c8), and draws only those — a descriptor needs both the active bit
//! 0x80 and the dirty bit 0x40 to be visited at all (0x694ed, 0x69680), and
//! loses the dirty bit once it has been drawn (0x69659).
//!
//! So switching a descriptor off is *not* enough to erase it. `SDINACTIVE`
//! marks its rectangle at its own level (0x6ab6e), which repaints everything
//! above it and nothing below, and the descriptor itself is no longer drawn —
//! its pixels stay. The only thing that erases is `SDAUTOBUF`: the original
//! keeps a copy of the surface under such a descriptor and pastes it back
//! (0x6ac33), and this rebuilds the same area from the descriptor list
//! instead — see [`Descriptor::auto_buffer`](crate::Descriptor::auto_buffer).
//! The in-game mailbox is built on exactly that difference: its terminal rows
//! carry no `SDAUTOBUF`, so `HIDSCR` leaves them standing and `CLSCR` can wipe
//! them a row at a time.

use crate::{Descriptor, Engine, Placement, Shows, line_height};
use motionvm_render::Framebuffer;
use motionvm_render::Palette;

/// The text backing's remap row, kept until the palette changes.
///
/// [`crate::text::backing_map`] is a 256×256 nearest-color search — about
/// 196 000 operations — and it depends on nothing but the palette. It was being
/// rebuilt for every text descriptor that asks for a backing, which measured at
/// 39 µs each: six of them in the classroom, so 234 µs of a 666 µs draw.
///
/// The key is the palette itself rather than a generation counter. Comparing
/// 768 bytes is nothing against rebuilding the row, and it cannot go stale —
/// the palette is written from four places and a counter would have to be
/// remembered at each of them.
pub(crate) struct BackingCache {
    for_palette: Option<Palette>,
    row: [u8; 256],
}

impl Default for BackingCache {
    fn default() -> Self {
        Self {
            for_palette: None,
            row: [0; 256],
        }
    }
}

impl Engine {
    /// The remap row for the palette in force, built if the palette has moved.
    ///
    /// The script's palette, not the display's: the original builds its
    /// tables inside `SETPAL`, so anything composed after one — a `FADEIN`'s
    /// full draw included — already works from the new entries, even while
    /// a queued fade still shows the old ones. (Spelled out rather than
    /// through [`Engine::script_palette`] so the cache can stay a disjoint
    /// borrow.)
    pub(crate) fn backing_row(&mut self) -> [u8; 256] {
        let palette = self
            .wipes
            .iter()
            .rev()
            .find_map(|w| w.palette_after.as_ref())
            .or_else(|| {
                self.curtains
                    .iter()
                    .rev()
                    .find_map(|c| c.palette_after.as_ref())
            })
            .unwrap_or(&self.display.palette);
        let cache = &mut self.backing;
        if cache.for_palette.as_ref() != Some(palette) {
            cache.row = crate::text::backing_map(palette);
            cache.for_palette = Some(palette.clone());
        }
        cache.row
    }

    /// The presenter, 0x1457D: what has been drawn becomes what is seen.
    ///
    /// The original never draws to the visible screen. Everything paints one
    /// software surface (`0xE7D7C`) and marks 8×8 tiles in an update map
    /// (`0xE7D84`); this routine walks the map and copies just the marked
    /// tiles into video memory. Video memory therefore **persists** — and
    /// that, not the drawing, is what makes a fade look the way it does. See
    /// `Engine::advance_curtain`, which is where a curtain writes its bands.
    pub fn present(&mut self) {
        self.video = self.display.compose();
    }

    /// The frame to show.
    ///
    /// Nothing is composed here: [`Self::present`] has already put the frame
    /// where the original keeps it, and a running curtain has written its own
    /// bands over it.
    pub fn render(&mut self) -> Framebuffer {
        let mut out = self.video.clone();
        // The request box, over the frame and under the pointer: the drawer
        // blits it and calls `SHOWMOUSE` after (`0104:7332`).
        self.draw_request(&mut out);
        // The pointer, last of all. The mouse layer paints it straight onto
        // the video surface (0x2543e: save-under, then the masked blit
        // 0x26594), so it sits above everything. The 8-pixel alignment in
        // that routine is block bookkeeping only: the shape goes into the
        // block at the `& 7` remainder, so the net position on screen is
        // exactly pointer minus hotspot.
        //
        // Not while a curtain runs, though: both handlers call `HIDEMOUSE`
        // (0x74ae2) before the band loop and `SHOWMOUSE` (0x74b64) after it.
        // Only the drawing is held back — `pointer_visible` is the script's
        // own `SHOWMOUSE`/`HIDEMOUSE` state and nothing is claimed here about
        // how the two nest. The 16-bit wipes bracket their ring loops the
        // same way (`05f1:28f5`/`05f1:29d7`, `05f1:2aff`/`05f1:2c8b`).
        if self.pointer_visible
            && self.curtains.is_empty()
            && self.wipes.is_empty()
            && let Some((id, hx, hy)) = self.cursor
        {
            let (mx, my) = (self.mouse.x, self.mouse.y);
            if let Some(sprite) = self.sprite(id) {
                out.blit_scaled(&sprite, mx - hx, my - hy, 1000, 1000);
            }
        }
        out
    }

    /// The drawer, 0x6915b: descriptors onto the screen buffers.
    ///
    /// This is the only thing that puts a picture anywhere, and it runs when
    /// the original runs it — once a frame out of `ANIMPLAY`, after the
    /// controller has had its turn, and once inside `FADEIN` (0x74af9) before
    /// the bands start moving. Nowhere else.
    ///
    /// That is the whole point of having it separate from [`Self::render`].
    /// The buffers persist between calls, so they hold *what has been drawn* —
    /// black until something draws, and unchanged while a fade runs, because a
    /// fade returns to nobody and no frame passes. Composing from the
    /// descriptor list at display time instead showed pictures the original had
    /// never drawn: the inventory bar stood in the very first frames and was
    /// then faded away, and a text that had just faded out came back for the
    /// next fade to hide again.
    pub fn draw(&mut self) {
        self.draw_screens(None);
    }

    /// `FADEIN`'s one draw, restricted to the fading screen.
    ///
    /// The handler passes the screen record itself to the drawer (0x74af9),
    /// right after `orb $0xD0` on its flags (0x74ae7) — the fade is what
    /// brings the screen to life — and the band loop then runs synchronously
    /// inside the handler, so no other screen gets a frame while it moves.
    /// Drawing everything here instead let changes on *other* screens show
    /// mid-fade, which the original never does.
    pub fn draw_screen(&mut self, screen: u32) {
        self.draw_screens(Some(screen));
    }

    pub(crate) fn draw_screens(&mut self, only: Option<u32>) {
        let mine = |screen: u32| only.is_none_or(|o| screen == o);

        // 0x6e8c8, once over the chain before anything is drawn: a descriptor
        // joins the pass when a tile it covers is waiting for a repaint at or
        // below its own level. The ones already marked are in either way, and
        // the ones neither marked nor active are skipped (0x6ea02).
        for i in 0..self.descriptors.len() {
            let d = &self.descriptors[i];
            if d.dirty || !d.active || !mine(d.screen) {
                continue;
            }
            let d = d.clone();
            let (x, y, w, h) = self.drawn_rect(&d);
            let wanted = self
                .display
                .screens
                .iter()
                .find(|s| s.handle == d.screen)
                .is_some_and(|s| s.damaged(x, y, w, h, d.level));
            self.descriptors[i].dirty = wanted;
        }

        // 0x69248: the map is spent, and the pass starts from an empty one.
        for screen in &mut self.display.screens {
            if mine(screen.handle) {
                screen.reset_damage();
            }
        }

        // Where this pass is allowed to change the picture. Two things go in:
        //
        // * every place a descriptor carrying `SDAUTOBUF` has left — the
        //   original pastes a remembered copy of the surface back there
        //   (0x6ac33), and this builds the place again out of the descriptor
        //   list instead; see
        //   [`Descriptor::auto_buffer`](crate::Descriptor::auto_buffer);
        // * every place a descriptor that has to be drawn is going to cover.
        //
        // Everything else on the surface stays exactly as it was, which is what
        // makes the drawer incremental — and what lets the mailbox keep the
        // rows it has switched off until its own black bars eat them.
        //
        // Only the screens this pass covers give their debt up: a `FADEIN`
        // draws one screen alone (0x74af9), and what another screen is owed
        // still is.
        let (owed, kept) = std::mem::take(&mut self.rebuild)
            .into_iter()
            .partition::<Vec<_>, _>(|&(screen, _)| mine(screen));
        self.rebuild = kept;

        let mut region: Vec<(u32, (i32, i32, i32, i32))> = owed;
        for i in 0..self.descriptors.len() {
            let d = &self.descriptors[i];
            if !d.active || !d.dirty || !mine(d.screen) {
                continue;
            }
            let d = d.clone();
            region.push((d.screen, self.drawn_rect(&d)));
        }
        if region.is_empty() {
            return;
        }

        // Painted in full and published in part.
        //
        // A pass that painted straight onto the surface would have to clip
        // every blit to the region, and a blit that is clipped still has to
        // know it — the text passes place themselves from their own
        // measurements, and the darkening a text backing does is not something
        // that can be run twice over the same pixels without showing (which is
        // exactly what it did: an answer's backing went a shade darker every
        // time a neighbour moved). Painting the whole screen and then taking
        // only the region out of the result gives every published pixel exactly
        // one pass over it.
        let screens: std::collections::BTreeSet<u32> =
            region.iter().map(|&(screen, _)| screen).collect();
        for screen in screens {
            let Some(before) = self
                .display
                .screens
                .iter()
                .find(|s| s.handle == screen)
                .map(|s| s.buffer.clone())
            else {
                continue;
            };
            self.paint_all(screen);
            if let Some(s) = self.display.screen_mut(screen) {
                let fresh = std::mem::replace(&mut s.buffer, before);
                for &(_, (x, y, w, h)) in region.iter().filter(|&&(o, _)| o == screen) {
                    s.buffer.paste_rect(&fresh, x, y, w, h);
                }
            }
        }

        // 0x69659: drawn is drawn.
        for d in self.descriptors.iter_mut().filter(|d| mine(d.screen)) {
            (d.dirty, d.changed) = (false, false);
        }
    }

    /// Every active descriptor of one screen, over a cleared surface.
    ///
    /// The whole picture, the way the drawer would build it if nothing had ever
    /// been drawn. `draw_screens` paints into this and then publishes only the
    /// part of it that the pass is allowed to change.
    fn paint_all(&mut self, screen: u32) {
        if let Some(s) = self.display.screen_mut(screen) {
            s.buffer.fill(0);
        }
        let mut order: Vec<usize> = (0..self.descriptors.len())
            .filter(|&i| self.descriptors[i].active && self.descriptors[i].screen == screen)
            .collect();
        // `(level, stamp)`: the 16-bit level chain's order — among equals
        // the freshest `SDLEV` draws on top. On the 32-bit machine the
        // stamps never move after creation, so this is the plain stable
        // sort it always was; see [`crate::Descriptor::stamp`].
        order.sort_by_key(|&i| (self.descriptors[i].level, self.descriptors[i].stamp));
        for i in order {
            let d = self.descriptors[i].clone();
            self.paint_descriptor(&d);
        }
    }

    /// One descriptor onto its screen's surface.
    fn paint_descriptor(&mut self, d: &Descriptor) {
        // `block` indexes the same graphics pool as `sprite` — the ids the
        // game passes to `SDBL` (43, 60, 64, 66, 1010) are all present as
        // Gfx8 items, and 66 and 1010 are full 640x400 backgrounds. The two
        // differ in descriptor *type*, not in where the picture comes from:
        // the handler writes 2 with an eight-byte payload for a sprite and
        // 3 with a four-byte one for a block, the difference being the
        // animation state a background does not need.
        if d.is_text() {
            self.draw_text_descriptor(d);
            return;
        }
        let Some(id) = d.shows.graphic() else {
            return;
        };
        let Some(sprite) = self.sprite(id) else {
            return;
        };
        // `SD%SHR` scales in thousandths; unset means full size.
        let all = d.fields.get("SD%SHR").copied().unwrap_or(0);
        let h = d.fields.get("SDH%SHR").copied().unwrap_or(all).max(0) as u32;
        let v = d.fields.get("SDV%SHR").copied().unwrap_or(all).max(0) as u32;
        let (h, v) = (if h == 0 { 1000 } else { h }, if v == 0 { 1000 } else { v });
        // The drawn corner depends on what the coordinate means. A centered
        // sprite sits half its *scaled* size to the left and above the
        // point — that is what makes the title logo, 320x200 at 2000 per
        // mille with its center at (320, 240), land on (0, 40).
        let (sw, sh) = (
            sprite.width as i32 * h as i32 / 1000,
            sprite.height as i32 * v as i32 / 1000,
        );
        let x = match d.x_mode {
            Placement::Edge => d.x,
            Placement::Center => d.x - sw / 2,
            Placement::FarEdge => d.x - sw,
        };
        let y = match d.y_mode {
            Placement::Edge => d.y,
            Placement::Center => d.y - sh / 2,
            Placement::FarEdge => d.y - sh,
        };
        // Bit 15 of `+0x10`, and nothing else: `016a:0fe8` sends a sprite to
        // the keyed blit and a block to the opaque one, out of the same pool.
        let opaque = self.opaque_blocks && matches!(d.shows, Shows::Picture(_));
        if let Some(screen) = self.display.screen_mut(d.screen) {
            if opaque && h == 1000 && v == 1000 {
                screen.buffer.blit_masked(&sprite, x, y, None);
            } else {
                screen.buffer.blit_scaled(&sprite, x, y, h, v);
            }
        }
    }

    /// Draws a text descriptor.
    ///
    /// The game gives a center point rather than a corner — `SDCEN` across and
    /// `SDVCEN` down — so the block is measured first and placed around it.
    /// Lines break on newlines that are already in the resource; word wrapping
    /// (`SDWORD`) is a separate thing and not done here.
    pub(crate) fn draw_text_descriptor(&mut self, d: &Descriptor) {
        let Some(text) = self.descriptor_text(d) else {
            return;
        };
        // `+FONT` only registers a font and hands back a handle; choosing one
        // for a descriptor is `SDFNT`. Where nothing chose, the engine falls
        // back to the system font, not to whatever was registered first —
        // measured: with fonts 5, 6 and 8 each registered in turn and no
        // `SDFNT`, the original reports the same width every time, and it is
        // the one `000.FNT` gives.
        //
        // The intro does choose: `XYLTITEM.` ends with `_F1 @ SDFNT`, and `_F1`
        // holds font 5.
        let font = match d
            .font
            .and_then(|f| self.fonts.get(&f))
            .or(self.system_font.as_ref())
        {
            Some(f) => f.clone(),
            None => return,
        };
        let Some(refs) = self.font_refs.clone() else {
            return;
        };
        // The value `SDCOL` carries is composite, and clamping it threw both
        // halves away. The drawer tests it at 0x69f50 with `cmpl $0x100` and,
        // for anything from 256 up, runs a backing pass before the glyphs; the
        // low part is the palette index. `SETT1` passes 18 + 256, the speaker
        // table 165 + 256.
        let raw = d.color;
        let color = (raw & 0xff) as u8;
        // An empty text gets no backing. The color asks for one, but the
        // drawer overrules it: having set the flag at 0x69f5d it runs the
        // layout, compares the layout's +0x18 against 1 (`cmpw $1` at 0x69f75)
        // and clears the flag again when it comes up short, so the backing
        // block is jumped clean over at 0x69f9e.
        //
        // That field is the string's length. The layout writes it last, from
        // its own text pointer at +0xC through 0x11d0e (0x6cdd2-0x6cde2), and
        // 0x11d0e is `strlen`: a null pointer answers 0 (0x11d26), anything
        // else is counted to the terminator (0x11d32).
        //
        // Without this an empty entry still painted its rectangle — four wide
        // and one line tall once `SDTDT` has added its 4, plus the 12 of
        // `backing_rect`, so a 16 x 34 box of darkened background standing in
        // the picture with nothing in it. Only the backing is held back; the
        // glyph passes below run either way and draw nothing, which is exactly
        // what the original does with the same jumps.
        let backing = raw >= 0x100 && !text.is_empty();

        // The template names a second font and its own glyph gap. `DEFTDT`
        // pops its nine values as id, font, then the fields at +0xc, +0xe, +4,
        // +6, +8, +0xa and +0x10 — so in push order the font is args[7] and the
        // gaps are args[6] and args[5]. Module 3 gives every template the same
        // set: `8 2 2 -1 -1 -1 -1 _F2@ n`.
        let template = d
            .template
            .and_then(|t| self.templates.iter().find(|x| x.id == t));
        let outline = template
            .and_then(|t| t.args.get(7).copied())
            .and_then(|f| self.fonts.get(&f))
            .cloned();
        let outline_gap = template
            .and_then(|t| t.args.get(6).copied())
            .unwrap_or(crate::text::SPACING);
        // The outline draws in its own color, out of the template's field
        // +0x10 masked to a byte — `mov 0x10(%eax),%ax; xor %ah,%ah` at
        // 0x6a128. The text takes the low byte of `SDCOL` instead
        // (`and $0xff` at 0x6a219). Handing both passes the same color is
        // what made the outline invisible: it was there, in the color of the
        // letters it was supposed to sit behind.
        let outline_color = template.and_then(|t| t.args.first().copied()).unwrap_or(0) as u8;
        let shadow_dx = template.and_then(|t| t.args.get(4).copied()).unwrap_or(0);
        let shadow_dy = template.and_then(|t| t.args.get(3).copied()).unwrap_or(0);
        let justify = self.text_runs && d.fields.get("SDBLK").copied().unwrap_or(0) != 0;
        let runs = self.text_runs;

        let lines: Vec<&str> = text.split('\n').collect();
        // The backing goes down first, under the whole block, on the rectangle
        // `backing_rect` derives — which is a good deal larger than the text.
        let (bx, by, bw, bh) = self.backing_rect(d);
        let row = backing.then(|| self.backing_row());
        let Some(screen) = self.display.screen_mut(d.screen) else {
            return;
        };
        if let Some(row) = &row {
            crate::text::darken_rect(&mut screen.buffer, row, bx, by, bw, bh);
        }
        // The text goes down twice, and that is where the outline comes from.
        //
        // The layout at 0x6c9f8 fills two font slots in its result: `out[0]`
        // from the descriptor's own font, `out[4]` from the *template's*, which
        // module 3 sets to `_F2` for all nine templates. The drawer then runs
        // 0x25c02 twice at the same position — first with `out[4]` and the
        // template's glyph gap, then with `out[0]` and a gap of 1. So the
        // outline is a second typeface drawn underneath, not an offset copy and
        // not a second color; the negative gap keeps the wider outline glyphs
        // lined up with the ones on top.
        //
        // The 16-bit drawer differs on an axis that is **not** centered:
        // there it starts the shadow pass at the anchor **plus the
        // template's x/y offsets** — `016a:0e04` and `016a:0e2b` add the
        // bytes at template +2/+3, `-1 -1` in every shipped template of both
        // games — where a centered axis re-centers with the pass's own
        // metrics and gets the same one-pixel shift out of the wider glyphs.
        // (The 32-bit drawer adds no such offset: 0x6a136 and 0x6a228
        // compute the same position for both passes.) Without it, an
        // edge-placed text wore its silhouette a pixel low and right —
        // doubled there, bare at the top left.
        // `SDBLK` justifies (16-bit, `14ee:11cf` with mode bit 2): every
        // line starts at the block's left edge — the widest line's — and
        // the deficit is spread one pixel at a time over the line's inner
        // spaces (`14ee:111c`), round robin from the left. A line that
        // opens with `#` stays ragged: the newspaper marks its headings
        // and paragraph ends with it.
        let passes = outline
            .iter()
            .map(|f| (f, outline_gap, outline_color, true))
            .chain([(&font, crate::text::SPACING, color, false)]);
        for (pass_font, gap, pass_color, is_shadow) in passes {
            // Each pass is placed with *its own* font, not with the text's.
            // The output routine at 0x257cf centers what it draws — it measures
            // and subtracts half, at 0x25845 and again at 0x25878 — so two
            // fonts of different height land concentric on the same point.
            // Font 5 carries the letters at 18 tall and font 6 the outline at
            // 20, which is exactly one pixel over and one under. Placing both
            // from a top computed once, out of the text font, dropped the
            // outline a pixel: doubled below, missing above. (The 16-bit run
            // drawer does the same per-pass centering, per line, with the
            // gaps the drawer set for the pass — `14ee:1231`, `14ee:1262`.)
            let height = line_height(pass_font, gap);
            // The gap belongs in the centering height, and this is why.
            //
            // At 0x2588e the output routine computes `font[+2] × lines`
            // with no gap (0x25897, 0x258a1, 0x258a6), and centering on that
            // looked like the faithful reading. It is not what the engine
            // does: `the_intro_shows_its_two_texts_in_order` measures a line at
            // 166 against a real run, and the gapless height puts it at 167.
            //
            // So either that branch is not the one this path takes, or the
            // height there serves something other than the centering. Measured
            // beats read — the same way the oracle test threw out a -1 truth
            // flag that had looked just as convincing. (The 16-bit measure is
            // the same sum, read this time: `lines × height + (lines − 1) ×
            // gap` at `14ee:1711`–`14ee:172b`.)
            let block = (lines.len() as i32 * height - gap).max(0);
            let off = |v: i32| if runs && is_shadow { v } else { 0 };
            let top = match d.y_mode {
                Placement::Edge => d.y + off(shadow_dy),
                Placement::Center => d.y - block / 2,
                Placement::FarEdge => d.y - block + off(shadow_dy),
            };
            let block_width = justify.then(|| {
                lines
                    .iter()
                    .map(|l| crate::text::text_width_spaced(pass_font, &refs, l, gap))
                    .max()
                    .unwrap_or(0)
            });
            for (i, line) in lines.iter().enumerate() {
                let y = top + i as i32 * height;
                let width = crate::text::text_width_spaced(pass_font, &refs, line, gap);
                let x = match d.x_mode {
                    Placement::Edge => d.x + off(shadow_dx),
                    Placement::Center => match block_width {
                        Some(bw) => d.x - bw / 2,
                        None => d.x - width / 2,
                    },
                    Placement::FarEdge => d.x - width + off(shadow_dx),
                };
                if runs {
                    let pads = match block_width {
                        Some(bw) => justify_pads(line, bw - width),
                        None => Vec::new(),
                    };
                    crate::text16::draw_line(
                        &mut screen.buffer,
                        pass_font,
                        &refs,
                        line,
                        x,
                        y,
                        pass_color,
                        gap,
                        &pads,
                    );
                } else {
                    crate::text::draw_text_spaced(
                        &mut screen.buffer,
                        pass_font,
                        &refs,
                        line,
                        x,
                        y,
                        pass_color,
                        gap,
                    );
                }
            }
        }
    }
}

/// The justification's per-space widenings for one line (`14ee:111c`): the
/// deficit against the block width, spread one pixel at a time over the
/// line's inner spaces, round robin from the left. Leading spaces and a
/// space at the line's end carry nothing, and a line that opens with `#`
/// stays ragged. Empty when there is nothing to spread.
fn justify_pads(line: &str, deficit: i32) -> Vec<i32> {
    if deficit <= 0 || line.starts_with('#') {
        return Vec::new();
    }
    let chars: Vec<char> = line.chars().collect();
    let lead = chars.iter().take_while(|&&c| c == ' ').count();
    let count = chars
        .iter()
        .enumerate()
        .skip(lead)
        .filter(|&(i, &c)| c == ' ' && i + 1 < chars.len())
        .count();
    if count == 0 {
        return Vec::new();
    }
    let mut pads = vec![0i32; count];
    for n in 0..deficit as usize {
        pads[n % count] += 1;
    }
    pads
}
#[cfg(test)]
mod tests {
    use super::justify_pads;

    /// The distribution the calculator at `14ee:111c` makes: one pixel at a
    /// time over the inner spaces, round robin from the left.
    #[test]
    fn the_deficit_lands_round_robin_on_the_inner_spaces() {
        assert_eq!(justify_pads("ein zwei drei", 5), vec![3, 2]);
        assert_eq!(justify_pads("ein zwei drei", 2), vec![1, 1]);
        assert_eq!(justify_pads("ein zwei", 3), vec![3]);
    }

    /// Leading spaces and a space at the line's end carry nothing; a line
    /// that opens with `#` stays ragged; nothing to spread means no pads.
    #[test]
    fn what_stays_ragged_stays_ragged() {
        assert_eq!(justify_pads("  ein zwei ", 4), vec![4]);
        assert_eq!(justify_pads("# ein zwei", 7), Vec::<i32>::new());
        assert_eq!(justify_pads("einzeln", 7), Vec::<i32>::new());
        assert_eq!(justify_pads("ein zwei", 0), Vec::<i32>::new());
    }
}
