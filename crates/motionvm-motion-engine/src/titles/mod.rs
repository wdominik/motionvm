//! What the engine knows about each game it runs, kept apart from the engine
//! itself: the files a game ships, the words its bootstrap names, the script
//! variables its input goes through. One module per game, one per engine
//! generation for what the generation's games share, and the openers behind
//! the family's one interface, [`Driven`].

use std::path::Path;

use crate::Result;

use crate::MusicSink;
pub use motionvm_motion_formats::Generation;
use motionvm_playable::{Button, KeyPress, PixelAspect, Size};

/// The shape of a pixel of a picture this size, on the 4:3 monitor the
/// modes of the time were shown on: a mode whose grid is not 4:3 had its
/// pixels stretched to make up the difference, so a pixel is `4h : 3w`,
/// reduced — square at 640×480, 5:6 at 320×200.
pub(crate) fn pixel_aspect_of(size: Size) -> PixelAspect {
    let (w, h) = (u32::from(size.width), u32::from(size.height));
    if w == 0 || h == 0 {
        return PixelAspect::default();
    }
    let (across, down) = (4 * h, 3 * w);
    let common = gcd(across, down);
    PixelAspect {
        width: across / common,
        height: down / common,
    }
}

/// Euclid's, for the ratio above.
fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// A game the family's front door drives — the family's own side of the
/// window's contract.
///
/// The shape mirrors `motionvm_playable::Playable` method for method, minus
/// the music opening: the engine deliberately owns no codec, so the sink it
/// plays into arrives from outside through [`Driven::set_music`], installed
/// by the front door before it starts the game. Errors are this crate's own
/// [`crate::Error`]; the front door boxes them at the seam. `Send`, so a
/// game may be opened on a worker thread.
///
/// **Why there are two traits and twenty forwarding methods.** Rust's orphan
/// rule allows `impl Playable for X` only in the crate that owns `Playable`
/// or the crate that owns `X`. `Playable` is the neutral layer's and
/// `Game<M>` is this crate's, and this crate must not depend on the audio
/// stack — a song leaves it as bytes and it never learns the codec — so the
/// one crate that could write that impl is the one that knows both halves:
/// the family's front door. It writes it over a `Box<dyn Driven>`, which is
/// what this trait is for.
///
/// The forwarding is the price of the seam and not a sign of one too many.
/// It is said here once; the methods below and the impl in `motionvm-motion`
/// do not repeat it.
pub trait Driven: Send {
    /// The game's full name, as a window shows it.
    fn name(&self) -> &str;
    /// Which generation of the engine runs it — what the opener decided,
    /// answered once so nothing has to detect the directory a second time.
    fn generation(&self) -> Generation;
    /// The size of the picture [`Driven::frame`] answers with.
    fn display_size(&self) -> Size;
    /// The shape of one of that picture's pixels on the game's own monitor.
    fn pixel_aspect(&self) -> PixelAspect;
    /// Reseeds what the game draws random numbers from, before
    /// [`Driven::start`]. A caller that never does gets the fixed seed the
    /// machine is built with, and with it the same run every time.
    fn seed(&mut self, seed: u64);
    /// Begins the game and returns once it is parked in its own frame loop.
    fn start(&mut self) -> Result<()>;
    /// One step of the game — one round of the original's native loop.
    fn step(&mut self) -> Result<()>;
    /// Moves the pointer the game reads, in its own coordinates.
    fn pointer(&mut self, x: i32, y: i32);
    /// Takes one button transition; the level follows it, as `MOUSELK` read.
    fn button(&mut self, which: Button, down: bool);
    /// Takes one key press, which is what reaches the keyboard buffer.
    fn key_down(&mut self, press: &KeyPress);
    /// Takes one key release. No game of this family reads one — `?KEY`
    /// answers keystrokes and a keystroke is a press — so the default body
    /// drops it, and the channel exists because the contract has one.
    fn key_up(&mut self, press: &KeyPress) {
        let _ = press;
    }
    /// The scroll wheel, in lines. No game of this family reads it; the
    /// default body drops it, and the channel exists so the front door can
    /// forward without asking.
    fn wheel(&mut self, dx: f32, dy: f32) {
        let _ = (dx, dy);
    }
    /// The typed stream. No game of this family reads it either.
    fn text(&mut self, text: &str) {
        let _ = text;
    }
    /// The modifiers' level state. The family's translator takes its
    /// modifiers off each press instead, so the default body drops this too.
    fn modifiers(&mut self, shift: bool, ctrl: bool, alt: bool) {
        let _ = (shift, ctrl, alt);
    }
    /// The frame to show, pointer and all, with the palette its indices mean.
    fn frame(&mut self) -> motionvm_render::Frame<'_>;
    /// How long the frame about to run should last, or `None` for no wait.
    fn frame_duration(&self) -> Option<std::time::Duration>;
    /// Where the music goes — installed by the front door before
    /// [`Driven::start`], where the first song can already fire.
    fn set_music(&mut self, sink: Box<dyn MusicSink>);
    /// Points saving and loading at a directory; the game adds its own name.
    fn set_saves(&mut self, dir: &Path) -> Result<()>;
    /// Where saving and loading go, or `None` while there is nowhere.
    fn saves(&self) -> Option<&Path>;
    /// What the run has to report about itself; see
    /// `motionvm_playable::Diagnostic`. Per generation, because what a
    /// machine notices is the machine's own.
    fn diagnostics(&self) -> Vec<motionvm_playable::Diagnostic>;
    /// Whether the game has run to its end.
    fn finished(&self) -> bool;
    /// Asks the game to begin at location `n` instead of where it would.
    fn request_location(&mut self, n: i32) -> Result<()>;
    /// The location the game itself wants to begin at, if it says.
    fn start_location(&self) -> Option<i32>;
}

