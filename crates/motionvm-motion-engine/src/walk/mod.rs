//! Walking: `DOWALK` (0x787ec) and the route planner `CROUTE` (0x7780c).
//!
//! A figure is an ordinary sprite descriptor. What moves it is a **command
//! queue** the scripts write and `DOWALK` consumes — and not one command per
//! call, however natural that reading looks; taken that way, the figures
//! stand still. The handler opens with
//!
//! ```text
//! 0x78881  cmpl $0,0x1C8(%eax)
//! 0x7888b  jne  0x78E14          ; walking already: step, fetch nothing
//! ```
//!
//! so the queue is only touched between walks. A `1002` fills a buffer of
//! ready-made steps through `CROUTE` and opens that gate; every call after it
//! spends one step, and the gate closes on arrival.
//!
//! The step buffer is why nothing here interpolates while it draws: the
//! stepper only replays what the planner already worked out.
//!
//! ```text
//! route   (36 B, after a 4 B prefix)  x0 y0 x1 y1, then 5 neighbors, −1 ends
//! extra   (24 B)                      z, kind, scale₀, scale₁, step size, flags
//! step    (24 B)                      x, y, z (−1 = end), scale, direction, route
//! ```
//!
//! Measured against the park: `_ROUTE` holds a prefix of 9 and a first record
//! `(−80,311)-(198,323)` — Karsten's spawn corner — with neighbors 1 and 2;
//! `_XROUTE` gives every route `z 48, step size 12` and the uphill one the
//! scale ramp 900→800. His queue reads `1002 288 −1 3 | 1003 2 | 1006 2 | 999`.
//!
//! **Both engines.** The addresses above are `ENGINE.EXE`'s, and so are the
//! record layouts: every offset in this file is a byte offset of the 32-bit
//! layout, as `DEF_KARSTEN` writes it. The 16-bit handlers — `DOWALK` at
//! file `0xf25f` of `ENVIRO.EXE`, `CROUTE` at `0xe6d0`, the step, turn and
//! route helpers around them — are the same code compiled for 2-byte cells:
//! the same records with every field at half the offset, the same command
//! numbers, the same turn table, the same route search. So the offsets are
//! scaled by the machine's cell in one place ([`at`]) and the rest reads
//! once for both. Where the 16-bit code does its arithmetic in 16 bits and
//! the 32-bit code in 32 — a product before a division in `on_line` and
//! `shrink_along`, both in [`planner`] — [`narrow`] says so.

use crate::{Engine, Placement};
use motionvm_motion_forth::cell;
use motionvm_motion_forth::{AddressSpace, Error, Result};

// The person record, in module memory. Offsets from `DEF_KARSTEN`, which
// writes every one of them in decimal.
/// Walk-cycle pair per heading. In list mode the first is a handle to a
/// zero-terminated array of sprite ids and the second the index into it;
/// otherwise they are the first and last id of a run.
const P_CYCLE: [(u32, u32); 9] = [
    (0x00, 0x04), // 0 is drawn as 3
    (0x18, 0x1c),
    (0x10, 0x14),
    (0x00, 0x04),
    (0x08, 0x0c),
    (0x30, 0x34),
    (0x38, 0x3c),
    (0x28, 0x2c),
    (0x20, 0x24),
];
/// Standing sprite per heading — the jump table at 0x78b9b, read out of the
/// image. Not a stride: 0 and 3 share a slot.
const P_STANDING: [u32; 9] = [0x80, 0x88, 0x8c, 0x80, 0x84, 0x98, 0x9c, 0x94, 0x90];
const P_SCALE: u32 = 0xa8;
const P_ZBIAS: u32 = 0xac;
const P_DESC: u32 = 0x190;
const P_SCREEN: u32 = 0x194;
const P_HEADING: u32 = 0x198;
const P_TURN: u32 = 0x19c;
const P_AT: u32 = 0x1a0;
const P_STEPS: u32 = 0x1a4;
const P_WALKING: u32 = 0x1a8;
const P_SHADOW: u32 = 0x1ac;
const P_ROUTES: u32 = 0x1b4;
const P_AUX: u32 = 0x1b8;
const P_ROOM: u32 = 0x1bc;
const P_QUEUE: u32 = 0x1c0;
const P_CURSOR: u32 = 0x1c4;
const P_GATE: u32 = 0x1c8;
const P_COMMAND: u32 = 0x1cc;
const P_LISTMODE: u32 = 0x1d0;

