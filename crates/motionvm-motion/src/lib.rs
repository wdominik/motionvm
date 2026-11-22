//! The MOTION family's front door: the crate a platform reaches the family
//! through, and the only one that knows the engine and the audio stacks both.
//!
//! `motionvm-motion-engine` deliberately has no audio dependency — a song
//! leaves it as bytes over the contract's `MusicSink`, and the engine never
//! learns the codec. `motionvm-motion-audio` knows the codecs and no engine.
//! Pairing the two is exactly this crate's job, and only this crate's: a
//! platform links the family here and needs neither half by name.

mod music;

use motionvm_motion_engine::titles;
use motionvm_motion_engine::titles::Driven;
use motionvm_playable::{AudioSource, Family, GameCard, LocationRequest, Playable, Result};
use std::path::Path;

/// The family, as a window's roster lists it.
#[derive(Debug)]
pub struct Motion;

/// The one value of [`Motion`], for a roster line to point at.
pub static MOTION: Motion = Motion;

impl Family for Motion {
    fn name(&self) -> &'static str {
        "MOTION"
    }

    /// Built from [`titles::Title::ALL`] once, so the roster a window shows
    /// cannot fall behind the roster the openers serve — and built at start-up
    /// rather than per call, because a usage text and a refusal both ask for
    /// it and neither wants a fresh `Vec`.
    fn games(&self) -> &'static [GameCard] {
        static CARDS: std::sync::OnceLock<Vec<GameCard>> = std::sync::OnceLock::new();
        CARDS.get_or_init(|| {
            titles::Title::ALL
                .iter()
                .map(|t| GameCard::new(t.name(), t.short(), t.needs()))
                .collect()
        })
    }

    fn detect(&self, dir: &Path) -> Option<GameCard> {
        titles::detect(dir).map(|t| GameCard::new(t.name(), t.short(), t.needs()))
    }

    fn open(&self, dir: &Path) -> Result<Box<dyn Playable>> {
        Ok(Box::new(Front {
            game: titles::open(dir)?,
            dir: dir.to_path_buf(),
        }))
    }
}

/// The front door's own face of the contract: one opened game, driven
/// through the family-internal [`Driven`] trait, with the one thing the
/// engine cannot do for itself — opening its music — done here, where both
/// halves are known.
///
/// Every method below forwards. That is the orphan rule's doing and this is
/// the crate it lands in; the reason is written once, on [`Driven`].
struct Front {
    /// The opened game, behind the family's own trait.
    game: Box<dyn Driven>,
    /// The game directory, kept for [`Playable::open_music`]: the driver
    /// files come from where the game is.
    dir: std::path::PathBuf,
}

impl Playable for Front {
    fn name(&self) -> &str {
        self.game.name()
    }

    fn display_size(&self) -> motionvm_playable::Size {
        self.game.display_size()
    }

    fn pixel_aspect(&self) -> motionvm_playable::PixelAspect {
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

    fn button(&mut self, which: motionvm_playable::Button, down: bool) {
        self.game.button(which, down);
    }

    fn key_down(&mut self, press: &motionvm_playable::KeyPress) {
        self.game.key_down(press);
    }

    fn key_up(&mut self, press: &motionvm_playable::KeyPress) {
        self.game.key_up(press);
    }

    fn wheel(&mut self, dx: f32, dy: f32) {
        self.game.wheel(dx, dy);
    }

    fn text(&mut self, text: &str) {
        self.game.text(text);
    }

    fn modifiers(&mut self, shift: bool, ctrl: bool, alt: bool) {
        self.game.modifiers(shift, ctrl, alt);
    }

    fn frame(&mut self) -> motionvm_playable::Frame<'_> {
        self.game.frame()
    }

    fn frame_duration(&self) -> Option<std::time::Duration> {
        self.game.frame_duration()
    }

    /// Every game this family plays ships music, so the answer is a source
    /// or a reason, never `None` — and the sink lands on the game here,
    /// before `start`, which is the whole reason the front door holds the
    /// directory.
    fn open_music(&mut self, rate: u32) -> Result<Option<Box<dyn AudioSource>>> {
        let (source, sink) = music::open_music(&self.dir, self.game.generation(), rate)?;
        self.game.set_music(sink);
        Ok(Some(source))
    }

    fn set_saves(&mut self, dir: &Path) -> Result<()> {
        Ok(self.game.set_saves(dir)?)
    }

    fn saves(&self) -> Option<&Path> {
        self.game.saves()
    }

    fn diagnostics(&self) -> Vec<motionvm_playable::Diagnostic> {
        self.game.diagnostics()
    }

    fn finished(&self) -> bool {
        self.game.finished()
    }

    fn request_location(&mut self, at: LocationRequest) -> Result<()> {
        let n = location_of(at, self.game.name())?;
        Ok(self.game.request_location(n)?)
    }

    fn start_location(&self) -> Option<i32> {
        self.game.start_location()
    }
}

/// The location number a request comes to in this family, or the refusal of
/// an arrival point.
///
/// A MOTION game that can be asked for a location is asked by writing its
/// number into the game's own request variable, which the scripts read on
/// the next frame — one number, so a request carries no way in, and the
/// arrival is whatever the location's own entry runs; a game that cannot be
/// asked has no way in either. An `entry` is refused by name rather than
/// dropped, so a request is never quietly a different one.
fn location_of(at: LocationRequest, game: &str) -> Result<i32> {
    match at.entry {
        None => Ok(at.location),
        Some(entry) => Err(format!(
            "{game} has no arrival points to enter a location through; \
             there is no arrival point {entry}"
        )
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_location_passes_through_by_its_number() {
        assert_eq!(location_of(LocationRequest::at(5), "Game").unwrap(), 5);
    }

    /// The message is what the player reads, so it is pinned.
    #[test]
    fn an_arrival_point_is_refused_by_name() {
        let e = location_of(LocationRequest::via(5, 2), "Game").unwrap_err();
        assert_eq!(
            e.to_string(),
            "Game has no arrival points to enter a location through; \
             there is no arrival point 2"
        );
    }
}
