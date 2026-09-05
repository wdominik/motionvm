//! Where saved games go, and the module table a savegame is made of.
//!
//! The slot table is not bookkeeping for its own sake: `=>PUTAS` (`0x6559e`)
//! walks descriptor slots 1 to 31 and writes each occupied one, `=>GETAS`
//! (`0x657a4`) walks the same slots and copies back, so *which* modules a
//! savegame carries and *in what order* is exactly what this holds. `=>GET`
//! takes the first free slot and `=>ERASE` frees one in place, which is what
//! makes the order reproducible — and what the word lookup walks.
//!
//! The directory is `None` until a caller names one. The original writes its
//! slots beside `ENGINE.EXE`; this refuses to write into the game directory at
//! all, so a run that was never given somewhere else simply cannot save, and
//! says so by name.

/// The save directory, the slot table, and the flips a load has to replay.
#[derive(Debug, Default)]
pub(crate) struct Persistence {
    /// Where savegames go, and the only directory anything here ever writes to.
    ///
    /// The original has no such notion: `PUT`, `=>PUTAS` and `PUTANIM` build
    /// their names from bare templates (`"#F0R3i.blk"` at 0xd4872 and its
    /// siblings) with no path component, so a save lands beside `ENGINE.EXE` —
    /// in among the game's own data. That is exactly what must not happen here,
    /// so the directory is explicit, it is separate, and [`crate::Engine::set_saves`]
    /// refuses one that lies inside the game data.
    ///
    /// `None` means no saving: the reading words answer as if the slot were
    /// empty, and the writing words stop by name rather than pick a directory
    /// of their own.
    pub(crate) dir: Option<std::path::PathBuf>,

    /// The game whose slots those are, as `Title::slug` names it.
    ///
    /// It is already the last component of the directory — `saves/enviro/` —
    /// and that is what keeps two games apart in practice. It goes into the
    /// header as well because a directory is a convention this port invented
    /// and a file can be moved: a slot carried into another game's directory
    /// by hand is then refused as that other game's, by name, instead of
    /// failing somewhere inside a module image.
    ///
    /// `""` until a directory is set, which is also when nothing can be
    /// written: the two arrive together and there is no moment where a save
    /// could be made with no name to put in it.
    pub(crate) slug: &'static str,

    /// Which module sits in each of the resource manager's descriptor slots.
    ///
    /// The table beside the memory: `=>GET` loads a module and takes it a
    /// slot here, `=>ERASE` gives both back. *Which* modules are resident, and
    /// in what order, is not a detail: `=>PUTAS` writes exactly that set, in
    /// exactly that order, and a savegame that carried more would restore
    /// state the original throws away on every change of location.
    ///
    /// Slot 0 is the kernel's own `Basismodul` and never holds a game module;
    /// `=>PUTAS` starts its walk at 1, which is why the array is indexed the
    /// same way. Thirty-two slots, from `[0xDB027]`.
    ///
    /// The sequence is fixed by the bytecode, not guessed. `SYSTEM.RSC` is two
    /// lines, `4 =>GET` and `START`; `4:START` at 0x5420 does `2 5 6 11 13 3
    /// =>GET`, runs `STARTUP`, `3 =>ERASE`, `12 =>GET`, `DS_INIT`, `12
    /// =>ERASE`; and `5:INCLLOC` frees the old location's three modules before
    /// it takes the new one's, so those keep slots 7, 8 and 9 across every
    /// change of location. In a running game that comes out as
    /// `[4, 2, 5, 6, 11, 13, L+100, L+200, L+300]`.
    pub(crate) slots: [Option<u32>; 32],

    /// Every sprite `GFXVFLIP` has mirrored, as `(source, destination)`.
    ///
    /// The mirrored copy lands in the graphics pool under an id that no
    /// resource file contains, so it cannot be loaded again — a savegame has to
    /// carry the instruction to make it. Only the ids: the source is a real
    /// resource, so redoing the flip costs nothing and keeps a savegame from
    /// carrying pixels.
    pub(crate) flips: Vec<(u32, u32)>,
}
