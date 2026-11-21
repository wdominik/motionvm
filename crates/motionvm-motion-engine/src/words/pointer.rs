//! The mouse pointer, and the twenty-four-argument status line.
//!
//! One of the groups `plain_word32` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::Placement;
use crate::cursor::NORMAL_COLORS;
use crate::stack::pop_n;
use crate::stack::pop1;
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;
use motionvm_motion_forth::cell;

/// What differs between the two engines' `MOUSEINFO`: the inventory bar's
/// geometry, read from each handler.
#[derive(Clone, Copy)]
pub(crate) struct Rules {
    /// Where the bar's first slot starts, and how wide a slot is; the bar
    /// holds eight.
    pub bar_x0: i32,
    pub slot_w: i32,
    /// Where an item's name stands over the bar: this far below the screen's
    /// origin, placed this way.
    pub label_y: i32,
    pub label_placement: Placement,
}

/// MOTION 32-bit, from `ENGINE.EXE`: slots of 64 from x 0x40, the name's
/// far edge 0x18c below the origin.
pub(crate) const M32_RULES: Rules = Rules {
    bar_x0: 0x40,
    slot_w: 64,
    label_y: 0x18c,
    label_placement: Placement::FarEdge,
};

/// MOTION 16-bit, from `ENVIRO.EXE` (`MOUSEINFO` at file `0x10015`): slots
/// of 32 from x 0x20 (`0a40:2d54`), the name centered 158 below the origin
/// (`0a40:2dda`–`0a40:2e1b`, `SDCEN`/`SDVCEN`).
pub(crate) const M16_RULES: Rules = Rules {
    bar_x0: 0x20,
    slot_w: 32,
    label_y: 158,
    label_placement: Placement::Center,
};

