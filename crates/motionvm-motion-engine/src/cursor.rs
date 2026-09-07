//! The pointer the engine draws over everything else.
//!
//! Its own subsystem because it is the one picture that is no descriptor's:
//! the mouse layer paints it straight onto the video surface, last of all,
//! and every engine blit that could cover it brackets itself with a hide and
//! a show. What the layer keeps is a shape, a hotspot and a count, and both
//! generations keep them the same way — the 16-bit layer (`14ee:000e`) is a
//! compile of the 32-bit one (R109 `0x24637`, R78 `0x1f500`), data for data:
//!
//! - **The shape** is either a sprite of the game's, which `XATMOUSE` and
//!   `ATMOUSE` install, or one of the engine's own — two 16×16 pictures in
//!   `ENGINE.EXE`'s data, the same 524 bytes in both builds (R78 `0xb58ac`,
//!   R109 `0xd6754`): an arrow with its hotspot at the corner and a crosshair
//!   with its hotspot at 7,7. Only the arrow has a way in: `TOGFX` installs
//!   it on entering graphics and `NORMMOUSE` installs it again. Its pixels
//!   are three values — 0 clear, 8 the body, 15 the outline — and the
//!   installer replaces 8 and 15 by two palette indices it is handed, or
//!   leaves them standing when handed −1 ([`Engine::arm_arrow`]).
//! - **The count** (R78 `0xb5884`, `14ee:16b4`): `SHOWMOUSE` adds one and
//!   draws when it reaches one, `HIDEMOUSE` takes one and restores when it
//!   reaches nought (R78 `0x20340`/`0x20540`, `14ee:0874`/`14ee:094b`), so
//!   the pointer shows while the count is one or more and a hide inside a
//!   hide leaves it down until both are undone. Both pairs leave without
//!   touching the count before a shape has armed the layer (`0xb5870`,
//!   `14ee:16aa`), which the first install does — and that is `TOGFX`.
//!
//! Installing a shape keeps the count: the installer hides while it works
//! and shows again to where it stood. So a pointer is on screen from the
//! first `SHOWMOUSE` after `TOGFX` on, whatever shape it wears — and on the
//! 32-bit engine `TOGFX` itself ends with that `SHOWMOUSE`
//! ([`crate::Profile::togfx_pointer`]).

use crate::Engine;
use motionvm_render::Picture;

/// The pointer: what it looks like and how deep its show counter stands.
#[derive(Debug)]
pub(crate) struct Cursor {
    /// The shape, once one armed the layer; `None` before `TOGFX`.
    pub(crate) shape: Option<PointerShape>,

    /// The show counter. Zero at power-on: the pointer is invisible until
    /// the first `SHOWMOUSE`, which on the 32-bit engine is `TOGFX`'s own
    /// and on the 16-bit one the script's — which is why Die Enviro-Kids'
    /// intro shows none: `RUN` arms the shape before `STARTINTRO` but shows
    /// only after it.
    pub(crate) shows: i32,
}

impl Cursor {
    /// A pointer with no shape and nothing shown — the layer before
    /// graphics.
    pub(crate) fn unarmed() -> Self {
        Self {
            shape: None,
            shows: 0,
        }
    }

    /// Whether the pointer is drawn: the count stands at one or more.
    pub(crate) fn visible(&self) -> bool {
        self.shows >= 1
    }
}

/// What the pointer looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerShape {
    /// A sprite of the game's, as `XATMOUSE` installs it: the sprite and
    /// the hotspot inside it, so the net position is the pointer minus the
    /// hotspot.
    Sprite {
        /// The sprite's resource id.
        id: u32,
        /// The hotspot's column inside the sprite.
        hot_x: i32,
        /// Its row.
        hot_y: i32,
    },
    /// The engine's own arrow, hotspot at its corner, in two palette
    /// indices.
    Arrow {
        /// The index drawn where the bitmap says 8: the arrow's inside.
        body: u8,
        /// The index drawn where it says 15: the line around it.
        outline: u8,
    },
}

impl PointerShape {
    /// Where the hotspot is inside the shape.
    pub(crate) fn hotspot(self) -> (i32, i32) {
        match self {
            Self::Sprite { hot_x, hot_y, .. } => (hot_x, hot_y),
            Self::Arrow { .. } => (0, 0),
        }
    }
}

/// The two colors `NORMMOUSE` and the 32-bit `TOGFX` give the arrow, as
/// six-bit red, green and blue: white for the body and a dark teal for the
/// outline (R78 `0x5ed5e`–`0x5ed82`, R109 `0x735ba`–`0x735de`). Each is
/// resolved to the nearest entry of the palette in force when the word
/// runs, by the engine's own lookup ([`crate::paint`]), and the indices
/// stay — a later `SETPAL` recolors the arrow with everything else.
pub(crate) const NORMAL_COLORS: ([u8; 3], [u8; 3]) = ([63, 63, 63], [0, 47, 47]);

/// The arrow's side.
const ARROW_SIDE: u16 = 16;

/// The value the installer replaces by the body color.
const BODY: u8 = 8;
/// The value it replaces by the outline color.
const OUTLINE: u8 = 15;

/// The engine's own arrow, row by row: `.` clear, `W` the body (8), `K` the
/// outline (15). Transcribed from the 262-byte record at R78 `0xb58ac` —
/// a six-byte header of width 16, height 16 and `0xff`, then the pixels —
/// which R109 holds byte for byte at `0xd6754`; the test
/// `the_arrow_is_the_one_both_binaries_hold` holds it against both.
const ARROW: [&str; 16] = [
    "K...............",
    "KK..............",
    "KWK.............",
    "KWWK............",
    "KWWWK...........",
    "KWWWWK..........",
    "KWWWWWK.........",
    "KWWWWWWK........",
    "KWWWWWWWK.......",
    "KWKWWWKKKK......",
    "KKKKWWWK........",
    "....KKKK........",
    "................",
    "................",
    "................",
    "................",
];

