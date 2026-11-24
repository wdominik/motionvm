//! *Captain Zins* (1994).
//!
//! The second title on the `parsec1994` generation: its own `XP01.EXE`,
//! linked against another build of the same library, which the runtime
//! reads off the program ([`parsec1994::Profile`]). Every row of the
//! location table has its handler's logic, one module per location, and
//! the global rules and the shared hook are this program's own.
//!
//! What is read, with the address it came from ([`xp01data`] carries the
//! sites):
//!
//! - A location is a row in a 44-byte record table at `1EBBh:0000h` whose
//!   `+1Bh` is the far pointer to that location's handler, and the main
//!   loop at `0x0074B` dispatches on `[1A5Ch]`.
//! - A fresh game opens at location **1**, the Wohnung, entry `7Fh`: the
//!   program stores no start, and the walk on from row 0 lands there
//!   (`0x00B00`–`0x00B56`).
//! - The status line comes from a sixteen-entry far-pointer row at
//!   `DS:0D3Eh` indexed by `[1E58h]` (`0x0B583`).
//! - The keys are mapped by `163Ch:0611h`'s table (`0x16CA1`–`0x16CF7`),
//!   Dunkle Schatten's `179Eh:0611h` but for the data addresses.
//! - `MUSADL.DRV` is resource 5 of `PSMDRV.DAT` (`DS:0982h`, opened at
//!   `0x012C9`), the digital driver resource 1.
//!
//! Play begins in the title program `XP00.EXE` ([`xp00`]) as `ZINS.BAT`
//! runs it, and its menu's choice starts `XP01`.

mod bau;
mod bits;
pub(crate) mod build;
mod ending;
mod extra;
mod formel;
mod global_rules;
mod hook;
mod igor;
mod kreuzung;
mod logic;
mod money;
mod onward;
mod pc;
#[cfg(test)]
mod playthrough;
mod prof;
mod restore;
mod robots;
mod saldo;
mod schrank;
mod schranke;
mod sharky;
mod sparplan;
mod telefon;
#[cfg(test)]
mod trace;
mod wohnung;
mod wucher;
mod xp00;
mod xp01data;

use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use motionvm_parsec_formats::Title;

use crate::error::Result;
use crate::titles::parsec1994;

use parsec1994::install::Install;
use parsec1994::program::Program;
use xp01data::ExeText;

#[cfg(test)]
use crate::titles::Start;

/// One loaded install of *Captain Zins*.
pub(crate) type CaptainZins = Install<xp00::Xp00>;

impl Install<xp00::Xp00> {
    /// Opens the install at `dir`. Nothing is played yet: the start, which
    /// reads the save directory, is [`crate::titles::Runtime::start`].
    ///
    /// The program is read twice over: by the generation, for its tables
    /// and its library's build, and by this title's own reading
    /// ([`ExeText`]) for what only it finds.
    pub(crate) fn open(dir: &Path) -> Result<Self> {
        let text = ExeText::read(dir)?;
        let game = Program::open(dir, &build::BUILD, |l| Ok(build::reading(&text, l)))?;
        Ok(Install::new(Title::CaptainZins, game, xp00::Xp00::open))
    }
}

/// The install's directory, or `None` on a machine that has not got the
/// game — said on the test's output, so that a skip does not read as a
/// pass.
#[cfg(test)]
pub(crate) fn gamedata_or_skip() -> Option<PathBuf> {
    let dir = motionvm_parsec_testutil::gamedata_capzins();
    if dir.is_none() {
        eprintln!("skipping: no Captain Zins gamedata directory");
    }
    dir
}

