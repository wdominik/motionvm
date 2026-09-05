//! The pointer the engine draws over everything else.
//!
//! Its own subsystem because the two generations show it differently and the
//! difference is one field: the 16-bit engine *counts* shows and hides, so a
//! hide inside a hide leaves the pointer down until both are undone, while the
//! 32-bit one keeps a flag. Which of the two this is comes off
//! [`crate::Profile::pointer_counted`], and the count lives here either way.
//!
//! The shape is `XATMOUSE`'s: a sprite id and the hotspot inside it, so the
//! net position is the pointer minus the hotspot. Nothing the engine draws
//! ever covers it — engine blits bracket themselves with
//! `HIDEMOUSE`/`SHOWMOUSE`, and the pointer goes on last.

/// The pointer: what it looks like, whether it is up, and how deep its
/// show counter is.
#[derive(Debug)]
pub(crate) struct Cursor {
    /// The mouse pointer: sprite and hotspot, as `XATMOUSE` sets it.
    pub(crate) shape: Option<(u32, i32, i32)>,

    pub(crate) visible: bool,

    /// The 16-bit pointer's show counter (`ds:0x16B4`): `SHOWMOUSE` adds
    /// one, `HIDEMOUSE` takes one, the pointer shows while it stands at one
    /// or more — and neither moves it before a shape armed the pointer
    /// (`ds:0x16AA`; both handlers leave without it, `14ee:0877`,
    /// `14ee:094e`). Zero at power-on: the pointer is invisible until the
    /// first `SHOWMOUSE` after `FATMOUSE`, which is why the intro shows
    /// none — `RUN` arms the shape before `STARTINTRO` but shows only
    /// after it.
    pub(crate) shows: i32,
}

impl Cursor {
    /// A pointer with no shape, shown or not as the engine build says.
    ///
    /// There is no `Default`: whether the pointer starts up is the
    /// generation's answer, and a default would have to pick one of the two
    /// silently. See [`crate::Profile::pointer_starts_visible`].
    pub(crate) fn starting(visible: bool) -> Self {
        Self {
            shape: None,
            visible,
            shows: 0,
        }
    }
}
