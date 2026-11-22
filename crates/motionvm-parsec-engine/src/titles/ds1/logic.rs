//! How *Dunkle Schatten*'s location logic meets the runtime.
//!
//! Each row of the location table names a handler; [`logic`] says which
//! module re-expresses it, and [`TABLE`] hands that to the generation's
//! visit driver ([`Visit`](crate::titles::parsec1994::logic::Visit)).
//!
//! The location's hooks and rule handlers are called by the engine at the
//! very points the original calls them (`call far [1E5Ch]`, `[1E60h]`,
//! the rule search's `call far es:[si+0Ah]`), through the three functions
//! below that this title installs.

use super::{
    bogenstrasse, brand, f1, f2, f3, forest_maze, global_rules, in_der_ruine, joerg, jz, karte,
    kiosk, krankenhaus, kueche, laden, mauer, monopol, onward, ralf, restore, ruine, schule,
    schuppen, stammtisch, strasse, wald, werkstatt, wohnzimmer, zimmer,
};
use crate::num;
pub(crate) use crate::titles::parsec1994::logic::Logic;
use crate::titles::parsec1994::logic::LogicTable;
use crate::titles::parsec1994::{Adventure, Far};

/// `DS:1693h`, the player's walker record. The logic hands it to the
/// walker routines and to the subject slots as `push ds ; push 1693h`.
pub(crate) const WALKER: u16 = 0x1693;

/// `2163h:0000h`, the in-memory saved game: a segment of its own below
/// `DGROUP` (`0x14817`, `0x14CDA`).
pub(crate) const SAVE_BUFFER: u16 = 0x2163;

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

/// A rule handler, by the far address its rule names: a location's own
/// in its handler's segment, else one of the global table's.
/// `None` is one this engine does not have; the rule search then says so.
pub(crate) fn rule_handler(adv: &mut Adventure, far: (u16, u16), flags: u16) -> Option<i16> {
    match logic_by_segment(far.0) {
        Some(l) => (l.rule)(adv, far.1, flags),
        None => global_rules::rule(adv, far, flags),
    }
}

/// The title's logic as the visit driver asks for it: each row's, and the
/// walk on for a row that is not a place.
pub(crate) const TABLE: LogicTable = LogicTable {
    logic,
    onward: onward::LOGIC,
};
