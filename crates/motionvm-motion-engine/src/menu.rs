//! The verb menu and the verbs it picks: the right click that opens the strip,
//! the modes it runs in, the native routines behind it, and `EXECORDER`'s own
//! eight cases.
//!
//! A right click asks what is under the pointer and puts a strip of verb icons
//! over it. Which verbs appear is a **bit mask** — the item's own flags, or the
//! area's when it stands for no item — with bit 1 forced in, so "look at" is
//! always offered. Bit 7 suppresses the menu altogether, which is how a plain
//! walk-to exit shows nothing.
//!
//! Their names are not guesses: each routine tags its stack-error messages
//! with them.
//!
//! ```text
//! 0x7a144  VERBOFSLOT      flags |= 2, then the k-th set bit — verb = bit + 1
//! 0x7a1bd  SPRRANGEOFVERB  verb → the icon's rest and end sprite
//! 0x7a2e2  GMSHOWMENU      lay the strip out and switch the pointer
//! 0x7a552  GMREMOVEMENU    take it away again
//! 0x7a759  HIGHLIGHTORDERS which slot the pointer is on
//! 0x7a900  ANIMATEORDERS   pulse that slot's icon between its two sprites
//! 0x7ad26  CHOOSEORDERS    the click that picks one
//! ```
//!
//! Measured against the park before any of it was written: the block sits at
//! 2:0x28ac with the verb table at +0x18 reading `286 287 <wort>` — a rest and
//! an end sprite and a handler, three fields to an entry and not two. Area 1 is
//! the rectangle (518,265)-(570,317), item 31, whose flags are 15: four icons,
//! bit 7 clear.

use crate::order::block::*;
use crate::order::{Conversation, Rules, get, put};
use crate::{Engine, Placement};
use motionvm_motion_forth::{Error, Host, Machine, Result};

const AREA: u32 = 64;
const ITEM: u32 = 20;
const ENTRY: u32 = 12;
/// An item record: +0 caption text, +4 flags, +8 sprite, +0xC description.
const I_FLAGS: u32 = 0x04;
const I_SPRITE: u32 = 0x08;
const I_INFO: u32 = 0x0c;

// An area record — the same 64 bytes the location's item block holds.
const A_X1: u32 = 0x00;
const A_Y1: u32 = 0x04;
const A_X2: u32 = 0x08;
const A_Y2: u32 = 0x0c;
/// The description an area shows when it stands for no item.
const A_INFO: u32 = 0x14;
const A_P3: u32 = 0x18;
const A_P1: u32 = 0x1c;
const A_P2: u32 = 0x20;
const A_EXIT: u32 = 0x24;
const A_ITEM: u32 = 0x28;
const A_ANCHOR_X: u32 = 0x2c;
const A_ANCHOR_Y: u32 = 0x30;
const A_P4: u32 = 0x34;
const A_FLAGS: u32 = 0x38;

/// The area search stores its hit as `1000 + index`, which is why every read
/// of an area field in the original carries a displacement near −0xFA00.
const BIAS: i32 = 1000;

fn area<M>(vm: &M, order: u32, n: i32, f: u32) -> Result<i32>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let areas = get(vm, order, AREAS)? as u32;
    get(vm, areas, (n - BIAS) as u32 * AREA + f)
}

fn item<M>(vm: &M, order: u32, n: i32, f: u32) -> Result<i32>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let items = get(vm, order, ITEMS)? as u32;
    get(vm, items, n as u32 * ITEM + f)
}

/// Whether the pointer is inside the descriptor the caller has selected.
///
/// The bar and the scene are two screens with two pointers, and the box test
/// takes whichever belongs to the screen the menu is on.
fn on_slot<M>(eng: &mut Engine, vm: &mut M, order: u32, screen: i32) -> Result<bool>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let (x1, x2) = (eng.descriptor_x(), eng.descriptor_far_x());
    let (y1, y2) = (eng.descriptor_y(), eng.descriptor_far_y());
    let (bar, scene) = (get(vm, order, BAR_SCREEN)?, get(vm, order, SCREEN)?);
    let (px, py) = if screen == bar {
        (get(vm, order, IMX)?, get(vm, order, IMY)?)
    } else if screen == scene {
        (get(vm, order, MMX)?, get(vm, order, MMY)?)
    } else {
        return Ok(false);
    };
    Ok(px >= x1 && px <= x2 && py >= y1 && py <= y2)
}

