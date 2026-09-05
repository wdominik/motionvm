//! What a generation decides when a game opens, and nothing moves afterwards.
//!
//! On one flat engine struct a capability the opener sets once and a counter
//! the drawer bumps every frame would sit side by side, indistinguishable, and
//! "immutable once the game is open" would be a convention — one an opener
//! writing into a built engine breaks without anything noticing.
//!
//! Here it is a type. A `Profile` is `Copy`, has no methods that change it,
//! and reaches the engine through [`crate::Engine::new`]; the two generations'
//! readings are [`Profile::motion32`] and [`Profile::motion16`], and the only
//! thing an opener may still decide is what it *probed out of the binary* —
//! which the 16-bit opener does before the engine exists rather than after.
//!
//! ## What belongs here and what does not
//!
//! A field belongs here when its value is a fact about the engine build the
//! game runs on: what the original's handler does, read at an address, and
//! settled the moment the container is opened. It does not belong here when it
//! is a fact about the game's *state* — where the pointer is, which descriptor
//! is selected, how many frames a fade has left — however constant it happens
//! to look at the start of a run.
//!
//! One field sits on the line and is worth naming: the pointer's initial
//! visibility. It is not a capability — nothing behaves differently because of
//! it — but it *is* the generation's, because the 16-bit engine starts with
//! the pointer unshown and the 32-bit one does not. So it is here, named for
//! what it is.
//!
//! **Never a generation enum.** Nothing below is `if generation == …`; each
//! field is one behavior, with the address that settled it. That is the whole
//! reason a further MOTION build can be added by reading it rather than by
//! extending a match.

use motionvm_motion_formats::Generation;
use motionvm_playable::Size;