// The shadow record, `person[0x1ac]`: what the figure is doing right now.
const S_ROUTE: u32 = 0x04;
const S_DEST_ROUTE: u32 = 0x08;
const S_DEST_X: u32 = 0x18;
const S_DEST_Y: u32 = 0x1c;
const S_EIGHT: u32 = 0x20;
const S_X: u32 = 0x24;
const S_Y: u32 = 0x28;
const S_HEADING: u32 = 0x2c;
const S_SPEED_X: u32 = 0x30;
const S_SPEED_Y: u32 = 0x34;
const S_SHRINK: u32 = 0x38;
const S_Z: u32 = 0x3c;

// One generated step.
const STEP: u32 = 0x18;
const T_X: u32 = 0x00;
const T_Y: u32 = 0x04;
const T_Z: u32 = 0x08;
const T_SHRINK: u32 = 0x0c;
const T_HEADING: u32 = 0x10;
const T_ROUTE: u32 = 0x14;

// A route record and its companion in the extended table.
const ROUTE: u32 = 36;
const AUX: u32 = 24;
const A_Z: u32 = 0x00;
const A_KIND: u32 = 0x04;
const A_SHRINK0: u32 = 0x08;
const A_SHRINK1: u32 = 0x0c;
const A_STEP: u32 = 0x10;
const A_FLAGS: u32 = 0x14;

/// Which way round the ring to turn, `0xdbc38` — rows are the heading the
/// figure has, columns the one it wants. 1 walks the ring
/// 3-8-2-7-4-6-1-5, 2 the other way; both are always the shorter arc.
#[rustfmt::skip]
const TURN_WAY: [[i32; 8]; 8] = [
    [0, 1, 1, 2, 1, 2, 2, 1],
    [2, 0, 2, 1, 2, 1, 1, 2],
    [2, 1, 0, 1, 2, 2, 1, 1],
    [1, 2, 1, 0, 1, 1, 2, 2],
    [2, 1, 1, 2, 0, 2, 2, 1],
    [1, 2, 1, 2, 1, 0, 2, 1],
    [1, 2, 2, 1, 1, 1, 0, 2],
    [2, 1, 2, 1, 2, 2, 1, 0],
];

/// One ring step: where the heading goes next, and the turn frames for it.
/// Both rings use the same eight frame pairs, played first to last either way.
fn ring(way: i32, heading: i32) -> (u32, u32, i32) {
    let forward = [
        (3, 0x78, 8),
        (8, 0x70, 2),
        (2, 0x68, 7),
        (7, 0x60, 4),
        (4, 0x58, 6),
        (6, 0x50, 1),
        (1, 0x48, 5),
        (5, 0x40, 3),
    ];
    let table = if way == 1 {
        forward
    } else {
        [
            (3, 0x40, 5),
            (5, 0x48, 1),
            (1, 0x50, 6),
            (6, 0x58, 4),
            (4, 0x60, 7),
            (7, 0x68, 2),
            (2, 0x70, 8),
            (8, 0x78, 3),
        ]
    };
    let heading = if heading == 0 { 3 } else { heading };
    let (_, at, next) = table
        .into_iter()
        .find(|(h, _, _)| *h == heading)
        .unwrap_or((3, 0x78, 8));
    (at, at + 4, next)
}

/// A field's address on this machine: `off` is the 32-bit layout's byte
/// offset, and the field sits `off / 4` cells into the record on either.
fn at(mem: &dyn AddressSpace, base: u32, off: u32) -> i32 {
    mem.offset(cell::signed(base), cell::signed(off / 4) * mem.cell_size())
}

/// A product the 16-bit engine forms in 16 bits — `mul` then `cwtd` — where
/// the 32-bit engine has 32; on a 2-byte-cell machine it wraps the same way.
fn narrow(mem: &dyn AddressSpace, v: i32) -> i32 {
    if mem.cell_size() == 2 {
        i32::from(cell::short(v))
    } else {
        v
    }
}

