//! Off-screen buffers, as the 16-bit kernel's `SETBUF`, `SDBUF`, `BUFON` and
//! `KILLNBUF` keep them.
//!
//! The 16-bit game draws through buffers where the 32-bit game never does:
//! `RUN` switches them on with `BUFON`, the intro gives every motif a buffer
//! of its own (`2 SDBUF` … `6 SDBUF`) and a 320×200 one to the screen it ends
//! on (`320 200 1 SETBUF 1 SDBUF`), and `NEWPERS` hands every person sprite
//! a 100×140 buffer. What the kernel does with them is read (`SETBUF` at
//! `ENVIRO.EXE` file `0xac33`, `SDBUF` `0xabf1`, `BUFON` `0xaab8`,
//! `KILLNBUF` `0xafab`, the frame loop's restore step at `016a:179e`): a
//! buffer is a **save-under** — `SETBUF` allocates an empty image of the
//! size given and copies nothing, `SDBUF` writes the number into the
//! descriptor, and each frame the loop pastes back what a dirty buffered
//! descriptor had saved under itself before the descriptor is drawn again
//! and saves again.
//!
//! This module keeps the buffers as state — sizes, attachments, the switch —
//! and the drawer composes the scene graph as it does for the 32-bit game,
//! rebuilding the place a buffered descriptor leaves from the descriptor
//! list. On a compositor that redraws from the descriptor list a save-under
//! changes no pixel, so the picture is the same; where the two could
//! differ is a departure.

use motionvm_motion_forth::cell;
use motionvm_render::Framebuffer;
use std::collections::BTreeMap;

/// One buffer, as `SETBUF` allocates it.
#[derive(Debug, Clone)]
pub struct Buffer {
    /// Pixels across, as given.
    pub width: u16,
    /// Rows, as given.
    pub height: u16,
    /// The surface itself, `width × height` of palette indices, never drawn
    /// into by the compositor — kept so that a reading of the handlers that
    /// needs it has the memory already.
    pub surface: Framebuffer,
}

/// The buffers the game has asked for, by the number it asked under.
#[derive(Debug, Default)]
pub struct Buffers {
    /// Whether `BUFON` has been called.
    pub on: bool,
    by_id: BTreeMap<i32, Buffer>,
}

impl Buffers {
    /// `SETBUF ( w h id -- )`: allocates or resizes buffer `id`. A size of
    /// 0×0 frees it — the intro's teardown is `0 0 1 SETBUF`.
    pub fn set(&mut self, id: i32, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            self.by_id.remove(&id);
            return;
        }
        let (w, h) = (cell::low16(width), cell::low16(height));
        self.by_id.insert(
            id,
            Buffer {
                width: w,
                height: h,
                surface: Framebuffer::new(w, h),
            },
        );
    }

    /// `KILLNBUF ( from to -- )`: frees the buffers numbered `from` up to
    /// and including `to`, or every one from `from` upward when `to` is -1.
    ///
    /// The one call site is `?LPB 1 + -1 KILLNBUF` in `INCLLOC`, beside
    /// `?LPD 1 + KILLNDESC` which drops the descriptors from a handle upward
    /// — so the buffers from the next person buffer upward are dropped with
    /// them. The -1 is read as "to the end" by that analogy; the handler is
    /// unread.
    pub fn kill(&mut self, from: i32, to: i32) {
        self.by_id
            .retain(|&id, _| id < from || (to != -1 && id > to));
    }

    /// `RESETBUF`: frees every buffer.
    pub fn reset(&mut self) {
        self.by_id.clear();
    }

    /// The buffer under `id`, if one was set.
    pub fn get(&self, id: i32) -> Option<&Buffer> {
        self.by_id.get(&id)
    }

    /// Every buffer that is set, by number, ascending.
    pub fn iter(&self) -> impl Iterator<Item = (i32, &Buffer)> {
        self.by_id.iter().map(|(&id, b)| (id, b))
    }

    /// How many buffers are set.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether no buffer is set.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}