impl Engine {
    pub(crate) fn words_pointer(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        mem: &dyn AddressSpace,
        rules: Rules,
    ) -> Result<Option<()>> {
        match word {
            // --- mouse pointer ----------------------------------------------
            // Both pairs keep the show counter and are inert until a shape
            // armed the layer (R78 `0x20340`/`0x20540`, `14ee:0874`/
            // `14ee:094b`); see [`crate::cursor`].
            Word::HIDEMOUSE => self.hide_pointer(),
            Word::SHOWMOUSE => self.show_pointer(),
            // The engine's own arrow, in white and a dark teal resolved
            // against the palette in force (R78 `0x5ed50`, R109 `0x735ac`).
            // The count is untouched: a hidden pointer stays hidden, wearing
            // the arrow. Only the 32-bit kernels have the word.
            Word::NORMMOUSE => self.arm_arrow(Some(NORMAL_COLORS)),
            Word::SETMOUSEX => self.input.mouse.x = pop1(stack, "SETMOUSEX")?,
            Word::SETMOUSEY => self.input.mouse.y = pop1(stack, "SETMOUSEY")?,
            Word::SETMOUSELB => self.input.mouse.left = pop1(stack, "SETMOUSELB")?,
            Word::SETMOUSERB => self.input.mouse.right = pop1(stack, "SETMOUSERB")?,
            // Which of a location's hot areas the point is in, or -1.
            //
            // The table is `count` entries of `stride` bytes; the first four
            // cells of each are the corners, and the test takes them as
            // inclusive. An entry that is four zeroes is a hole, not a
            // rectangle at the origin, and is skipped even when the point
            // "matches" it.
            Word::Q_XINSIDE => {
                let a = pop_n(stack, 5, "?XINSIDE")?;
                let hit =
                    area_containing(mem, a[0], a[1], a[2], a[3], a[4], self.profile.skips_holes);
                stack.push(hit);
            }
            // `( a b rect -- flag )`: whether the point is inside that one
            // rectangle. `?XINSIDE` over a single record, answering yes or no
            // instead of an index — and with no hole test in any build, not
            // even the two whose `?XINSIDE` has one: `0104:2b0f` in `LL.EXE`
            // is four comparisons and nothing else.
            //
            // The record's four cells pair up with the arguments the way the
            // handler reads them: the cells at `+0` and `+4` bound the value
            // popped last, the ones at `+2` and `+6` the value popped before
            // it, all four inclusive. Only Victor Loomes calls it.
            Word::Q_INSIDE => {
                let a = pop_n(stack, 3, "?INSIDE")?;
                let (first, second, rect) = (a[0], a[1], a[2]);
                let cell = mem.cell_size();
                let c: Vec<i32> = (0..4)
                    .map(|k| mem.fetch_cell(mem.offset(rect, k * cell)).unwrap_or(0))
                    .collect();
                let inside = first >= c[0] && first <= c[2] && second >= c[1] && second <= c[3];
                stack.push(i32::from(inside));
            }
            Word::MOUSEINFO => self.word_mouseinfo(stack, mem, rules)?,
            Word::MOUSEX => {
                self.polled();
                stack.push(self.input.mouse.x);
            }
            Word::MOUSEY => {
                self.polled();
                stack.push(self.input.mouse.y);
            }
            Word::MOUSELK => {
                self.polled();
                stack.push(self.input.mouse.left);
            }
            Word::MOUSERK => {
                self.polled();
                stack.push(self.input.mouse.right);
            }
            // y first, then x, so x is what ends up on top — that is the order
            // the handler pushes the two record fields in, and it was the other
            // way round here until the handler was read.
            Word::MOUSEXY => {
                self.polled();
                stack.extend([self.input.mouse.y, self.input.mouse.x]);
            }

            // Gives the pointer a sprite of the game's at the given hotspot;
            // see [`Engine::arm_sprite`].
            Word::XATMOUSE => {
                let a = pop_n(stack, 3, "XATMOUSE")?;
                self.arm_sprite(a[2], a[0], a[1]);
            }
            // The handler pops one value and pushes 0, 0 and it back before
            // calling `XATMOUSE`: a cursor sprite with the hotspot at its
            // corner. The two zeros are constants, not the pointer's own
            // position — which is the plausible reading and the wrong way
            // round, since the hotspot is an offset *into* the sprite.
            Word::ATMOUSE => {
                let sprite = pop1(stack, "ATMOUSE")?;
                self.arm_sprite(sprite, 0, 0);
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}

/// `?XINSIDE`: which of a location's hot areas a point is in, or -1.
///
/// The table is `count` entries of `stride` bytes; the first four cells of each
/// are the corners, and the test takes them as inclusive.
///
/// `skip_holes` is the build's, not the format's. `ENVIRO.EXE` (`0a40:1b37`)
/// and `BMZ.EXE` follow the four comparisons with four more that ask whether
/// every corner is zero, and pass over the entry when they all are — an entry
/// of four zeroes is a hole rather than a rectangle at the origin. The two
/// older builds, `HPPLAY.EXE` and `LL.EXE`, stop after the comparisons: 71
/// instructions against 101, with no `cmpw $0` among them. So a hole in their
/// tables is a rectangle at the origin, and a point at 0,0 is inside it.
///
/// Takes the machine's memory rather than the engine because the table is
/// the game's, not ours; the corners are one cell each, whatever the machine's
/// cell is. A read that runs off the end answers zero rather than failing,
/// which is what the 32-bit handler's own bounds-free walk does.
pub(crate) fn area_containing(
    mem: &dyn AddressSpace,
    x: i32,
    y: i32,
    table: i32,
    stride: i32,
    count: i32,
    skip_holes: bool,
) -> i32 {
    let cell = mem.cell_size();
    let at = |i: i32, c: i32| mem.offset(table, i * stride + c * cell);
    for i in 0..count.max(0) {
        let c: Vec<i32> = (0..4)
            .map(|k| mem.fetch_cell(at(i, k)).unwrap_or(0))
            .collect();
        if x >= c[0] && x <= c[2] && y >= c[1] && y <= c[3] && !(skip_holes && c == [0, 0, 0, 0]) {
            return i;
        }
    }
    -1
}

/// The values `MOUSEINFO` pops that it reads, by what they are: the
/// pointer over the bar and over the scene, the area table and its stride,
/// the item tables, the inventory list, the info line's screen and
/// descriptor, and the pointer shapes with the word that sets them.
struct InfoArgs {
    imx: i32,
    mmx: i32,
    mmy: i32,
    lditem: i32,
    a_lditem: i32,
    s_lditem: i32,
    one: i32,
    fitem: i32,
    actinv: i32,
    screen: i32,
    minfo: i32,
    fitem2: i32,
    flag: i32,
    bmnr: i32,
    fxatmouse: i32,
    t390: i32,
    t391: i32,
    t392: i32,
}

impl InfoArgs {
    /// Out of the twenty-four, deepest first.
    fn of(a: &[i32]) -> Self {
        Self {
            imx: a[0],
            mmx: a[2],
            mmy: a[3],
            lditem: a[4],
            a_lditem: a[5],
            s_lditem: a[6],
            one: a[7],
            fitem: a[8],
            actinv: a[9],
            screen: a[13],
            minfo: a[14],
            fitem2: a[15],
            flag: a[16],
            bmnr: a[17],
            fxatmouse: a[18],
            t390: a[19],
            t391: a[20],
            t392: a[21],
        }
    }
}

// Word bodies moved out of the dispatcher above.
impl Engine {
    /// The status line under the pointer, and the pointer's own shape.
    ///
    /// `SCANITEM` in module 5 calls it with twenty-four values; the
    /// handler pops exactly that many, so unlike `ANIMPLAY` the count is
    /// real. Three cases: over the scene, over the inventory bar, or
    /// over neither. Each ends by setting the cursor through
    /// `FXATMOUSE`, a module-5 word — the handler runs the interpreter
    /// again for it, and since every one of those eight calls is the
    /// last thing its branch does, it is requested as a tail call here.
    ///
    /// Returns the hot area the pointer is in, or -1.
    fn word_mouseinfo(
        &mut self,
        stack: &mut Vec<i32>,
        mem: &dyn AddressSpace,
        rules: Rules,
    ) -> Result<()> {
        let a = pop_n(stack, 24, "MOUSEINFO")?;
        let p = InfoArgs::of(&a);
        let mode = a[23];

        // The handler keeps the last mode in a global and forces the
        // "something changed" flag when it differs, so a mode switch
        // always redraws even if the pointer has not moved.
        let mut changed = a[22];
        if self.last_info_mode != Some(mode) {
            self.last_info_mode = Some(mode);
            changed = 1;
        }

        // Every case starts on the info line's screen and descriptor.
        self.select_screen(cell::unsigned(p.screen));
        self.select_descriptor(cell::unsigned(p.minfo));
        // Modes 2, 4 and 5 are the menus: the info line belongs to them
        // and this word keeps its hands off.
        if mode == 2 || mode == 4 || mode == 5 {
            stack.push(-1);
            return Ok(());
        }

        let over_scene = p.mmx != -1 && matches!(mode, 0 | 3 | 8 | 9);
        let over_bar = p.imx != -1 && matches!(mode, 0 | 3 | 8 | 9 | 0xE);
        let (result, cursor) = if over_scene {
            self.info_over_scene(&p, mem, changed)?
        } else if over_bar {
            (-1, self.info_over_bar(&p, mem, rules)?)
        } else {
            (-1, self.info_elsewhere(&p, mem)?)
        };

        if let Some(nr) = cursor {
            self.pending_call = Some((p.fxatmouse, vec![nr, 0, 0]));
        }
        stack.push(result);
        Ok(())
    }

    /// The pointer over the scene: the caption of the area under it, and
    /// the shape that says what a click there would do — an exit, a usable
    /// item, or plain. Answers the area, or -1, and the shape to set.
    fn info_over_scene(
        &mut self,
        p: &InfoArgs,
        mem: &dyn AddressSpace,
        changed: i32,
    ) -> Result<(i32, Option<i32>)> {
        let cell = mem.cell_size();
        // Records are addressed by cell: the area's caption is its fifth
        // cell, its exit the tenth and its item the eleventh; an item record
        // is five cells with the flags second.
        let field = |base: i32, cells: i32| mem.offset(base, cells * cell);
        let result = area_containing(
            mem,
            p.mmx,
            p.mmy,
            p.lditem,
            p.s_lditem,
            p.a_lditem,
            self.profile.skips_holes,
        );
        if result == -1 {
            self.blank_info(p.one, mem)?;
            let cursor = (p.flag != 0 && p.bmnr != p.t390).then_some(p.t390);
            return Ok((result, cursor));
        }
        let tb = self.descriptor_table();
        let txt = self.descriptor_text_entry();
        let label = mem.fetch_cell(mem.offset(p.lditem, p.s_lditem * result + 4 * cell))?;
        if !(tb == p.one && txt == label && changed == 0) {
            self.set_text_table(p.one)?;
            self.set_text(label)?;
            // The handler measures the new text on the spot (`SDTXT`, as the
            // word does), so a `GDX` or `SDX` in the same frame sees its size.
            self.lay_out_current(mem);
            self.place_x(p.mmx, Placement::Center)?;
            self.place_y(p.mmy, Placement::Center)?;
        }
        if p.flag == 0 {
            return Ok((result, None));
        }
        let area = |cells: i32| field(p.lditem, result * 16 + cells);
        let exit = mem.fetch_cell(area(9))?;
        let item = mem.fetch_cell(area(10))?;
        let usable = item != 0 && mem.fetch_cell(field(p.fitem2, item * 5 + 1))? & 1 != 0;
        // Nothing to do when the pointer is already plain and nothing has
        // changed.
        let cursor = if exit != 0 {
            Some(p.t392)
        } else if usable {
            Some(p.t391)
        } else if p.bmnr != p.t390 || changed != 0 {
            Some(p.t390)
        } else {
            None
        };
        Ok((result, cursor))
    }

    /// The pointer over the inventory bar: the name of the item in the slot
    /// under it, placed over the slot, and the shape to set.
    fn info_over_bar(
        &mut self,
        p: &InfoArgs,
        mem: &dyn AddressSpace,
        rules: Rules,
    ) -> Result<Option<i32>> {
        let cell = mem.cell_size();
        // The bar's slots are where `CCALCINV` puts them; where the bar
        // starts and how wide a slot is differs between the two engines, see
        // [`Rules`].
        let slot = (p.imx - rules.bar_x0) / rules.slot_w;
        let offset = mem.fetch_cell(p.actinv)?;
        let index = slot + offset;
        let item = if (0..=7).contains(&slot) && p.imx >= rules.bar_x0 {
            mem.fetch_cell(mem.offset(p.actinv, 4 + index * cell))?
        } else {
            0
        };
        if item == 0 {
            self.blank_info(p.one, mem)?;
            return Ok((p.flag != 0 && p.bmnr != p.t390).then_some(p.t390));
        }
        self.set_text_table(p.one)?;
        let label = mem.fetch_cell(mem.offset(p.fitem, item * 5 * cell))?;
        self.set_text(label)?;
        // The handler measures the new text on the spot (`SDTXT`, as the
        // word does), so a `GDX` or `SDX` in the same frame sees its size.
        self.lay_out_current(mem);
        let x = self.screen_origin_x() + (index - offset) * rules.slot_w + rules.slot_w * 3 / 2;
        self.place_x(x, Placement::Center)?;
        let y = self.screen_origin_y() + rules.label_y;
        self.place_y(y, rules.label_placement)?;
        Ok((p.flag != 0 && p.bmnr != p.t391).then_some(p.t391))
    }

    /// The pointer over neither: the info line goes blank, and the pointer
    /// is put back to plain — only worth doing if it is one of the two
    /// special shapes now.
    fn info_elsewhere(&mut self, p: &InfoArgs, mem: &dyn AddressSpace) -> Result<Option<i32>> {
        self.blank_info(p.one, mem)?;
        Ok(((p.bmnr == p.t391 || p.bmnr == p.t392) && p.flag != 0).then_some(p.t390))
    }

    /// Entry 1 of table `one` on the info line, unless it stands there
    /// already: the blank caption.
    fn blank_info(&mut self, one: i32, mem: &dyn AddressSpace) -> Result<()> {
        let tb = self.descriptor_table();
        let txt = self.descriptor_text_entry();
        if !(tb == one && txt == 1) {
            self.set_text_table(one)?;
            self.set_text(1)?;
            // The handler measures the new text on the spot (`SDTXT`, as the
            // word does), so a `GDX` or `SDX` in the same frame sees its size.
            self.lay_out_current(mem);
        }
        Ok(())
    }
}