fn get(mem: &dyn AddressSpace, base: u32, off: u32) -> Result<i32> {
    mem.fetch_cell(at(mem, base, off))
}

fn put(mem: &mut dyn AddressSpace, base: u32, off: u32, v: i32) -> Result<()> {
    mem.store_cell(at(mem, base, off), v)
}

/// A cell of the command queue, which is addressed in cells throughout.
fn queued(mem: &dyn AddressSpace, queue: u32, cursor: i32) -> Result<i32> {
    get(mem, queue, cell::unsigned(cursor) * 4)
}

/// A field of route `n`. The 4-byte prefix — the record count — is never read
/// by the original; every access is `base + 4 + 36*n`.
fn route(mem: &dyn AddressSpace, table: u32, n: i32, f: u32) -> Result<i32> {
    if n < 0 {
        return Err(Error::Unread {
            what: format!(
                "CROUTE: route record {n} — the original indexes here unchecked, and \
                 with connected routes this cannot arise"
            ),
            binary: "ENGINE.EXE",
            at: "0x76dcc, 0x77a2b",
        });
    }
    get(mem, table, 4 + cell::unsigned(n) * ROUTE + f)
}

fn aux(mem: &dyn AddressSpace, table: u32, n: i32, f: u32) -> Result<i32> {
    if n < 0 {
        return Err(Error::Unread {
            what: format!("CROUTE: auxiliary record {n} — as for the route record"),
            binary: "ENGINE.EXE",
            at: "0x76dcc, 0x77a2b",
        });
    }
    get(mem, table, cell::unsigned(n) * AUX + f)
}

fn step_at(mem: &dyn AddressSpace, steps: u32, i: i32, f: u32) -> Result<i32> {
    get(mem, steps, cell::unsigned(i) * STEP + f)
}

fn set_step(mem: &mut dyn AddressSpace, steps: u32, i: i32, f: u32, v: i32) -> Result<()> {
    put(mem, steps, cell::unsigned(i) * STEP + f, v)
}

/// The original divides without checking. Where the data cannot make the
/// divisor zero this never fires; if it does, the reading is wrong and saying
/// so is worth more than a quietly clamped result.
fn div(a: i32, b: i32, at: &str) -> Result<i32> {
    match b {
        0 => Err(Error::Unsupported(format!(
            "CROUTE: division by zero in {at}"
        ))),
        b => Ok(a / b),
    }
}

/// `DOWALK ( person -- )`, the handler at 0x787ec.
pub(crate) fn do_walk(eng: &mut Engine, mem: &mut dyn AddressSpace, person: u32) -> Result<()> {
    let desc = get(mem, person, P_DESC)?;
    let screen = get(mem, person, P_SCREEN)?;
    eng.select_descriptor(cell::unsigned(desc));
    eng.select_screen(cell::unsigned(screen));

    // 0x78881. Everything below the gate belongs to a walk in progress.
    if get(mem, person, P_GATE)? == 0 {
        dispatch(eng, mem, person)?;
    }
    stepper(eng, mem, person)
}