/// The arrow's pixels as the binary holds them: 0, 8 and 15.
pub(crate) fn arrow_pixels() -> Vec<u8> {
    ARROW
        .iter()
        .flat_map(|row| {
            row.bytes().map(|c| match c {
                b'W' => BODY,
                b'K' => OUTLINE,
                _ => 0,
            })
        })
        .collect()
}

/// The arrow as a picture to blit, its two values replaced by the indices
/// the install resolved — with 0 left clear, which is what the masked blit
/// skips.
pub(crate) fn arrow_picture(body: u8, outline: u8) -> Picture {
    Picture {
        width: ARROW_SIDE,
        height: ARROW_SIDE,
        pixels: arrow_pixels()
            .into_iter()
            .map(|v| match v {
                BODY => body,
                OUTLINE => outline,
                other => other,
            })
            .collect(),
    }
}

impl Engine {
    /// `SHOWMOUSE`: one more on the count, and the pointer draws when it
    /// reaches one. Nothing before a shape armed the layer.
    pub(crate) fn show_pointer(&mut self) {
        if self.cursor_state.shape.is_some() {
            self.cursor_state.shows += 1;
        }
    }

    /// `HIDEMOUSE`: one off the count, and the pointer goes when it reaches
    /// nought. Nothing before a shape armed the layer.
    pub(crate) fn hide_pointer(&mut self) {
        if self.cursor_state.shape.is_some() {
            self.cursor_state.shows -= 1;
        }
    }

    /// `XATMOUSE`: gives the pointer a sprite of the game's.
    ///
    /// The handler looks the sprite up, hides the pointer, installs the
    /// shape at the given hotspot and shows again to the count it found —
    /// state, not nothing, which is why it is not on the inert list. A
    /// negative sprite is floored at zero, as the handler's lookup does.
    pub(crate) fn arm_sprite(&mut self, sprite: i32, hot_x: i32, hot_y: i32) {
        self.cursor_state.shape = Some(PointerShape::Sprite {
            id: motionvm_motion_forth::cell::unsigned(sprite.max(0)),
            hot_x,
            hot_y,
        });
    }

    /// Installs the engine's own arrow, hotspot at its corner.
    ///
    /// With colors, each is resolved against the palette in force and the
    /// arrow's body and outline take the two indices — `NORMMOUSE`'s way,
    /// and the 32-bit `TOGFX`'s. Without, the bitmap's own values stand,
    /// index 8 for the body and 15 for the outline — the 16-bit `TOGFX`'s
    /// way, which hands the installer −1 for both (`05f1:011e`). The count
    /// is untouched either way.
    pub(crate) fn arm_arrow(&mut self, colors: Option<([u8; 3], [u8; 3])>) {
        let (body, outline) = match colors {
            Some(([r, g, b], [r2, g2, b2])) => {
                (self.nearest_color(r, g, b), self.nearest_color(r2, g2, b2))
            }
            None => (BODY, OUTLINE),
        };
        self.cursor_state.shape = Some(PointerShape::Arrow { body, outline });
    }

    /// The pointer's picture, to blit at the pointer minus the hotspot.
    pub(crate) fn pointer_picture(&mut self) -> Option<Picture> {
        match self.cursor_state.shape? {
            PointerShape::Sprite { id, .. } => self.sprite(id),
            PointerShape::Arrow { body, outline } => Some(arrow_picture(body, outline)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arrow_is_sixteen_square_of_three_values() {
        let px = arrow_pixels();
        assert_eq!(px.len(), 256);
        assert_eq!(px[0], OUTLINE, "the corner is the hotspot's outline");
        assert_eq!(px[16 + 1], OUTLINE);
        assert_eq!(px[2 * 16 + 1], BODY);
        assert!(px.iter().all(|&v| matches!(v, 0 | BODY | OUTLINE)));
        assert_eq!(px.iter().filter(|&&v| v == BODY).count(), 35);
        assert_eq!(px.iter().filter(|&&v| v == OUTLINE).count(), 32);
    }

    #[test]
    fn the_picture_wears_the_indices_it_is_given() {
        let p = arrow_picture(1, 52);
        assert_eq!((p.width, p.height), (16, 16));
        assert_eq!(p.pixels[0], 52);
        assert_eq!(p.pixels[2 * 16 + 1], 1);
        assert_eq!(p.pixels[15 * 16 + 15], 0, "clear stays clear");
    }

    /// The transcription above against the record in each shipped build:
    /// the six-byte header, then the 256 pixels. Skips without the games.
    #[test]
    fn the_arrow_is_the_one_both_binaries_hold() {
        use motionvm_motion_formats::m32::le::Image;
        let builds = [
            (motionvm_motion_testutil::gamedata_checker(), 0xb58ac, "R78"),
            (motionvm_motion_testutil::gamedata_ds2(), 0xd6754, "R109"),
        ];
        let mut expected = vec![16, 0, 16, 0, 0xff, 0];
        expected.extend(arrow_pixels());
        for (dir, at, build) in builds {
            let Some(dir) = dir else {
                eprintln!("skipping {build}: no gamedata directory");
                continue;
            };
            let img = Image::open(dir.join("ENGINE.EXE")).expect("the engine opens");
            let record = img
                .slice(at, expected.len())
                .expect("the record is in the image");
            assert_eq!(record, &expected[..], "{build}'s arrow at {at:#x}");
        }
    }
}
