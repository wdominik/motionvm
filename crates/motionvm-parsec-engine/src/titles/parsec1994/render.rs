//! Composing the 320×200 picture: layers, objects, the figure, the strip,
//! the status band, a caption, the pointer.

use super::{
    Adventure, Body, Caption, Framebuffer, Picture, RoomFile, STATUS_INK, STATUS_INKS, STATUS_Y,
    Walker, objects, scaled,
};
use crate::num;

impl Adventure {
    /// Where slot `n` is drawn.
    ///
    /// `0x0C566` divides the slot number by four: the remainder is the
    /// column, at twenty-seven pixels apart from `0C8h`, and the quotient
    /// is the row, `92h` or `0ADh`.
    pub(crate) fn slot_origin(n: usize) -> (i32, i32) {
        (
            num::count(n % 4) * 0x1B + 0xC8,
            if n < 4 { 0x92 } else { 0xAD },
        )
    }

    /// The eight slots, each with whatever it holds.
    pub(super) fn draw_items(&self, fb: &mut Framebuffer) {
        for n in 0..8 {
            let Some(id) = self.item_in_slot(n) else {
                continue;
            };
            let Some(cel) = self.item_cels.get(usize::from(id).wrapping_sub(1)) else {
                continue;
            };
            let (x, y) = Self::slot_origin(n);
            crate::draw::blit_masked(fb, cel, x, y, Some(0));
        }
    }

    /// The screen: while an effect plays, the picture it draws on;
    /// otherwise the current picture with the pointer over it.
    pub(crate) fn render(&self) -> Framebuffer {
        if let Some(screen) = &self.effects.screen {
            return screen.clone();
        }
        if let Some(fb) = self.options_render() {
            return fb;
        }
        self.render_live()
    }

    /// The current picture with the pointer, as the frame pass presents it
    /// (`10B8:0009`).
    pub(crate) fn render_live(&self) -> Framebuffer {
        let mut fb = self.render_scene();
        self.draw_pointer(&mut fb);
        fb
    }

    /// The pointer over a finished picture.
    pub(super) fn draw_pointer(&self, fb: &mut Framebuffer) {
        match &self.cursor {
            // `0x17D18`: the shown cel at `[3EEh]`/`[3F0h]`, index 0
            // skipped (`0x17D2F`), clipped to the screen (`0x17C96`).
            Some(c) => {
                if let Some((n, x, y)) = c.sprite()
                    && let Some(cel) = self.pointer_cel(n)
                {
                    crate::draw::blit_masked(fb, cel, x, y, Some(0));
                }
            }
            None => {
                if let Some(p) = &self.pointer {
                    crate::draw::mouse(fb, p, self.mouse.0, self.mouse.1);
                }
            }
        }
    }

    /// The current picture: the location's playfield, the strip below it —
    /// the back buffer `[1CB0h]`, without the pointer.
    ///
    /// The playfield is the drawer `0BB6h:027Ah`'s (`0x0BDDA`): one pass
    /// over the room's layers, the 14h-byte records the header's `+0Ch`
    /// counts (`0x0BE21`, `0x0C076`), and in each layer three things in
    /// this order — its tiles (`0x0BE2B`–`0x0BFA0`), its objects, the
    /// 20h-byte records its `+0Ch` points at (`0x0BFA2`–`0x0C008`), and
    /// its chain, from the head at `+10h` along `+20h`
    /// (`0x0C00A`–`0x0C074`). Objects and chain records alike are drawn
    /// only when `(+2 & 3) == 1` (`0x0BFC2`, `0x0C020`).
    pub(crate) fn render_scene(&self) -> Framebuffer {
        let mut fb = crate::draw::display();
        if let Some(room) = self.room() {
            // Layer 0 upwards, the way the room stores them.
            for (i, layer) in room.room.layers.iter().enumerate() {
                if !layer.pixels.is_empty() {
                    crate::draw::raw_masked(
                        &mut fb,
                        &layer.pixels,
                        room.room.view.0,
                        room.room.view.1,
                        0,
                        0,
                    );
                }
                self.draw_objects(&mut fb, room, num::byte(i));
                // Then the layer's chain, from its head: the player and
                // the people hang in the chain of the layer their node or
                // edge named, so they are drawn after that layer and
                // before the next — which is what puts things in front of
                // them — and the record linked last is drawn first.
                let mut at = self.chain_head(i);
                while let Some(body) = at {
                    match body {
                        Body::Figure => self.draw_figure(&mut fb),
                        Body::Mover(m) => self.draw_mover(&mut fb, m),
                        Body::Object(_) => {}
                    }
                    at = self.chain_next(body);
                }
            }
        }
        crate::draw::blit_masked(&mut fb, &self.panel, 0, self.panel_y, Some(0));
        self.draw_items(&mut fb);
        // `0x0DB8B` freezes the panel; `0x0E6E4` draws the menu over a
        // fresh strip (`0x0C909`). Status composition/flattening must not
        // run over its first line at y=132.
        if let Some(notice) = &self.notice {
            // A message the original would end the program with, set in the
            // status band where the line goes.
            let text = motionvm_parsec_formats::to_cp437(notice);
            let width = i32::from(self.font.measure(&text));
            crate::draw::text::line(
                &mut fb,
                &self.font,
                &text,
                (320 - width) / 2,
                STATUS_Y,
                STATUS_INK,
            );
        } else if !self.menu.up {
            // The status band: twelve rows at 128, the line as it was last
            // set, in the inks its controls choose.
            self.draw_status(&mut fb);
            // `0C3Fh:005Ch`: while any part of the band is held dark, the
            // whole band is flattened after the line is set (`0x0C8A1`,
            // `0724h:0221h(0, 80h, 140h, 0Ch)`).
            if self.status_dim != 0 {
                Caption::flatten_rect(&mut fb, 0, STATUS_Y, 320, 12, self.flatten);
            }
        }
        self.paint_menu(&mut fb, &self.font);
        self.paint_tasks(&mut fb);
        fb
    }