/// The engine build a game runs on, as the opener read it.
///
/// The fields are the crate's own: from outside, a profile is one of the two
/// readings below and nothing else, because a caller who could assemble a
/// profile field by field could describe a build that has never existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    /// The size of the picture before the script enters graphics — and, on
    /// a build without `SETRES`, the one `TOGFX` enters: 320×200 for the
    /// 16-bit engine. The 32-bit engine's `TOGFX` sizes the display to the
    /// mode `SETRES` selected ([`Profile::has_setres`]), so 640×480 here is
    /// the mode its one game asks for, and what a frame composed before
    /// `TOGFX` would be.
    pub(crate) display: Size,

    /// Whether a screen refuses its hundred-and-first descriptor: the three
    /// later 16-bit builds test the count against a hundred at the top of
    /// `NEWSETDESC` and jump past every pop and the push, `LL.EXE` takes the
    /// count and raises it without looking. Read off the binary the game was
    /// opened with ([`motionvm_motion_formats::m16::mz::newsetdesc_capped`]);
    /// the 32-bit engine numbers its descriptors across screens and has no
    /// such table.
    pub(crate) screen_holds_a_hundred: bool,

    /// Whether the build has `SETRES`, so that `TOGFX` enters the mode it
    /// selected rather than the one the build is fixed to. The 32-bit
    /// engine's table at `0x13fc0` holds seven modes; the 16-bit kernel has
    /// no such word and its `TOGFX` enters 320×200.
    pub(crate) has_setres: bool,

    /// Whether the pointer is drawn before anything has shown it.
    ///
    /// Not a capability — no handler behaves differently — but the
    /// generation's all the same: the 16-bit engine counts shows and starts at
    /// zero, so nothing is drawn until `SHOWMOUSE` runs.
    pub(crate) pointer_starts_visible: bool,

    /// Whether `?XINSIDE` passes over a hot area whose four corners are all
    /// zero. This is the build's and not the format's: `ENVIRO.EXE`
    /// (`0a40:1b37`) and `BMZ.EXE` test for it, `HPPLAY.EXE` and `LL.EXE` do
    /// not — 101 instructions against 71, and the older two have no `cmpw $0`
    /// in the handler at all. Set from the binary the game was opened with;
    /// the default is the 32-bit reading, where the area search treats a
    /// record of four zeros as a hole and `?XINSIDE` applies the same rule
    /// (the click dispatch at `0x7ce73` and its area walk).
    pub(crate) skips_holes: bool,

    /// Whether `CROUTE` takes a shadow record's zero shrink as 1000 for the
    /// walk's first step. The 32-bit routine does (`0x778ca`) and so does
    /// `ENVIRO.EXE` (`0a40:1176`); `HPPLAY.EXE`, `BMZ.EXE` and `LL.EXE` copy
    /// the field as it stands. Read off the binary the game was opened with
    /// ([`motionvm_motion_formats::m16::mz::croute_defaults_shrink`]); the
    /// default is the 32-bit reading.
    pub(crate) walk_defaults_shrink: bool,

    /// Whether `CROUTE` ends with `LL.EXE`'s pass over the finished buffer
    /// (`0104:516d`), which rewrites the heading of a run of one or two
    /// steps that sits between a run of three or more and a run of one or
    /// more heading the same way, when the short run's heading and theirs
    /// fall on different sides of 2. No other build has it
    /// ([`motionvm_motion_formats::m16::mz::croute_smooths_headings`]).
    pub(crate) walk_smooths_headings: bool,

    /// Whether a block descriptor — `SDBL`, a picture by the same id pool as
    /// a sprite — is copied onto its screen with every pixel, index 0
    /// included.
    ///
    /// The 16-bit drawer (`ENVIRO.EXE` `016a:0aac`) keeps the two kinds
    /// apart by bit 15 of the descriptor's picture cell and takes two paths:
    /// a sprite goes through the keyed blit at `14ee:0d1e`, a block through
    /// the plain copy at `14ee:0d47` — the one the save-under restores with.
    /// Die Enviro-Kids greifen ein builds its backgrounds out of 80-pixel
    /// block strips that are dark where they hold index 0, and drawing those
    /// keyed let the previous location show through. The 32-bit drawer's
    /// block path is not read on this point; the 32-bit game keeps the keyed
    /// blit it has always had.
    ///
    /// Read on the latest build and set for the whole 16-bit generation,
    /// which the earlier framing confirms rather than assumes: Victor Loomes'
    /// intro is eight full-screen block pictures, and all eight match a
    /// recording of the original pixel for pixel. A keyed block path would
    /// have shown the picture before through every index 0 in them.
    pub(crate) opaque_blocks: bool,

    /// Whether text is drawn and measured the run drawer's way
    /// (`ENVIRO.EXE` `016a:0aac` and `14ee:11cf`): the shadow pass shifted by
    /// the template's x/y offsets on an axis that is not centered, `#` eaten
    /// by the run drawer, `SDBLK` justifying a block, and the `GD*` sizes
    /// measured bare — where the 32-bit engine stores them padded by 4.
    ///
    /// Confirmed on the earlier framing the same way as
    /// [`Profile::opaque_blocks`]: three of the eight intro pictures Victor
    /// Loomes holds are text over a picture, and they match to the pixel.
    pub(crate) text_runs: bool,

    /// Whether `SDTB` allocates the text record itself, marking the
    /// descriptor as `SDTXT` would (`0x71d45`). Where it does not, the value
    /// is read as a block until `SDTXT` runs (`05f1:0d7b`).
    pub(crate) sdtb_allocates_text: bool,

    /// Whether `SDTDT` takes only templates 1..=20 (`05f1:0c78`: `cmp $1` /
    /// `jl`, `cmp $0x14` / `jg` skip the store and the dirty mark alike), so
    /// `0 SDTDT` cannot clear a template. The 32-bit handler has no such
    /// gate.
    pub(crate) templates_gated: bool,

    /// Whether `GDTB` answers a sprite with its bit 15 still on
    /// (`05f1:0d5c` reads `+0x10` and masks nothing). Only that machine
    /// keeps the marker in the value; the other has a type field of its own.
    pub(crate) table_marks_sprites: bool,

    /// Whether `SDLEV` re-inserts into the level chain even when the level
    /// does not change — the 16-bit engine's read behavior; see
    /// [`crate::Descriptor::stamp`].
    pub(crate) level_chain: bool,

    /// Whether the `SD*` setters mark the descriptor dirty even when the
    /// value does not change — the 16-bit handlers write and set the bit
    /// unconditionally (`SDX` `05f1:0df2`, `SDFNT` `05f1:1038`, `SDLEV`,
    /// `SDNORM`), where the read 32-bit ones skip an unchanged value
    /// (`SDX` 0x7111a, `SDSPR` 0x71715, `SD%SHR` 0x721e5).
    pub(crate) sd_marks_always: bool,

    /// Whether a descriptor's `SDWORD` callback fires only while the
    /// descriptor is **active**. The 16-bit frame loop's callback walk
    /// (`016a:05d6`–`06da`) gates on `cb != -1 && (flags & 0x80) &&
    /// wait != -1` — flag bit 0x80 is `SDACTIVE` — and it runs with the
    /// descriptor made current (`DS:0x5de2`/`DS:0x3058`), so a skipped
    /// callback also leaves the script's own `SMDESC` selection
    /// standing. The intro leans on that: its `_DINFO` text descriptor
    /// carries `1082 SDWORD` from birth but stays `SDINACTIVE` through
    /// the motif phases, whose per-frame `SDH%SHR`/`SDSPR` writes come
    /// without a re-select.
    pub(crate) callbacks_need_active: bool,

    /// Whether descriptor handles are indices into the *active screen's*
    /// list, as the 16-bit kernel has them: `NEWDESC`/`NEWSETDESC` (`ENVIRO.EXE`
    /// file `0x9bb8`, `0x9bd8`) hand back the screen's count and raise it,
    /// `ACTDESC` (file `0x9b92`) stores the number and nothing else — the pair
    /// screen and number is resolved when a word touches the descriptor — and
    /// `KILLNDESC n` (file `0x9d3a`) frees the active screen's descriptors
    /// from `n` up and sets its count back to `n`. So the same number names
    /// one descriptor on each screen, and a number the scripts compute —
    /// `?LPD 1 +`, the slot after the permanent ones — means what it means
    /// on whichever screen is active. The 32-bit engine's handles are unique
    /// across screens and stay as they were.
    pub(crate) per_screen_descriptors: bool,

    /// Whether this machine's `SHOWMOUSE`/`HIDEMOUSE` keep that counter —
    /// the read 16-bit behavior. The 32-bit pair is unread and keeps the
    /// plain on/off it always had here.
    pub(crate) pointer_counted: bool,

    /// Which game's savegame files this engine writes and reads.
    pub(crate) save_layout: Generation,
}

