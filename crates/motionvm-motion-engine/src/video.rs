//! The video mode: what `SETRES` selects and `TOGFX` enters.
//!
//! On the 32-bit engine the display is the script's to size. `SETRES`
//! (`0x6efe2`) stores its argument at `0xdb4a0` and selects that mode's
//! parameters (`0x13fc0`): a table of seven modes, each an ordinal, a VESA
//! mode number and a width and height, of which three have a kernel word that
//! pushes their ordinal. `TOGFX` (`0x6ee64`) selects them again, enters the
//! mode (`0x141dc`: the mode set, then a surface of the mode's width and
//! height, its clip rectangle and its damage map, allocated there and then),
//! copies the width and height into the display's own words (`0xf26bc`,
//! `0xf26be`), installs the system palette (`0x14734`, the handle the init
//! filled from `000.pal`), gives the pointer the engine's own arrow and
//! shows it ([`crate::cursor`]), and registers `GFXTO` to run when the
//! program exits. `GFXTO` (`0x6ef45`)
//! returns to text mode 3 and frees the four buffers (`0x1433e`). So the
//! picture has no size until the script enters graphics, and then it has the
//! size the script asked for — which is what the display here follows.
//!
//! A script need not ask. The kernel init (`0x6845a`; R78 `0x56d00`) takes
//! the selection as its second argument and stores it at `0xdb4a0`, and its
//! one caller — the Forth system's init at `0x37297`, R78 `0x2fd5b` — passes
//! 2, `640x480x256`, just before `system.rsc` is read. So a bootstrap without
//! a `SETRES`, which is what Checker 2000's `STARTUP` is, enters 640×480 all
//! the same, and Dunkle Schatten 2's says the same thing once more. The two
//! builds agree on each of these, address for address: `SETRES` is `0x5b050`
//! in R78, the table `0x13e00`, `TOGFX` `0x5aec0`, the entry `0x14030`, the
//! selection `0xba4ec`, the mode's words `0xb5790`, `0xb5798` and `0xb579a`.
//!
//! The 16-bit engine has no `SETRES`: its `TOGFX` enters 320×200 and nothing
//! else, and the profile says so ([`crate::Profile::has_setres`]).

use crate::Engine;
use crate::cursor::NORMAL_COLORS;
use crate::profile::TogfxPointer;
use motionvm_motion_forth::{Error, Result};
use motionvm_playable::Size;

/// A mode of the 32-bit engine's table at `0x13fc0`: the ordinal `SETRES`
/// takes, the VESA mode number selected for it, and the picture it enters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VideoMode {
    /// What `SETRES` stores, and what the mode's word pushes where one exists.
    pub(crate) ordinal: i32,
    /// The mode number handed to the video BIOS (`0x80e84`).
    pub(crate) vesa: u16,
    /// The picture's width in pixels.
    pub(crate) width: u16,
    /// Its height.
    pub(crate) height: u16,
    /// Whether the mode is one of the three 32K-color ones, which is what
    /// `HICOLOR` answers (`0xd6600`, set at `0x140fa`).
    pub(crate) hicolor: bool,
}

/// The seven modes, in the table's order. The first three have words —
/// `320x200x256`, `640x480x256` and `640x480x32K` push 1, 2 and 4 at
/// `0x6f023`, `0x6f047` and `0x6f06b` — and the other four can only be asked
/// for by number.
pub(crate) const MODES: [VideoMode; 7] = [
    VideoMode {
        ordinal: MODE_320X200X256,
        vesa: 0x13,
        width: 320,
        height: 200,
        hicolor: false,
    },
    VideoMode {
        ordinal: MODE_640X480X256,
        vesa: 0x101,
        width: 640,
        height: 480,
        hicolor: false,
    },
    VideoMode {
        ordinal: MODE_640X480X32K,
        vesa: 0x110,
        width: 640,
        height: 480,
        hicolor: true,
    },
    VideoMode {
        ordinal: 8,
        vesa: 0x103,
        width: 800,
        height: 600,
        hicolor: false,
    },
    VideoMode {
        ordinal: 0x10,
        vesa: 0x113,
        width: 800,
        height: 600,
        hicolor: true,
    },
    VideoMode {
        ordinal: 0x20,
        vesa: 0x105,
        width: 1024,
        height: 768,
        hicolor: false,
    },
    VideoMode {
        ordinal: 0x40,
        vesa: 0x116,
        width: 1024,
        height: 768,
        hicolor: true,
    },
];

/// What `320x200x256` pushes.
pub(crate) const MODE_320X200X256: i32 = 1;
/// What `640x480x256` pushes — the one mode Dunkle Schatten 2 asks for.
pub(crate) const MODE_640X480X256: i32 = 2;
/// What `640x480x32K` pushes.
pub(crate) const MODE_640X480X32K: i32 = 4;

/// The mode an ordinal selects, or `None` for a number the table does not
/// hold — which `0x13fc0` answers by leaving the last selection standing.
pub(crate) fn mode(ordinal: i32) -> Option<VideoMode> {
    MODES.iter().copied().find(|m| m.ordinal == ordinal)
}