/// `VERBOFSLOT` (0x7a144): which verb the *k*-th icon of the strip stands for.
///
/// The mask decides both how many icons there are and which; the verb is the
/// bit's position plus one. Bit 1 is forced in here as well as in
/// [`show_menu`], so "look at" is on every menu the game ever puts up.
fn verb_of_slot(flags: i32, k: i32) -> i32 {
    let flags = flags | 2;
    let mut seen = 0;
    for bit in 0..16 {
        if flags & (1 << bit) != 0 {
            if seen == k {
                return bit + 1;
            }
            seen += 1;
        }
    }
    0x11
}

/// `SPRRANGEOFVERB` (0x7a1bd): the icon's two sprites, rest and end.
///
/// A verb outside 1..=8 leaves the caller's pair untouched — unsigned compare
/// against 7 on `verb - 1` — so there is nothing to answer with.
fn sprite_range<M>(vm: &M, order: u32, verb: i32) -> Result<Option<(i32, i32)>>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let Ok(k) = u32::try_from(verb - 1) else {
        return Ok(None);
    };
    if k > 7 {
        return Ok(None);
    }
    Ok(Some((
        get(vm, order, VERBS + k * ENTRY)?,
        get(vm, order, VERBS + k * ENTRY + 4)?,
    )))
}

