//! How *Bi-Fi 2*'s location logic meets the runtime.
//!
//! The location table at `2181h:0000h` (92 records of `44h` bytes, read by
//! [`crate::titles::xp1994::Locations`]) names a handler per row at `+33h`;
//! [`logic`] says which module re-expresses it, and [`TABLE`] hands that
//! to the generation's visit driver, which walks on past a row with no
//! logic here and tells it as not built.

use super::{
    bi_fi, buero, denkfabrik, dino_zoo, enterhaken_bruecke, enterhaken_maschinenraum, flur,
    gefaengnis, global_rules, interlude_2, interlude_3, interlude_6, interlude_13, interlude_39,
    jones1, jones2, kaffee, kantine, labyrinth, lift, location_46, maske, nullachtfuenfzehn,
    oeliens, oeliens_halle, onward, pforte, rambowsky, requisite, restore, sfx, technik,
    tontechnik, vip_lounge, vorfuehrraum, vorzimmer,
};
use crate::titles::parsec1994::Api;
pub(crate) use crate::titles::parsec1994::logic::Logic;
use crate::titles::parsec1994::logic::{LogicTable, RecordHook};
use crate::titles::parsec1994::restore::MainLoop;

/// The main loop's three words around the location the dispatch reads
/// (`0x0077D`): `[200Ah]` the entry, `[200Ch]` the location, `[200Eh]` the
/// location the running handler was entered for (`0x0078E`: `mov ax,
/// [200Ch] ; cmp ax, [200Eh]`).
pub(super) const WORDS: MainLoop = MainLoop {
    entry: 0x200A,
    location: 0x200C,
    here: 0x200E,
};

/// Every row's logic, by number. Rows 0, 37 and 38 are of kind 0 and name
/// the walk on (`0060h:04BFh`).
pub(crate) fn logic(loc: u16) -> Option<Logic> {
    Some(match loc {
        0 | 37 | 38 => onward::LOGIC,
        1 => kaffee::LOGIC,
        2 => interlude_2::LOGIC,
        3 => interlude_3::LOGIC,
        4 => pforte::LOGIC,
        5 => vorzimmer::LOGIC,
        6 => interlude_6::LOGIC,
        7 => rambowsky::LOGIC,
        8 => jones1::LOGIC,
        9 => jones2::LOGIC,
        10 => enterhaken_bruecke::LOGIC,
        12 => bi_fi::LOGIC,
        13 => interlude_13::LOGIC,
        11 => enterhaken_maschinenraum::LOGIC,
        14 => dino_zoo::LOGIC,
        15 => technik::LOGIC,
        16 => requisite::LOGIC,
        17 => denkfabrik::LOGIC,
        18 | 19 | 30..=35 => flur::LOGIC,
        20 => tontechnik::LOGIC,
        21 => sfx::LOGIC,
        22 => maske::LOGIC,
        23 => kantine::LOGIC,
        24 => buero::LOGIC,
        25 => vip_lounge::LOGIC,
        26 => vorfuehrraum::LOGIC,
        27 => oeliens::LOGIC,
        28 => oeliens_halle::LOGIC,
        29 => nullachtfuenfzehn::LOGIC,
        36 => lift::LOGIC,
        39 => interlude_39::LOGIC,
        41 => gefaengnis::LOGIC,
        46 => location_46::LOGIC,
        91 => restore::LOGIC,
        79 => labyrinth::LOGIC_SKELETT,
        40 | 42..=45 | 47..=90 => labyrinth::LOGIC,
        _ => return None,
    })
}

/// Each location's handler segment, for its rules and its gate.
const SEGMENTS: &[(u16, u16)] = &[
    (kaffee::SEGMENT, 1),
    (pforte::SEGMENT, 4),
    (vorzimmer::SEGMENT, 5),
    (rambowsky::SEGMENT, 7),
    (jones1::SEGMENT, 8),
    (jones2::SEGMENT, 9),
    (enterhaken_bruecke::SEGMENT, 10),
    (enterhaken_maschinenraum::SEGMENT, 11),
    (dino_zoo::SEGMENT, 14),
    (technik::SEGMENT, 15),
    (requisite::SEGMENT, 16),
    (denkfabrik::SEGMENT, 17),
    (flur::SEGMENT, 18),
    (tontechnik::SEGMENT, 20),
    (sfx::SEGMENT, 21),
    (maske::SEGMENT, 22),
    (kantine::SEGMENT, 23),
    (buero::SEGMENT, 24),
    (vip_lounge::SEGMENT, 25),
    (vorfuehrraum::SEGMENT, 26),
    (oeliens::SEGMENT, 27),
    (oeliens_halle::SEGMENT, 28),
    (nullachtfuenfzehn::SEGMENT, 29),
    (lift::SEGMENT, 36),
    (labyrinth::SEGMENT, 40),
    (gefaengnis::SEGMENT, 41),
    (location_46::SEGMENT, 46),
];

/// A gate of a location's own: the location whose handler's segment holds
/// it answers.
fn gate(api: &mut dyn Api, far: (u16, u16), arg: i16) -> Option<i16> {
    match far {
        (kaffee::SEGMENT, _) => kaffee::gate(api, far.1, arg),
        (flur::SEGMENT, _) => flur::gate(api, far.1, arg),
        (tontechnik::SEGMENT, _) => tontechnik::gate(api, far.1, arg),
        (dino_zoo::SEGMENT, _) => dino_zoo::gate(api, far.1, arg),
        // The door's gate the walk verb's rule installs (`0x10605`).
        (0x0FE3, 0x000E) => Some(global_rules::door_gate(api, arg)),
        _ => None,
    }
}

/// The rows that name a record hook (`+47h`): the Vorführraum's,
/// `167Bh:015Bh`, which runs its roll. Every other row's is null.
fn record_hook(loc: u16) -> Option<RecordHook> {
    match loc {
        26 => Some(vorfuehrraum::record_hook),
        _ => None,
    }
}

pub(crate) const TABLE: LogicTable = LogicTable {
    logic,
    onward: onward::LOGIC,
    segments: SEGMENTS,
    global: global_rules::rule,
    gate,
    record_hook,
};
