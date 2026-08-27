//! What the engine knows about each game it runs, kept apart from the engine
//! itself: the files a game ships, the words its bootstrap names, the script
//! variables its input goes through. One module per game, one per engine
//! generation for what the generation's games share, and the one interface a
//! window drives all of them through.

use std::path::Path;

use motionvm_render::Framebuffer;

use crate::MusicSink;
use crate::game::Res;

pub mod ds2;
pub mod enviro;
pub mod jeffjet;
pub mod motion16;

/// The games motionvm knows, by the files they ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Title {
    /// *Im Netzwerk gefangen – Dunkle Schatten 2*, on the 32-bit engine:
    /// `NNN.RSC` containers beside `ENGINE.EXE`.
    DunkleSchatten2,
    /// *Die Enviro-Kids greifen ein*, on the 16-bit engine: one `DATA.-1-`
    /// beside `ENVIRO.EXE`.
    EnviroKids,
    /// *Jeff Jet - Abenteuer InfoHighway*, on the 16-bit engine as well:
    /// `DATA.-1-` and `DATA.-2-` beside `HPPLAY.EXE`.
    JeffJet,
}

impl Title {
    /// The game's title, as a window shows it.
    pub fn name(self) -> &'static str {
        match self {
            Title::DunkleSchatten2 => "Dunkle Schatten 2",
            Title::EnviroKids => "Die Enviro-Kids greifen ein",
            Title::JeffJet => "Jeff Jet - Abenteuer InfoHighway",
        }
    }
}

/// Which game a directory holds, told by its files — or `None` for none of
/// them.
///
/// A `DATA.-1-` is one of the two 16-bit games, and which one is the **engine
/// binary** beside it: both ship a container of that name, and neither ships
/// the other's binary. A `DATA.-1-` with neither is not a game this can open,
/// and saying so beats naming one of them and then failing on its missing
/// files. Otherwise a `NNN.RSC` container is the 32-bit game. Every lookup is
/// case-insensitive, because a copied install is often lower-cased.
///
/// This reads file names and nothing else, and file names are as far as they
/// go on the 32-bit side: MOTION made more games than the three here, and
/// Checker 2000 ships `NNN.RSC` beside an `ENGINE.EXE` exactly as Dunkle
/// Schatten 2 does, so this answers `DunkleSchatten2` for it. What settles
/// that case is the container's own script, which the opener asks for with
/// [`ds2::SIGNATURE`].
pub fn detect(dir: &Path) -> Option<Title> {
    if motionvm_formats::find_ci(dir, "DATA.-1-").is_some() {
        if motionvm_formats::find_ci(dir, enviro::ENGINE).is_some() {
            return Some(Title::EnviroKids);
        }
        if motionvm_formats::find_ci(dir, jeffjet::ENGINE).is_some() {
            return Some(Title::JeffJet);
        }
        return None;
    }
    let rsc = std::fs::read_dir(dir).ok()?.flatten().any(|e| {
        let p = e.path();
        p.extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| x.eq_ignore_ascii_case("rsc"))
            && p.file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
    });
    rsc.then_some(Title::DunkleSchatten2)
}

/// A game a window can drive, whichever machine it runs on.
///
/// Exactly what `motionvm-app` needs and nothing more: start it, step it,
/// feed it input, take the picture and the palette, know how long a frame
/// lasts, give it music and a place to save, ask whether it has ended. The
/// engine behind it stays out of reach — a frontend that could reach in and
/// move a descriptor would be able to produce a picture the original never
/// could.
pub trait Playable {
    /// Which game this is.
    fn title(&self) -> Title;
    /// The size of the picture [`Playable::render`] answers with.
    fn display_size(&self) -> (u16, u16);
    /// The shape of one of that picture's pixels on the game's own monitor,
    /// as height:width. `(1, 1)` — square — unless the title says otherwise:
    /// the display modes of the time were all shown on 4:3 screens, and a
    /// mode whose grid is not 4:3 had pixels stretched to make up the
    /// difference. A frontend that wants to show the picture as the player
    /// of 1996 saw it scales its two axes in this ratio.
    fn pixel_aspect(&self) -> (u32, u32) {
        (1, 1)
    }
    /// Begins the game the way it begins itself.
    fn start(&mut self) -> Res<()>;
    /// One frame of whatever is running. Returns whether it is still going.
    fn pump(&mut self) -> Res<bool>;
    /// One step of the game — what the original engine's native loop does
    /// once per frame.
    fn step(&mut self) -> Res<()>;
    /// Hands the game this frame's input.
    fn set_input(&mut self, x: i32, y: i32, left: bool, right: bool, key: i32) -> Res<()>;
    /// The frame to show, pointer and all.
    fn render(&mut self) -> Framebuffer;
    /// The palette the frame's indices mean.
    fn palette(&self) -> &motionvm_formats::Palette;
    /// How long to wait before the next frame, as the game asked; `None`
    /// until it has.
    fn frame_duration(&self) -> Option<std::time::Duration>;
    /// Where the music goes. Without one, the game plays silently.
    fn set_music(&mut self, sink: Box<dyn MusicSink>);
    /// Points saving and loading at a directory.
    fn set_saves(&mut self, dir: &Path) -> Res<()>;
    /// Whether the game has run to its end.
    fn finished(&self) -> bool;
    /// Asks the game to begin at location `n` instead of where it would.
    fn request_location(&mut self, n: i32) -> Res<()>;
    /// The location the game itself wants to begin at, if it says.
    fn start_location(&self) -> Option<i32>;
}

/// Opens the game in `dir`, whichever it is.
///
/// The error names the directory and what it lacks, the way each game's own
/// opener does; a directory holding none of their files says so, and lists
/// what each of them would need.
pub fn open(dir: &Path) -> Res<Box<dyn Playable>> {
    match detect(dir) {
        Some(Title::DunkleSchatten2) => {
            Ok(Box::new(crate::Game::<motionvm_forth::m32::Vm>::open(dir)?))
        }
        Some(Title::EnviroKids) => Ok(Box::new(enviro::open(dir)?)),
        Some(Title::JeffJet) => Ok(Box::new(jeffjet::open(dir)?)),
        None => {
            if !dir.is_dir() {
                return Err(format!("{}: no such directory", dir.display()).into());
            }
            Err(format!(
                "{} is not a game motionvm can open\n  \
                 Dunkle Schatten 2 needs 001.RSC and ENGINE.EXE\n  \
                 Die Enviro-Kids greifen ein needs DATA.-1- and ENVIRO.EXE\n  \
                 Jeff Jet - Abenteuer InfoHighway needs DATA.-1-, DATA.-2- and HPPLAY.EXE\n  \
                 Another MOTION game, or an incomplete copy of one of these; \
                 see \"Game data\" in the README.",
                dir.display()
            )
            .into())
        }
    }
}