/// `GMSHOWMENU` (0x7a2e2): lay the strip out around a point.
///
/// One icon per set bit, a step apart (0x30 on the 32-bit engine, 0x14 on
/// the 16-bit), the whole run centered on the anchor by shifting it half a
/// step left per icon. At most five slots are ever cleared
/// again afterwards, so five is what the menu really holds.
// Eight, because `GMSHOWMENU` takes eight. Bundling them into a struct would
// put a shape between this function and the handler it mirrors, and the point
// of the mirror is that the two can be read side by side.
#[allow(clippy::too_many_arguments)]
pub(crate) fn show_menu<M>(
    eng: &mut Engine,
    vm: &mut M,
    order: u32,
    screen: i32,
    base: i32,
    x: i32,
    y: i32,
    flags: i32,
    rules: Rules,
) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
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
            at: "0x7a2e2",
        });
    }
    let n = flags.count_ones() as i32;
    let mut x = x - rules.icon_step / 2 * n + 2;
    eng.select_screen(screen as u32);
    let mut rest = flags;
    for j in 0..n {
        eng.select_descriptor((base + j) as u32);
        eng.set_active(true);
        eng.place_x(x, Placement::Edge)?;
        eng.place_y(y, Placement::Edge)?;
        let bit = rest.trailing_zeros() as i32;
        let sprite = match bit {
            0..=7 => get(vm, order, VERBS + bit as u32 * ENTRY)?,
            _ => 0,
        };
        eng.set_sprite(sprite)?;
        rest &= !(1 << bit);
        x += rules.icon_step;
    }
    // Only five, however many were just switched on — the handler's own
    // asymmetry, and the reason the menu is capped at five in practice.
    for j in n..5 {
        eng.select_descriptor((base + j) as u32);
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
        eng.select_screen(screen as u32);
        for i in 0..5 {
            eng.select_descriptor((base + i) as u32);
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
    eng.select_screen(screen as u32);
    for i in 0..5u32 {
        eng.select_descriptor((base + i as i32) as u32);
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
            // are ever active in a yes/no menu, so it cannot be reached.
            _ => Err(Error::Unread {
                what: format!(
                    "ANIMATEORDERS: slot {i} of a yes/no menu — the original leaves its \
                     local uninitialized there"
                ),
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
    eng.select_screen(screen as u32);
    for i in 0..5u32 {
        eng.select_descriptor((base + i as i32) as u32);
        if eng.descriptor_active() == 0 {
            continue;
        }
        let verb = verb_at(vm, order, i as i32)?;
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
        eng.select_screen(screen as u32);
        eng.select_descriptor((base + i) as u32);
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
                            "CHOOSEORDERS: slot {i} of a yes/no menu — the original \
                             complains there and waits for a key"
                        ),
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

    // The bar — eight slots, as wide as the engine's rules say, and the
    // interval is half-open here where mode 3 has it closed.
    let imx = get(vm, order, IMX)?;
    if (rules.bar_x0..rules.bar_x0 + 8 * rules.slot_w).contains(&imx) {
        let list = get(vm, order, INVENTORY)?;
        let scroll = get(vm, list as u32, 0)?;
        let index = scroll + (imx - rules.bar_x0) / rules.slot_w;
        let held = list_slot(vm, list, index)?;
        if held == 0 {
            return Ok(());
        }
        let flags = item(vm, order, held, I_FLAGS)? | 2;
        put(vm, order, TARGET, held)?;
        put(vm, order, SLOT, index)?;
        let x = (index - scroll) * rules.slot_w + rules.slot_w * 3 / 2;
        let (screen, base) = (get(vm, order, BAR_SCREEN)?, get(vm, order, BAR_MENU)?);
        show_menu(
            eng,
            vm,
            order,
            screen,
            base,
            x,
            rules.bar_menu_y,
            flags,
            rules,
        )?;
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
    eng.select_screen(screen as u32);
    let left = eng.screen_origin_x();
    // The handler takes the width and throws the height away.
    let (width, _) = eng.screen_view_size();
    let right = left + width;
    if mx - 0x20 < left + 8 {
        mx = left + 0x28;
    } else if mx + 0x20 > right - 8 {
        mx = right - 0x28;
    }

    let base = get(vm, order, SCENE_MENU)?;
    show_menu(eng, vm, order, screen, base, mx, my, flags, rules)?;
    put(vm, order, MODE, 4)?;
    put(vm, order, VERB, 0)?;

    // The caption rides above the strip.
    eng.select_screen(screen as u32);
    let caption = get(vm, order, CAPTION)?;
    eng.select_descriptor(caption as u32);
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
                    let index = get(vm, list as u32, 0)? + (imx - rules.bar_x0) / rules.slot_w;
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
    let task = get(vm, order, TASK)? as u32;
    Ok(get(vm, task, 0x1a8)? == 0)
}

/// The walk is over when the task's command word is empty or the "arrived"
/// marker 999.
fn walk_done<M: Machine>(vm: &M, order: u32) -> Result<bool> {
    let task = get(vm, order, TASK)? as u32;
    let at = get(vm, task, 0x1cc)?;
    Ok(at == 0 || at == 0x3e7)
}

// ---------------------------------------------------------------------------
// The verbs — `EXECORDER`'s eight cases
// ---------------------------------------------------------------------------
//
// Each case is thin. It sets a few descriptors up, runs the verb's **script
// word** out of the table at +0x18 (third field of the entry), and reads one
// value back:
//
// ```text
// 0  nothing special — show the description   1  not finished, call me again
// 2  finished, and put the text back          3  finished
// ```
//
// The words are `CALCTAKE`, `CALCEXAMINE`, `CALCHANDLE`, `CALCUSE`,
// `CALCTALK`, `CALCINFO`, `CALCGIVE` and `CALCLEAVE` in module 5, and each of
// them asks the location's own hook first (`_LC_EXAMINE` and friends, set by
// every location macro and cleared by `INCLLOC`) before falling back on a
// global in module 13.
//
// Code 1 is what `CALLDIR` answers while it is still turning the figure
// towards the thing, and the bit it sets at +0x11C is what brings the order
// back on the next frame.

/// What a verb's script word answered, in the shape the cases branch on.
enum Said {
    Show,
    Again,
    Reset,
    Done,
}

fn said(code: i32) -> Said {
    match code {
        0 => Said::Show,
        1 => Said::Again,
        2 => Said::Reset,
        // 3 has a comparison of its own in the handler and an empty body, so
        // it lands where everything unrecognized lands.
        _ => Said::Done,
    }
}

/// Bit 0 of +0x11C says "an order is unfinished". Setting it is a byte
/// operation in the original and clearing it a 32-bit one, which also wipes
/// whatever the upper bits held — mirrored, because it is what the game runs.
fn keep_going<M>(vm: &mut M, order: u32, unfinished: bool) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let v = get(vm, order, AFTER)?;
    put(vm, order, AFTER, if unfinished { v | 1 } else { v & 0xfe })
}

/// Runs the verb's script word with the arguments the case pushes, and takes
/// the one value it answers with.
fn ask_script<M>(
    eng: &mut Engine,
    vm: &mut M,
    order: u32,
    slot: u32,
    args: &[i32],
) -> Result<Option<i32>>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    if get(vm, order, slot)? == 0 {
        return Ok(None);
    }
    eng.order_callback(vm, order, slot, args)?;
    Ok(Some(vm.data().pop().unwrap_or(0)))
}

/// `TEXTTOPERSON` (0x7b053): hang a text over the figure's head.
///
/// Center and foot come from the actor's own descriptor — `person[0x190]`,
/// the same one the walk drives — and the text sits 0x14 above it. The hook
/// at +0x168 runs afterwards with nothing passed and nothing taken.
fn text_to_person<M>(eng: &mut Engine, vm: &mut M, order: u32, desc: i32) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let person = get(vm, order, TASK)? as u32;
    let screen = get(vm, order, SCREEN)?;
    eng.select_screen(screen as u32);
    let actor = get(vm, person, 0x190)?;
    eng.select_descriptor(actor as u32);
    let x = eng.descriptor_center_x();
    let y = eng.descriptor_y() - 0x14;
    eng.select_descriptor(desc as u32);
    eng.place_x(x, Placement::Center)?;
    eng.place_y(y, Placement::Center)?;
    if get(vm, order, PLACED)? != 0 {
        eng.order_callback(vm, order, PLACED, &[])?;
    }
    Ok(())
}

/// The description a thing carries: an item keeps it at +0x0C, an area at
/// +0x14, and a thing with none falls back on the block's spare text.
fn description<M>(vm: &M, order: u32, target: i32) -> Result<i32>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let text = if target < BIAS {
        item(vm, order, target, I_INFO)?
    } else {
        area(vm, order, target, A_INFO)?
    };
    match text {
        0 => get(vm, order, INFO_SPARE),
        t => Ok(t),
    }
}

/// Puts the description descriptor back the way the block describes it.
fn info_descriptor<M>(eng: &mut Engine, vm: &mut M, order: u32) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let screen = get(vm, order, SCREEN)?;
    eng.select_screen(screen as u32);
    let desc = get(vm, order, INFO_DESC)?;
    eng.select_descriptor(desc as u32);
    let color = get(vm, order, INFO_COLOR)?;
    eng.set_color(color)?;
    let font = get(vm, order, INFO_FONT)?;
    eng.set_template(font)
}

