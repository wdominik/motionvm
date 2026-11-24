//! The PARSEC family's front door: the crate a window reaches the family
//! through, and the only one that names both the engine and the contract.
//!
//! The engine drives a game through its own [`Runtime`], shaped method for
//! method like the contract's [`Playable`]; the contract is not the
//! family's, so the one cannot simply be the other, and this crate is where
//! they meet. Every method of the face below forwards. A window holds a
//! roster of families and nothing else, and this family is one line of it:
//! [`PARSEC`].
//!
//! Sound: the engine leaves its driver calls as bytes over its own
//! `MusicSink`, and never learns the codec; [`Playable::open_music`] pairs
//! it here with `motionvm_psm`'s player of the game's own `MUSADL.DRV` and
//! its `DMABLAST.DRV` — both handed over by the engine as bytes.

mod music;

use std::path::Path;

use motionvm_parsec_engine::{Game, Runtime, Title};
use motionvm_parsec_formats::detect;
use motionvm_playable::{
    AudioSource, Button, Diagnostic, Family, Frame, GameCard, KeyPress, LocationRequest,
    PixelAspect, Playable, Result, Size,
};

/// The family, as a window's roster lists it.
#[derive(Debug)]
pub struct Parsec;

/// The one value of [`Parsec`], for a roster line to point at.
pub static PARSEC: Parsec = Parsec;

/// A game's card, from the names the engine keeps for it.
fn card(title: Title) -> GameCard {
    GameCard::new(title.name(), title.short(), title.needs())
}

impl Family for Parsec {
    fn name(&self) -> &'static str {
        "PARSEC"
    }

    /// The games the engine plays.
    fn games(&self) -> &'static [GameCard] {
        static CARDS: std::sync::OnceLock<Vec<GameCard>> = std::sync::OnceLock::new();
        CARDS.get_or_init(|| Title::ALL.into_iter().map(card).collect())
    }

    /// A directory holding the game's room container.
    fn detect(&self, dir: &Path) -> Option<GameCard> {
        detect::identify(dir).map(card)
    }

    fn open(&self, dir: &Path) -> Result<Box<dyn Playable>> {
        Ok(playable(Box::new(Game::open(dir)?)))
    }
}

/// A runtime the family opened itself, behind the contract.
///
/// What [`Family::open`] does after the engine has opened a game, for a
/// caller that has to touch the engine first — one that skips the title
/// program through `Game::skip_title`, say, before it drives the game the
/// way a window does.
pub fn playable(game: Box<dyn Runtime>) -> Box<dyn Playable> {
    Box::new(Front { game })
}

/// The front door's own face of the contract: one opened game, driven
/// through the engine's [`Runtime`]. Every method forwards; the reason it
/// has to is the orphan rule, and this is the crate it lands in.
struct Front {
    /// The opened game.
    game: Box<dyn Runtime>,
}

impl Playable for Front {
    fn name(&self) -> &str {
        self.game.name()
    }

    fn display_size(&self) -> Size {
        self.game.display_size()
    }

    fn pixel_aspect(&self) -> PixelAspect {
        self.game.pixel_aspect()
    }

    fn seed(&mut self, seed: u64) {
        self.game.seed(seed);
    }

    fn start(&mut self) -> Result<()> {
        Ok(self.game.start()?)
    }

    fn step(&mut self) -> Result<()> {
        Ok(self.game.step()?)
    }

    fn pointer(&mut self, x: i32, y: i32) {
        self.game.pointer(x, y);
    }

    fn button(&mut self, which: Button, down: bool) {
        self.game.button(which, down);
    }

    fn key_down(&mut self, press: &KeyPress) {
        self.game.key_down(press);
    }

    fn key_up(&mut self, press: &KeyPress) {
        self.game.key_up(press);
    }