pub mod ds2;
pub mod enviro;
pub mod hfa;
pub mod jeffjet;
pub mod motion16;
pub mod motion32;
pub mod vloomes;

/// The games motionvm knows, by the files they ship.
///
/// A game is named in three registers, and which one to use follows from what
/// the name is for. [`Title::name`] is the full title, the one on the box and
/// in the window. [`Title::short`] drops the part after the dash — the
/// subtitle — and is what prose, error messages and test output use, because
/// a sentence carrying "Im Netzwerk gefangen – Dunkle Schatten 2" twice reads
/// worse than one that says it once. [`Title::slug`] is neither: it is the
/// key the game's files are found under, and it never appears in a sentence.
///
/// The variants are the short form in PascalCase, ASCII for the umlaut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Title {
    /// *Die Enviro-Kids greifen ein*, on the 16-bit engine: one `DATA.-1-`
    /// beside `ENVIRO.EXE`.
    DieEnviroKidsGreifenEin,
    /// *Im Netzwerk gefangen – Dunkle Schatten 2*, on the 32-bit engine:
    /// `NNN.RSC` containers beside `ENGINE.EXE`.
    DunkleSchatten2,
    /// *Hilfe für Amajambere*, on the 16-bit engine: `DATA.-1-` and `DATA.-2-`
    /// beside `BMZ.EXE`.
    HilfeFuerAmajambere,
    /// *Jeff Jet - Abenteuer InfoHighway*, on the 16-bit engine as well:
    /// `DATA.-1-` and `DATA.-2-` beside `HPPLAY.EXE`.
    JeffJet,
    /// *Victor Loomes – Das Spiel*, on the oldest build of the 16-bit engine:
    /// one `DATA.-1-` beside `LL.EXE`, in the earlier container framing.
    VictorLoomes,
}