/// Verb 2 (0x7c111), "look at" — and, with another handler slot and another
/// refusal text, verbs 3 and 4 as well. The three cases are the same shape.
fn examine<M>(
    eng: &mut Engine,
    vm: &mut M,
    order: u32,
    slot: u32,
    refusal: u32,
    two: bool,
) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let target = get(vm, order, TARGET)?;
    // Verb 2 dresses the descriptor before asking; 3 and 4 only afterwards.
    let up_front = slot == 0x2c;
    if up_front {
        info_descriptor(eng, vm, order)?;
        let text = description(vm, order, target)?;
        let block = get(vm, order, INFO_BLOCK)?;
        // The handler reads the descriptor's current block and line here and
        // never looks at either again — two dead stores, left out.
        if get(vm, order, AFTER)? & 1 == 0 {
            eng.set_text_table(block)?;
            eng.set_text(text)?;
        }
    }

    let args: Vec<i32> = if two {
        vec![target, get(vm, order, OBJECT)?]
    } else {
        vec![target]
    };
    let answer = ask_script(eng, vm, order, slot, &args)?;

    let mut unfinished = false;
    let mut blank = false;
    if let Some(code) = answer {
        match said(code) {
            Said::Show => {
                let desc = get(vm, order, INFO_DESC)?;
                eng.select_descriptor(desc as u32);
                eng.set_active(true);
                if !up_front {
                    let color = get(vm, order, INFO_COLOR)?;
                    eng.set_color(color)?;
                    let font = get(vm, order, INFO_FONT)?;
                    eng.set_template(font)?;
                }
            }
            Said::Again => unfinished = true,
            Said::Reset => {
                blank = up_front;
                let screen = get(vm, order, SCREEN)?;
                eng.select_screen(screen as u32);
                let desc = get(vm, order, INFO_DESC)?;
                eng.select_descriptor(desc as u32);
                if up_front {
                    eng.set_active(true);
                }
                let block = get(vm, order, INFO_BLOCK)?;
                eng.set_text_table(block)?;
                // Verb 2 falls back on the spare text, 3 and 4 each have their
                // own way of saying no.
                let text = get(vm, order, if up_front { INFO_SPARE } else { refusal })?;
                eng.set_text(text)?;
                eng.set_active(true);
                let color = get(vm, order, INFO_COLOR)?;
                eng.set_color(color)?;
                let font = get(vm, order, INFO_FONT)?;
                eng.set_template(font)?;
            }
            Said::Done => {}
        }
    }

    keep_going(vm, order, unfinished)?;
    if blank {
        // Block 2 line 1 is the empty text — the same pair `FT1` watches for.
        let block = get(vm, order, INFO_BLOCK)?;
        eng.set_text_table(block)?;
        eng.set_text(1)?;
    }
    let desc = get(vm, order, INFO_DESC)?;
    text_to_person(eng, vm, order, desc)
}