/// What `SETRES` has selected and `TOGFX` has entered.
#[derive(Debug)]
pub(crate) struct Video {
    /// The ordinal `SETRES` stored (`0xdb4a0`) — 2 before it ran, the
    /// kernel init's seed (see the module doc).
    pub(crate) selected: i32,
    /// The mode the selection last resolved to — the table's entry for the
    /// ordinal, as `0x13fc0` writes it into `0xd65fc` and the width and
    /// height beside it. Written for an ordinal the table holds and left
    /// standing for one it does not, and `TOGFX` enters what stands. `None`
    /// until an ordinal of the table has been selected: the original's words
    /// start as mode `0x13` with a width and height of nought, a picture
    /// nothing could be drawn into.
    pub(crate) resolved: Option<VideoMode>,
    /// The picture the first `TOGFX` built, which every later one has to
    /// enter again: the contract promises a window one size for the game's
    /// lifetime, and a family whose game switched modes mid-run would have
    /// to render into one size and say so.
    pub(crate) entered: Option<Size>,
    /// Whether the game is in graphics, between `TOGFX` and `GFXTO`
    /// (`0xd6602`). No hardware stands behind it here, and the picture exists
    /// either way; it is kept because the game asks.
    pub(crate) on: bool,
}

impl Video {
    /// The state the kernel init leaves: `640x480x256` selected and nothing
    /// resolved or entered yet.
    ///
    /// Not a `Default`, because 2 is not a default — it is what the init at
    /// `0x37297` (R78 `0x2fd5b`) hands the kernel init to store, read off
    /// both builds; see the module doc.
    pub(crate) fn seeded() -> Self {
        Self {
            selected: MODE_640X480X256,
            resolved: None,
            entered: None,
            on: false,
        }
    }
}

impl Engine {
    /// `SETRES`: stores the mode's ordinal and selects the mode's parameters
    /// (`0x13fc0`) — for an ordinal the table holds; another leaves the last
    /// resolved mode standing, as the routine's chain of comparisons falls
    /// through without a store. `TOGFX` does the same selection again before
    /// it enters, which is why it is a method and not a line in `SETRES`.
    pub(crate) fn select_mode(&mut self, ordinal: i32) {
        self.mode.selected = ordinal;
        if let Some(m) = mode(ordinal) {
            self.mode.resolved = Some(m);
        }
    }

    /// `HICOLOR`: whether the resolved mode is a 32K-color one — `0xd6600`,
    /// which the selection writes at `0x140fa` and an unknown ordinal leaves
    /// as it was.
    pub(crate) fn hicolor(&self) -> bool {
        self.mode.resolved.is_some_and(|m| m.hicolor)
    }

    /// `TOGFX`: enters graphics, and with it the size the display has from
    /// now on — the resolved mode's on a build with `SETRES`, the profile's
    /// on one without.
    ///
    /// Three refusals, each named after the word that caused it. A selection
    /// that never resolved: the original would enter VGA `0x13` with the
    /// display's width and height still nought (`0xd65fc` starts as `0x13`,
    /// `0xd6604` as zero) — a picture nothing could be drawn into; the init's
    /// seed of 2 means only a script's own `SETRES` of an ordinal outside the
    /// table, before any inside it, gets here. A 32K-color mode: this
    /// renderer composes indexed pixels and nothing else, so it would show a
    /// wrong picture rather than the game's. And a second entry at another
    /// size, which the window cannot follow.
    pub(crate) fn enter_graphics(&mut self) -> Result<()> {
        let size = if self.profile.has_setres {
            // `TOGFX` selects again (`0x6ee81`, R78 `0x5aedd`) before it
            // enters (`0x141dc`, R78 `0x14030`).
            let ordinal = self.mode.selected;
            self.select_mode(ordinal);
            let mode = self.mode.resolved.ok_or_else(|| {
                Error::Unsupported(format!(
                    "TOGFX enters the mode SETRES selected, and {ordinal} names none of the \
                     engine's seven"
                ))
            })?;
            if mode.hicolor {
                return Err(Error::Unsupported(format!(
                    "TOGFX enters {}x{} in 32K colors (mode {ordinal}); only 256-color modes \
                     are drawn",
                    mode.width, mode.height
                )));
            }
            Size {
                width: mode.width,
                height: mode.height,
            }
        } else {
            self.profile.display
        };
        if let Some(first) = self.mode.entered
            && first != size
        {
            return Err(Error::Unsupported(format!(
                "TOGFX enters {}x{} after the game began at {}x{}; a window is one size for a \
                 game's lifetime",
                size.width, size.height, first.width, first.height
            )));
        }
        self.mode.entered = Some(size);
        self.display.size = size;
        self.mode.on = true;
        self.arm_pointer_for_graphics();
        Ok(())
    }

