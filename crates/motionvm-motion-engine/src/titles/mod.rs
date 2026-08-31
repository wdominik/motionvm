//! What the engine knows about each game it runs, kept apart from the engine
//! itself: the files a game ships, the words its bootstrap names, the script
//! variables its input goes through. One module per game, one per engine
//! generation for what the generation's games share, and the openers behind
//! the family's one interface, [`Driven`].

use std::path::Path;

use crate::Result;

use crate::MusicSink;
use motionvm_playable::{Button, KeyPress, PixelAspect};

/// A game the family's front door drives — the family's own side of the
/// window's contract.
///
/// The shape mirrors `motionvm_playable::Playable` method for method, minus
/// the music opening: the engine deliberately owns no codec, so the sink it
/// plays into arrives from outside through [`Driven::set_music`], installed
/// by the front door before it starts the game. Errors are this crate's own
/// [`crate::Error`]; the front door boxes them at the seam. `Send`, so a
/// game may be opened on a worker thread.
pub trait Driven: Send {
    /// The game's full name, as a window shows it.
    fn name(&self) -> &str;
    /// The size of the picture [`Driven::render`] answers with.
    fn display_size(&self) -> (u16, u16);
    /// The shape of one of that picture's pixels on the game's own monitor.
    fn pixel_aspect(&self) -> PixelAspect;
    /// Begins the game and returns once it is parked in its own frame loop.
    fn start(&mut self) -> Result<()>;
    /// One step of the game — one round of the original's native loop.
    fn step(&mut self) -> Result<()>;
    /// Moves the pointer the game reads, in its own coordinates.
    fn pointer(&mut self, x: i32, y: i32);
    /// Takes one button transition; the level follows it, as `MOUSELK` read.
    fn button(&mut self, which: Button, down: bool);
    /// Takes one key transition; only presses reach the keyboard buffer.
    fn key(&mut self, press: &KeyPress, down: bool);
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
    /// The frame to show, pointer and all.
    fn render(&mut self) -> motionvm_render::Framebuffer;
    /// The palette the frame's indices mean.
    fn palette(&self) -> &motionvm_render::Palette;
    /// How long the frame about to run should last, or `None` for no wait.
    fn frame_duration(&self) -> Option<std::time::Duration>;
    /// Where the music goes — installed by the front door before
    /// [`Driven::start`], where the first song can already fire.
    fn set_music(&mut self, sink: Box<dyn MusicSink>);
    /// Points saving and loading at a directory; the game adds its own name.
    fn set_saves(&mut self, dir: &Path) -> Result<()>;
    /// Where saving and loading go, or `None` while there is nowhere.
    fn saves(&self) -> Option<&Path>;
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

/// Which of the engine's two generations a game runs on.
///
/// The engine knows this several times over — which container is open, which
/// savegame layout is written, which of the capability flags are set — and
/// none of that is anything a caller should have to reconstruct. The roster
/// answers it in one place — [`Title::generation`] — and the family's music
/// opener is who asks: the two stacks are different code over different
/// files. Everything else driving a game is the same for both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    /// The 32-bit engine: `ENGINE.EXE`, 32-bit cells, `NNN.RSC` containers,
    /// HMI music.
    Motion32,
    /// The 16-bit engine: `ENVIRO.EXE` and its older builds, 16-bit cells, a
    /// `DATA.-n-` container, PSM 2 music.
    Motion16,
}

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