/// Verb 1 (0x7c00c), "take": the thing in hand goes back and the new one
/// comes in. This is the only case that publishes its state, at +0x1DC, and
/// the menu's modes 6 and 7 read it back.
fn take<M>(eng: &mut Engine, vm: &mut M, order: u32, rules: Rules) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let target = get(vm, order, TARGET)?;
    let answer = ask_script(eng, vm, order, VERBS + HANDLER, &[target])?;
    let state = match answer.map(said) {
        Some(Said::Again) => 2,
        Some(Said::Reset) => 0,
        _ => 1,
    };
    if state == 1 {
        let (held, list) = (get(vm, order, HELD)?, get(vm, order, INVENTORY)?);
        crate::words::remove_from_inventory(vm.space_mut(), held, list)?;
        crate::words::add_to_inventory(vm.space_mut(), target, list, rules.inventory)?;
        // The handler bumps the bar's scroll cell itself, on top of whatever
        // `ADDTOINV` may already have set it to.
        let inv = list as u32;
        let scroll = get(vm, inv, 0)?;
        put(vm, inv, 0, scroll + 1)?;
        eng.order_callback(vm, order, CALCINV, &[])?;
    }
    keep_going(vm, order, state == 2)?;
    put(vm, order, STARTED, state)
}

/// Verb 8 (0x7cd43), "leave": run the word and clear the unfinished bit. Two
/// lines in the original, and the whole of the player-driven scene change.
fn leave<M>(eng: &mut Engine, vm: &mut M, order: u32) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    if get(vm, order, VERBS + 7 * ENTRY + HANDLER)? != 0 {
        eng.order_callback(vm, order, VERBS + 7 * ENTRY + HANDLER, &[])?;
    }
    keep_going(vm, order, false)
}

/// A 32-bit field of a record wherever it happens to start — what
/// `Engine::cell` reads on the 32-bit machine, for the packed conversation
/// records that begin mid-cell.
fn packed_cell<M: Machine>(vm: &M, base: u32, off: u32) -> Result<i32> {
    let mem = vm.space();
    let bytes = mem.read_bytes(mem.offset(base as i32, off as i32), 4)?;
    Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Slot `i` of an inventory list: four bytes past the head on either
/// machine, see [`crate::words::slot_address`].
fn list_slot<M: Machine>(vm: &M, list: i32, i: i32) -> Result<i32> {
    vm.space()
        .fetch_cell(crate::words::slot_address(vm.space(), list, i))
}

/// A name in module memory, as `CompareString` (0x11950) reads one: the bytes
/// up to the first NUL. Only an exact match counts — that routine also answers
/// −2 and −3 for a prefix or a suffix, and both verbs test for −1 alone.
fn name_of<M>(vm: &M, at: u32) -> Result<String>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let mut name = String::new();
    let mem = vm.space();
    for i in 0..64i32 {
        match mem.fetch_byte(mem.offset(at as i32, i))? {
            0 => break,
            b => name.push(b as char),
        }
    }
    Ok(name)
}

