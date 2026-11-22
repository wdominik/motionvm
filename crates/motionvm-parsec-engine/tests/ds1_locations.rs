//! The location table, read out of the program handed over.
//!
//! Listing is not opening: it reads `XP01.EXE` and nothing a game needs
//! beside it.
//!
//! They need the game's files — `MOTIONVM_GAMEDATA_DS1` — and skip without
//! them.
//!
//! The game this file drives is Dunkle Schatten (PARSEC).

use motionvm_parsec_engine::{locations, rules};
use motionvm_parsec_testutil::gamedata_ds1;

/// The table lists the rows its own program bounds.
#[test]
fn the_table_lists_the_rows_its_program_bounds() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let list = locations(&dir).expect("DS1");
    assert_eq!(list.len(), 61, "rows");
    assert_eq!(
        list.iter().filter(|l| l.name.is_some()).count(),
        30,
        "named rows"
    );
    // Row 0 is not a place: its kind is zero, which is what sends the
    // main loop somewhere else.
    assert_eq!(list[0].kind, 0, "row 0");
}

/// A row's name and kind are what its own record carries.
#[test]
fn a_row_carries_its_own_name_and_kind() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let list = locations(&dir).expect("DS1");
    assert_eq!(list[2].name.as_deref(), Some("Zimmer"));
    assert_eq!(list[2].kind, 2);
    // Location 20 carries a real pointer to an empty name, which is not
    // the same as carrying none — rows 30 and up carry none.
    assert_eq!(list[20].name.as_deref(), Some(""));
    assert_eq!(list[30].name, None);
    assert_eq!(list[30].kind, 3);
}

/// A location's exits are in its own record, and they lead back.
///
/// Two parallel byte lists that the main loop resolves after the handler
/// has run. The Zimmer's one exit reaches the Küche and the Küche's first
/// comes back — which is what says the two lists are read the right way
/// round.
#[test]
fn the_exits_a_location_carries_lead_back() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let list = locations(&dir).expect("DS1");
    assert_eq!(list[2].exits, vec![(100, 3)]);
    assert_eq!(list[3].exits, vec![(100, 2), (101, 4), (109, 1)]);
    assert_eq!(list[11].exits, vec![(109, 1), (200, 12)]);
    assert_eq!(list[12].exits, vec![(200, 11)]);
    // Every destination names a row the table has.
    for row in &list {
        for (id, dest) in &row.exits {
            assert!(
                usize::from(*dest) < list.len(),
                "location {} exit {id} goes to {dest}",
                row.number
            );
        }
    }
}

/// A location lists the objects its entry sets up — not its hotspots.
///
/// Short lists, and they name the things the room has to bring to life:
/// the Küche lists 200, which is its `Mutter`. A room's clickable
/// furniture is not in here; that is the hotspot table's business.
#[test]
fn a_location_lists_the_objects_its_entry_sets_up() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let list = locations(&dir).expect("DS1");
    assert_eq!(list[2].objects, Vec::<u8>::new());
    assert_eq!(list[3].objects, vec![200]);
    assert_eq!(list[4].objects, vec![200, 130, 202]);
    // Nine slots in the record, so no row can list more.
    assert!(list.iter().all(|l| l.objects.len() <= 9));
}

/// The program carries an interaction table.
///
/// The table is found by the shape of its installer — a routine at offset
/// `0Ch` of its own segment that stashes the pointer it is handed in
/// `cs:[0]` — rather than by an address.
#[test]
fn the_program_carries_an_interaction_table() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let table = rules(&dir).expect("DS1");
    assert_eq!(table.len(), 33, "rules");
    // Every rule takes part in some phase, or the walk would have
    // stopped on it, and every one names a handler.
    assert!(table.iter().all(|r| r.phase != 0));
    assert!(table.iter().all(|r| r.handler.0 != 0));
}

/// A location carries its own interaction rules, and they are about its
/// own objects.
///
/// The entry function installs them before the global ones, taking the
/// far pointer from a record field it builds as `record + 7 + disp`; the
/// engine reads the field rather than carrying it.
#[test]
fn a_location_carries_its_own_interaction_rules() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let list = locations(&dir).expect("DS1");
    let sum: usize = list.iter().map(|l| l.rules.len()).sum();
    assert_eq!(sum, 116, "location rules");

    // The bedroom answers for three objects of its own picture — 200 is
    // the sleeper the opening arms a dialogue on, 203 and 1 the other two
    // the room's object chain carries. They are not the record's object
    // list, which is a different set and empty here.
    let bedroom = &list[2];
    let mut about: Vec<i16> = bedroom.rules.iter().map(|r| r.object).collect();
    about.sort_unstable();
    assert_eq!(about, [1, 200, 203]);
    let verbs: Vec<u16> = bedroom.rules.iter().map(|r| r.verb).collect();
    assert_eq!(verbs, [4, 1, 12], "Benutze 200, Betrachte 203, Nimm 1");
}
