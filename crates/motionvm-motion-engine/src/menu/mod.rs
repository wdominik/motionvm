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

use crate::Engine;
use crate::order::block::{
    AREAS, BAR_MENU, BAR_SCREEN, IMX, IMY, INVENTORY, ITEMS, MMX, MMY, SCREEN, VERBS,
};
use crate::order::{Conversation, Rules, get};
use motionvm_motion_forth::cell;
use motionvm_motion_forth::{Host, Machine, Result};

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
    let areas = cell::unsigned(get(vm, order, AREAS)?);
    get(vm, areas, cell::unsigned(n - BIAS) * AREA + f)
}

fn item<M>(vm: &M, order: u32, n: i32, f: u32) -> Result<i32>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let items = cell::unsigned(get(vm, order, ITEMS)?);
    get(vm, items, cell::unsigned(n) * ITEM + f)
}

/// Slot `i` of an inventory list: four bytes past the head on either
/// machine, see [`crate::words::slot_address`].
fn list_slot<M: Machine>(vm: &M, list: i32, i: i32) -> Result<i32> {
    vm.space()
        .fetch_cell(crate::words::slot_address(vm.space(), list, i))
}

/// One of the bar's eight slots, with the pointer on it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BarSlot {
    /// Its index in the list, the scroll offset included.
    pub index: i32,
    /// The list's scroll offset — what the bar's first slot shows.
    pub scroll: i32,
    /// The item in it, or zero for an empty slot.
    pub item: i32,
}

/// The bar's slot under the pointer, or `None` off the eight.
///
/// The interval is half-open — a pointer on the bar's far edge is on no slot
/// — which is how the right click (0x7d0b3) and the conversation's item menu
/// (0x7e559) both have it; mode 3 closes it, and says so where it does.
pub(crate) fn bar_slot<M>(vm: &M, order: u32, rules: Rules) -> Result<Option<BarSlot>>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let imx = get(vm, order, IMX)?;
    if !(rules.bar_x0..rules.bar_x0 + 8 * rules.slot_w).contains(&imx) {
        return Ok(None);
    }
    let list = get(vm, order, INVENTORY)?;
    let scroll = get(vm, cell::unsigned(list), 0)?;
    let index = scroll + (imx - rules.bar_x0) / rules.slot_w;
    let item = list_slot(vm, list, index)?;
    Ok(Some(BarSlot {
        index,
        scroll,
        item,
    }))
}

/// The strip a slot of the bar gets: the bar's screen and its five
/// descriptors, centered over the slot, the rules' distance down.
pub(crate) fn bar_strip<M>(vm: &M, order: u32, slot: BarSlot, rules: Rules) -> Result<Strip>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    Ok(Strip {
        screen: get(vm, order, BAR_SCREEN)?,
        base: get(vm, order, BAR_MENU)?,
        x: (slot.index - slot.scroll) * rules.slot_w + rules.slot_w * 3 / 2,
        y: rules.bar_menu_y,
    })
}

/// Whether the pointer is inside the descriptor the caller has selected.
///
/// The bar and the scene are two screens with two pointers, and the box test
/// takes whichever belongs to the screen the menu is on.
fn on_slot<M>(eng: &mut Engine, vm: &M, order: u32, screen: i32) -> Result<bool>
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

mod modes;
mod strip;
mod verbs;

pub(crate) use modes::{figure_free, mode_chain, right_click, walk_done};
pub(crate) use strip::{Strip, animate, choose, highlight, remove_menu, show_menu};
pub(crate) use verbs::exec_verb;