/// Verbs 6 (INFO, 0x7c7e8) and 7 (GIVE, 0x7ca7f): step into a conversation at
/// the answer that carries a given name.
///
/// The two are the same routine three substitutions apart. Each asks its own
/// script word — `CALCINFO` or `CALCGIVE`, and through them the location's
/// `DO_INFO`/`DO_GIVE` — which answers with the **address of a name**, the
/// thing `_PutStringAdr` pushes: `"GDISK1"`, `"GANRUFBA"`, and as a last
/// resort `"DINFO"`/`"DGIVE"`. That name is then looked for among the
/// conversation's answers, and the one that matches becomes the node.
///
/// Three passes, and the third always succeeds by decree:
///
/// 1. the keyword — the word's answer if it gave one, otherwise the field's
///    own value, which in the shipped game points nowhere and therefore
///    matches nothing;
/// 2. the literal `"DINFO"` / `"DGIVE"`;
/// 3. the game complains on its diagnostic channel, settles for **answer 0**,
///    and looks once more with the *field's* keyword — throwing away whatever
///    the word had said. So the node is never less than 1000, and the check
///    the handler makes for that is dead code.
fn by_keyword<M>(
    eng: &mut Engine,
    vm: &mut M,
    order: u32,
    slot: u32,
    keyword: u32,
    literal: &str,
) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let record = get(vm, order, RECORD)? as u32;
    let fields = get(vm, order, FIELDS)? as u32;
    let mut key = get(vm, order, keyword)? as u32;

    let target = get(vm, order, TARGET)?;
    if let Some(answered) = ask_script(eng, vm, order, slot, &[target])?
        && answered != 0
    {
        key = answered as u32;
    }

    // The record's own fields are read byte-exact like everything else in a
    // conversation; the packed strides make cell reads a trap here. The
    // answer stride and the name's place in it are the 32-bit record's, the
    // only one read — the 16-bit answer table is the conversation machine's,
    // which stops before this on that machine.
    let count = packed_cell(vm, record, 8)?;
    let look = |vm: &M, want: &str| -> Result<Option<i32>> {
        for i in 0..count {
            let at = vm
                .space()
                .offset(fields as i32, (i as u32 * ANSWER + A_NAME) as i32)
                as u32;
            if name_of(vm, at)? == want {
                return Ok(Some(1000 + i));
            }
        }
        Ok(None)
    };

    let wanted = name_of(vm, key)?;
    let mut node = look(vm, &wanted)?;
    if node.is_none() {
        node = look(vm, literal)?;
    }
    let node = match node {
        Some(n) => n,
        None => {
            let field_key = get(vm, order, keyword)? as u32;
            let again = name_of(vm, field_key)?;
            look(vm, &again)?.unwrap_or(1000)
        }
    };

    put(vm, order, NODE, node)?;
    let screen = get(vm, order, SCREEN)?;
    eng.select_screen(screen as u32);
    let base = get(vm, order, ANSWER_DESCS)?;
    for i in 0..4 {
        eng.select_descriptor((base + i) as u32);
        eng.set_active(false);
    }
    eng.hide_pointer();
    eng.conversation_enter(vm, order, 1)
}

/// One verb on one target — the body of `EXECORDER` past its jump table.
pub(crate) fn exec_verb<M>(
    eng: &mut Engine,
    vm: &mut M,
    order: u32,
    verb: i32,
    rules: Rules,
) -> Result<bool>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let slot = |n: i32| VERBS + (n - 1) as u32 * ENTRY + HANDLER;
    match verb {
        1 => take(eng, vm, order, rules)?,
        2 => examine(eng, vm, order, slot(2), INFO_SPARE, false)?,
        3 => examine(eng, vm, order, slot(3), NO_HANDLE, false)?,
        4 => examine(eng, vm, order, slot(4), NO_USE, true)?,
        6 => by_keyword(eng, vm, order, slot(6), INFO_KEY, "DINFO")?,
        7 => by_keyword(eng, vm, order, slot(7), GIVE_KEY, "DGIVE")?,
        8 => leave(eng, vm, order)?,
        _ => return Ok(false),
    }
    Ok(true)
}
