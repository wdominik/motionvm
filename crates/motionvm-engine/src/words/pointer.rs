//! The mouse pointer, and the twenty-four-argument status line.
//!
//! One of the ten groups `plain_word` hands a word to, in the order the
//! original's own match had them. A group that does not know the word
//! answers `None` and the next one is asked.

use crate::Engine;
use crate::Placement;
use crate::Result;
use crate::stack::pop_n;
use crate::stack::pop1;
use motionvm_forth::Address;
use motionvm_forth::Memory;

impl Engine {
    pub(crate) fn words_pointer(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut Memory,
    ) -> Result<Option<()>> {
        match name {
            // --- mouse pointer ----------------------------------------------
            "HIDEMOUSE" => self.hide_pointer(),
            "SHOWMOUSE" | "NORMMOUSE" => self.pointer_visible = true,
            "SETMOUSEX" => self.mouse.x = pop1(stack, "SETMOUSEX")?,
            "SETMOUSEY" => self.mouse.y = pop1(stack, "SETMOUSEY")?,
            "SETMOUSELB" => self.mouse.left = pop1(stack, "SETMOUSELB")?,
            "SETMOUSERB" => self.mouse.right = pop1(stack, "SETMOUSERB")?,
            // Which of a location's hot areas the point is in, or -1.
            //
            // The table is `count` entries of `stride` bytes; the first four
            // cells of each are the corners, and the test takes them as
            // inclusive. An entry that is four zeroes is a hole, not a
            // rectangle at the origin, and is skipped even when the point
            // "matches" it.
            "?XINSIDE" => {
                let a = pop_n(stack, 5, "?XINSIDE")?;
                let hit = area_containing(mem, a[0], a[1], a[2] as u32, a[3], a[4]);
                stack.push(hit);
            }
            // The status line under the pointer, and the pointer's own shape.
            //
            // `SCANITEM` in module 5 calls it with twenty-four values; the
            // handler pops exactly that many, so unlike `ANIMPLAY` the count is
            // real. Three cases: over the scene, over the inventory bar, or
            // over neither. Each ends by setting the cursor through
            // `FXATMOUSE`, a module-5 word — the handler runs the interpreter
            // again for it, and since every one of those eight calls is the
            // last thing its branch does, it is requested as a tail call here.
            //
            // Returns the hot area the pointer is in, or -1.
            "MOUSEINFO" => {
                let a = pop_n(stack, 24, "MOUSEINFO")?;
                let (imx, mmx, mmy) = (a[0], a[2], a[3]);
                let (lditem, a_lditem, s_lditem, one) = (a[4], a[5], a[6], a[7]);
                let (fitem, actinv) = (a[8] as u32, a[9] as u32);
                let (is, screen, minfo, fitem2) = (a[11], a[13], a[14], a[15] as u32);
                let (flag, bmnr, fxatmouse) = (a[16], a[17], a[18] as u32);
                let (t390, t391, t392) = (a[19], a[20], a[21]);
                let mode = a[23];
                let _ = is;

                // The handler keeps the last mode in a global and forces the
                // "something changed" flag when it differs, so a mode switch
                // always redraws even if the pointer has not moved.
                let mut changed = a[22];
                if self.last_info_mode != Some(mode) {
                    self.last_info_mode = Some(mode);
                    changed = 1;
                }

                let cell = |base: u32, off: u32| {
                    Address::new(base >> 16, (base & 0xffff).wrapping_add(off))
                };
                let mut result = -1;
                // Modes 2, 4 and 5 are the menus: the info line belongs to them
                // and this word keeps its hands off.
                if mode == 2 || mode == 4 || mode == 5 {
                    self.select_screen((screen) as u32);
                    self.select_descriptor((minfo) as u32);
                    stack.push(result);
                    return Ok(Some(()));
                }

                let over_scene = mmx != -1 && matches!(mode, 0 | 3 | 8 | 9);
                let over_bar = imx != -1 && matches!(mode, 0 | 3 | 8 | 9 | 0xE);
                let mut cursor = None;

                if over_scene {
                    self.select_screen((screen) as u32);
                    self.select_descriptor((minfo) as u32);
                    result = area_containing(mem, mmx, mmy, lditem as u32, s_lditem, a_lditem);
                    let tb = self.descriptor_table();
                    let txt = self.descriptor_text_entry();

                    if result != -1 {
                        let areas = lditem as u32;
                        let label =
                            mem.fetch(cell(areas, (s_lditem * result) as u32 + 0x10))? as i32;
                        if !(tb == one && txt == label && changed == 0) {
                            self.set_text_table(one)?;
                            self.set_text(label)?;
                            self.place_x(mmx, Placement::Center)?;
                            self.place_y(mmy, Placement::Center)?;
                        }
                        if flag != 0 {
                            let area = |off: u32| cell(areas, (result as u32) * 64 + off);
                            let exit = mem.fetch(area(0x24))?;
                            let item = mem.fetch(area(0x28))?;
                            let usable =
                                item != 0 && mem.fetch(cell(fitem2, item * 20 + 4))? & 1 != 0;
                            // Nothing to do when the pointer is already plain
                            // and nothing has changed.
                            cursor = if exit != 0 {
                                Some(t392)
                            } else if usable {
                                Some(t391)
                            } else if bmnr != t390 || changed != 0 {
                                Some(t390)
                            } else {
                                None
                            };
                        }
                    } else {
                        if !(txt == 1 && tb == one) {
                            self.set_text_table(one)?;
                            self.set_text(1)?;
                        }
                        if flag != 0 && bmnr != t390 {
                            cursor = Some(t390);
                        }
                    }
                } else if over_bar {
                    self.select_screen((screen) as u32);
                    self.select_descriptor((minfo) as u32);
                    // The bar starts at x = 64 and gives each of its eight
                    // slots 64 pixels, which is exactly where `CCALCINV` puts
                    // them.
                    let slot = (imx - 0x40) / 64;
                    let offset = mem.fetch(cell(actinv, 0))? as i32;
                    let index = slot + offset;
                    let item = if (0..=7).contains(&slot) && imx >= 0x40 {
                        mem.fetch(cell(actinv, 4 + index as u32 * 4))? as i32
                    } else {
                        0
                    };
                    if item != 0 {
                        self.set_text_table(one)?;
                        let label = mem.fetch(cell(fitem, item as u32 * 20))? as i32;
                        self.set_text(label)?;
                        let x = self.screen_origin_x() + ((index - offset) << 6) + 0x60;
                        self.place_x(x, Placement::Center)?;
                        let y = self.screen_origin_y() + 0x18c;
                        self.place_y(y, Placement::FarEdge)?;
                        if flag != 0 && bmnr != t391 {
                            cursor = Some(t391);
                        }
                    } else {
                        let tb = self.descriptor_table();
                        let txt = self.descriptor_text_entry();
                        if !(tb == one && txt == 1) {
                            self.set_text_table(one)?;
                            self.set_text(1)?;
                        }
                        if flag != 0 && bmnr != t390 {
                            cursor = Some(t390);
                        }
                    }
                } else {
                    self.select_screen((screen) as u32);
                    self.select_descriptor((minfo) as u32);
                    let tb = self.descriptor_table();
                    let txt = self.descriptor_text_entry();
                    if !(tb == one && txt == 1) {
                        self.set_text_table(one)?;
                        self.set_text(1)?;
                    }
                    // Only worth resetting if the pointer is currently one of
                    // the two special shapes.
                    if (bmnr == t391 || bmnr == t392) && flag != 0 {
                        cursor = Some(t390);
                    }
                }

                if let Some(nr) = cursor {
                    let target = Address::new(fxatmouse >> 16, fxatmouse & 0xffff);
                    self.pending_call = Some((target, vec![nr, 0, 0]));
                }
                stack.push(result);
            }
            "MOUSEX" => stack.push(self.mouse.x),
            "MOUSEY" => stack.push(self.mouse.y),
            "MOUSELK" => stack.push(self.mouse.left),
            "MOUSERK" => stack.push(self.mouse.right),
            // y first, then x, so x is what ends up on top — that is the order
            // the handler pushes the two record fields in, and it was the other
            // way round here until the handler was read.
            "MOUSEXY" => stack.extend([self.mouse.y, self.mouse.x]),

            // --- input, timers, sound ---------------------------------------
            // Gives the pointer a shape. The handler looks the sprite up, hides
            // the pointer and installs the new one at the given hotspot — state,
            // not nothing, which is why it is not on the inert list.
            "XATMOUSE" => {
                let a = pop_n(stack, 3, "XATMOUSE")?;
                self.set_pointer_sprite(a[2], a[0], a[1]);
            }
            // The handler pops one value and pushes 0, 0 and it back before
            // calling `XATMOUSE`: a cursor sprite with the hotspot at its
            // corner. The two zeros are constants, not the pointer's own
            // position — which is the plausible reading and the wrong way
            // round, since the hotspot is an offset *into* the sprite.
            "ATMOUSE" => {
                let sprite = pop1(stack, "ATMOUSE")?;
                self.set_pointer_sprite(sprite, 0, 0);
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}

impl Engine {
    /// `HIDEMOUSE`: takes the pointer off the screen.
    ///
    /// `SHOWMOUSE` and `NORMMOUSE` put it back. Only visibility: the shape and
    /// the hotspot stay as [`Engine::set_pointer_sprite`] left them.
    pub(crate) fn hide_pointer(&mut self) {
        self.pointer_visible = false;
    }

    /// `XATMOUSE`: gives the pointer a shape.
    ///
    /// The handler looks the sprite up, hides the pointer and installs the new
    /// one at the given hotspot — state, not nothing, which is why it is not on
    /// the inert list. A negative sprite is floored at zero, as the handler's
    /// lookup does.
    pub(crate) fn set_pointer_sprite(&mut self, sprite: i32, hot_x: i32, hot_y: i32) {
        self.cursor = Some((sprite.max(0) as u32, hot_x, hot_y));
    }
}

/// `?XINSIDE`: which of a location's hot areas a point is in, or -1.
///
/// The table is `count` entries of `stride` bytes; the first four cells of each
/// are the corners, and the test takes them as inclusive. An entry that is four
/// zeroes is a hole, not a rectangle at the origin, and is skipped even when the
/// point "matches" it.
///
/// Takes module memory rather than the engine because the table is the game's,
/// not ours. A read that runs off the end answers zero rather than failing,
/// which is what the handler's own bounds-free walk does.
pub(crate) fn area_containing(
    mem: &Memory,
    x: i32,
    y: i32,
    table: u32,
    stride: i32,
    count: i32,
) -> i32 {
    let at = |i: i32, c: u32| {
        Address::new(
            table >> 16,
            (table & 0xffff).wrapping_add((i * stride) as u32 + c * 4),
        )
    };
    for i in 0..count.max(0) {
        let c: Vec<i32> = (0..4)
            .map(|k| mem.fetch(at(i, k)).unwrap_or(0) as i32)
            .collect();
        if x >= c[0] && x <= c[2] && y >= c[1] && y <= c[3] && c != [0, 0, 0, 0] {
            return i;
        }
    }
    -1
}
