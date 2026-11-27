//! How *Captain Zins*'s location logic meets the runtime.
//!
//! The location table at `1EBBh:0000h` (20 records of `2Ch` bytes, read by
//! [`crate::titles::xp1994::Locations`]) names a handler per row; [`logic`]
//! says which module re-expresses it, and [`TABLE`] hands that to the
//! generation's visit driver
//! ([`Visit`](crate::titles::parsec1994::logic::Visit)).
//!
//! The location's hooks and rule handlers are called by the engine at the
//! points the original calls them (`call far [18CEh]`, `[18D4h]`, the rule
//! search's `call far es:[si+0Ah]`), through the same table: a rule's
//! handler by its segment ([`SEGMENTS`]), else the global rules'.
//! The callback the Bits and the Schranke bend `[1980h]` to goes through
//! [`CALLBACKS`]. The location's own hook is `[18CEh]`, installed from the
//! record's `+27h` at `0x0A310`; its animation hook `[18D4h]`, from `+2Bh`
//! at `0x0A33A`.

use super::{
    bau, bits, extra, formel, global_rules, igor, kreuzung, money, onward, pc, prof, restore,
    robots, saldo, schrank, schranke, sharky, sparplan, telefon, wohnung, wucher,
};
use crate::titles::parsec1994::Callback;
pub(crate) use crate::titles::parsec1994::logic::Logic;
use crate::titles::parsec1994::logic::LogicTable;
use crate::titles::parsec1994::restore::MainLoop;

/// `DS:11DCh`, the player's walker record: the save's fourth block copies
/// `30h` bytes from `push 11DCh` (`0x100C7`), and the logic hands it to the
/// walker routines as `push ds ; push 11DCh` (`0x11475`).
pub(crate) const WALKER: u16 = 0x11DC;
/// `[1204h]`, the walker record's word `+28h`: the node it last reached,
/// which the frame loops wait on (`0x1145B`).
pub(crate) const WALKER_28: u16 = 0x1204;

/// `1F3Bh:0000h`, the in-memory saved game: `mov word [bp-2], 1F3Bh` at
/// the start's load (`0x0FC6F`) and in the restore row's handler
/// (`0x10122`).
pub(crate) const SAVE_BUFFER: u16 = 0x1F3B;

/// `[1A5Ah]`: the entry id, and non-zero once an exit is asked for
/// (`mov word [1A5Ah], 7Fh`, `0x00B50`).
pub(crate) const ENTRY: u16 = 0x1A5A;
/// `[1A5Ch]`: the location the main loop dispatches on (`0x0077D`).
pub(crate) const LOCATION: u16 = 0x1A5C;
/// `[1A5Eh]`: the location the running handler was entered for
/// (`0x0073E`, `0x0078E`).
pub(crate) const HERE: u16 = 0x1A5E;
/// The main loop's three words, for the walk on and the way back.
pub(super) const WORDS: MainLoop = MainLoop {
    entry: ENTRY,
    location: LOCATION,
    here: HERE,
};
/// `DS:1A68h`: 32 bytes of object states per location (`add ax, 1A68h`
/// after `shl ax, 5`, `0x114A7`).
pub(crate) const OBJECT_STATES: u16 = 0x1A68;
/// `[1E5Ch]`: the object the pointer chose (`0x11390`).
pub(crate) const OBJECT: u16 = 0x1E5C;

/// Every row's logic, by number: rows 0 and 19 are not places, the walk on
/// and the way back into a loaded game; `None` is a number past the table.
pub(crate) fn logic(loc: u16) -> Option<Logic> {
    Some(match loc {
        0 => onward::LOGIC,
        1 => wohnung::LOGIC,
        2 => prof::LOGIC,
        3 => kreuzung::LOGIC,
        4 => telefon::LOGIC,
        5 => pc::LOGIC,
        6 => bits::LOGIC,
        7 => formel::LOGIC,
        8 => schranke::LOGIC,
        9 => sparplan::LOGIC,
        10 => saldo::LOGIC,
        11 => robots::LOGIC,
        12 => bau::LOGIC,
        13 => schrank::LOGIC,
        14 => sharky::LOGIC,
        15 => extra::LOGIC,
        16 => wucher::LOGIC,
        17 => igor::LOGIC,
        18 => money::LOGIC,
        19 => restore::LOGIC,
        _ => return None,
    })
}

/// A location's rule handlers live in its handler's segment: the segment
/// of each row's `+1Bh` far pointer.
const SEGMENTS: &[(u16, u16)] = &[
    (0x112E, 1),
    (0x1156, 2),
    (0x119D, 3),
    (0x11C7, 4),
    (0x11F2, 5),
    (0x120C, 6),
    (0x1246, 7),
    (0x1254, 8),
    (0x1292, 9),
    (0x12AC, 10),
    (0x130A, 11),
    (0x1323, 12),
    (0x134A, 13),
    (0x1380, 14),
    (0x13A5, 15),
    (0x13BC, 16),
    (0x1406, 17),
    (0x143E, 18),
];

/// The routines a location bends the callback `[1980h]` to: the Bits'
/// (`mov word [1982h], 120Ch ; mov word [1980h], 0001h`, `0x1227E`,
/// `0x12284`) and the Schranke's (`1254h:0055h`, `0x1261C`, `0x12622`).
pub(crate) const CALLBACKS: &[Callback] = &[
    ((0x120C, 0x0001), bits::object_state),
    ((0x1254, 0x0055), schranke::object_state),
];

/// The title's logic as the visit driver asks for it: each row's, and the
/// walk on for a row that is not a place.
pub(crate) const TABLE: LogicTable = LogicTable {
    logic,
    onward: onward::LOGIC,
    segments: SEGMENTS,
    global: global_rules::rule,
    gate: crate::titles::parsec1994::logic::no_gate,
    record_hook: crate::titles::parsec1994::logic::no_record_hook,
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
