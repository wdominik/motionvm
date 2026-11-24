//! *Dunkle Schatten* (1994).
//!
//! The game this engine opens. It loads the install and runs walking,
//! interaction, animation and dialogue machinery, and every row of the
//! location table has its handler's logic, one module per location. The
//! music goes to the Ad Lib driver, the sound effects and the songs'
//! digital voices to the digital driver's mixer.
//!
//! What is read, with the address it came from ([`xp01data`] carries the
//! sites):
//!
//! - A location is a row in a 52-byte record table at `204Bh:0000` whose
//!   `+23h` is the far pointer to that location's handler, and the main
//!   loop at `0x4FC0` dispatches on `[1F4Ah]`.
//! - A fresh game opens at location **2**, entry `7Fh` (`0x4F37`), which
//!   the program takes when the loader left `[1F4Ah]` at zero (`0x4F2A`).
//! - The status line comes from a sixteen-entry far-pointer row at
//!   `DS:1084` indexed by `[288Dh]` (`0x10124`). Those templates carry
//!   `%c` ink controls — `%cBetrachte %c%s`.
//!
//! What the shipped files say, read with `motionvm-parsec-formats`:
//!
//! - `DSROOM.DAT` is an FGROUP of 60 slots; 28 of them carry an `RDV1.00`
//!   playfield, and slot 0 is the `NULLPTR` marker.
//! - `PANELS.DAT` resource 0 is a **320×72** picture and resource 1 a
//!   palette bank at 160.
//! - `CMBMOU.DAT` resource 0 is a palette bank at 224 and resource 15 the
//!   font; `PLAY00.DAT` resource 0 is a bank at 128 and the rest are the
//!   player's 25×46 cels.
//!
//! The keys are mapped by `179Eh:0611h` (`0x18041`–`0x18097`); the title
//! program's input module is the same (`08E6h` there: the mapping at
//! `0x9401` is `0x18041`'s bytes). `MUSADL.DRV` is resource 5 of
//! `PSMDRV.DAT` (`DS:0A06h`, installed at `0x5B7B`), `DMABLAST.DRV`
//! resource 1 (`0x5BB3`, installed at `0x5BEC`).

// The location logic, one module per location the game names in its
// location table, written against the engine boundary `parsec1994::Api` and
// driven by `logic`.
mod bogenstrasse;
mod brand;
pub(crate) mod build;
mod ending;
mod f1;
mod f2;
mod f3;
mod forest_maze;
mod global_rules;
mod hook;
mod hotspot_rules;
mod in_der_ruine;
mod joerg;
mod jz;
mod karte;
mod kiosk;
mod krankenhaus;
mod kueche;
mod laden;
mod logic;
mod mauer;
mod monopol;
mod onward;
mod ralf;
mod restore;
mod ruine;
mod schule;
mod schuppen;
mod stammtisch;
mod story;
mod strasse;
#[cfg(test)]
mod trace;
mod wald;
mod werkstatt;
mod wohnzimmer;
mod xp00;
mod xp01data;
mod zimmer;

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
use crate::num;
#[cfg(test)]
use crate::titles::Start;
#[cfg(test)]
use motionvm_render::Framebuffer;
#[cfg(test)]
use parsec1994::title::host::TitleProgram as _;
#[cfg(test)]
use std::time::Duration;

/// One loaded install of *Dunkle Schatten*.
pub(crate) type DunkleSchatten = Install<xp00::Xp00>;

impl Install<xp00::Xp00> {
    /// Opens the install at `dir`. Nothing is played yet: the start, which
    /// reads the save directory, is [`crate::titles::Runtime::start`].
    ///
    /// The program is read twice over: by the generation, for its tables,
    /// and by this title's own reading ([`ExeText`]) for what only it
    /// finds. Most of that is read and pinned before the runtime needs it,
    /// which is deliberate: a fact taken out of a listing with an address
    /// beside it is worth keeping the moment it is read.
    pub(crate) fn open(dir: &Path) -> Result<Self> {
        let text = ExeText::read(dir)?;
        let game = Program::open(dir, &build::BUILD, |l| Ok(build::reading(&text, l)))?;
        Ok(Install::new(Title::DunkleSchatten, game, xp00::Xp00::open))
    }
}

/// The install's directory, or `None` on a machine that has not got the
/// game — said on the test's output, so that a skip does not read as a
/// pass. Every test helper that answers `None` for want of the game asks
/// here, so its callers return without saying it again.
#[cfg(test)]
pub(crate) fn gamedata_or_skip() -> Option<PathBuf> {
    let dir = motionvm_parsec_testutil::gamedata_ds1();
    if dir.is_none() {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
    }
    dir
}

/// The install, or `None` on a machine that has not got this title.
#[cfg(test)]
fn ds1_or_skip() -> Option<DunkleSchatten> {
    let dir = gamedata_or_skip()?;
    let mut g = DunkleSchatten::open(&dir).expect("open Dunkle Schatten");
    g.start_in_xp01(Start::New).expect("start");
    close_start_options(&mut g);
    Some(g)
}

/// The options `XP01` opens before its first location, closed the way a
/// player closes them: Escape once they have dissolved in (keys typed
/// while they do are flushed, `0x12106`). The main loop then enters the
/// first location.
#[cfg(test)]
fn close_start_options(g: &mut DunkleSchatten) {
    use crate::titles::Runtime;
    while g.game.inner.options_fading() {
        g.tick();
    }
    g.key(27);
    g.tick();
    assert!(!g.game.inner.options_up(), "Escape closes the options");
}

/// The parsec1994 runtime of a fresh game, for the shared runtime's own
/// tests — which need an install to stand on, and this is the one they have.
#[cfg(test)]
pub(crate) fn adventure_for_test() -> Option<parsec1994::Adventure> {
    ds1_or_skip().map(|g| g.game.inner)
}

#[cfg(test)]
mod tests;
