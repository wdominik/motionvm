//! What the engine knows about each game it runs, kept apart from the engine
//! itself: the files a game ships, the words its bootstrap names, the script
//! variables its input goes through. One module per game, one per engine
//! generation for what the generation's games share, and the one interface a
//! window drives all of them through.

use std::path::Path;

use motionvm_render::Framebuffer;

use crate::MusicSink;
use crate::Result;

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
/// A `DATA.-1-` is one of the four 16-bit games, and which one is the
/// **engine binary** beside it: all of them ship a container of that name, and
/// none ships another's binary. A `DATA.-1-` with none of them is not a game
/// this can open, and saying so beats naming one of them and then failing on
/// its missing files. Otherwise a `NNN.RSC` container is the 32-bit game. Every lookup is
/// case-insensitive, because a copied install is often lower-cased.
///
/// This reads file names and nothing else, and file names are as far as they
/// go on the 32-bit side: MOTION made more games than the five here, and
/// Checker 2000 ships `NNN.RSC` beside an `ENGINE.EXE` exactly as Dunkle
/// Schatten 2 does, so this answers `DunkleSchatten2` for it. What settles
/// that case is the container's own script, which the opener asks for with
/// [`ds2::SIGNATURE`].
pub fn detect(dir: &Path) -> Option<Title> {
    if motionvm_formats::find_ci(dir, "DATA.-1-").is_some() {
        if motionvm_formats::find_ci(dir, enviro::ENGINE).is_some() {
            return Some(Title::DieEnviroKidsGreifenEin);
        }
        if motionvm_formats::find_ci(dir, jeffjet::ENGINE).is_some() {
            return Some(Title::JeffJet);
        }
        if motionvm_formats::find_ci(dir, hfa::ENGINE).is_some() {
            return Some(Title::HilfeFuerAmajambere);
        }
        if motionvm_formats::find_ci(dir, vloomes::ENGINE).is_some() {
            return Some(Title::VictorLoomes);
        }
        return None;
    }
    motion32::has_container(dir).then_some(Title::DunkleSchatten2)
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
    fn start(&mut self) -> Result<()>;
    /// One frame of whatever is running. Returns whether it is still going.
    fn pump(&mut self) -> Result<bool>;
    /// One step of the game — what the original engine's native loop does
    /// once per frame.
    fn step(&mut self) -> Result<()>;
    /// Hands the game this frame's input.
    fn set_input(&mut self, x: i32, y: i32, left: bool, right: bool, key: i32) -> Result<()>;
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
    fn set_saves(&mut self, dir: &Path) -> Result<()>;
    /// Whether the game has run to its end.
    fn finished(&self) -> bool;
    /// Asks the game to begin at location `n` instead of where it would.
    fn request_location(&mut self, n: i32) -> Result<()>;
    /// The location the game itself wants to begin at, if it says.
    fn start_location(&self) -> Option<i32>;
}

/// Opens the game in `dir`, whichever it is.
///
/// The error names the directory and what it lacks, the way each game's own
/// opener does; a directory holding none of their files says so, and lists
/// what each of them would need.
pub fn open(dir: &Path) -> Result<Box<dyn Playable>> {
    match detect(dir) {
        Some(Title::HilfeFuerAmajambere) => Ok(Box::new(hfa::open(dir)?)),
        Some(Title::DunkleSchatten2) => {
            Ok(Box::new(crate::Game::<motionvm_forth::m32::Vm>::open(dir)?))
        }
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