/// Takes the next command off the queue, 0x78891 to 0x78e14.
fn dispatch(eng: &mut Engine, mem: &mut dyn AddressSpace, person: u32) -> Result<()> {
    let queue = cell::unsigned(get(mem, person, P_QUEUE)?);
    let shadow = cell::unsigned(get(mem, person, P_SHADOW)?);

    let mut cursor = get(mem, person, P_CURSOR)?;
    let mut command = queued(mem, queue, cursor)?;
    put(mem, person, P_COMMAND, command)?;
    cursor += 1;
    put(mem, person, P_CURSOR, cursor)?;

    // 1001, the figure's appearance — `PINITFIG` sends one before anything
    // else. It is the only command that reads the *next* one in the same call
    // (0x789d4), which is how `1001 … 999` gets worked off in a single pump.
    // The cursor is left standing on that next command rather than past it —
    // the original's own off-by-one, invisible because only a `999` ever
    // follows, and `999` reads no operands.
    if command == 1001 {
        let shrink = queued(mem, queue, cursor + 3)?;
        put(mem, shadow, S_SHRINK, shrink)?;
        eng.set_shrink(shrink);
        let x = queued(mem, queue, cursor)?;
        put(mem, shadow, S_X, x)?;
        eng.place_x(x, Placement::Center)?;
        let y = queued(mem, queue, cursor + 1)?;
        put(mem, shadow, S_Y, y)?;
        eng.place_y(y, Placement::FarEdge)?;
        let z = queued(mem, queue, cursor + 2)?;
        eng.set_level(z)?;
        let zbias = get(mem, person, P_ZBIAS)?;
        put(mem, shadow, S_Z, z + zbias)?;
        let at = queued(mem, queue, cursor + 4)?;
        put(mem, shadow, S_ROUTE, at)?;
        cursor += 5;
        put(mem, person, P_CURSOR, cursor)?;
        command = queued(mem, queue, cursor)?;
        put(mem, person, P_COMMAND, command)?;
    }

    match command {
        // 1002, walk somewhere. 0x78a01 first asks whether the figure is
        // already standing on that spot in that route; if it is, nothing is
        // planned and the gate stays shut — the queue simply goes on.
        1002 => {
            let (x, y, at) = (
                queued(mem, queue, cursor)?,
                queued(mem, queue, cursor + 1)?,
                queued(mem, queue, cursor + 2)?,
            );
            let there = get(mem, shadow, S_X)? == x
                && get(mem, shadow, S_Y)? == y
                && get(mem, shadow, S_ROUTE)? == at;
            if !there {
                put(mem, shadow, S_DEST_X, x)?;
                put(mem, shadow, S_DEST_Y, y)?;
                put(mem, shadow, S_DEST_ROUTE, at)?;
                croute(eng, mem, person)?;
                put(mem, person, P_AT, 1)?;
                put(mem, person, P_TURN, -1)?;
                put(mem, person, P_GATE, 1)?;
            }
            put(mem, person, P_CURSOR, cursor + 3)?;
        }
        // 1003, turn on the spot: a heading to reach, and the gate open so the
        // stepper does the turning (0x78b46).
        1003 => {
            let turn = queued(mem, queue, cursor)?;
            put(mem, person, P_TURN, turn)?;
            put(mem, person, P_CURSOR, cursor + 1)?;
            put(mem, person, P_GATE, 1)?;
        }
        // 1006, face a way without moving (0x78bbf): the standing sprite, and
        // the position read back off the descriptor and written straight back,
        // so only the picture changes.
        1006 => {
            let x = eng.descriptor_center_x();
            let y = eng.descriptor_far_y();
            let heading = queued(mem, queue, cursor)?;
            put(mem, person, P_HEADING, heading)?;
            put(mem, person, P_CURSOR, cursor + 1)?;
            put(mem, shadow, S_HEADING, heading)?;
            if let Some(&slot) = usize::try_from(heading)
                .ok()
                .and_then(|h| P_STANDING.get(h))
            {
                let sprite = get(mem, person, slot)?;
                if sprite != 0 {
                    eng.set_sprite(sprite)?;
                }
            }
            // No 0 → 1000 here, unlike the turn and the step: both binaries
            // multiply the shadow's size as it stands (`ENGINE.EXE` 0x78d7f,
            // `ENVIRO.EXE` file `0xf6ec`).
            let shrink = get(mem, shadow, S_SHRINK)? * get(mem, person, P_SCALE)? / 10;
            eng.set_shrink(shrink);
            eng.place_x(x, Placement::Center)?;
            eng.place_y(y, Placement::FarEdge)?;
        }
        // 1007 hands the queue a script word and runs it there and then
        // (0x78dc8, through the interpreter entry 0x60c65). No queue any
        // script writes carries one, so the path has never been exercised and
        // running bytecode from inside a primitive needs the interpreter this
        // handler does not get.
        1007 => {
            return Err(Error::Unread {
                what: "DOWALK: command 1007 runs a script word — no queue in the game \
                       writes one, so the path to it was never built"
                    .into(),
                binary: "ENGINE.EXE",
                at: "0x78dc8",
            });
        }
        // 999 ends the queue. Clearing the flag is what stops `WALKKARSTEN`
        // calling `DOWALK` again; the cursor is deliberately left past the end.
        999 => put(mem, person, P_WALKING, 0)?,
        _ => {}
    }
    Ok(())
}

