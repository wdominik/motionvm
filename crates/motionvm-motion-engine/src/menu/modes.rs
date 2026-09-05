//! The modes the interaction machine runs in: the right click that opens a
//! strip, and the chain that carries the click through to a verb.
//!
//! `CTRL`'s block keeps one mode number, and every frame is a case of it.
//! This is the case machine — which mode leads to which, what each waits
//! for, and what it hands on — with the strip it puts up in [`super::strip`]
//! and the verb it ends at in [`super::verbs`].

use super::{
    A_ANCHOR_X, A_ANCHOR_Y, A_FLAGS, A_ITEM, A_P1, A_P2, A_P3, A_P4, A_X1, A_X2, A_Y1, A_Y2, BIAS,
    I_FLAGS, Strip, area, bar_slot, bar_strip, choose, highlight, item, list_slot, remove_menu,
    show_menu,
};
use crate::order::block::{
    AFTER, APPROACH, AREA_HIT, ARROW, BAR_MENU, BAR_SCREEN, CALCINV, CANCELED, CAPTION, FINISHED,
    GATE1, GATE2, HELD, IMX, INVENTORY, MLK, MODE, MRK, OBJECT, OPENED, PICKED, PRESSED,
    SCENE_MENU, SCREEN, SET_CURSOR, SLOT, STARTED, TAKEN, TARGET, TASK, VERB,
};
use crate::order::{Conversation, Rules, get, put};
use crate::{Engine, Placement};
use motionvm_motion_forth::cell;
use motionvm_motion_forth::{Host, Machine, Result};
/// The right click, 0x7d0b3: open the menu on whatever is under the pointer.
pub(crate) fn right_click<M>(eng: &mut Engine, vm: &mut M, order: u32, rules: Rules) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    // A walk that was armed and not yet run is called off by a right click.
    if get(vm, order, MODE)? == 9 {
        put(vm, order, MODE, 0)?;
    }

    // The bar — eight slots, as wide as the engine's rules say.
    if let Some(slot) = bar_slot(vm, order, rules)? {
        if slot.item == 0 {
            return Ok(());
        }
        let flags = item(vm, order, slot.item, I_FLAGS)? | 2;
        put(vm, order, TARGET, slot.item)?;
        put(vm, order, SLOT, slot.index)?;
        let strip = bar_strip(vm, order, slot, rules)?;
        show_menu(eng, vm, order, strip, flags, rules)?;
        put(vm, order, MODE, 2)?;
        put(vm, order, VERB, 0)?;
        return eng.order_callback(vm, order, OPENED, &[3]);
    }

    eng.area_under_pointer(vm, order)?;
    let hit = get(vm, order, AREA_HIT)?;
    // Nothing under the pointer, nothing to offer — and no mode change, which
    // is why a right click on the title screen simply does nothing.
    if hit == 0 {
        return Ok(());
    }
    let target = match area(vm, order, hit, A_ITEM)? {
        0 => hit,
        held => held,
    };
    put(vm, order, TARGET, target)?;
    let flags = if target < BIAS {
        item(vm, order, target, I_FLAGS)?
    } else {
        area(vm, order, target, A_FLAGS)?
    } | 2;
    // Bit 7 is the walk-to verb, and a place that only offers that gets no
    // menu at all.
    if flags & 0x80 != 0 {
        return Ok(());
    }

    let (mut mx, mut my) = (
        area(vm, order, hit, A_ANCHOR_X)?,
        area(vm, order, hit, A_ANCHOR_Y)?,
    );
    if mx == 0 && my == 0 {
        mx = (area(vm, order, hit, A_X1)? + area(vm, order, hit, A_X2)?) / 2;
        my = (area(vm, order, hit, A_Y1)? + area(vm, order, hit, A_Y2)?) / 2;
    }

    let screen = get(vm, order, SCREEN)?;
    eng.select_screen(cell::unsigned(screen));
    let left = eng.screen_origin_x();
    // The handler takes the width and throws the height away.
    let (width, _) = eng.screen_view_size();
    let right = left + width;
    if mx - 0x20 < left + 8 {
        mx = left + 0x28;
    } else if mx + 0x20 > right - 8 {
        mx = right - 0x28;
    }

    let strip = Strip {
        screen,
        base: get(vm, order, SCENE_MENU)?,
        x: mx,
        y: my,
    };
    show_menu(eng, vm, order, strip, flags, rules)?;
    put(vm, order, MODE, 4)?;
    put(vm, order, VERB, 0)?;

    // The caption rides above the strip.
    eng.select_screen(cell::unsigned(screen));
    let caption = get(vm, order, CAPTION)?;
    eng.select_descriptor(cell::unsigned(caption));
    my -= 0x14;
    if my < 2 {
        // Note it takes the anchor again, not the value just adjusted.
        my = area(vm, order, hit, A_ANCHOR_Y)? + 0x14;
    }
    eng.place_x(mx, Placement::Center)?;
    eng.place_y(my, Placement::Center)
}

/// Is this a fresh press of that button? A held one never fires twice.
fn fresh<M>(vm: &M, order: u32, button: u32) -> Result<bool>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    Ok(get(vm, order, button)? != 0 && get(vm, order, PRESSED)? == 0)
}