impl Profile {
    /// The 32-bit engine, as Dunkle Schatten 2 runs on it.
    ///
    /// Every field here is that build's reading. Nothing is probed: there is
    /// one 32-bit engine in the corpus, and a second one would be read the way
    /// the 16-bit builds are.
    pub fn motion32() -> Self {
        Self {
            display: Size {
                width: 640,
                height: 480,
            },
            screen_holds_a_hundred: false,
            has_setres: true,
            pointer_starts_visible: true,
            skips_holes: true,
            walk_defaults_shrink: true,
            walk_smooths_headings: false,
            opaque_blocks: false,
            text_runs: false,
            sdtb_allocates_text: true,
            templates_gated: false,
            table_marks_sprites: false,
            level_chain: false,
            sd_marks_always: false,
            callbacks_need_active: false,
            per_screen_descriptors: false,
            pointer_counted: false,
            save_layout: Generation::Motion32,
        }
    }

    /// The 16-bit engine, as its four games run on it.
    ///
    /// Four of these differ between the five builds and are **probed out of
    /// the shipped binary** by the opener, which corrects them here before the
    /// engine is built: `skips_holes`, `walk_defaults_shrink`,
    /// `walk_smooths_headings` and `screen_holds_a_hundred`. The values below
    /// are the ones
    /// `ENVIRO.EXE` reads as, so a build nobody has probed behaves like the
    /// one that was.
    pub fn motion16() -> Self {
        Self {
            display: Size {
                width: 320,
                height: 200,
            },
            screen_holds_a_hundred: true,
            has_setres: false,
            pointer_starts_visible: false,
            opaque_blocks: true,
            text_runs: true,
            sdtb_allocates_text: false,
            templates_gated: true,
            table_marks_sprites: true,
            level_chain: true,
            sd_marks_always: true,
            callbacks_need_active: true,
            per_screen_descriptors: true,
            pointer_counted: true,
            save_layout: Generation::Motion16,
            ..Self::motion32()
        }
    }
}
