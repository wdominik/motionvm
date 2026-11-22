//! The game's location table, for reading rather than for playing.
//!
//! The table is in the executable, so answering "what locations does this
//! game have" needs the engine's reader and not the format readers alone.
//! It is separate from [`crate::Game`] because opening a game and listing
//! its locations are different questions: listing needs `XP01.EXE` and
//! nothing that playing needs beside it.

use std::path::Path;

use motionvm_parsec_formats::detect;

use crate::exe::Image;
use crate::titles::xp1994::{Locations, Rule};

use crate::error::{Error, Result};

/// One row of a location table, as far as it has been read.
#[derive(Debug, Clone)]
pub struct Location {
    /// The number the program dispatches on.
    pub number: usize,
    /// What the row calls itself. `None` when it carries no pointer;
    /// `Some("")` when it carries one to an empty string.
    pub name: Option<String>,
    /// The word at `+7`, which the main loop reads before it dispatches
    /// and treats as "not a place" when it is zero.
    pub kind: u16,
    /// Whether the row names a handler inside the image.
    pub playable: bool,
    /// The exits the row carries, as `(id, destination)`. Two parallel
    /// byte lists in the record, which the main loop resolves itself.
    pub exits: Vec<(u8, u8)>,
    /// The object ids the row lists, the ones the entry function walks.
    pub objects: Vec<u8>,
    /// The location's own interaction rules, which the entry function
    /// installs and the search reaches before the global ones.
    pub rules: Vec<Rule>,
}

/// Every location the game in `dir` has.
pub fn locations(dir: &Path) -> Result<Vec<Location>> {
    detect::identify(dir).ok_or(Error::Unrecognized {
        missing: Vec::new(),
    })?;
    let program = "XP01.EXE";
    // No expected length: this is a reader, and refusing a build it has
    // not measured would defeat the point of being able to look.
    let image = Image::open_any(dir, program)?;
    let table = Locations::read(&image)?;
    Ok((0..table.count)
        .map(|n| Location {
            number: n,
            name: table.names.get(n).cloned().flatten(),
            kind: table.kind(&image, n).unwrap_or(0),
            playable: table.handler(&image, n).is_some(),
            exits: table.exits(&image, n),
            objects: table.objects(&image, n),
            rules: table.rules(&image, n),
        })
        .collect())
}

/// Every rule of the game's global interaction table.
///
/// What a verb does to an object is a table rather than a switch, and the
/// table is found by the shape of its installer.
pub fn rules(dir: &Path) -> Result<Vec<Rule>> {
    detect::identify(dir).ok_or(Error::Unrecognized {
        missing: Vec::new(),
    })?;
    Rule::read_table(&Image::open_any(dir, "XP01.EXE")?)
}