/// The mode chain at 0x7d4a8: modes 2 to 8, one arm each.
pub(crate) fn mode_chain<M>(
    eng: &mut Engine,
    vm: &mut M,
    order: u32,
    mode: i32,
    rules: Rules,
) -> Result<bool>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    match mode {
        // The menu stands open over the bar or over the scene. Same shape
        // either way, on its own screen and its own descriptors.
        2 | 4 => {
            let bar = mode == 2;
            let screen = get(vm, order, if bar { BAR_SCREEN } else { SCREEN })?;
            let base = get(vm, order, if bar { BAR_MENU } else { SCENE_MENU })?;
            highlight(eng, vm, order, screen, base)?;

            if fresh(vm, order, MLK)? && get(vm, order, GATE2)? == 0 {
                let flags = if bar {
                    let list = get(vm, order, INVENTORY)?;
                    let index = get(vm, order, SLOT)?;
                    let held = list_slot(vm, list, index)?;
                    item(vm, order, held, I_FLAGS)? | 2
                } else {
                    // The scene reads the target afresh: an area may stand for
                    // an item, and then it is the item's flags that count.
                    let target = get(vm, order, TARGET)?;
                    if target >= BIAS {
                        match area(vm, order, target, A_ITEM)? {
                            0 => area(vm, order, target, A_FLAGS)?,
                            held => item(vm, order, held, I_FLAGS)?,
                        }
                    } else {
                        item(vm, order, target, I_FLAGS)?
                    }
                };
                choose(eng, vm, order, screen, base, flags, rules)?;
            }

            let cancel = (fresh(vm, order, MRK)? && get(vm, order, GATE1)? == 0)
                || get(vm, order, GATE2)? != 0;
            if cancel {
                remove_menu(eng, vm, order)?;
                if !bar {
                    let (held, list) = (get(vm, order, HELD)?, get(vm, order, INVENTORY)?);
                    crate::words::remove_from_inventory(vm.space_mut(), held, list)?;
                    eng.order_callback(vm, order, CALCINV, &[])?;
                }
            }
            Ok(true)
        }
        // Something rides on the cursor and the next click says what to use it
        // on. The bar's interval is closed here, where every other test has it
        // half-open — the handler's own inconsistency.
        3 => {
            if fresh(vm, order, MLK)? && get(vm, order, GATE2)? == 0 {
                let imx = get(vm, order, IMX)?;
                if (rules.bar_x0..=rules.bar_x0 + 8 * rules.slot_w).contains(&imx) {
                    let list = get(vm, order, INVENTORY)?;
                    let index =
                        get(vm, cell::unsigned(list), 0)? + (imx - rules.bar_x0) / rules.slot_w;
                    let second = list_slot(vm, list, index)?;
                    if second != 0 {
                        put(vm, order, OBJECT, second)?;
                        let arrow = get(vm, order, ARROW)?;
                        eng.order_callback(vm, order, SET_CURSOR, &[arrow, 0, 0])?;
                        eng.order_callback(vm, order, PICKED, &[])?;
                        put(vm, order, MODE, 6)?;
                        put(vm, order, 0x104, -1)?;
                        let (held, inv) = (get(vm, order, HELD)?, get(vm, order, INVENTORY)?);
                        crate::words::remove_from_inventory(vm.space_mut(), held, inv)?;
                        eng.order_callback(vm, order, CALCINV, &[])?;
                        if get(vm, order, TAKEN)? == 1 {
                            put(vm, order, TAKEN, 2)?;
                        }
                    } else if get(vm, order, GATE1)? != 0 {
                        put(vm, order, MODE, 0)?;
                        let arrow = get(vm, order, ARROW)?;
                        eng.order_callback(vm, order, SET_CURSOR, &[arrow, 0, 0])?;
                        eng.order_callback(vm, order, CANCELED, &[])?;
                        let (held, inv) = (get(vm, order, HELD)?, get(vm, order, INVENTORY)?);
                        crate::words::remove_from_inventory(vm.space_mut(), held, inv)?;
                        eng.order_callback(vm, order, CALCINV, &[])?;
                    }
                } else {
                    eng.area_under_pointer(vm, order)?;
                    let hit = get(vm, order, AREA_HIT)?;
                    if hit != 0 {
                        let second = match area(vm, order, hit, A_ITEM)? {
                            0 => hit,
                            held => held,
                        };
                        put(vm, order, OBJECT, second)?;
                        eng.order_callback(vm, order, PICKED, &[])?;
                        put(vm, order, MODE, 6)?;
                        for (slot, f) in [
                            (0x104u32, A_P1),
                            (0x108, A_P2),
                            (0x10c, A_P3),
                            (0x118, A_P4),
                        ] {
                            let v = area(vm, order, hit, f)?;
                            put(vm, order, slot, v)?;
                        }
                        if get(vm, order, TAKEN)? == 1 {
                            put(vm, order, TAKEN, 2)?;
                        }
                    } else if get(vm, order, GATE1)? != 0 {
                        put(vm, order, MODE, 0)?;
                        let arrow = get(vm, order, ARROW)?;
                        eng.order_callback(vm, order, SET_CURSOR, &[arrow, 0, 0])?;
                        eng.order_callback(vm, order, CANCELED, &[])?;
                    }
                }
            }
            if (fresh(vm, order, MRK)?) || get(vm, order, GATE2)? != 0 {
                put(vm, order, MODE, 0)?;
                let arrow = get(vm, order, ARROW)?;
                eng.order_callback(vm, order, SET_CURSOR, &[arrow, 0, 0])?;
                eng.order_callback(vm, order, CANCELED, &[])?;
                let (held, inv) = (get(vm, order, HELD)?, get(vm, order, INVENTORY)?);
                crate::words::remove_from_inventory(vm.space_mut(), held, inv)?;
                eng.order_callback(vm, order, CALCINV, &[])?;
            }
            Ok(true)
        }
        // Run it now, if no script of the location is in the way.
        5 => {
            if figure_free(vm, order, rules)? {
                put(vm, order, MODE, 0x63)?;
                let (v, t, o) = triple(vm, order)?;
                eng.exec_order(vm, order, v, t, o, rules)?;
            }
            Ok(true)
        }
        // Waiting for the figure to arrive.
        6 => {
            if walk_done(vm, order)? {
                if get(vm, order, TAKEN)? == 2 {
                    let target = get(vm, order, TARGET)?;
                    eng.exec_order(vm, order, 1, target, 0, rules)?;
                    if get(vm, order, STARTED)? == 0 {
                        let (held, inv) = (get(vm, order, HELD)?, get(vm, order, INVENTORY)?);
                        crate::words::remove_from_inventory(vm.space_mut(), held, inv)?;
                        eng.order_callback(vm, order, CALCINV, &[])?;
                        put(vm, order, MODE, 0x63)?;
                    } else {
                        put(vm, order, MODE, 7)?;
                    }
                    put(vm, order, TAKEN, -1)?;
                } else {
                    put(vm, order, AFTER, 0)?;
                    put(vm, order, MODE, 7)?;
                }
            }
            Ok(true)
        }
        // Arrived: either the thing is done, or the figure still has to reach
        // for it.
        7 => {
            if get(vm, order, AFTER)? & 1 == 0 {
                if get(vm, order, OBJECT)? == get(vm, order, HELD)? {
                    eng.order_callback(vm, order, FINISHED, &[])?;
                    put(vm, order, MODE, 0)?;
                    eng.order_callback(vm, order, OPENED, &[3])?;
                } else {
                    if get(vm, order, 0x104)? == -1 {
                        eng.order_callback(vm, order, OPENED, &[3])?;
                    } else {
                        let args: Vec<i32> = [0x104u32, 0x108, 0x10c, 0x118]
                            .iter()
                            .map(|&f| get(vm, order, f))
                            .collect::<Result<_>>()?;
                        eng.order_callback(vm, order, APPROACH, &args)?;
                    }
                    put(vm, order, MODE, 5)?;
                    put(vm, order, VERB, 4)?;
                }
            } else {
                let target = get(vm, order, TARGET)?;
                eng.exec_order(vm, order, 1, target, 0, rules)?;
                if get(vm, order, STARTED)? == 0 {
                    put(vm, order, MODE, 0x63)?;
                }
            }
            Ok(true)
        }
        // The verb the menu picked.
        8 => {
            if get(vm, order, GATE2)? != 0 {
                put(vm, order, MODE, 0)?;
            } else if walk_done(vm, order)? {
                put(vm, order, MODE, 0x63)?;
                let (v, t, o) = triple(vm, order)?;
                eng.exec_order(vm, order, v, t, o, rules)?;
                eng.order_callback(vm, order, PICKED, &[])?;
            }
            Ok(true)
        }
        _ => Ok(true),
    }
}