    /// What `TOGFX` does with the pointer once the mode is entered, by
    /// [`TogfxPointer`].
    ///
    /// The 32-bit handler first installs the system palette — `000.pal`,
    /// which the init resolved through the resource layer to palette 0 of
    /// whatever holds it (R78 `0x21b4c`–`0x21b72`, the handle at `0xb5854`;
    /// R109 `0xd66fc`) and which no `SETPAL` touches — so the arrow's two
    /// colors are resolved against *that* palette, whatever a script set
    /// before `TOGFX`; the indices then stay through every `SETPAL` after,
    /// which is why Checker 2000's arrow wears its boards' colors 1 and 52.
    fn arm_pointer_for_graphics(&mut self) {
        match self.profile.togfx_pointer {
            TogfxPointer::Shown => {
                if let Some(p) = self.scene.system_palette.clone() {
                    self.display.palette = p;
                }
                self.arm_arrow(Some(NORMAL_COLORS));
                self.show_pointer();
            }
            TogfxPointer::Armed => self.arm_arrow(None),
        }
    }

    /// `GFXTO`: leaves graphics. The picture stays what it is; the original
    /// frees its buffers and returns to text mode, and re-enters through
    /// `TOGFX` with a fresh surface of the same mode.
    pub(crate) fn leave_graphics(&mut self) {
        self.mode.on = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Profile;

    fn size(width: u16, height: u16) -> Size {
        Size { width, height }
    }

    /// The three mode words select modes of the table, and the table's
    /// pictures are the ones `0x13fc0` writes.
    #[test]
    fn the_words_ordinals_name_the_pictures_the_table_holds() {
        assert_eq!(
            mode(MODE_320X200X256).map(|m| (m.vesa, m.width, m.height)),
            Some((0x13, 320, 200))
        );
        assert_eq!(
            mode(MODE_640X480X256).map(|m| (m.vesa, m.width, m.height)),
            Some((0x101, 640, 480))
        );
        assert_eq!(
            mode(MODE_640X480X32K).map(|m| (m.vesa, m.hicolor)),
            Some((0x110, true))
        );
        assert_eq!(mode(3), None, "3 is between two ordinals and names nothing");
        assert_eq!(MODES.iter().filter(|m| m.hicolor).count(), 3);
    }

    /// `320x200x256 SETRES TOGFX` on the 32-bit engine sizes the display to
    /// the mode, and `GFXTO` then `TOGFX` keeps it.
    #[test]
    fn togfx_builds_the_display_at_the_selected_mode() {
        let mut e = Engine::new(Profile::motion32());
        assert_eq!(e.display_size(), size(640, 480), "the size before graphics");
        e.select_mode(MODE_320X200X256);
        e.enter_graphics().unwrap();
        assert_eq!(e.display_size(), size(320, 200));
        assert!(e.mode.on);
        e.leave_graphics();
        assert!(!e.mode.on);
        e.enter_graphics().unwrap();
        assert_eq!(e.display_size(), size(320, 200));
    }

    /// `TOGFX` with no `SETRES` before it — Checker 2000's `STARTUP` — enters
    /// the 640×480 the kernel init seeded, and an ordinal the table does not
    /// hold leaves the last resolved mode standing, `HICOLOR` included.
    #[test]
    fn togfx_without_setres_enters_the_seeded_640x480() {
        let mut e = Engine::new(Profile::motion32());
        assert_eq!(e.mode.selected, MODE_640X480X256);
        assert!(!e.hicolor(), "nothing resolved yet answers no");
        e.enter_graphics().unwrap();
        assert_eq!(e.display_size(), size(640, 480));

        let mut e = Engine::new(Profile::motion32());
        e.select_mode(MODE_640X480X32K);
        assert!(e.hicolor());
        e.select_mode(3);
        assert!(e.hicolor(), "3 names nothing, so the 32K mode stands");
        e.select_mode(MODE_640X480X256);
        e.select_mode(7);
        e.enter_graphics().unwrap();
        assert_eq!(e.display_size(), size(640, 480));
        assert!(!e.hicolor());
    }

    /// The three refusals, each naming what was asked.
    #[test]
    fn a_mode_the_renderer_cannot_draw_is_refused_at_togfx() {
        let mut e = Engine::new(Profile::motion32());
        e.select_mode(3);
        let before = e.enter_graphics().unwrap_err().to_string();
        assert!(before.contains("3 names none"), "{before}");
        e.select_mode(MODE_640X480X32K);
        let hicolor = e.enter_graphics().unwrap_err().to_string();
        assert!(hicolor.contains("32K colors"), "{hicolor}");
        assert!(
            e.hicolor(),
            "HICOLOR answers for the selection, entered or not"
        );
        e.select_mode(MODE_640X480X256);
        e.enter_graphics().unwrap();
        e.select_mode(MODE_320X200X256);
        let again = e.enter_graphics().unwrap_err().to_string();
        assert!(again.contains("after the game began at 640x480"), "{again}");
        assert_eq!(
            e.display_size(),
            size(640, 480),
            "a refused entry changes nothing"
        );
    }

    /// The 16-bit engine has no `SETRES`, and its `TOGFX` enters the
    /// profile's 320×200.
    #[test]
    fn a_build_without_setres_enters_its_one_mode() {
        let mut e = Engine::new(Profile::motion16());
        e.enter_graphics().unwrap();
        assert_eq!(e.display_size(), size(320, 200));
        assert!(!e.hicolor());
    }
}
