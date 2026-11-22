//! How *Dunkle Schatten*'s location logic meets the runtime.
//!
//! The original's main loop (`0x4FC0`) calls the handler of the location
//! in `[1F4Ah]`; the handler enters, runs its own frame loop until an exit
//! is asked for, and leaves. Here the loop is turned inside out once, at
//! this one place: [`Visit`] holds the running location's [`Logic`] and
//! calls its `enter`, then its `frame` once per tick, then its `leave`,
//! and the next location's `enter` — the order the original runs them in.
//!
//! The location's hooks and rule handlers are called by the engine at the
//! very points the original calls them (`call far [1E5Ch]`, `[1E60h]`,
//! the rule search's `call far es:[si+0Ah]`), through the three functions
//! below that this title installs.

use super::{
    bogenstrasse, brand, f1, f2, f3, forest_maze, in_der_ruine, joerg, jz, karte, kiosk,
    krankenhaus, kueche, laden, mauer, monopol, onward, ralf, restore, ruine, schule, schuppen,
    stammtisch, strasse, wald, werkstatt, wohnzimmer, zimmer,
};
use crate::num;
use crate::titles::parsec1994::{Adventure, Api, DsMap, Far};
use crate::titles::xp1994::Locations;

/// `DS:1693h`, the player's walker record. The logic hands it to the
/// walker routines and to the subject slots as `push ds ; push 1693h`.
pub(crate) const WALKER: u16 = 0x1693;

/// `2163h:0000h`, the in-memory saved game: a segment of its own below
/// `DGROUP` (`0x14817`, `0x14CDA`).
pub(crate) const SAVE_BUFFER: u16 = 0x2163;

/// Where this build keeps what the logic shares with the engine.
pub(crate) fn ds_map(locations: &Locations, inventory: (u16, u16, u16)) -> DsMap {
    DsMap {
        // The main loop's three words sit together around the location
        // the dispatch reads (`0x4FC0`): `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`.
        entry: locations.location.wrapping_sub(2),
        location: locations.location,
        here: locations.location.wrapping_add(2),
        walker: WALKER,
        // `[1C82h]`, the hundredths the Zimmer's opening waits on
        // (`0x9D4`), and `[1C76h]`, the ticks not yet taken (`0xC5B`).
        hundredths: 0x1C82,
        ticks_taken: 0x1C76,
        // `0x9F5`: `add ax, 1F56h` after `shl ax, 5`.
        object_states: 0x1F56,
        // `0x11A1`: `mov [1689h], dx`, entry 0 of the row `02 n` reads.
        substitutions: 0x1689,
        substitution_count: 2,
        // `0x610`: the map's rule reads the object the click chose, and its
        // other two rules write the verb (`0x665`).
        verb: 0x288D,
        object: 0x2891,
        // `0x757`: the map's frame loop reads the event record the poll
        // fills.
        input_event: 0x184E,
        // `0x0F1FE`: `call far [28BBh]`, the gate the machine asks; the
        // Wald's rule takes it down (`0x3F2A`).
        gate: 0x28BB,
        // `0x7A0C`: the room's header the loader keeps; F3 reads its
        // palette through it (`0x487B`).
        room_header: 0x2851,
        // `0x16B1`: the Straße waits for the door it came in by; `0x1803`
        // its hook sets one.
        entry_door: 0x1EDE,
        // `0x0CD67`, `0x0CABC`, `0x0CD71`.
        inventory,
        // `0x14817`: `mov word [bp-2], 2163h`, the buffer the start's load
        // clears (`0x14821`) and the save and restore copy through.
        save_buffer: SAVE_BUFFER,
    }
}

/// One location's logic.
#[derive(Clone, Copy)]
pub(crate) struct Logic {
    pub enter: fn(&mut dyn Api),
    pub frame: fn(&mut dyn Api) -> bool,
    pub leave: fn(&mut dyn Api),
    pub hook: fn(&mut dyn Api, u16, u16, u16),
    pub anim_hook: Option<fn(&mut dyn Api, Far, u16)>,
    pub rule: fn(&mut dyn Api, u16, u16) -> Option<i16>,
}