fn triple<M>(vm: &M, order: u32) -> Result<(i32, i32, i32)>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    Ok((
        get(vm, order, VERB)?,
        get(vm, order, TARGET)?,
        get(vm, order, OBJECT)?,
    ))
}

/// Whether the figure is free for a verb that starts at once — mode 5 and
/// the armed walk. The 32-bit engine asks the task's walking flag
/// (`person[0x1a8]`, 0x7dab3 and 0x7d459), the 16-bit one its command cell
/// like every other wait (`0d34:2ade`, `0d34:2621`).
pub(crate) fn figure_free<M: Machine>(vm: &M, order: u32, rules: Rules) -> Result<bool> {
    if rules.idle_by_command {
        return walk_done(vm, order);
    }
    let task = cell::unsigned(get(vm, order, TASK)?);
    Ok(get(vm, task, 0x1a8)? == 0)
}

/// The walk is over when the task's command word is empty or the "arrived"
/// marker 999 — every wait of the 16-bit engine, and the conversation's item
/// order on the 32-bit one (0x7e9ce).
pub(crate) fn walk_done<M: Machine>(vm: &M, order: u32) -> Result<bool> {
    let task = cell::unsigned(get(vm, order, TASK)?);
    let at = get(vm, task, 0x1cc)?;
    Ok(at == 0 || at == 0x3e7)
}
