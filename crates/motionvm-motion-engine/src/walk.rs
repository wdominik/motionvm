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
//! the 32-bit code in 32 — a product before a division in [`on_line`] and
//! [`shrink_along`] — [`narrow`] says so.

use crate::{Engine, Placement};
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
    mem.offset(base as i32, (off / 4) as i32 * mem.cell_size())
}

/// A product the 16-bit engine forms in 16 bits — `mul` then `cwtd` — where
/// the 32-bit engine has 32; on a 2-byte-cell machine it wraps the same way.
fn narrow(mem: &dyn AddressSpace, v: i32) -> i32 {
    if mem.cell_size() == 2 {
        v as i16 as i32
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
    get(mem, queue, cursor as u32 * 4)
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
            at: "0x76dcc, 0x77a2b",
        });
    }
    get(mem, table, 4 + n as u32 * ROUTE + f)
}

fn aux(mem: &dyn AddressSpace, table: u32, n: i32, f: u32) -> Result<i32> {
    if n < 0 {
        return Err(Error::Unread {
            what: format!("CROUTE: auxiliary record {n} — as for the route record"),
            at: "0x76dcc, 0x77a2b",
        });
    }
    get(mem, table, n as u32 * AUX + f)
}

fn step_at(mem: &dyn AddressSpace, steps: u32, i: i32, f: u32) -> Result<i32> {
    get(mem, steps, i as u32 * STEP + f)
}

fn set_step(mem: &mut dyn AddressSpace, steps: u32, i: i32, f: u32, v: i32) -> Result<()> {
    put(mem, steps, i as u32 * STEP + f, v)
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
    eng.select_descriptor(desc as u32);
    eng.select_screen(screen as u32);

    // 0x78881. Everything below the gate belongs to a walk in progress.
    if get(mem, person, P_GATE)? == 0 {
        dispatch(eng, mem, person)?;
    }
    stepper(eng, mem, person)
}

