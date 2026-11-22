//! What the pointer and a click do: the strip, the verbs, the hotspots.
//!
//! The mode word `[288Fh]` and the two roads a click takes — a widget
//! above `100h`, a room object at or below it — are read here. The
//! pointer's own effect, once a frame, is [`Adventure::hover_frame`]'s.

use super::{Adventure, PLAYFIELD_H, Panel, mode};
use crate::num;

impl Adventure {
    /// Where the pointer is.
    ///
    /// With the pointer library installed the position is held inside its
    /// limits, `[30Ah]`–`[310h]` (`0x185B2`, `0x185F1`), as the library's
    /// own copy is that the poll hands the program (`0x17AD1`).
    pub(crate) fn mouse_move(&mut self, x: i32, y: i32) {
        self.mouse = match &self.cursor {
            Some(c) => c.clamp(x, y),
            None => (x, y),
        };
        self.menu_hover();
    }

    /// The hotspot under the pointer, if the pointer is on the playfield.
    ///
    /// Front to back, the way a list of rectangles is searched when the
    /// later ones are drawn over the earlier: the last one that contains
    /// the point wins. A hotspot the table marks inactive is not there.
    #[cfg(test)]
    pub(crate) fn hotspot_under_pointer(&self) -> Option<&motionvm_parsec_formats::Hotspot> {
        let i = self.hotspot_index_under_pointer()?;
        self.room()?.hotspots.spots.get(i)
    }

    /// The same, as its index into the room's list.
    pub(crate) fn hotspot_index_under_pointer(&self) -> Option<usize> {
        let (x, y) = self.mouse;
        if y >= PLAYFIELD_H {
            return None;
        }
        // The rectangles and the active bits as they stand now: an
        // object's live bit sets the bit, and an animation may move the
        // rectangle.
        self.hotspots_rt
            .iter()
            .enumerate()
            .rev()
            .find(|(_, h)| h.active && x >= h.x && y >= h.y && x < h.x + h.w && y < h.y + h.h)
            .map(|(i, _)| i)
    }

    /// The status line as the composer `0x10051` sets it, in CP437 with
    /// its ink controls.
    ///
    /// The mode `[288Fh]` picks the ink controls through a nine-arm table at
    /// `cs:162Ch` (`0x10059`, `0x10064`): in mode 0 with nothing chosen,
    /// `[2899h]` zero, there is no line (`0x10069`); mode 0 with something,
    /// mode 1 and anything past 8 take `9 9 9` (`0x10074`), mode 2
    /// `0Dh 9 9` (`0x1007C`), mode 3 `0Dh 0Dh 9` (`0x10081`) and modes 4 to
    /// 8 `0Dh 0Dh 0Dh` (`0x1008C`). Without a verb, `[288Dh]` zero, there is
    /// no line either (`0x1009C`). The first name is `[28ABh]` (`0x100A6`),
    /// a null one standing in as `DS:053Fh`, "\n ??? " (`0x100CB`); with
    /// `[2899h] = 5` the two names trade places and the second and third
    /// controls with them (`0x100E0`). The line is then `sprintf` of the
    /// template (`1FF2h:0040h`, `0x10138`) with the controls and names in
    /// the order first control, second, first name, first control, third,
    /// second name, first control.
    ///
    /// Every template takes its arguments in that order — `%c%c%s%c%c%s%c`
    /// as far as it goes, `%cBetrachte %c%s` taking the first three. The
    /// names are the chosen objects', stored when they were chosen: an
    /// item's from the row at `DS:1000h` (`0x0F5B6`), a hotspot's from its
    /// record (`0x0F5DC`, `0x0F6A0` and `0x0F77C` store them); the hover's
    /// tail clears them with the objects (`0x0FF49`, `0x0FF73`).
    pub(crate) fn status_bytes(&self) -> Vec<u8> {
        let (bx, si, cx) = match self.mode {
            0 if self.two_object == 0 => return Vec::new(),
            2 => (0x0D, 9, 9),
            3 => (0x0D, 0x0D, 9),
            4..=8 => (0x0D, 0x0D, 0x0D),
            _ => (9, 9, 9),
        };
        if self.verb == 0 {
            return Vec::new();
        }
        let Some(t) = self.templates.get(usize::from(self.verb)) else {
            return Vec::new();
        };
        let none = b"\n ??? ".to_vec();
        let mut name1 = self
            .chosen_name(self.object1, self.hover_spot)
            .unwrap_or_else(|| none.clone());
        let mut name2 = self
            .chosen_name(self.second_object.0, self.second_spot)
            .unwrap_or(none);
        let (mut si, mut cx) = (si, cx);
        if self.two_object == 5 {
            std::mem::swap(&mut name1, &mut name2);
            std::mem::swap(&mut si, &mut cx);
        }
        enum Arg {
            C(u8),
            S(Vec<u8>),
        }
        let args = [
            Arg::C(bx),
            Arg::C(si),
            Arg::S(name1),
            Arg::C(bx),
            Arg::C(cx),
            Arg::S(name2),
            Arg::C(bx),
        ];
        let mut args = args.iter();
        let template = motionvm_parsec_formats::to_cp437(t);
        let mut out = Vec::new();
        let mut i = 0;
        while i < template.len() {
            let b = template[i];
            if b != b'%' || i + 1 >= template.len() {
                out.push(b);
                i += 1;
                continue;
            }
            match (template[i + 1], args.next()) {
                (b'c', Some(Arg::C(c))) => out.push(*c),
                (b's', Some(Arg::S(s))) => out.extend_from_slice(s),
                (b'%', _) => out.push(b'%'),
                (other, _) => {
                    out.push(b'%');
                    out.push(other);
                }
            }
            i += 2;
        }
        out
    }

