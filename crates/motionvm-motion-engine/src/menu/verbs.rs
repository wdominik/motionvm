//! `EXECORDER`'s eight cases: the verbs the strip beside it picks.
//!
//! Split from the strip because the two are different subjects. [`super`]
//! decides *which* verb the player asked for — the mask, the icons, the
//! click — and stops there; this runs the one that was chosen, and every
//! case of it is a script word out of the verb table with one value read
//! back.
//!
//! The records both halves read — the area, the item, the verb entry — and
//! the readers that take a field out of one are the parent module's.

use super::{A_INFO, BIAS, ENTRY, I_INFO, area, item};
use crate::order::block::{
    A_NAME, AFTER, ANSWER, ANSWER_DESCS, CALCINV, FIELDS, GIVE_KEY, HANDLER, HELD, INFO_BLOCK,
    INFO_COLOR, INFO_DESC, INFO_FONT, INFO_KEY, INFO_SPARE, INVENTORY, NO_HANDLE, NO_USE, NODE,
    OBJECT, PLACED, RECORD, SCREEN, STARTED, TARGET, TASK, VERBS,
};
use crate::order::{Conversation, Rules, get, put};
use crate::{Engine, Placement};
use motionvm_motion_forth::cell;
use motionvm_motion_forth::{Host, Machine, Result};
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
    let person = cell::unsigned(get(vm, order, TASK)?);
    let screen = get(vm, order, SCREEN)?;
    eng.select_screen(cell::unsigned(screen));
    let actor = get(vm, person, 0x190)?;
    eng.select_descriptor(cell::unsigned(actor));
    let x = eng.descriptor_center_x();
    let y = eng.descriptor_y() - 0x14;
    eng.select_descriptor(cell::unsigned(desc));
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
fn info_descriptor<M>(eng: &mut Engine, vm: &M, order: u32) -> Result<()>
where
    M: Machine,
    Engine: Host<M> + Conversation<M>,
{
    let screen = get(vm, order, SCREEN)?;
    eng.select_screen(cell::unsigned(screen));
    let desc = get(vm, order, INFO_DESC)?;
    eng.select_descriptor(cell::unsigned(desc));
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
                eng.select_descriptor(cell::unsigned(desc));
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
                eng.select_screen(cell::unsigned(screen));
                let desc = get(vm, order, INFO_DESC)?;
                eng.select_descriptor(cell::unsigned(desc));
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
        let inv = cell::unsigned(list);
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
    let bytes = mem.read_bytes(mem.offset(cell::signed(base), cell::signed(off)), 4)?;
    Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
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
        match mem.fetch_byte(mem.offset(cell::signed(at), i))? {
            0 => break,
            b => name.push(char::from(b)),
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
    let record = cell::unsigned(get(vm, order, RECORD)?);
    let fields = cell::unsigned(get(vm, order, FIELDS)?);
    let mut key = cell::unsigned(get(vm, order, keyword)?);

    let target = get(vm, order, TARGET)?;
    if let Some(answered) = ask_script(eng, vm, order, slot, &[target])?
        && answered != 0
    {
        key = cell::unsigned(answered);
    }

    // The record's own fields are read byte-exact like everything else in a
    // conversation; the packed strides make cell reads a trap here. The
    // answer stride and the name's place in it are the 32-bit record's, the
    // only one read — the 16-bit answer table is the conversation machine's,
    // which stops before this on that machine.
    let count = packed_cell(vm, record, 8)?;
    let look = |vm: &M, want: &str| -> Result<Option<i32>> {
        for i in 0..count {
            let at = cell::unsigned(vm.space().offset(
                cell::signed(fields),
                cell::signed(cell::unsigned(i) * ANSWER + A_NAME),
            ));
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
            let field_key = cell::unsigned(get(vm, order, keyword)?);
            let again = name_of(vm, field_key)?;
            look(vm, &again)?.unwrap_or(1000)
        }
    };

    put(vm, order, NODE, node)?;
    let screen = get(vm, order, SCREEN)?;
    eng.select_screen(cell::unsigned(screen));
    let base = get(vm, order, ANSWER_DESCS)?;
    for i in 0..4 {
        eng.select_descriptor(cell::unsigned(base + i));
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
    let slot = |n: i32| VERBS + cell::unsigned(n - 1) * ENTRY + HANDLER;
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