/// `wert * person[0xa8] / 10` — the figure's own scale, ten meaning full size.
fn shrink_of(mem: &dyn AddressSpace, person: u32, raw: i32) -> Result<i32> {
    let raw = if raw == 0 { 1000 } else { raw };
    Ok(raw * get(mem, person, P_SCALE)? / 10)
}

/// The stepper, 0x78e14 to 0x798d3: one tick of a walk or a turn.
fn stepper(eng: &mut Engine, mem: &mut dyn AddressSpace, person: u32) -> Result<()> {
    if get(mem, person, P_GATE)? == 0 {
        return Ok(());
    }
    let command = get(mem, person, P_COMMAND)?;
    let turning = (command == 1002 && get(mem, person, P_TURN)? != -1) || command == 1003;
    if turning {
        turn(eng, mem, person)
    } else if command == 1002 {
        advance(eng, mem, person)
    } else {
        Ok(())
    }
}

/// The turn machinery, 0x78e52 to 0x792c4.
fn turn(eng: &mut Engine, mem: &mut dyn AddressSpace, person: u32) -> Result<()> {
    let shadow = cell::unsigned(get(mem, person, P_SHADOW)?);
    let want = get(mem, person, P_TURN)?;
    let heading = get(mem, person, P_HEADING)?;
    let done = |mem: &mut dyn AddressSpace| -> Result<()> {
        put(mem, person, P_TURN, -1)?;
        if get(mem, person, P_COMMAND)? == 1003 {
            put(mem, person, P_GATE, 0)?;
        }
        Ok(())
    };
    if want == -1 || want == heading {
        return done(mem);
    }
    if heading == 0 {
        put(mem, person, P_HEADING, 3)?;
    }
    let heading = get(mem, person, P_HEADING)?;

    // A plain heading is first turned into a direction of travel around the
    // ring, and nothing is drawn on that tick (0x78e90).
    if want < 100 {
        let want = if want == 0 { 3 } else { want };
        put(mem, person, P_TURN, want)?;
        let (Ok(row), Ok(col)) = (usize::try_from(heading - 1), usize::try_from(want - 1)) else {
            return done(mem);
        };
        let Some(way) = TURN_WAY.get(row).and_then(|r| r.get(col)) else {
            return done(mem);
        };
        return put(mem, person, P_TURN, want + 100 * way);
    }
    let way = match want {
        101..=199 => 1,
        201..=299 => 2,
        _ => return done(mem),
    };

    let (first, last, next) = ring(way, heading);
    let (first, last) = (get(mem, person, first)?, get(mem, person, last)?);
    turn_frame(eng, mem, person, shadow, first, last, next)?;

    // 0x79270: the digit under the ring encoding is the heading wanted, and
    // the frame above may just have committed it.
    if want % 100 == get(mem, person, P_HEADING)? {
        return done(mem);
    }
    Ok(())
}

/// One turn frame, 0x786bd. With no frames for the transition the heading is
/// simply taken; otherwise the pair is played through and the heading lands on
/// the tick that shows the last frame.
fn turn_frame(
    eng: &mut Engine,
    mem: &mut dyn AddressSpace,
    person: u32,
    shadow: u32,
    first: i32,
    last: i32,
    next: i32,
) -> Result<()> {
    if first == 0 {
        put(mem, person, P_HEADING, next)?;
        return put(mem, shadow, S_HEADING, next);
    }
    let mut frame = eng.descriptor_sprite();
    if frame < first || frame > last {
        frame = first;
    } else if frame < last {
        frame += 1;
    }
    if frame >= last {
        put(mem, person, P_HEADING, next)?;
        put(mem, shadow, S_HEADING, next)?;
    }
    eng.set_sprite(frame)?;
    let shrink = shrink_of(mem, person, get(mem, shadow, S_SHRINK)?)?;
    eng.set_shrink(shrink);
    let (x, y, z) = (
        get(mem, shadow, S_X)?,
        get(mem, shadow, S_Y)?,
        get(mem, shadow, S_Z)?,
    );
    eng.place_x(x, Placement::Center)?;
    eng.place_y(y, Placement::FarEdge)?;
    eng.set_level(z)
}

