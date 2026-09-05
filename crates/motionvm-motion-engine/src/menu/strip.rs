//! The strip itself: laying the verb icons out, pulsing the one under the
//! pointer, and reading off which was clicked.
//!
//! `GMSHOWMENU`, `GMREMOVEMENU`, `HIGHLIGHTORDERS`, `ANIMATEORDERS` and
//! `CHOOSEORDERS` — five native routines that between them do nothing but
//! put descriptors on a screen and take them off again. What decides that a
//! strip should be there at all is [`super::modes`], and what happens after
//! one is clicked is [`super::verbs`].

use super::{
    A_EXIT, A_FLAGS, A_ITEM, BIAS, ENTRY, I_FLAGS, I_SPRITE, area, item, on_slot, sprite_range,
    verb_of_slot,
};
use crate::order::block::{
    ARROW, BAR_MENU, BAR_SCREEN, CALCINV, CANCELED, GATE1, MODE, POINTER, PULSE, SCENE_MENU,
    SCREEN, SET_CURSOR, TAKEN, TARGET, VERB, VERBS,
};
use crate::order::{Conversation, Rules, get, put};
use crate::{Engine, Placement};
use motionvm_motion_forth::cell;
use motionvm_motion_forth::{Error, Host, Machine, Result};
/// Where a strip goes: the screen it is laid on, the first of its five
/// descriptors, and the point it centers on.
///
/// The screen and the base are a pair the order block itself keeps twice
/// over — `BAR_SCREEN`/`BAR_MENU` for the strip above the inventory,
/// `SCREEN`/`SCENE_MENU` for the one in the picture — and every caller
/// fetches them together, [`remove_menu`] included, which switches both
/// pairs off in a loop. The anchor rides along because the two are never
/// decided apart: whichever strip is chosen, the point is the one that
/// strip is placed at.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Strip {
    /// The screen, as `SCRSEL` selects it.
    pub screen: i32,
    /// The first of the five descriptors; the icons take `base + j`.
    pub base: i32,
    /// The anchor's x, which the run is centered on.
    pub x: i32,
    /// The anchor's y, taken as the icons' top edge.
    pub y: i32,
}

/// `GMSHOWMENU` (0x7a2e2): lay the strip out around a point.
///
/// One icon per set bit, a step apart (0x30 on the 32-bit engine, 0x14 on
/// the 16-bit), the whole run centered on the anchor by shifting it half a
/// step left per icon. At most five slots are ever cleared
/// again afterwards, so five is what the menu really holds.
pub(crate) fn show_menu<M>(
    eng: &mut Engine,
    vm: &mut M,
    order: u32,
    strip: Strip,
    flags: i32,
    rules: Rules,
) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let Strip { screen, base, x, y } = strip;
    let mode = get(vm, order, MODE)?;
    let flags = if mode != 0x11 && mode != 0x0e {
        flags | 2
    } else {
        flags
    };
    if flags < 0 {
        // The handler counts bits by shifting arithmetically and would never
        // come back. No mask in the game's data has the top bit set.
        return Err(Error::Unread {
            what: "GMSHOWMENU: a negative verb mask — the original counts bits by \
                   shifting arithmetically and would never come back"
                .into(),
            binary: "ENGINE.EXE",
            at: "0x7a2e2",
        });
    }
    let n = cell::signed(flags.count_ones());
    let mut x = x - rules.icon_step / 2 * n + 2;
    eng.select_screen(cell::unsigned(screen));
    let mut rest = flags;
    for j in 0..n {
        eng.select_descriptor(cell::unsigned(base + j));
        eng.set_active(true);
        eng.place_x(x, Placement::Edge)?;
        eng.place_y(y, Placement::Edge)?;
        let bit = cell::signed(rest.trailing_zeros());
        let sprite = match bit {
            0..=7 => get(vm, order, VERBS + cell::unsigned(bit) * ENTRY)?,
            _ => 0,
        };
        eng.set_sprite(sprite)?;
        rest &= !(1 << bit);
        x += rules.icon_step;
    }
    // Only five, however many were just switched on — the handler's own
    // asymmetry, and the reason the menu is capped at five in practice.
    for j in n..5 {
        eng.select_descriptor(cell::unsigned(base + j));
        eng.set_active(false);
    }
    let pointer = get(vm, order, POINTER)?;
    let (hx, hy) = rules.pointer_hot;
    eng.order_callback(vm, order, SET_CURSOR, &[pointer, hx, hy])
}

