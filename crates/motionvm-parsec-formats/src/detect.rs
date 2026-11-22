//! Whether a directory holds the PARSEC game this engine plays.
//!
//! Identification is by the file the original opens, not by a version
//! string: *Dunkle Schatten* ships packed `XP0n.EXE` binaries and a
//! `RESOURCE/` folder, and its room container `RESOURCE/DSROOM.DAT` is what
//! tells a copy of it from any other directory. `XP01.EXE` alone does not:
//! it is a name, not a signature.
//!
//! A flattened copy that put the DAT files next to the EXEs still matches:
//! [`resource_dir`] looks in `RESOURCE/` when it exists and in `dir` itself
//! otherwise.

use crate::find_ci;
use std::path::{Path, PathBuf};

/// A PARSEC game this engine plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Title {
    /// *Dunkle Schatten* (1994). `DSROOM.DAT` contains the room resources,
    /// including the RDV playfields.
    DunkleSchatten,
}

impl Title {
    /// Every game, in the order [`identify`] looks for their room containers.
    pub const ALL: [Title; 1] = [Self::DunkleSchatten];

    /// The key the game's files are found under, never a word in a
    /// sentence: the folder its install is usually unpacked into, in lower
    /// case. It names the game's save directory (`saves/<slug>/`), its
    /// test data's environment variable (`MOTIONVM_GAMEDATA_<SLUG>`) and
    /// its place in logs and `--help`. The namespace is shared with every
    /// game's slug in the workspace.
    pub fn slug(self) -> &'static str {
        match self {
            Self::DunkleSchatten => "ds1",
        }
    }

    /// The original German title, as the box and the BAT file spell it.
    pub fn name(self) -> &'static str {
        match self {
            Self::DunkleSchatten => "Dunkle Schatten",
        }
    }

    /// The title without a subtitle — what prose calls it. *Dunkle
    /// Schatten* carries none, so it is its name.
    pub fn short(self) -> &'static str {
        self.name()
    }

    /// The file that identifies a copy, the way a player is told where to
    /// find it: the room container the game ships.
    pub fn needs(self) -> &'static str {
        match self {
            Self::DunkleSchatten => "RESOURCE/DSROOM.DAT beside XP01.EXE",
        }
    }
}

/// The `RESOURCE/` subdirectory when present, otherwise `dir` itself.
pub fn resource_dir(dir: &Path) -> PathBuf {
    find_ci_dir(dir, "RESOURCE").unwrap_or_else(|| dir.to_path_buf())
}

/// [`find_ci`] for a directory: the exact name when it is one, otherwise
/// the first differently-cased spelling in byte order.
fn find_ci_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.is_dir() {
        return Some(exact);
    }
    let names = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(str::to_owned));
    crate::first_ci(names, name).map(|n| dir.join(n))
}

/// The room container each game opens, in the order [`identify`] looks.
const ROOM_FILES: [(&str, Title); 1] = [("DSROOM.DAT", Title::DunkleSchatten)];

/// Which game `dir` holds, or `None` when it holds none.
pub fn identify(dir: &Path) -> Option<Title> {
    let res = resource_dir(dir);
    ROOM_FILES
        .iter()
        .find(|(file, _)| find_ci(&res, file).is_some())
        .map(|&(_, title)| title)
}

/// A file from `dir` or its `RESOURCE/` folder, whatever case it is spelled
/// in. Looks in `RESOURCE/` first, then `dir`.
pub fn game_file(dir: &Path, name: &str) -> Option<PathBuf> {
    find_ci(&resource_dir(dir), name).or_else(|| find_ci(dir, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch directory holding empty files of these names.
    fn holding(name: &str, files: &[&str]) -> PathBuf {
        let dir = motionvm_parsec_testutil::saves_dir(&format!("ds1-detect-{name}"));
        for f in files {
            std::fs::write(dir.join(f), b"").expect("a scratch file");
        }
        dir
    }

    #[test]
    fn the_room_container_is_the_game() {
        let dir = holding("one", &["DSROOM.DAT"]);
        assert_eq!(identify(&dir), Some(Title::DunkleSchatten));
        let dir = holding("none", &["PANELS.DAT"]);
        assert_eq!(identify(&dir), None);
    }
}