/// Takes the next command off the queue, 0x78891 to 0x78e14.
fn dispatch(eng: &mut Engine, mem: &mut dyn AddressSpace, person: u32) -> Result<()> {
    let queue = get(mem, person, P_QUEUE)? as u32;
    let shadow = get(mem, person, P_SHADOW)? as u32;

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
    let shadow = get(mem, person, P_SHADOW)? as u32;
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
    let shadow = get(mem, person, P_SHADOW)? as u32;
    let steps = get(mem, person, P_STEPS)? as u32;
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
        let list = get(mem, person, a)? as u32;
        let mut index = get(mem, person, b)?;
        let frame = get(mem, list, index as u32 * 4)?;
        for _ in 0..eng.step_multi.max(1) {
            index += 1;
            if get(mem, list, index as u32 * 4)? <= 0 {
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

// ---------------------------------------------------------------------------
// The planner
// ---------------------------------------------------------------------------

/// Everything `CROUTE` keeps in globals while it lays out a path.
struct Plan {
    at: i32,
    hop: i32,
    after: i32,
    x: i32,
    y: i32,
    leg_x: i32,
    leg_y: i32,
    /// The waypoint, each axis set to −1 once it has been reached.
    to_x: i32,
    to_y: i32,
    /// The same point kept whole, for the direction and the step sizes.
    keep_x: i32,
    keep_y: i32,
}

/// `CROUTE` (0x7780c): fills the step buffer between here and the destination.
///
/// The 32-bit games and the later 16-bit ones reach it through `DOWALK`,
/// which hands over a person record; Victor Loomes has no `DOWALK` and calls
/// the kernel word itself with the same five pointers on the stack
/// (`0104:4a45` in `LL.EXE` pops aux, routes, shadow, steps and room in that
/// order). Both go through [`croute_with`].
pub(crate) fn croute(eng: &mut Engine, mem: &mut dyn AddressSpace, person: u32) -> Result<()> {
    let shadow = get(mem, person, P_SHADOW)? as u32;
    let steps = get(mem, person, P_STEPS)? as u32;
    let routes = get(mem, person, P_ROUTES)? as u32;
    let extra = get(mem, person, P_AUX)? as u32;
    let room = get(mem, person, P_ROOM)?;
    croute_with(eng, mem, shadow, steps, routes, extra, room)
}

/// The same, with the five pointers the person record would have held.
pub(crate) fn croute_with(
    eng: &mut Engine,
    mem: &mut dyn AddressSpace,
    shadow: u32,
    steps: u32,
    routes: u32,
    extra: u32,
    room: i32,
) -> Result<()> {
    let at = get(mem, shadow, S_ROUTE)? - 1;
    let (x, y) = (get(mem, shadow, S_X)?, get(mem, shadow, S_Y)?);
    set_step(mem, steps, 0, T_X, x)?;
    set_step(mem, steps, 0, T_Y, y)?;
    let z = aux(mem, extra, at, A_Z)?;
    set_step(mem, steps, 0, T_Z, z)?;
    let shrink = match get(mem, shadow, S_SHRINK)? {
        0 => 1000,
        v => v,
    };
    set_step(mem, steps, 0, T_SHRINK, shrink)?;
    set_step(mem, steps, 0, T_HEADING, -1)?;
    set_step(mem, steps, 0, T_ROUTE, at + 1)?;

    let goal = get(mem, shadow, S_DEST_ROUTE)? - 1;
    let (hop, after) = if goal == at {
        (-1, -1)
    } else {
        let hop = calc_route(mem, routes, goal, at)?;
        (
            hop,
            if goal == hop {
                -1
            } else {
                calc_route(mem, routes, goal, hop)?
            },
        )
    };

    let mut p = Plan {
        at,
        hop,
        after,
        x,
        y,
        leg_x: x,
        leg_y: y,
        to_x: 0,
        to_y: 0,
        keep_x: 0,
        keep_y: 0,
    };
    waypoint(mem, &mut p, routes, extra, shadow)?;

    let mut i = 1;
    while i < room {
        if p.to_x == -1 && p.to_y == -1 {
            if goal == p.at {
                break;
            }
            p.at = p.hop;
            p.hop = p.after;
            p.after = if goal == p.hop || p.hop == -1 {
                -1
            } else {
                calc_route(mem, routes, goal, p.hop)?
            };
            p.leg_x = step_at(mem, steps, i - 1, T_X)?;
            p.leg_y = step_at(mem, steps, i - 1, T_Y)?;
            p.x = p.leg_x;
            p.y = p.leg_y;
            waypoint(mem, &mut p, routes, extra, shadow)?;
        }

        let was_x = step_at(mem, steps, i - 1, T_X)?;
        let was_y = step_at(mem, steps, i - 1, T_Y)?;
        let scale = step_at(mem, steps, i - 1, T_SHRINK)?;

        // Each axis walks toward the waypoint and is retired on arrival. When
        // both step sizes come out zero the axis with the longer way left is
        // nudged by one, so a leg can never stall.
        if p.to_x != -1 {
            let mut d = step_x(eng, mem, &p, routes, extra, shadow, scale)?;
            if d == 0
                && step_y(eng, mem, &p, routes, extra, shadow, scale)? == 0
                && (p.x - p.to_x).abs() > (p.y - p.to_y).abs()
            {
                d = 1;
            }
            let now = if was_x < p.to_x { was_x + d } else { was_x - d };
            let done = if was_x < p.to_x {
                now >= p.to_x
            } else {
                now <= p.to_x
            };
            set_step(mem, steps, i, T_X, if done { p.to_x } else { now })?;
            if done {
                p.to_x = -1;
            }
        } else {
            set_step(mem, steps, i, T_X, was_x)?;
        }
        if p.to_y != -1 {
            let mut d = step_y(eng, mem, &p, routes, extra, shadow, scale)?;
            if d == 0
                && step_x(eng, mem, &p, routes, extra, shadow, scale)? == 0
                && (p.x - p.to_x).abs() <= (p.y - p.to_y).abs()
            {
                d = 1;
            }
            let now = if was_y < p.to_y { was_y + d } else { was_y - d };
            let done = if was_y < p.to_y {
                now >= p.to_y
            } else {
                now <= p.to_y
            };
            set_step(mem, steps, i, T_Y, if done { p.to_y } else { now })?;
            if done {
                p.to_y = -1;
            }
        } else {
            set_step(mem, steps, i, T_Y, was_y)?;
        }

        p.x = step_at(mem, steps, i, T_X)?;
        p.y = step_at(mem, steps, i, T_Y)?;
        let shrink = shrink_along(mem, &p, routes, extra)?;
        set_step(mem, steps, i, T_SHRINK, shrink)?;
        let z = aux(mem, extra, p.at, A_Z)?;
        set_step(mem, steps, i, T_Z, z)?;

        // Only the first step of a leg carries a heading; the rest keep it.
        let fresh = i == 1 || step_at(mem, steps, i - 1, T_ROUTE)? != p.at + 1;
        let heading = if fresh && !(p.leg_x == p.keep_x && p.leg_y == p.keep_y) {
            facing(mem, &p, extra, shadow)?
        } else {
            -1
        };
        set_step(mem, steps, i, T_HEADING, heading)?;
        set_step(mem, steps, i, T_ROUTE, p.at + 1)?;
        i += 1;
    }
    // 0x78241: room left over means the path ended, and the end is a Z of −1.
    if i < room {
        set_step(mem, steps, i, T_Z, -1)?;
    }
    Ok(())
}

/// `CALCROUTE` (0x7829c): the first route to enter on the cheapest way there.
fn calc_route(mem: &dyn AddressSpace, routes: u32, goal: i32, at: i32) -> Result<i32> {
    if at == goal {
        return Ok(at);
    }
    let mut best = (-1, 0x7fff);
    let mut path = vec![at];
    search(mem, routes, goal, &mut path, &mut best)?;
    Ok(best.0)
}

/// The search itself, 0x78347. Five neighbors at most, the list ending at the
/// first −1; a room already on the path is not entered again; the cost of a
/// path is the width plus the height of the rooms crossed on the way.
fn search(
    mem: &dyn AddressSpace,
    routes: u32,
    goal: i32,
    path: &mut Vec<i32>,
    best: &mut (i32, i32),
) -> Result<()> {
    let here = *path.last().unwrap_or(&0);
    let depth = path.len() - 1;
    for k in 0..5u32 {
        let next = route(mem, routes, here, 0x10 + k * 4)?;
        if next == -1 {
            break;
        }
        if path[..depth].contains(&next) {
            continue;
        }
        path.push(next);
        if next == goal {
            let mut cost = 0;
            for &n in &path[1..=depth] {
                cost += route(mem, routes, n, 8)? - route(mem, routes, n, 0)?;
                cost += route(mem, routes, n, 0x0c)? - route(mem, routes, n, 4)?;
            }
            if cost < best.1 {
                *best = (path[1], cost);
            }
        } else {
            search(mem, routes, goal, path, best)?;
        }
        path.pop();
    }
    Ok(())
}

/// Where the current leg is headed, 0x76dcc.
///
/// Inside the destination route that is the walk target itself, with a missing
/// y worked out from the route's own line. Otherwise it is the point the two
/// routes share — found by comparing coordinates for equality, so neighboring
/// routes have to meet exactly.
fn waypoint(
    mem: &dyn AddressSpace,
    p: &mut Plan,
    routes: u32,
    extra: u32,
    shadow: u32,
) -> Result<()> {
    let kind = aux(mem, extra, p.at, A_KIND)?;
    if get(mem, shadow, S_DEST_ROUTE)? - 1 == p.at {
        p.to_x = get(mem, shadow, S_DEST_X)?;
        p.to_y = get(mem, shadow, S_DEST_Y)?;
        if kind != 0 || p.to_y == -1 {
            p.to_y = on_line_y(mem, routes, extra, p.at, p.to_x)?;
        }
    } else {
        let n = p.hop;
        let next = aux(mem, extra, n, A_KIND)?;
        let c = |f: u32| route(mem, routes, p.at, f);
        let o = |f: u32| route(mem, routes, n, f);
        let (c0, c1, c2, c3) = (c(0)?, c(4)?, c(8)?, c(0x0c)?);
        let (o0, o1, o2, o3) = (o(0)?, o(4)?, o(8)?, o(0x0c)?);
        let start = (c0, c1);
        let end = (c2, c3);
        let meets = |pt: (i32, i32)| pt == (o0, o1) || pt == (o2, o3);
        match (kind, next) {
            (0, 0) => {
                p.to_x = if p.x > o2 {
                    if o2 > c0 { o2 } else { c0 }
                } else if p.x < o0 {
                    if o0 < c2 { o0 } else { c2 }
                } else {
                    p.x
                };
                p.to_y = if p.y > o3 {
                    if o3 > c1 { o3 } else { c1 }
                } else if p.y < o1 {
                    if o1 < c3 { o1 } else { c3 }
                } else {
                    p.y
                };
            }
            (0, 1) => {
                if c0 == o2 || c1 == o3 {
                    p.to_x = o2;
                    p.to_y = o3;
                } else if c2 == o0 || c3 == o1 {
                    p.to_x = o0;
                    p.to_y = o1;
                }
            }
            (0, 2) => {
                if c0 == o2 || c3 == o3 {
                    p.to_x = o2;
                    p.to_y = o3;
                } else if c2 == o0 || c1 == o1 {
                    p.to_x = o0;
                    p.to_y = o1;
                }
            }
            (1, 0) => {
                if c0 == o0 || c0 == o2 || c1 == o1 || c1 == o3 {
                    p.to_x = c0;
                    p.to_y = c1;
                } else if c2 == o0 || c2 == o2 || c3 == o1 || c3 == o3 {
                    p.to_x = c2;
                    p.to_y = c3;
                }
            }
            (1, 1) => {
                if meets(start) {
                    p.to_x = c0;
                    p.to_y = c1;
                } else if meets(end) {
                    p.to_x = c2;
                    p.to_y = c3;
                } else {
                    p.to_x = -1;
                    p.to_y = -1;
                }
            }
            (1, 2) => {
                if meets(start) {
                    p.to_x = c0;
                    p.to_y = c1;
                } else {
                    p.to_x = c2;
                    p.to_y = c3;
                }
            }
            (2, 0) => {
                if c2 == o0 || c2 == o2 || c3 == o1 || c3 == o3 {
                    p.to_x = c2;
                    p.to_y = c3;
                } else if c0 == o0 || c0 == o2 || c1 == o1 || c1 == o3 {
                    p.to_x = c0;
                    p.to_y = c1;
                } else {
                    p.to_x = -1;
                    p.to_y = -1;
                }
            }
            (2, 1) => {
                if meets(start) {
                    p.to_x = c0;
                    p.to_y = c1;
                } else {
                    p.to_x = c2;
                    p.to_y = c3;
                }
            }
            (2, 2) => {
                if meets(end) {
                    p.to_x = c2;
                    p.to_y = c3;
                } else if meets(start) {
                    p.to_x = c0;
                    p.to_y = c1;
                }
            }
            _ => {}
        }
    }
    p.keep_x = p.to_x;
    p.keep_y = p.to_y;
    Ok(())
}

/// The y on a route's line at a given x, 0x76448. Asking a rectangle for one
/// is the original's own complaint — it prints and answers zero.
fn on_line_y(mem: &dyn AddressSpace, routes: u32, extra: u32, at: i32, x: i32) -> Result<i32> {
    let (x0, y0, x1, y1) = corners(mem, routes, at)?;
    on_line(mem, extra, at, x, (x0, y0, x1, y1), "0x76448")
}

/// The x on a route's line at a given y, 0x76537.
fn on_line_x(mem: &dyn AddressSpace, routes: u32, extra: u32, at: i32, y: i32) -> Result<i32> {
    let (x0, y0, x1, y1) = corners(mem, routes, at)?;
    on_line(mem, extra, at, y, (y0, x0, y1, x1), "0x76537")
}

/// A route's two corners, `(x0, y0, x1, y1)`.
fn corners(mem: &dyn AddressSpace, routes: u32, at: i32) -> Result<(i32, i32, i32, i32)> {
    Ok((
        route(mem, routes, at, 0)?,
        route(mem, routes, at, 4)?,
        route(mem, routes, at, 8)?,
        route(mem, routes, at, 0x0c)?,
    ))
}

/// Where a route's line stands at one coordinate, with the axis chosen by the
/// order the corners are handed in: `(a0, b0, a1, b1)` solves for *b* given *a*.
///
/// The two handlers this serves — 0x76448 for y, 0x76537 for x — are the same
/// arithmetic with the axes swapped, and each writes it twice, once per route
/// kind. Those two spellings are **numerically identical**: they differ only in
/// which side of the division carries the minus sign, and Rust truncates toward
/// zero, so `div(-a, b) == -div(a, b)`. A route of kind 0 answers 0 in both.
///
/// `at` is passed through for the kind lookup, and the address for the error, so
/// a division by zero still names the handler it came from.
fn on_line(
    mem: &dyn AddressSpace,
    extra: u32,
    at: i32,
    a: i32,
    (a0, b0, a1, b1): (i32, i32, i32, i32),
    address: &str,
) -> Result<i32> {
    match aux(mem, extra, at, A_KIND)? {
        1 | 2 => Ok(b0 + div(narrow(mem, (a - a0) * (b1 - b0)), a1 - a0, address)?),
        _ => Ok(0),
    }
}

/// How far this tick carries the figure across, 0x76626.
///
/// **Kept apart from [`step_y`] on purpose.** The two read as fifty-line twins
/// and invite being folded onto an axis parameter. They are two handlers, at
/// 0x76626 and 0x769f9, and one function per handler is what makes either of
/// them checkable against the original — the property this whole port is built
/// on. Their smaller siblings `on_line_x` and `on_line_y`
/// *were* folded, because the arithmetic there is provably the same on both
/// axes; here the dominance test, the clamp and the interpolation each pick a
/// different pair of fields, and a folded version would carry the axis into
/// every one of them. That is the same code with a parameter threaded through
/// it, not less of it.
#[allow(clippy::too_many_arguments)]
fn step_x(
    eng: &Engine,
    mem: &dyn AddressSpace,
    p: &Plan,
    routes: u32,
    extra: u32,
    shadow: u32,
    scale: i32,
) -> Result<i32> {
    let fixed = aux(mem, extra, p.at, A_STEP)?;
    let speed = get(mem, shadow, S_SPEED_X)?;
    let other = get(mem, shadow, S_SPEED_Y)?;
    let mut d = if fixed == -1 {
        scaled(speed, scale, true)
    } else {
        let ticks_x = div((p.x - p.keep_x).abs(), speed, "0x76626")?;
        let ticks_y = div((p.y - p.keep_y).abs(), other, "0x76626")?;
        if aux(mem, extra, p.at, A_KIND)? != 0 && ticks_x < ticks_y {
            // Stuck to the line: step exactly onto it, and never past it.
            let line = on_line_x(mem, routes, extra, p.at, p.y)?;
            let d = (p.x - line).abs();
            if d != 0 && ((p.to_x < p.x && p.x - d < line) || (p.to_x > p.x && p.x + d > line)) {
                0
            } else {
                d
            }
        } else {
            fixed
        }
    };
    if aux(mem, extra, p.at, A_FLAGS)? & 1 == 1 && p.to_x != -1 && p.to_y != -1 {
        let ticks_x = div((p.x - p.to_x).abs(), speed, "0x76626")?;
        let ticks_y = div((p.y - p.to_y).abs(), other, "0x76626")?;
        d = if ticks_x < ticks_y {
            let os = scaled(other, scale, false).max(1);
            let total = div((p.leg_y - p.to_y).abs(), os, "0x76626")?;
            let left = div((p.y - p.to_y).abs(), os, "0x76626")?;
            let whole = (p.leg_x - p.to_x).abs();
            let rest = (p.x - p.to_x).abs();
            let want = div(whole * left, total, "0x76626")?;
            if want < rest { rest - want } else { 0 }
        } else {
            scaled(speed, scale, false)
        };
    }
    Ok(d * eng.step_multi)
}

/// The same downwards, 0x769f9. The two differ only in which axis dominates;
/// see [`step_x`] for why they are not one function.
#[allow(clippy::too_many_arguments)]
fn step_y(
    eng: &Engine,
    mem: &dyn AddressSpace,
    p: &Plan,
    routes: u32,
    extra: u32,
    shadow: u32,
    scale: i32,
) -> Result<i32> {
    let fixed = aux(mem, extra, p.at, A_STEP)?;
    let speed = get(mem, shadow, S_SPEED_Y)?;
    let other = get(mem, shadow, S_SPEED_X)?;
    let mut d = if fixed == -1 {
        scaled(speed, scale, true)
    } else {
        let ticks_x = div((p.x - p.keep_x).abs(), other, "0x769f9")?;
        let ticks_y = div((p.y - p.keep_y).abs(), speed, "0x769f9")?;
        if aux(mem, extra, p.at, A_KIND)? != 0 && ticks_x > ticks_y {
            let line = on_line_y(mem, routes, extra, p.at, p.x)?;
            let d = (p.y - line).abs();
            if d != 0 && ((p.to_y < p.y && p.y - d < line) || (p.to_y > p.y && p.y + d > line)) {
                0
            } else {
                d
            }
        } else {
            fixed
        }
    };
    if aux(mem, extra, p.at, A_FLAGS)? & 1 == 1 && p.to_x != -1 && p.to_y != -1 {
        let ticks_x = div((p.x - p.to_x).abs(), other, "0x769f9")?;
        let ticks_y = div((p.y - p.to_y).abs(), speed, "0x769f9")?;
        d = if ticks_x > ticks_y {
            let os = scaled(other, scale, false).max(1);
            let total = div((p.leg_x - p.to_x).abs(), os, "0x769f9")?;
            let left = div((p.x - p.to_x).abs(), os, "0x769f9")?;
            let whole = (p.leg_y - p.to_y).abs();
            let rest = (p.y - p.to_y).abs();
            let want = div(whole * left, total, "0x769f9")?;
            if want < rest { rest - want } else { 0 }
        } else {
            scaled(speed, scale, false)
        };
    }
    Ok(d * eng.step_multi)
}

/// A speed shrinks with the figure: `speed * (scale/10) / 100`.
fn scaled(speed: i32, scale: i32, floor: bool) -> i32 {
    if scale == -1 {
        return speed;
    }
    let v = speed * (scale / 10) / 100;
    if floor && v < 1 { 1 } else { v }
}

/// How big the figure is at this point, 0x77480: the two ends of the route
/// carry a size each and the way between them is interpolated.
fn shrink_along(mem: &dyn AddressSpace, p: &Plan, routes: u32, extra: u32) -> Result<i32> {
    let (near, far) = (
        aux(mem, extra, p.at, A_SHRINK0)?,
        aux(mem, extra, p.at, A_SHRINK1)?,
    );
    if near == -1 && far == -1 {
        return Ok(1000);
    }
    if near == far {
        return Ok(near);
    }
    let (x0, y0, x1, y1) = (
        route(mem, routes, p.at, 0)?,
        route(mem, routes, p.at, 4)?,
        route(mem, routes, p.at, 8)?,
        route(mem, routes, p.at, 0x0c)?,
    );
    if y0 == y1 {
        return Ok(near);
    }
    let t = if aux(mem, extra, p.at, A_FLAGS)? & 4 != 0 {
        div(narrow(mem, (p.x - x0) * 100), x1 - x0, "0x77480")?
    } else if aux(mem, extra, p.at, A_KIND)? != 2 {
        div(narrow(mem, (p.y - y0) * 100), y1 - y0, "0x77480")?
    } else {
        div(narrow(mem, (p.y - y1) * 100), y0 - y1, "0x77480")?
    };
    Ok(if near < far {
        near + (far - near) * t / 100
    } else {
        near - (near - far) * t / 100
    })
}

/// Which way the figure faces on this leg, 0x77efa.
///
/// The slope is in hundredths, and the thresholds 20, 130 and 500 split it
/// into the drawn directions. A four-way figure only ever gets 1 to 4.
fn facing(mem: &dyn AddressSpace, p: &Plan, extra: u32, shadow: u32) -> Result<i32> {
    let (wx, wy) = (p.keep_x, p.keep_y);
    let down = wy > p.leg_y;
    let slope = || -> i32 {
        let dy = match wy - p.leg_y {
            0 => 1,
            d => d,
        };
        (100 * (wx - p.leg_x) / dy).abs()
    };
    let eight = get(mem, shadow, S_EIGHT)? != 0;
    let mut dir = if wx == -1 && wy != -1 {
        if down { 3 } else { 4 }
    } else if !eight {
        if wy != -1 && slope() < 130 {
            if down { 3 } else { 4 }
        } else if wx < p.leg_x {
            2
        } else {
            1
        }
    } else {
        let (left, s) = (wx < p.leg_x, slope());
        if (20..=500).contains(&s) {
            match (left, down) {
                (true, true) => 8,
                (true, false) => 7,
                (false, true) => 5,
                (false, false) => 6,
            }
        } else if s < 20 {
            if down { 3 } else { 4 }
        } else if left {
            2
        } else {
            1
        }
    };
    // A route may be walked mirrored, and then front and back swap over.
    if aux(mem, extra, p.at, A_FLAGS)? & 2 != 0 {
        dir = match dir {
            3 => 4,
            4 => 3,
            5 => 6,
            6 => 5,
            7 => 8,
            8 => 7,
            d => d,
        };
    }
    Ok(dir)
}