/// `GMREMOVEMENU` (0x7a552): both strips off, the plain arrow back, mode 0.
pub(crate) fn remove_menu<M>(eng: &mut Engine, vm: &mut M, order: u32) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    for (screen, base) in [(BAR_SCREEN, BAR_MENU), (SCREEN, SCENE_MENU)] {
        let (screen, base) = (get(vm, order, screen)?, get(vm, order, base)?);
        eng.select_screen(cell::unsigned(screen));
        for i in 0..5 {
            eng.select_descriptor(cell::unsigned(base + i));
            eng.set_active(false);
        }
    }
    eng.order_callback(vm, order, CALCINV, &[])?;
    let arrow = get(vm, order, ARROW)?;
    eng.order_callback(vm, order, SET_CURSOR, &[arrow, 0, 0])?;
    eng.order_callback(vm, order, CANCELED, &[])?;
    put(vm, order, MODE, 0)
}

/// `HIGHLIGHTORDERS` (0x7a759): which slot the pointer is on, once a frame.
///
/// It only ever *starts* a pulse — a slot already pulsing is left alone, so
/// the animation keeps its phase while the pointer rests on it.
///
/// The original (`menu_pulse`, 16-bit `0d34:0492`) opens with
/// `ACTSCR(screen)` before its `ACTDESC` loop; descriptor numbers are
/// screen-relative, and `CTRL` leaves the bar screen current whenever
/// the pointer sits below the view, so selecting first is load-bearing.
pub(crate) fn highlight<M>(
    eng: &mut Engine,
    vm: &mut M,
    order: u32,
    screen: i32,
    base: i32,
) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    eng.select_screen(cell::unsigned(screen));
    for i in 0..5u32 {
        eng.select_descriptor(cell::unsigned(base + cell::signed(i)));
        if eng.descriptor_active() == 0 {
            put(vm, order, PULSE + i * 4, 0)?;
            continue;
        }
        if on_slot(eng, vm, order, screen)? {
            if get(vm, order, PULSE + i * 4)? == 0 {
                put(vm, order, PULSE + i * 4, 1)?;
            }
        } else {
            put(vm, order, PULSE + i * 4, 0)?;
        }
    }
    Ok(())
}

/// Which verb the slot at `i` shows, for the two routines that need to know.
fn verb_at<M>(vm: &M, order: u32, i: i32) -> Result<i32>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    if get(vm, order, MODE)? == 0x11 {
        return match i {
            0 => Ok(6),
            1 => Ok(7),
            // The handler leaves its local uninitialized here. Only two slots
            // are ever active in an item menu, so it cannot be reached.
            _ => Err(Error::Unread {
                what: format!(
                    "ANIMATEORDERS: slot {i} of an item menu — the original leaves its \
                     local uninitialized there"
                ),
                binary: "ENGINE.EXE",
                at: "0x7a900",
            }),
        };
    }
    let target = get(vm, order, TARGET)?;
    let flags = if target >= BIAS {
        match area(vm, order, target, A_ITEM)? {
            0 => area(vm, order, target, A_FLAGS)?,
            held => item(vm, order, held, I_FLAGS)?,
        }
    } else {
        item(vm, order, target, I_FLAGS)?
    };
    Ok(verb_of_slot(flags, i))
}