/// The install started as a fresh game, its start's options closed the
/// way a player closes them, or `None` on a machine without the title.
#[cfg(test)]
fn capzins_or_skip() -> Option<CaptainZins> {
    use crate::titles::Runtime;
    let dir = gamedata_or_skip()?;
    let mut g = CaptainZins::open(&dir).expect("open Captain Zins");
    g.start_past_title(Start::New).expect("start");
    while g.game.inner.options_fading() {
        g.tick();
    }
    g.key(27);
    g.tick();
    assert!(!g.game.inner.options_up(), "Escape closes the options");
    Some(g)
}

/// The parsec1994 runtime of a fresh game of this title, in its first
/// location, for the tests that pin what was read from its program.
#[cfg(test)]
pub(crate) fn adventure_for_test() -> Option<parsec1994::Adventure> {
    capzins_or_skip().map(|g| g.game.inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::titles::Runtime;
    use crate::titles::parsec1994::Event;

    /// The Bits are items of their own class: the choice of an item takes
    /// its class from the table at `DS:049Fh` (`0x0AAB9`), 5 for items 18
    /// to 25 and 0 for the rest — without it the Bits rule's key never
    /// matches and a Bit cannot be put on the Codepoint.
    #[test]
    fn a_bit_is_chosen_with_its_own_class() {
        let Some(mut a) = adventure_for_test() else {
            return;
        };
        a.mode = 0;
        a.object1 = 0;
        a.select_object(18, None);
        assert_eq!((a.object1, a.object2), (18, 5));
        a.object1 = 0;
        a.select_object(2, None);
        assert_eq!((a.object1, a.object2), (2, 0));
    }

    /// The figure set travels in the player's record: a set loaded is
    /// kept in `+1Eh`, and the start loads the set the record remembers,
    /// so a game saved past location 2 starts with the second strip and
    /// figure, its carried list split for them.
    #[test]
    fn the_start_loads_the_figure_set_the_record_remembers() {
        let Some(mut a) = adventure_for_test() else {
            return;
        };
        assert_eq!(a.figure_digit, 0);
        a.figure_set_load(1);
        assert_eq!(a.player_record_bytes()[0x1E..0x20], [1, 0]);
        a.figure_set_load(0);
        assert_eq!(a.player_record_bytes()[0x1E..0x20], [0, 0]);
        // A restored record that remembers set 1.
        a.set_player_record_byte(0x1E, 1);
        a.items = vec![2, 7, 9];
        a.figure_set_at_start();
        assert_eq!(a.figure_digit, 1);
        // Strip split at 6: the second file's items are 7 and 9.
        assert_eq!((a.inv_lo, a.inv_hi), (1, 3));
    }

    /// Captain Zins's arrows are read with a row of three, its strip's
    /// columns, and a click with the second button scrolls a row.
    #[test]
    fn the_strip_arrows_scroll_a_row_of_three() {
        let Some(mut g) = capzins_or_skip() else {
            return;
        };
        let arrows = g.game.inner.panel_widgets.arrows;
        assert_eq!((arrows.up, arrows.down, arrows.row), (0x115, 0x116, 3));
        // The down arrow's widget, clicked with the second button through
        // the host's road: a row on.
        let w = g
            .game
            .inner
            .panel_widgets
            .widgets
            .iter()
            .find(|w| w.id == 0x116)
            .map(|w| (i32::from(w.x) + 2, i32::from(w.y) + 2))
            .expect("the down arrow is registered");
        let a = &mut g.game.inner;
        a.items = (1..=5).collect();
        a.strip_range();
        // The opening's script holds the input and talks; neither may
        // keep the click from the strip here.
        a.bits = 0;
        a.tasks.t[0].kind = 0;
        a.mouse_move(w.0, w.1);
        g.click(false);
        assert_eq!(
            g.game.inner.scroll_for_test(),
            2,
            "5 items, a row more than 3 below: on to 2"
        );
    }

    /// A fresh game opens the options first, as `_main` does
    /// (`0CD4h:045Ah`, `0x00744`), and once they are closed the main loop
    /// enters the Wohnung, row 1, on entry `7Fh` — the walk on's answer
    /// from row 0. Nothing the location's entry asks of the engine is
    /// reported unbuilt.
    #[test]
    fn a_fresh_game_opens_in_the_wohnung() {
        let Some(mut g) = capzins_or_skip() else {
            return;
        };
        for _ in 0..20 {
            g.tick();
        }
        assert_eq!(g.location(), Some(1), "the first location");
        assert_eq!(g.location_name(), Some("Wohnung"), "its name");
        // What only the Wohnung's own entry leaves: `[1A5Eh]` set from the
        // location word (`0x11416`) and the opening marked run
        // (`[1CEAh] := 1`, `0x1142F`).
        let a = &g.game.inner;
        assert_eq!(a.here_word, 1, "the handler entered for location 1");
        assert_eq!(a.mem_byte(0x1CEA), 1, "the opening has run");
        let events = std::mem::take(&mut g.game.inner.events);
        let unbuilt: Vec<_> = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::Unbuilt(..) | Event::UnreadRule { .. } | Event::FarWrite { .. }
                )
            })
            .collect();
        assert!(unbuilt.is_empty(), "nothing unbuilt: {unbuilt:?}");
    }

    /// A node's edges blocked and opened again through the logic's
    /// boundary, as the Schranke, the Saldo and the Wucher do with node
    /// `3E8h`: the room the runtime answers carries `3FFh` on every edge of
    /// the node's ring, then 0, and the next load of the slot brings the
    /// file's again.
    #[test]
    fn a_node_s_edges_are_blocked_for_the_visit() {
        use crate::titles::parsec1994::{Api, Far};
        let Some(mut g) = capzins_or_skip() else {
            return;
        };
        for _ in 0..20 {
            g.tick();
        }
        let node = 0x03E8;
        let ring = |a: &parsec1994::Adventure| -> Vec<u16> {
            let walk = &a.room().expect("a room").walks[0];
            let n = walk
                .nodes
                .iter()
                .rposition(|x| x.id == node)
                .expect("the node");
            walk.edges
                .iter()
                .filter(|e| usize::from(e.a) == n || usize::from(e.b) == n)
                .map(|e| e.blocked)
                .collect()
        };
        // The Saldo: its graph has the node.
        g.set_location_via(10, 0x7F);
        for _ in 0..5 {
            g.tick();
        }
        let a = &mut g.game.inner;
        let file = ring(a);
        assert!(!file.is_empty(), "the node has edges");
        let player = Far {
            off: logic::WALKER,
            seg: a.ds(0).seg,
        };
        a.node_open(player, node);
        assert!(ring(a).iter().all(|&b| b == 0), "opened");
        a.node_block(player, node);
        assert!(ring(a).iter().all(|&b| b == 0x03FF), "blocked");
        a.node_open(player, node);
        assert!(ring(a).iter().all(|&b| b == 0), "opened again");
        a.node_block(player, node);
        g.set_location_via(10, 0x7F);
        assert_eq!(ring(&g.game.inner), file, "the file's edges again");
    }

    /// The program's own start, read off the walk on: row 1 on `7Fh`.
    #[test]
    fn the_start_is_the_walk_on_s_answer_from_row_0() {
        let Some(dir) = gamedata_or_skip() else {
            return;
        };
        let text = ExeText::read(&dir).expect("reads");
        assert_eq!(text.start, (1, 0x7F));
        assert_eq!(text.template_row, 0x0D3E);
        assert_eq!(text.inventory_at, (0x1138, 0x1139, 0x113C));
        assert!(
            text.initial_items.is_empty(),
            "a fresh game carries nothing"
        );
        assert_eq!(text.substitutions, ["9990122233300", "39452"]);
        assert_eq!(
            text.panel_curves,
            [[[3, 30], [1, 25], [1, 25]], [[0, 8], [0, 8], [0, 8]]]
        );
        assert_eq!(text.gray_ramp.0, 0xC0);
    }
}