/// One walk step, 0x792c9 to 0x798d3.
fn advance(eng: &mut Engine, mem: &mut dyn AddressSpace, person: u32) -> Result<()> {
    let shadow = cell::unsigned(get(mem, person, P_SHADOW)?);
    let steps = cell::unsigned(get(mem, person, P_STEPS)?);
    let mut at = get(mem, person, P_AT)?;

    // The planner marks the end of what it worked out with a Z of −1.
    if at >= 0 && step_at(mem, steps, at, T_Z)? == -1 {
        at = -1;
        put(mem, person, P_AT, -1)?;
    }
    if at == -1 {
        return put(mem, person, P_GATE, 0);
    }

    let wants = step_at(mem, steps, at, T_HEADING)?;
    let heading = get(mem, person, P_HEADING)?;
    if wants != heading && wants != -1 {
        // 0x797c8: the step needs another heading, so this tick turns instead
        // of moving. In list mode the new heading's cycle starts over.
        if get(mem, person, P_LISTMODE)? & 1 == 1 {
            let h = if wants == 0 { 3 } else { wants };
            if let Some(&(_, index)) = usize::try_from(h).ok().and_then(|h| P_CYCLE.get(h)) {
                put(mem, person, index, 0)?;
            }
        }
        if wants == 0 {
            set_step(mem, steps, at, T_HEADING, 3)?;
        }
        let heading = step_at(mem, steps, at, T_HEADING)?;
        put(mem, person, P_TURN, heading)?;
        return put(mem, person, P_WALKING, 1);
    }

    let h = if heading == 0 { 3 } else { heading };
    let Some(&(a, b)) = usize::try_from(h).ok().and_then(|h| P_CYCLE.get(h)) else {
        // 0x795d2 shouts at the programmer and waits for a key. There is
        // nothing to mirror in that; a heading outside 0..8 is a data fault.
        return Err(Error::Unsupported(format!(
            "DOWALK: heading {heading} has no walk cycle (0x795d2)"
        )));
    };

    let frame = if get(mem, person, P_LISTMODE)? & 1 == 0 {
        // The pair is the first and last id of a run of sprites.
        let (first, last) = (get(mem, person, a)?, get(mem, person, b)?);
        let now = eng.descriptor_sprite();
        if now >= first && now + eng.step_multi <= last {
            now + eng.step_multi
        } else {
            first
        }
    } else {
        // The pair is a zero-terminated list of ids and an index into it. The
        // frame shown is the one the index stands on *before* it moves.
        let list = cell::unsigned(get(mem, person, a)?);
        let mut index = get(mem, person, b)?;
        let frame = get(mem, list, cell::unsigned(index) * 4)?;
        for _ in 0..eng.step_multi.max(1) {
            index += 1;
            if get(mem, list, cell::unsigned(index) * 4)? <= 0 {
                index = 0;
            }
        }
        put(mem, person, b, index)?;
        frame
    };
    eng.set_sprite(frame)?;

    let shrink = shrink_of(mem, person, step_at(mem, steps, at, T_SHRINK)?)?;
    eng.set_shrink(shrink);
    let kept = match step_at(mem, steps, at, T_SHRINK)? {
        v if v <= 0 => 1000,
        v => v,
    };
    put(mem, shadow, S_SHRINK, kept)?;

    let x = step_at(mem, steps, at, T_X)?;
    eng.place_x(x, Placement::Center)?;
    put(mem, shadow, S_X, x)?;
    let y = step_at(mem, steps, at, T_Y)?;
    eng.place_y(y, Placement::FarEdge)?;
    put(mem, shadow, S_Y, y)?;
    let z = step_at(mem, steps, at, T_Z)? + get(mem, person, P_ZBIAS)?;
    eng.set_level(z)?;
    put(mem, shadow, S_Z, z)?;
    let route_of_step = step_at(mem, steps, at, T_ROUTE)?;
    put(mem, shadow, S_ROUTE, route_of_step)?;

    at += 1;
    put(mem, person, P_AT, at)?;
    // Out of ready-made steps: plan the next stretch and go on from its first.
    if at >= get(mem, person, P_ROOM)? {
        croute(eng, mem, person)?;
        put(mem, person, P_AT, 1)?;
    }
    put(mem, person, P_WALKING, 1)
}

mod planner;

pub(crate) use planner::{croute, croute_with};