/// `ANIMATEORDERS` (0x7a900): the highlighted icon breathes.
///
/// One sprite a frame between the entry's rest and end pictures, turning round
/// at each end; every other icon is pushed back to its rest picture. Nothing
/// else in the block changes.
pub(crate) fn animate<M>(
    eng: &mut Engine,
    vm: &mut M,
    order: u32,
    screen: i32,
    base: i32,
) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    eng.select_screen(cell::unsigned(screen));
    for i in 0..5u32 {
        eng.select_descriptor(cell::unsigned(base + cell::signed(i)));
        if eng.descriptor_active() == 0 {
            continue;
        }
        let verb = verb_at(vm, order, cell::signed(i))?;
        let Some((rest, end)) = sprite_range(vm, order, verb)? else {
            continue;
        };
        let pulse = get(vm, order, PULSE + i * 4)?;
        if pulse == 0 {
            if eng.descriptor_sprite() != rest {
                eng.set_sprite(rest)?;
            }
            continue;
        }
        if end - rest < 2 {
            // No room to animate in: the icon simply sits on its end picture.
            if eng.descriptor_sprite() != end {
                eng.set_sprite(end)?;
            }
        } else if pulse == 1 {
            let mut s = eng.descriptor_sprite() + 1;
            if s >= end {
                put(vm, order, PULSE + i * 4, 2)?;
                s = end;
            }
            eng.set_sprite(s)?;
        } else if pulse == 2 {
            let mut s = eng.descriptor_sprite() - 1;
            if s <= rest {
                put(vm, order, PULSE + i * 4, 1)?;
                s = rest;
            }
            eng.set_sprite(s)?;
        }
    }
    Ok(())
}

/// `CHOOSEORDERS` (0x7ad26): the left click that picks an icon.
///
/// A pick sets the verb and leaves mode **8**, which is where it is executed —
/// not 9. Two verbs go elsewhere: 4 takes the thing onto the cursor (mode 3),
/// and 8 on an area is a walk, so the target becomes the area's exit and the
/// mode becomes 9.
pub(crate) fn choose<M>(
    eng: &mut Engine,
    vm: &mut M,
    order: u32,
    screen: i32,
    base: i32,
    flags: i32,
    rules: Rules,
) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    for i in 0..5 {
        eng.select_screen(cell::unsigned(screen));
        eng.select_descriptor(cell::unsigned(base + i));
        if eng.descriptor_active() == 0 {
            continue;
        }
        if !on_slot(eng, vm, order, screen)? {
            if get(vm, order, GATE1)? == 1 {
                remove_menu(eng, vm, order)?;
            }
            continue;
        }
        let chosen = match get(vm, order, MODE)? {
            0x11 => match i {
                0 => 6,
                1 => 7,
                _ => {
                    return Err(Error::Unread {
                        what: format!(
                            "CHOOSEORDERS: slot {i} of an item menu — the original \
                             complains there and waits for a key"
                        ),
                        binary: "ENGINE.EXE",
                        at: "0x7ad26",
                    });
                }
            },
            _ => verb_of_slot(flags, i),
        };
        put(vm, order, VERB, chosen)?;
        remove_menu(eng, vm, order)?;
        put(
            vm,
            order,
            MODE,
            if chosen == 6 || chosen == 7 { 0x12 } else { 8 },
        )?;

        let scene = get(vm, order, SCREEN)?;
        let target = get(vm, order, TARGET)?;
        if chosen == 4 {
            let sprite = item(vm, order, target, I_SPRITE)?;
            eng.order_callback(vm, order, SET_CURSOR, &[sprite + 1, 0, 0])?;
            put(vm, order, MODE, 3)?;
            if screen == scene {
                eng.set_flash_entry(vm, order, rules)?;
                put(vm, order, TAKEN, 1)?;
            } else {
                put(vm, order, TAKEN, -1)?;
            }
        } else if chosen == 8 && target >= BIAS {
            put(vm, order, MODE, 9)?;
            let exit = area(vm, order, target, A_EXIT)?;
            put(vm, order, TARGET, exit)?;
        }
    }
    Ok(())
}