    /// The name stored for a chosen object: an item's out of the item
    /// row, a hotspot's out of its record; `None` when nothing is chosen.
    fn chosen_name(&self, id: u16, spot: Option<usize>) -> Option<Vec<u8>> {
        if id == 0 {
            return None;
        }
        if id < 0x17 {
            return self
                .item_name(num::low8(id))
                .map(motionvm_parsec_formats::to_cp437);
        }
        let spot = spot?;
        self.room()?
            .hotspots
            .spots
            .get(spot)
            .map(|h| motionvm_parsec_formats::to_cp437(&h.name))
    }

    /// The status line's visible text, its ink controls left out — what a
    /// reader of the band sees, for a test or a diagnostic.
    #[cfg(test)]
    pub(crate) fn status(&self) -> String {
        let bytes: Vec<u8> = self
            .status_bytes()
            .into_iter()
            .filter(|&b| b >= 0x20)
            .collect();
        motionvm_parsec_formats::cp437_to_string(&bytes)
    }

    /// What a click on a verb button does, which is not simply "pick it".
    ///
    /// The strip's side of the mode machine is a nine-arm switch through
    /// the table at `CS:148Bh`, and it has five arms. In modes 0, 1 and 3
    /// the verb is taken, `[288Fh] = 2`, and what was chosen before is
    /// forgotten, `[2899h]` and `[2891h]` cleared (`0x0FC12`–`0x0FC26`). In
    /// mode 2 the same verb again (`0x0FC67`) parks it, `[288Fh] = 1`
    /// (`0x0FC6D`). Modes 4, 5 and 6 ask the gate with 1 (`0x0FC88`), mode 8
    /// with 2 (`0x0FC97`), and both end in the shared tail that sets
    /// `[288Fh] = 1` (`0x0FCA4`).
    ///
    /// Mode **7** has an arm that does nothing at all (`0x0FFEF`).
    ///
    /// Parking is not un-picking: the verb stays in `[288Dh]` and the
    /// status line still names it. What changes is that the object road
    /// answers for modes 0, 2 and 3 only, so in mode 1 a click on a
    /// hotspot does nothing.
    pub(super) fn pick_verb(&mut self, v: u8) {
        match self.mode {
            // `0x0FC88`: the gate is asked, and only when it agrees does
            // the tail run — mode 1 with the new verb.
            4..=6 | 8 => {
                let arg = if self.mode == 8 { 2 } else { 1 };
                if self.ask_gate(arg) == 0 {
                    return;
                }
                self.mode = 1;
                self.verb = v;
                self.verb_chosen = u16::from(v);
            }
            7 => {}
            2 => {
                self.two_object = 0;
                self.object1 = 0;
                self.object2 = 0;
                if self.verb == v {
                    self.mode = 1;
                } else {
                    self.verb = v;
                    self.verb_chosen = u16::from(v);
                    // `0x0FC7E`: the sixth verb takes the same road as from
                    // the other modes (`0x0FC86`, to `0x0FC3A`).
                    if v == 6 {
                        self.two_object = 3;
                        self.mode = 5;
                        self.walk_target = 0;
                        self.status_refresh_age = 0x0A;
                    }
                }
            }
            // 0, 1, 3 and anything past the bound.
            _ => {
                self.mode = 2;
                self.verb = v;
                self.verb_chosen = u16::from(v);
                self.two_object = 0;
                self.object1 = 0;
                self.object2 = 0;
                // Verb 6 is not like the others (`0x0FC3A`).
                if v == 6 {
                    self.two_object = 3;
                    self.mode = 5;
                    self.walk_target = 0;
                    self.status_refresh_age = 0x0A;
                }
            }
        }
    }

