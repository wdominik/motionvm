//! The families this window plays, as data.
//!
//! This is the one place in the window that names a family crate. Everything
//! else reaches a game through the contract: the window picks a family off
//! this list and drives whatever it opens. Adding a family to the program is
//! adding a line here — the usage text, the folder dialog's complaints and
//! the roster a player sees all follow from the list.

use motionvm_playable::Family;
use std::path::Path;

/// Every family this build knows, in the order they are asked.
pub static FAMILIES: &[&dyn Family] = &[&motionvm_motion::MOTION];

/// The family for `dir`: the first whose `detect` claims it, or `None` when
/// nobody does — and the caller then answers with [`nobodys`], built from
/// every family's roster.
///
/// First claim wins. A family's `detect` may claim more than its openers can
/// open — a shape test cannot always tell two games of one machine apart —
/// and with one family on the list that costs nothing: the claimer's own
/// refusal explains itself. With a second family, a claim that another
/// family could have opened becomes the case to handle here.
pub fn find(dir: &Path) -> Option<&'static dyn Family> {
    FAMILIES.iter().copied().find(|f| f.detect(dir).is_some())
}

/// The complaint for a directory no family claims: what every game this
/// build plays would need, whichever family it belongs to.
pub fn nobodys(dir: &Path) -> String {
    let mut message = format!("{} is not a game motionvm can open\n", dir.display());
    for family in FAMILIES {
        for card in family.games() {
            message.push_str(&format!("  {} needs {}\n", card.short, card.needs));
        }
    }
    message.push_str(
        "  A game this program does not play, or an incomplete copy of one \
         of these; see \"What a game needs\" in the README.",
    );
    message
}
