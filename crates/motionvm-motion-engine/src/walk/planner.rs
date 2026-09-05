//! The route planner: `CROUTE` (0x7780c) and the pass that lays a path out
//! into the step buffer the stepper beside it replays.
//!
//! Named for what it does rather than for the route table it walks, because
//! the reader of one of that table's fields is [`super::route`] and two
//! things called the same would have to be told apart every time either is
//! named.
//!
//! Split from the stepper because the two are different subjects on the same
//! records: [`super`] consumes the command queue and spends one ready-made
//! step a frame, and nothing in it looks at a route table; this searches the
//! route graph, picks the waypoints, works out the shrink and the facing
//! along each leg, and writes the steps. The records they share — the person,
//! the shadow, the step and route tables — and the readers that scale their
//! offsets to the machine's cell are the parent module's, and reached
//! through it.

use super::{
    A_FLAGS, A_KIND, A_SHRINK0, A_SHRINK1, A_STEP, A_Z, P_AUX, P_ROOM, P_ROUTES, P_SHADOW, P_STEPS,
    S_DEST_ROUTE, S_DEST_X, S_DEST_Y, S_EIGHT, S_ROUTE, S_SHRINK, S_SPEED_X, S_SPEED_Y, S_X, S_Y,
    T_HEADING, T_ROUTE, T_SHRINK, T_X, T_Y, T_Z, aux, div, get, narrow, route, set_step, step_at,
};
use crate::Engine;
use motionvm_motion_forth::cell;
use motionvm_motion_forth::{AddressSpace, Error, Result};
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
pub(crate) fn croute(eng: &Engine, mem: &mut dyn AddressSpace, person: u32) -> Result<()> {
    let shadow = cell::unsigned(get(mem, person, P_SHADOW)?);
    let steps = cell::unsigned(get(mem, person, P_STEPS)?);
    let routes = cell::unsigned(get(mem, person, P_ROUTES)?);
    let extra = cell::unsigned(get(mem, person, P_AUX)?);
    let room = get(mem, person, P_ROOM)?;
    croute_with(eng, mem, shadow, steps, routes, extra, room)
}

