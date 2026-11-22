//! The families this window plays, as data.
//!
//! This is the one place in the window that names a family crate. Everything
//! else reaches a game through the contract: the window picks a family off
//! this list and drives whatever it opens. Adding a family to the program is
//! adding a line here — the usage text, the folder dialog's complaints and
//! the roster a player sees all follow from the list.

use motionvm_playable::Family;
use std::path::Path;

/// Every family this build knows, in the order their games are listed.
pub(crate) static FAMILIES: &[&dyn Family] = &[&motionvm_motion::MOTION];

/// The family for `dir`, or the sentence that says why there is none.
///
/// Every family is asked, and exactly one claim opens. A directory nobody
/// claims is answered with what every game of every family needs; one that
/// two families claim — a directory holding two games' files, copied
/// together — is refused with every claimant's card named, rather than
/// opened by whichever family happens to be listed first. The order of [`FAMILIES`] therefore decides
/// nothing about which game a directory is.
///
/// A family's `detect` may claim more than its openers can open — a shape
/// test cannot always tell two games of one machine apart — and that stays
/// the family's own affair: its `open` refuses what it claimed and cannot
/// play, in its own words.
pub(crate) fn find(dir: &Path) -> Result<&'static dyn Family, String> {
    claimant(FAMILIES, dir)
}

/// [`find`] over any list, so a test can hand it families of its own.
fn claimant<'a>(families: &[&'a dyn Family], dir: &Path) -> Result<&'a dyn Family, String> {
    let claims: Vec<_> = families
        .iter()
        .filter_map(|f| f.detect(dir).map(|card| (*f, card)))
        .collect();
    match claims.as_slice() {
        [] => Err(nobody_in(families, dir)),
        [(family, _)] => Ok(*family),
        several => {
            let mut message = format!(
                "{} holds the files of more than one game, and motionvm opens \
                 only one at a time:\n",
                dir.display()
            );
            for (family, card) in several {
                message.push_str(&format!("  {} ({})\n", card.name, family.name()));
            }
            message.push_str("  Give each game a directory of its own.");
            Err(message)
        }
    }
}

/// The complaint for a directory no family claims: what every game this
/// build plays would need, whichever family it belongs to.
fn nobody_in(families: &[&dyn Family], dir: &Path) -> String {
    let mut message = format!("{} is not a game motionvm can open\n", dir.display());
    for family in families {
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

#[cfg(test)]
mod tests {
    use super::*;
    use motionvm_playable::{GameCard, Playable, Result};

    /// A family that claims a directory when a file of a given name is in it.
    struct Fake {
        name: &'static str,
        marker: &'static str,
        cards: &'static [GameCard],
    }

    impl Family for Fake {
        fn name(&self) -> &'static str {
            self.name
        }
        fn games(&self) -> &'static [GameCard] {
            self.cards
        }
        fn detect(&self, dir: &Path) -> Option<GameCard> {
            dir.join(self.marker).is_file().then(|| self.cards[0])
        }
        fn open(&self, _dir: &Path) -> Result<Box<dyn Playable>> {
            Err("a fake opens nothing".into())
        }
    }

    static FIRST: Fake = Fake {
        name: "First",
        marker: "FIRST.DAT",
        cards: &[GameCard::new(
            "First Game: The Box",
            "First Game",
            "FIRST.DAT",
        )],
    };
    static SECOND: Fake = Fake {
        name: "Second",
        marker: "SECOND.DAT",
        cards: &[GameCard::new("Second Game", "Second Game", "SECOND.DAT")],
    };
    static BOTH: &[&dyn Family] = &[&FIRST, &SECOND];

    /// A fresh directory holding the named files, removed when dropped.
    struct Dir(std::path::PathBuf);

    impl Dir {
        fn with(tag: &str, files: &[&str]) -> Self {
            let dir =
                std::env::temp_dir().join(format!("motionvm-roster-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("a temporary directory");
            for f in files {
                std::fs::write(dir.join(f), b"").expect("a marker file");
            }
            Self(dir)
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn one_claim_opens_wherever_its_family_is_listed() {
        let dir = Dir::with("second", &["SECOND.DAT"]);
        let found = claimant(BOTH, &dir.0).expect("one family claims it");
        assert_eq!(found.name(), "Second", "the second family's claim stands");
    }

    #[test]
    fn two_claims_are_refused_naming_both() {
        let dir = Dir::with("both", &["FIRST.DAT", "SECOND.DAT"]);
        let e = claimant(BOTH, &dir.0)
            .err()
            .expect("two claims open nothing");
        assert!(e.contains("more than one game"), "{e}");
        assert!(e.contains("First Game: The Box (First)"), "{e}");
        assert!(e.contains("Second Game (Second)"), "{e}");
    }

    #[test]
    fn no_claim_lists_every_familys_games() {
        let dir = Dir::with("none", &[]);
        let e = claimant(BOTH, &dir.0).err().expect("nobody claims it");
        assert!(e.contains("is not a game motionvm can open"), "{e}");
        assert!(e.contains("First Game needs FIRST.DAT"), "{e}");
        assert!(e.contains("Second Game needs SECOND.DAT"), "{e}");
    }
}