    fn frame(&mut self) -> Frame<'_> {
        self.game.frame()
    }

    fn frame_duration(&self) -> Option<std::time::Duration> {
        self.game.frame_duration()
    }

    /// The game's drivers, from its own files, and the sink on the game —
    /// before `start`, because the title program's first song comes before
    /// any input. A copy whose library or drivers will not read
    /// answers why sound is off.
    fn open_music(&mut self, rate: u32) -> Result<Option<Box<dyn AudioSource>>> {
        let driver = self.game.music_driver()?;
        let dac = self.game.dac_driver()?;
        let (source, sink) = music::open_music(&driver, dac, rate)?;
        self.game.set_music(sink);
        Ok(Some(source))
    }

    fn set_saves(&mut self, dir: &Path) -> Result<()> {
        Ok(self.game.set_saves(dir)?)
    }

    fn saves(&self) -> Option<&Path> {
        self.game.saves()
    }

    fn diagnostics(&self) -> Vec<Diagnostic> {
        self.game.diagnostics()
    }

    fn finished(&self) -> bool {
        self.game.finished()
    }

    fn request_location(&mut self, at: LocationRequest) -> Result<()> {
        Ok(self.game.request_location(at)?)
    }

    fn start_location(&self) -> Option<i32> {
        self.game.start_location()
    }

    fn location_name(&self) -> Option<&str> {
        self.game.location_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch directory holding empty files of these names.
    fn holding(name: &str, files: &[&str]) -> std::path::PathBuf {
        let dir = motionvm_parsec_testutil::saves_dir(&format!("ds1-front-{name}"));
        for f in files {
            std::fs::write(dir.join(f), b"").expect("a scratch file");
        }
        dir
    }

    #[test]
    fn the_roster_lists_what_plays() {
        let names: Vec<&str> = PARSEC.games().iter().map(|c| c.name).collect();
        assert_eq!(names, ["Dunkle Schatten", "Captain Zins"]);
    }

    #[test]
    fn the_room_container_is_claimed() {
        let dir = holding("ds1", &["DSROOM.DAT"]);
        let found = PARSEC.detect(&dir).expect("claimed");
        assert_eq!(found.short, "Dunkle Schatten");
        assert_eq!(found.needs, "RESOURCE/DSROOM.DAT beside XP01.EXE");
    }

    #[test]
    fn the_second_title_s_room_container_is_claimed() {
        let dir = holding("capzins", &["CZROOM.DAT"]);
        let found = PARSEC.detect(&dir).expect("claimed");
        assert_eq!(found.short, "Captain Zins");
        assert_eq!(found.needs, "RESOURCE/CZROOM.DAT beside XP01.EXE");
    }

    #[test]
    fn a_directory_without_a_title_is_not_claimed() {
        assert_eq!(PARSEC.detect(&holding("none", &["README.TXT"])), None);
    }

    /// Driven through the contract alone, the game starts, steps and lends
    /// a frame of the size it says.
    #[test]
    fn a_game_plays_through_the_contract() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        let mut game = PARSEC.open(&dir).expect("open");
        assert_eq!(
            game.pixel_aspect(),
            PixelAspect {
                width: 5,
                height: 6
            }
        );
        game.start().expect("start");
        game.request_location(LocationRequest::at(2))
            .expect("the Zimmer");
        assert_eq!(game.start_location(), Some(2));
        game.step().expect("a step");
        assert!(!game.finished());
        let size = game.display_size();
        let shown = game.frame();
        assert_eq!(
            (shown.pixels.width, shown.pixels.height),
            (size.width, size.height)
        );
    }

    /// The same through the contract for *Captain Zins*: it starts, is
    /// sent to its first location, steps and lends a frame.
    #[test]
    fn the_second_title_plays_through_the_contract() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_capzins() else {
            eprintln!("skipping: no Captain Zins gamedata directory");
            return;
        };
        let mut game = PARSEC.open(&dir).expect("open");
        game.start().expect("start");
        game.request_location(LocationRequest::at(1))
            .expect("the Wohnung");
        assert_eq!(game.start_location(), Some(1));
        game.step().expect("a step");
        assert!(!game.finished());
        let size = game.display_size();
        let shown = game.frame();
        assert_eq!(
            (shown.pixels.width, shown.pixels.height),
            (size.width, size.height)
        );
    }
}