/// The same, with the five pointers the person record would have held.
pub(crate) fn croute_with(
    eng: &Engine,
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
    // A zero shrink is 1000 in the 32-bit routine (`0x778ca`) and in
    // `ENVIRO.EXE` (`0a40:1176`); the other three 16-bit builds copy the
    // field as it stands — see [`crate::Profile::walk_defaults_shrink`].
    let shrink = match get(mem, shadow, S_SHRINK)? {
        0 if eng.profile.walk_defaults_shrink => 1000,
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
        let legs = Legs {
            routes,
            extra,
            shadow,
            scale: step_at(mem, steps, i - 1, T_SHRINK)?,
        };

        // Each axis walks toward the waypoint and is retired on arrival. When
        // both step sizes come out zero the axis with the longer way left is
        // nudged by one, so a leg can never stall.
        if p.to_x != -1 {
            let mut d = step_x(eng, mem, &p, legs)?;
            if d == 0
                && step_y(eng, mem, &p, legs)? == 0
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
            let mut d = step_y(eng, mem, &p, legs)?;
            if d == 0
                && step_x(eng, mem, &p, legs)? == 0
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
    if eng.profile.walk_smooths_headings {
        smooth_headings(mem, steps, room)?;
    }
    Ok(())
}

/// The pass `LL.EXE`'s `CROUTE` closes with (`0104:516d`–`0x5318`), over the
/// buffer the loop above has just filled.
///
/// It walks the buffer as runs of equal heading — the field as the loop left
/// it, so a leg's later steps, which carry −1, are a run of their own. Where
/// a run of at least three steps is followed by a run of one or two whose
/// heading sits on the other side of 2 from it (0, 1 or 2 against 3 and
/// up, or the reverse), and that by a run of at least one step heading the
/// same way as the first, the short run is rewritten to the first run's
/// heading (`0x52da`–`0x52f7`) and the walk goes on from where the third run
/// *ended* — so that run is never a first run of its own; otherwise it goes
/// on from the second run's first step. The pass stops at an end marker
/// (`0x530b`), but a run does not: inside a run the marker is tested only
/// together with an index at or past `room` (`0x5189`–`0x51a2`), so a run
/// reads on through the marker while the headings beyond it — whatever an
/// earlier, longer walk left there — keep matching, and what it finds
/// there counts.
///
/// One bound is this engine's: a run stops at `room` where the original's
/// would read on into whatever memory follows the buffer.
fn smooth_headings(mem: &mut dyn AddressSpace, steps: u32, room: i32) -> Result<()> {
    let heading = |mem: &dyn AddressSpace, i: i32| step_at(mem, steps, i, T_HEADING);
    let ended =
        |mem: &dyn AddressSpace, i: i32| Ok::<bool, Error>(step_at(mem, steps, i, T_Z)? == -1);
    let run_end = |mem: &dyn AddressSpace, from: i32, h: i32| -> Result<i32> {
        let mut i = from;
        while i < room && heading(mem, i)? == h {
            i += 1;
        }
        Ok(i)
    };
    let mut si = 0;
    while si < room && !ended(mem, si)? {
        let h1 = heading(mem, si)?;
        let i = run_end(mem, si, h1)?;
        if i >= room || ended(mem, i)? {
            si = i;
            continue;
        }
        let h2 = heading(mem, i)?;
        let di = run_end(mem, i, h2)?;
        if di >= room || ended(mem, di)? || heading(mem, di)? != h1 {
            si = i;
            continue;
        }
        let j = run_end(mem, di, h1)?;
        let (len1, len2, len3) = (i - si, di - i, j - di);
        let across = (h2 <= 2 && h1 >= 3) || (h2 >= 3 && h1 <= 2);
        if !across || len2 > 2 || len1 < 3 || len3 < 1 {
            si = i;
            continue;
        }
        for k in i..di {
            set_step(mem, steps, k, T_HEADING, h1)?;
        }
        si = j;
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
/// The three records a walking step reads, and the scale it reads them at.
///
/// Not a convenience bundle: these are the pointers the *person record* holds
/// in the original, and a step reads all three of them together — the route
/// table it is walking, the auxiliary record beside it, and the shadow record
/// the walker's own speeds live in. Passing them one by one made a step take
/// seven arguments and let two of them be swapped without a word, since all
/// three are `u32` addresses into the same memory.
#[derive(Debug, Clone, Copy)]
struct Legs {
    /// The route table, as `CALCROUTE` walks it.
    routes: u32,
    /// The auxiliary record beside it, holding a leg's fixed step.
    extra: u32,
    /// The walker's shadow record: its speeds are `S_SPEED_X` and
    /// `S_SPEED_Y` in it.
    shadow: u32,
    /// The shrink this leg walks at, out of the step table's `T_SHRINK`.
    scale: i32,
}

fn step_x(eng: &Engine, mem: &dyn AddressSpace, p: &Plan, legs: Legs) -> Result<i32> {
    let Legs {
        routes,
        extra,
        shadow,
        scale,
    } = legs;
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
fn step_y(eng: &Engine, mem: &dyn AddressSpace, p: &Plan, legs: Legs) -> Result<i32> {
    let Legs {
        routes,
        extra,
        shadow,
        scale,
    } = legs;
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

#[cfg(test)]
mod tests {
    use super::super::STEP;
    use super::*;

    /// A flat, four-byte-celled memory: enough of an [`AddressSpace`] for
    /// the step buffer the closing pass reads and writes.
    struct Flat(Vec<i32>);

    impl AddressSpace for Flat {
        fn fetch_cell(&self, raw: i32) -> Result<i32> {
            Ok(self.0[cell::at(raw).unwrap() / 4])
        }
        fn store_cell(&mut self, raw: i32, value: i32) -> Result<()> {
            self.0[cell::at(raw).unwrap() / 4] = value;
            Ok(())
        }
        fn fetch_byte(&self, raw: i32) -> Result<u8> {
            Ok(cell::low8(
                self.0[cell::at(raw).unwrap() / 4] >> (8 * (raw % 4)),
            ))
        }
        fn read_bytes(&self, _raw: i32, _n: usize) -> Result<Vec<u8>> {
            Err(Error::Unsupported("bytes".into()))
        }
        fn write_bytes(&mut self, _raw: i32, _bytes: &[u8]) -> Result<()> {
            Err(Error::Unsupported("bytes".into()))
        }
        fn offset(&self, raw: i32, bytes: i32) -> i32 {
            raw + bytes
        }
        fn cell_size(&self) -> i32 {
            4
        }
        fn callable(&self, raw: i32) -> i32 {
            raw
        }
        fn is_live(&self, _raw: i32) -> bool {
            true
        }
        fn module_image(&self, _module: u32) -> Option<Vec<u8>> {
            None
        }
        fn restore_module(&mut self, _module: u32, _image: &[u8]) -> Result<()> {
            Ok(())
        }
    }

    const ROOM: i32 = 16;

    /// A step buffer holding `headings`, closed with the end marker.
    fn buffer(headings: &[i32]) -> Flat {
        let room = cell::at(ROOM).expect("a positive constant");
        let mut mem = Flat(vec![0; room * cell::index(STEP) / 4 + 8]);
        for (i, &h) in headings.iter().enumerate() {
            set_step(&mut mem, 0, cell::count(i), T_HEADING, h).unwrap();
        }
        set_step(&mut mem, 0, cell::count(headings.len()), T_Z, -1).unwrap();
        mem
    }

    fn headings(mem: &Flat, n: usize) -> Vec<i32> {
        (0..cell::count(n))
            .map(|i| step_at(mem, 0, i, T_HEADING).unwrap())
            .collect()
    }

    fn smoothed(input: &[i32]) -> Vec<i32> {
        let mut mem = buffer(input);
        smooth_headings(&mut mem, 0, ROOM).unwrap();
        headings(&mem, input.len())
    }

    /// The pass as read: three or more one way, one or two the other way
    /// across 2, one or more back — the short run takes the long run's
    /// heading.
    #[test]
    fn a_short_flip_between_two_runs_is_rewritten() {
        assert_eq!(smoothed(&[3, 3, 3, 1, 3, 3]), [3, 3, 3, 3, 3, 3]);
        assert_eq!(smoothed(&[3, 3, 3, 1, 1, 3]), [3, 3, 3, 3, 3, 3]);
        assert_eq!(smoothed(&[0, 0, 0, 0, 7, 0]), [0, 0, 0, 0, 0, 0]);
    }

    /// A leg's later steps carry −1, which sits on the low side of 2: a
    /// single leg start heading 3 or up between them loses its heading.
    #[test]
    fn a_leg_start_after_a_long_leg_is_folded_into_it() {
        assert_eq!(smoothed(&[-1, -1, -1, 5, -1]), [-1, -1, -1, -1, -1]);
        assert_eq!(smoothed(&[-1, -1, -1, 2, -1]), [-1, -1, -1, 2, -1]);
    }

    /// What the pass leaves alone: a first run under three, a middle run
    /// over two, a flip that stays on one side of 2, no run to come back
    /// to, and a middle run that does not lead back to the first heading.
    #[test]
    fn everything_else_stands() {
        assert_eq!(smoothed(&[3, 3, 1, 3]), [3, 3, 1, 3]);
        assert_eq!(smoothed(&[3, 3, 3, 1, 1, 1, 3]), [3, 3, 3, 1, 1, 1, 3]);
        assert_eq!(smoothed(&[3, 3, 3, 5, 3]), [3, 3, 3, 5, 3]);
        assert_eq!(smoothed(&[1, 1, 1, 3]), [1, 1, 1, 3]);
        assert_eq!(smoothed(&[3, 3, 3, 1, 4]), [3, 3, 3, 1, 4]);
        assert_eq!(smoothed(&[]), Vec::<i32>::new());
    }

    /// After a rewrite the walk goes on from where the third run ended, so
    /// that run is never a first run of its own: a second flip right behind
    /// it stands, and one behind a fresh run of three is rewritten.
    #[test]
    fn the_pass_goes_on_past_the_third_run() {
        assert_eq!(
            smoothed(&[3, 3, 3, 1, 3, 3, 3, 2, 3]),
            [3, 3, 3, 3, 3, 3, 3, 2, 3]
        );
        assert_eq!(
            smoothed(&[3, 3, 3, 1, 3, 4, 4, 4, 1, 4]),
            [3, 3, 3, 3, 3, 4, 4, 4, 4, 4]
        );
    }

    /// A run reads on through the end marker while the headings beyond it
    /// keep matching: what an earlier walk left there counts. A run that
    /// *ends* on the marker ends the pass (`0x51bc`, `0x5225`).
    #[test]
    fn a_run_reads_on_through_the_marker() {
        // Live: three 3s and a 1. The marker at 4 still carries a 1 from an
        // earlier walk, so the short run reaches past it, and the stale 3s
        // behind it make the third run.
        let mut mem = buffer(&[3, 3, 3, 1]);
        set_step(&mut mem, 0, 4, T_HEADING, 1).unwrap();
        for i in 5..8 {
            set_step(&mut mem, 0, i, T_HEADING, 3).unwrap();
        }
        smooth_headings(&mut mem, 0, ROOM).unwrap();
        assert_eq!(headings(&mem, 4), [3, 3, 3, 3]);

        // With the marker itself heading 3, the short run ends on it, and
        // the pass stops there.
        let mut mem = buffer(&[3, 3, 3, 1]);
        for i in 4..8 {
            set_step(&mut mem, 0, i, T_HEADING, 3).unwrap();
        }
        smooth_headings(&mut mem, 0, ROOM).unwrap();
        assert_eq!(headings(&mem, 4), [3, 3, 3, 1]);
    }
}