    /// `0x0C730`: the held status line painted, centered as
    /// `0C3Fh:0B3Eh` placed it — measured by `1601h:1465h`, which steps
    /// over the ink controls as bytes it has no glyph for — and set glyph
    /// by glyph from ink `0F4h` (`0x0C750`), a control of the painter's
    /// table switching the ink and drawing nothing.
    fn draw_status(&self, fb: &mut Framebuffer) {
        let line = &self.status_line;
        if line.first().is_none_or(|&b| b == 0) {
            return;
        }
        let width = i32::from(self.font.measure(line));
        let mut pen = (320 - width) / 2;
        let mut ink = STATUS_INK;
        for &b in line {
            if b == 0 {
                break;
            }
            if let Some(&(_, to)) = STATUS_INKS.iter().find(|(c, _)| *c == b) {
                ink = to;
                continue;
            }
            let Some(g) = self.font.glyph(b) else {
                continue;
            };
            crate::draw::text::glyph(fb, &self.font, b, pen, STATUS_Y, ink);
            // 0x0C7B8–0x0C7CA: a nonzero width, plus the record's spacing.
            if g.advance != 0 {
                pen += i32::from(g.advance) + i32::from(motionvm_parsec_formats::font::SPACING);
            }
        }
    }

    /// The objects a layer's list holds.
    ///
    /// One blit per object the draw pass would draw, after that layer's
    /// tiles and before the next layer — the draw order the chain is for.
    /// The pass draws an object when `(+2 & 3) == 1` (`0x0BFC2`): live,
    /// and not hidden by its animation. Doors and people start clear in
    /// the shipped rooms, and a location's own code brings them up.
    ///
    /// The blitter `0BB6h:000Ah` takes the cel `+0Ah` names out of the
    /// bundle `+8` names (`0x0BB96`), scales it by `+0Eh` and `+10h` with
    /// each extent capped at `FFh` (`0x0BBFE`), mirrors it by bit 6 of
    /// `+2` (`0x0BB79`), and aligns the box by `+3`: 1 centers it on the
    /// point, 2 centers it and stands it on the point (`0x0BC4B`).
    pub(super) fn draw_objects(&self, fb: &mut Framebuffer, room: &RoomFile, layer: u8) {
        let _ = room;
        for o in self.objects.iter() {
            if o.layer != layer || !o.visible() {
                continue;
            }
            let Some(cel) = self.object_cel(o.bundle, o.cel) else {
                continue;
            };
            let mirror = o.flags & objects::MIRROR != 0;
            Self::blit_record(fb, cel, (o.x, o.y), o.scale, o.align, mirror);
        }
    }

    /// The blit of `0BB6h:000Ah` for one record, object or person: the
    /// cel scaled by the two factors with each extent capped at `FFh`,
    /// aligned by `+3`, mirrored by bit 6 of `+2`.
    pub(super) fn blit_record(
        fb: &mut Framebuffer,
        cel: &Picture,
        (x, y): (i32, i32),
        scale: (u16, u16),
        align: u8,
        mirror: bool,
    ) {
        let unit = |s: u16| s == 0 || s == 0x100;
        let w = if unit(scale.0) {
            i32::from(cel.width)
        } else {
            scaled(i32::from(cel.width), scale.0).min(0xFF)
        };
        let h = if unit(scale.1) {
            i32::from(cel.height)
        } else {
            scaled(i32::from(cel.height), scale.1).min(0xFF)
        };
        let (mut x, mut y) = (x, y);
        match align {
            1 => {
                x -= w >> 1;
                y -= h >> 1;
            }
            2 => {
                x -= w >> 1;
                y -= h;
            }
            _ => {}
        }
        if unit(scale.0) && unit(scale.1) && !mirror {
            crate::draw::blit_masked(fb, cel, x, y, Some(0));
        } else {
            crate::draw::scaled_xy(fb, cel, x, y, (num::small(w), num::small(h)), mirror);
        }
    }

    /// The player, scaled and standing on its feet.
    ///
    /// Align 2, which is what the walker record carries (`+3`):
    /// the cel is centered on `x` and its foot sits on `y`. The extent is
    /// the aligned one, so the foot lands on the walker's own row.
    pub(super) fn draw_figure(&self, fb: &mut Framebuffer) {
        let Some(p) = self.walker.as_ref() else {
            return;
        };
        let (cel, mirror) = figure_cel(p);
        let Some(cel) = self.figure_cels.get(cel) else {
            return;
        };
        let scale = num::small(p.scale);
        let w = i32::from(crate::draw::scale::aligned_extent(cel.width, scale));
        let h = i32::from(crate::draw::scale::aligned_extent(cel.height, scale));
        crate::draw::scaled(fb, cel, p.x - w / 2, p.y - h, scale, mirror);
    }
}

/// The figure's `+0Ah` and mirror bit: [`Walker::cel`], or, while an
/// animation runs, `+0Ch` plus the frame the entry set (`0x1526E`) —
/// `14DDh:0098h` rebased `+0Ch` to the file's first cel, and the mirror
/// bit is the stream's.
pub(crate) fn figure_cel(p: &Walker) -> (usize, bool) {
    if p.state == objects::STATE_FIGURE_ANIM && p.cel_held.is_none() {
        (
            usize::from(p.base.wrapping_add(u16::from(p.anim_frame))),
            p.mirror,
        )
    } else {
        p.cel()
    }
}
