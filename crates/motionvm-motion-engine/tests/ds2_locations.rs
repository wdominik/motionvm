//! Every location the game can enter, entered.
//!
//! Five of the sixteen read through stray pointers, and the engine has to
//! tolerate it. The original's `@` (0x626fc) checks nothing: it works the
//! address out and reads, so a script that dereferences a stray value gets a
//! harmless answer and carries on. Refusing instead is stricter than the
//! engine, and ends the game where the original plays on.
//!
//! What the five scripts actually do:
//!
//! - **5 and 10** test `KRSCHRZIEHE @ 0 =`, but `KRSCHRZIEHER` is the *item
//!   number* 24 from module 11, not the flag `_?KRSCHRZIEHER` the author meant
//!   — every neighboring test in the same chain uses the underscore form. So
//!   the read lands on address 24, which is module 0: the kernel's own module,
//!   built at run time and not something we have.
//! - **6** calls `PSETWALK` on Gaby before her shadow record exists, so it
//!   reads `0 + 44`.
//! - **7 and 8** call `?INVINCL`. Those two stay inside loaded modules doing
//!   it, in the frames the test below drives; the other three do not.
//!
//! Location 12 is left out on purpose: its entry in the location table is
//! uninitialized in the shipped data, so entering it is a jump into nowhere in
//! the original too, and nothing in the game ever asks for it.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_motion_engine::Game;
use motionvm_motion_testutil::gamedata_ds2;

/// All five load and keep running, stray reads and all — and the run says
/// afterwards which of them read past a module and how often.
///
/// The second half is what the departures ledger promises: such a read is
/// "counted … and names the total at the end of a run". Which locations
/// actually produce one is asserted rather than assumed, because that set is
/// worth knowing and a new member of it is worth being told about. Three of
/// the five do within the frames driven here — 5 and 10 through `KRSCHRZIEHE`,
/// 6 through the shadow record that is not there yet. Locations 7 and 8 reach
/// `?INVINCL` and stay inside loaded modules doing it.
#[test]
fn the_locations_that_read_through_stray_pointers_still_load() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut strayed = Vec::new();
    for want in [5, 6, 7, 8, 10] {
        let mut game = Game::open(&dir).expect("game opens");
        game.start().expect("4:START");
        while game.pump().expect("startup runs") {}
        game.set_var(2, "_NEXTLOC", want)
            .expect("ask for the location");

        // Reached, not still standing there: several of these are mid-game
        // scenes whose task manager hands straight on when it is entered out
        // of context. What is being pinned is that entering them runs.
        let mut reached = false;
        for frame in 0..600 {
            game.set_input(0, 0, false, false, 0).expect("input");
            if let Err(e) = game.step() {
                panic!("location {want} stopped on frame {frame}: {e}");
            }
            reached |= game.get_var(2, "_ACTLOC") == Some(want);
        }
        assert!(reached, "location {want} was never entered");

        // And the run says so afterwards. The ledger's promise about these
        // reads is that they are "counted … and names the total at the end of
        // a run", which for a long time nothing outside a test ever asked for;
        // these five locations are where the counting has something to count.
        let notes = motionvm_motion_engine::Driven::diagnostics(&game);
        if let Some(strays) = notes
            .iter()
            .find(|d| d.subject == "reads into modules that are not loaded")
        {
            assert!(
                strays.detail.starts_with(|c: char| c.is_ascii_digit()),
                "the line opens with the total: {}",
                strays.detail
            );
            strayed.push(want);
        }
    }
    assert_eq!(
        strayed,
        [5, 6, 10],
        "a location started or stopped reading past a module"
    );
}

/// `=>GET` (0x64999) loads a module out of the resource file every time it
/// is asked, so a location's three modules come back pristine on every
/// re-entry and their variables start over — `INCLLOC` frees the outgoing
/// location's three (module 5, `0x1a08`–`0x1a48`) and takes the incoming
/// one's the same way (`0x1b08`–`0x1b48`). `_ZOOMIT` lives in module 223,
/// location 23's own; what it is for does not matter here, only that the
/// word puts it back. The word is driven directly, the way the savegame
/// tests drive `PUT` and `GET`: a location that stands still when entered
/// out of context does not always let go again on request.
#[test]
fn a_locations_modules_come_back_pristine_on_re_entry() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 23)
        .expect("ask for the location");
    for frame in 0..600 {
        game.set_input(0, 0, false, false, 0).expect("input");
        if let Err(e) = game.step() {
            panic!("location 23 stopped on frame {frame}: {e}");
        }
        if game.get_var(2, "_ACTLOC") == Some(23) {
            break;
        }
    }
    assert_eq!(
        game.get_var(2, "_ACTLOC"),
        Some(23),
        "location 23 was never entered"
    );

    let initial = game
        .get_var(223, "_ZOOMIT")
        .expect("_ZOOMIT is in module 223");
    game.set_var(223, "_ZOOMIT", initial + 77).expect("write");
    assert_eq!(game.get_var(223, "_ZOOMIT"), Some(initial + 77));

    // What `INCLLOC` does on the way out and back in: the memory goes with
    // the slot, and comes back from the container.
    let depth = game.vm.data.len();
    game.vm.data.push(223);
    assert!(
        game.kernel_word("=>ERASE").expect("=>ERASE"),
        "=>ERASE is a word this engine implements"
    );
    assert!(
        game.vm.module(223).is_none(),
        "`=>ERASE` gives module 223's memory back"
    );
    game.vm.data.push(223);
    assert!(
        game.kernel_word("=>GET").expect("=>GET"),
        "=>GET is a word this engine implements"
    );
    assert_eq!(
        game.get_var(223, "_ZOOMIT"),
        Some(initial),
        "`=>GET` loads module 223 afresh"
    );
    assert_eq!(game.vm.data.len(), depth, "neither word leaves anything");
}