    /// A click. `true` when it started something.
    ///
    /// The dispatcher's order (`0EB9h:0FCEh`): a script that holds the
    /// input takes the click (`0x0FB9E`); a text that is up takes it and
    /// shortens itself (`0CFDh:0FD9h`); a menu that is up takes it as a
    /// choice; on the strip a verb button is a verb and a filled slot an
    /// item chosen (`0x0FCB5`); in the room the hotspot under the pointer
    /// is the object clicked (`0x0FDD4`), and the click goes down the
    /// object road — chosen, walked to, acted on.
    ///
    /// A click that gets past the text and the menu forgets what the
    /// pointer was over, whichever road it then takes: the strip's at
    /// `0x0FBE0` and the room's at `0x0FD94` — the arm the event table
    /// `cs:1479h` sends button events 4 to 7 to — both write `FFFFh` to
    /// `[28A5h]` and `[28A3h]` before anything else. So the dispatcher's
    /// tail (`0x0FDE1`) finds no hotspot the same as last time and lays
    /// the hover text again at the pointer.
    pub(crate) fn click(&mut self) -> bool {
        if self.bits & mode::LOCKED != 0 {
            return false;
        }
        if self.tasks.t[0].kind == 5 {
            self.menu_click();
            return true;
        }
        if matches!(self.tasks.t[0].kind, 3 | 4) {
            self.click_on_text();
            return true;
        }
        self.last_hover = mode::Hover::None;
        let (x, y) = self.mouse;
        if y >= self.panel_y {
            let Some(w) = self.panel_widgets.hit(x, y) else {
                return false;
            };
            // `0x0FBEC`: a widget the strip took asks for the status line
            // again before the verb or the item is acted on.
            if let Some(v) = Panel::verb_of(w) {
                self.status_refresh_age = 1;
                self.pick_verb(v);
                return true;
            }
            if let Some(id) = Panel::slot_of(w).and_then(|n| self.item_in_slot(n)) {
                self.status_refresh_age = 1;
                self.click_object(i16::from(id), None);
                return true;
            }
            return false;
        }
        let Some(i) = self.hotspot_index_under_pointer() else {
            return false;
        };
        let id = self
            .room()
            .and_then(|r| r.hotspots.spots.get(i))
            .map_or(0, |h| i16::from(num::low8(h.verb_mask)));
        self.click_object(id, Some(i));
        true
    }

    /// `0x0DFF7`: a click while task 0 shows a text.
    ///
    /// The word `[1DB8h]` decides (`0x0DFF7`): masked with `8400h` and equal
    /// to `400h` — a `03 05` page, not armed — the page is over at once and
    /// the flag `[1D9Eh]` spent (`0x0DFFD`, `0x0E002`); with any of `8300h`
    /// set — armed, held or `03 02` — the click does nothing (`0x0E010`);
    /// otherwise at most 40 ticks of the page remain (`0x0E028`).
    ///
    /// The developer's bit `[1EDCh]` bit 4 ends the page at once instead;
    /// it is not set in a shipped game.
    /// The same for a key (`0x0DFDD`): the developer's `403h` aside, which
    /// needs `[1EDCh]` bit 1, a key is a click.
    pub(super) fn click_on_text_key(&mut self) {
        self.click_on_text();
    }

    fn click_on_text(&mut self) {
        let t = &mut self.tasks.t[0];
        if t.flags & 0x8400 == 0x400 {
            t.countdown = 0;
            t.flags &= !0x400;
        } else if t.flags & 0x8300 == 0 && t.countdown > 0x28 {
            t.countdown = 0x28;
        }
    }
}

#[cfg(test)]
mod tests {
    /// No status line the game can compose comes near the setter's bound
    /// of `140h` (`0x0CF9C`): every template with each `%s` filled by the
    /// widest name an item or a hotspot of any room carries, measured with
    /// the shipped font, is at most 220 pixels wide.
    #[test]
    fn no_status_line_reaches_the_setters_bound() {
        let Some(a) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        let font = &a.font;
        let names: Vec<Vec<u8>> = (1..0x17u8)
            .filter_map(|id| a.item_name(id))
            .map(motionvm_parsec_formats::to_cp437)
            .chain(
                a.rooms
                    .iter()
                    .flatten()
                    .flat_map(|r| &r.hotspots.spots)
                    .map(|h| motionvm_parsec_formats::to_cp437(&h.name)),
            )
            .chain([b"\n ??? ".to_vec()])
            .collect();
        let widest = names.iter().max_by_key(|n| font.measure(n)).expect("names");
        let worst = a
            .templates
            .iter()
            .map(|t| {
                let mut line = Vec::new();
                for (i, part) in motionvm_parsec_formats::to_cp437(t)
                    .split(|&b| b == b'%')
                    .enumerate()
                {
                    // A part after a `%` starts with its conversion: `%s`
                    // is a name, `%c` an ink control the walk steps over.
                    let rest = if i == 0 {
                        part
                    } else {
                        if part.first() == Some(&b's') {
                            line.extend_from_slice(widest);
                        }
                        part.get(1..).unwrap_or_default()
                    };
                    line.extend_from_slice(rest);
                }
                font.measure(&line)
            })
            .max()
            .expect("templates");
        assert_eq!(
            (
                motionvm_parsec_formats::cp437_to_string(widest),
                font.measure(widest)
            ),
            ("Krankenschwester".to_string(), 82)
        );
        assert_eq!(worst, 220);
        assert!(worst <= 0x140);
    }
}