impl Title {
    /// The game's full title, as a window shows it.
    pub fn name(self) -> &'static str {
        match self {
            Title::DieEnviroKidsGreifenEin => "Die Enviro-Kids greifen ein",
            Title::DunkleSchatten2 => "Im Netzwerk gefangen – Dunkle Schatten 2",
            Title::HilfeFuerAmajambere => "Hilfe für Amajambere",
            Title::JeffJet => "Jeff Jet - Abenteuer InfoHighway",
            Title::VictorLoomes => "Victor Loomes – Das Spiel",
        }
    }

    /// The game's title without its subtitle — what prose calls it.
    ///
    /// Two of the five titles carry a second half after a dash; those lose it.
    /// The other three are already as short as they get and answer the same as
    /// [`Title::name`].
    pub fn short(self) -> &'static str {
        match self {
            Title::DieEnviroKidsGreifenEin => "Die Enviro-Kids greifen ein",
            Title::DunkleSchatten2 => "Dunkle Schatten 2",
            Title::HilfeFuerAmajambere => "Hilfe für Amajambere",
            Title::JeffJet => "Jeff Jet",
            Title::VictorLoomes => "Victor Loomes",
        }
    }

    /// The key the game's own files are kept under.
    ///
    /// It is the name of the directory the original was installed into —
    /// `games/DS2`, `games/ENVIRO` — lower-cased, and everything the program
    /// files away per game is named after it: `MOTIONVM_GAMEDATA_<SLUG>`, the
    /// module in `titles/`, the savegame directory, the documentation page.
    /// It is a key and not a name: it belongs in a path, never in a sentence.
    ///
    /// The namespace it keys is the workspace's, not this family's: every
    /// family's games share `saves/<slug>/`, the `MOTIONVM_GAMEDATA_<SLUG>`
    /// variables and `docs/motion/games/<slug>/`. A slug that collides across
    /// families would put two games' slots in one directory — which is the
    /// very loss `Game::set_saves` exists to prevent — so a new game checks
    /// the whole roster's slugs, not just its own family's.
    pub fn slug(self) -> &'static str {
        match self {
            Title::DieEnviroKidsGreifenEin => "enviro",
            Title::DunkleSchatten2 => "ds2",
            Title::HilfeFuerAmajambere => "hfa",
            Title::JeffJet => "jeffjet",
            Title::VictorLoomes => "vloomes",
        }
    }

    /// The files a copy has to hold, as the start-up message names them.
    ///
    /// Beside [`Title::short`] this is what [`crate::Error::Unrecognized`]
    /// lists, so that the message cannot fall behind the enum: a game added
    /// without a line here does not compile.
    pub fn needs(self) -> &'static str {
        match self {
            Title::DieEnviroKidsGreifenEin => "DATA.-1- and ENVIRO.EXE",
            Title::DunkleSchatten2 => "001.RSC and ENGINE.EXE",
            Title::HilfeFuerAmajambere => "DATA.-1-, DATA.-2- and BMZ.EXE",
            Title::JeffJet => "DATA.-1-, DATA.-2- and HPPLAY.EXE",
            Title::VictorLoomes => "DATA.-1- and LL.EXE",
        }
    }

    /// Which generation of the engine runs the game.
    ///
    /// A total match like the four accessors above it, and for the same
    /// reason: a game added without an answer here does not compile.
    pub fn generation(self) -> Generation {
        match self {
            Title::DieEnviroKidsGreifenEin => Generation::Motion16,
            Title::DunkleSchatten2 => Generation::Motion32,
            Title::HilfeFuerAmajambere => Generation::Motion16,
            Title::JeffJet => Generation::Motion16,
            Title::VictorLoomes => Generation::Motion16,
        }
    }

    /// Every game, in the order the documentation lists them: the 32-bit game
    /// first, then the 16-bit ones as they were taken on.
    pub const ALL: [Title; 5] = [
        Title::DunkleSchatten2,
        Title::DieEnviroKidsGreifenEin,
        Title::JeffJet,
        Title::HilfeFuerAmajambere,
        Title::VictorLoomes,
    ];
}

/// Which game a directory holds, told by its files — or `None` for none of
/// them.
///
/// The container names the generation and nothing more — a `DATA.-1-` is the
/// 16-bit machine's, `NNN.RSC` the 32-bit one's — so which game it is, is a
/// question each generation answers for its own roster: [`motion16`] by the
/// engine binary beside the container, [`motion32`] by the shape, with the
/// container's own script settling what the shape cannot.
/// Every lookup is case-insensitive, because a copied install is often
/// lower-cased.
pub fn detect(dir: &Path) -> Option<Title> {
    if motionvm_motion_formats::find_ci(dir, "DATA.-1-").is_some() {
        return motion16::detect(dir);
    }
    motion32::detect(dir)
}

/// Opens the game in `dir`, whichever it is.
///
/// The error names the directory and what it lacks, the way each game's own
/// opener does; a directory holding none of their files says so, and lists
/// what each of them would need.
pub fn open(dir: &Path) -> Result<Box<dyn Driven>> {
    match detect(dir) {
        Some(Title::HilfeFuerAmajambere) => Ok(Box::new(hfa::open(dir)?)),
        Some(Title::DunkleSchatten2) => Ok(Box::new(ds2::open(dir)?)),
        Some(Title::DieEnviroKidsGreifenEin) => Ok(Box::new(enviro::open(dir)?)),
        Some(Title::JeffJet) => Ok(Box::new(jeffjet::open(dir)?)),
        Some(Title::VictorLoomes) => Ok(Box::new(vloomes::open(dir)?)),
        None => {
            if !dir.is_dir() {
                return Err(crate::Error::NoSuchDirectory {
                    dir: dir.to_path_buf(),
                });
            }
            Err(crate::Error::Unrecognized {
                dir: dir.to_path_buf(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::pixel_aspect_of;
    use motionvm_playable::{PixelAspect, Size};

    /// The two modes the games enter, and the two the table also holds.
    #[test]
    fn a_pixel_is_what_stretches_the_mode_to_a_4_by_3_screen() {
        let aspect = |width, height| pixel_aspect_of(Size { width, height });
        assert_eq!(aspect(640, 480), PixelAspect::default(), "square");
        assert_eq!(
            aspect(320, 200),
            PixelAspect {
                width: 5,
                height: 6
            }
        );
        assert_eq!(aspect(800, 600), PixelAspect::default());
        assert_eq!(aspect(1024, 768), PixelAspect::default());
        assert_eq!(
            aspect(0, 480),
            PixelAspect::default(),
            "no picture, no shape"
        );
    }
}