/// Every row's logic, by number: all 61 rows of the table at `204Bh:0000h`
/// name a handler, and `None` is a number past the table.
pub(crate) fn logic(loc: u16) -> Option<Logic> {
    Some(match loc {
        0 | 20 => onward::LOGIC,
        1 => karte::LOGIC,
        2 => zimmer::LOGIC,
        3 => kueche::LOGIC,
        4 => wohnzimmer::LOGIC,
        5 => ralf::LOGIC,
        6 => strasse::LOGIC,
        7 => joerg::LOGIC,
        8 => jz::LOGIC,
        9 => laden::LOGIC,
        10 => monopol::LOGIC,
        11 => schule::LOGIC,
        12 => mauer::LOGIC,
        13 => werkstatt::LOGIC,
        14 => bogenstrasse::LOGIC,
        15 => brand::LOGIC,
        16 => krankenhaus::LOGIC,
        17 => kiosk::LOGIC,
        18 => ruine::LOGIC,
        19 => in_der_ruine::LOGIC,
        21 => schuppen::LOGIC,
        22 => stammtisch::LOGIC,
        23 => wald::LOGIC,
        24 => jz::LOGIC,
        25 => jz::LOGIC,
        26 => jz::LOGIC,
        27 => f1::LOGIC,
        28 => f2::LOGIC,
        29 => f3::LOGIC,
        30..=59 => forest_maze::LOGIC,
        60 => restore::LOGIC,
        _ => return None,
    })
}

/// A location's rule handlers live in its handler's segment.
fn logic_by_segment(seg: u16) -> Option<Logic> {
    match seg {
        0x0060 => logic(1),
        0x0089 => logic(2),
        0x00D2 => logic(3),
        0x0100 => logic(4),
        0x0140 => logic(5),
        0x0158 => logic(6),
        0x018B => logic(7),
        0x01AD => logic(8),
        0x020C => logic(9),
        0x025E => logic(10),
        0x02AD => logic(11),
        0x02E5 => logic(12),
        0x02FD => logic(13),
        0x0333 => logic(14),
        0x035C => logic(15),
        0x0373 => logic(16),
        0x0395 => logic(17),
        0x03B9 => logic(18),
        0x03C7 => logic(19),
        0x03D2 => logic(21),
        0x03E6 => logic(22),
        0x03F1 => logic(23),
        0x03FE => logic(27),
        0x046C => logic(28),
        0x0485 => logic(29),
        0x04B0 => logic(30),
        0x147A => logic(60),
        _ => None,
    }
}

/// The location's own hook (`[1E5Ch]`), installed into the runtime.
pub(crate) fn location_hook(adv: &mut Adventure, code: u16, arg: i16, task: usize) {
    if let Some(l) = logic(num::low16(adv.hook_location)) {
        (l.hook)(adv, code, num::unsigned(arg), num::word(task));
    }
}

/// The location's animation hook (`[1E60h]`), installed into the runtime.
pub(crate) fn anim_hook(adv: &mut Adventure, rec: Far, code: u16) {
    if let Some(hook) = logic(num::low16(adv.hook_location)).and_then(|l| l.anim_hook) {
        hook(adv, rec, code);
    }
}

/// A location's own rule handler, by the far address its rule names.
/// `None` is one this engine does not have; the rule search then says so.
pub(crate) fn rule_handler(adv: &mut Adventure, far: (u16, u16), flags: u16) -> Option<i16> {
    let l = logic_by_segment(far.0)?;
    (l.rule)(adv, far.1, flags)
}

/// The visit in progress.
pub(crate) struct Visit {
    logic: Logic,
}

impl Visit {
    /// The main loop's next call: the handler of the location in its word,
    /// entered. A word past the table, or a row of kind 0, is not
    /// dispatched through the table at all: the main loop bounds the word
    /// against 3Dh and asks the row's kind (`0x04F8E`–`0x04FA7`) and calls
    /// the walk at `0x053B2` instead ([`onward`]), which is also the code
    /// rows 0 and 20 name as their handler.
    pub(crate) fn enter(adv: &mut Adventure) -> Visit {
        let loc = adv.location_word;
        let walks = usize::from(loc) >= adv.locations.count
            || adv.locations.kind(&adv.image, usize::from(loc)) == Some(0);
        let logic = logic(loc).filter(|_| !walks).unwrap_or(onward::LOGIC);
        // A fresh call of a handler: a fresh stack frame.
        adv.handler_locals = [0; 8];
        (logic.enter)(adv);
        Visit { logic }
    }

    /// One turn of the handler's frame loop; when it ends, the handler
    /// leaves and the next location's is entered.
    ///
    /// `None` when the main loop is over: it runs while `[1F4Ah]` is not
    /// below zero (`0x4FE9`), and quitting sets it to −2 (`0x12403`).
    pub(crate) fn frame(self, adv: &mut Adventure) -> Option<Visit> {
        if (self.logic.frame)(adv) {
            return Some(self);
        }
        (self.logic.leave)(adv);
        adv.take_leave();
        if num::signed(adv.location_word) < 0 {
            return None;
        }
        Some(Visit::enter(adv))
    }

    /// Leaves the running location without its frame loop ending — for a
    /// jump the host asks for.
    pub(crate) fn leave(self, adv: &mut Adventure) {
        (self.logic.leave)(adv);
        adv.take_leave();
    }
}
