//! How *Dunkle Schatten*'s location logic meets the runtime.
//!
//! Each row of the location table names a handler; [`logic`] says which
//! module re-expresses it, and [`TABLE`] hands that to the generation's
//! visit driver ([`Visit`](crate::titles::parsec1994::logic::Visit)).
//!
//! The location's hooks and rule handlers are called by the engine at the
//! very points the original calls them (`call far [1E5Ch]`, `[1E60h]`,
//! the rule search's `call far es:[si+0Ah]`), through the same table: a
//! rule's handler by its segment ([`SEGMENTS`]), else the global
//! rules'.

use super::{
    bogenstrasse, brand, f1, f2, f3, forest_maze, global_rules, in_der_ruine, joerg, jz, karte,
    kiosk, krankenhaus, kueche, laden, mauer, monopol, onward, ralf, restore, ruine, schule,
    schuppen, stammtisch, strasse, wald, werkstatt, wohnzimmer, zimmer,
};
pub(crate) use crate::titles::parsec1994::logic::Logic;
use crate::titles::parsec1994::logic::LogicTable;
use crate::titles::parsec1994::restore::MainLoop;

/// `DS:1693h`, the player's walker record. The logic hands it to the
/// walker routines and to the subject slots as `push ds ; push 1693h`.
pub(crate) const WALKER: u16 = 0x1693;

/// `[1F48h]`: the entry id, and non-zero once an exit is asked for.
pub(crate) const ENTRY: u16 = 0x1F48;
/// `[1F4Ah]`: the location the main loop dispatches on (`0x4FC0`).
pub(crate) const LOCATION: u16 = 0x1F4A;
/// `[1F4Ch]`: the location the running handler was entered for.
pub(crate) const HERE: u16 = 0x1F4C;
/// `DS:1F56h`: 32 bytes of object states per location (`add ax, 1F56h`
/// after `shl ax, 5`, `0x9F5`).
pub(crate) const OBJECT_STATES: u16 = 0x1F56;

/// The main loop's three words, for the walk on and the way back.
pub(super) const WORDS: MainLoop = MainLoop {
    entry: ENTRY,
    location: LOCATION,
    here: HERE,
};

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
const SEGMENTS: &[(u16, u16)] = &[
    (0x0060, 1),
    (0x0089, 2),
    (0x00D2, 3),
    (0x0100, 4),
    (0x0140, 5),
    (0x0158, 6),
    (0x018B, 7),
    (0x01AD, 8),
    (0x020C, 9),
    (0x025E, 10),
    (0x02AD, 11),
    (0x02E5, 12),
    (0x02FD, 13),
    (0x0333, 14),
    (0x035C, 15),
    (0x0373, 16),
    (0x0395, 17),
    (0x03B9, 18),
    (0x03C7, 19),
    (0x03D2, 21),
    (0x03E6, 22),
    (0x03F1, 23),
    (0x03FE, 27),
    (0x046C, 28),
    (0x0485, 29),
    (0x04B0, 30),
    (0x147A, 60),
];

/// The title's logic as the visit driver asks for it: each row's, and the
/// walk on for a row that is not a place.
pub(crate) const TABLE: LogicTable = LogicTable {
    logic,
    onward: onward::LOGIC,
    segments: SEGMENTS,
    global: global_rules::rule,
};

#[cfg(test)]
mod tests {
    #[test]
    fn each_segment_is_its_locations_handlers() {
        let Some(adv) = super::super::adventure_for_test() else {
            return;
        };
        crate::titles::parsec1994::logic::assert_segments_are_the_handlers(&super::TABLE, &adv);
    }

    /// The words the logic names are the ones the runtime reads off the
    /// dispatch.
    #[test]
    fn the_main_loops_words_are_the_dispatchs() {
        let Some(adv) = super::super::adventure_for_test() else {
            return;
        };
        let m = adv.ds_map;
        assert_eq!(
            (m.entry, m.location, m.here, m.object_states),
            (
                super::ENTRY,
                super::LOCATION,
                super::HERE,
                super::OBJECT_STATES
            )
        );
    }
}
