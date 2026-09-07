//! The screen words as methods: which surface is selected, and what it reads back.
//!
//! A screen is the original's drawing surface — a framebuffer with a size, a
//! view onto it and a position. `ACTSCR` selects one and every `SCR…` and
//! `GSCR…` word after it acts on that selection, which is the same shape the
//! descriptors have and for the same reason: the original keeps the selection in
//! a global and the bytecode relies on it standing.
//!
//! The screens themselves live in [`crate::Display`]; this is the
//! engine's side of them. `words/screens.rs` is the stack ABI over what is here.

use crate::Engine;
use motionvm_motion_forth::cell;

impl Engine {
    /// `ACTSCR`: makes a screen the current one.
    ///
    /// Every `SCR…` and `GSCR…` word that follows acts on it, and the selection
    /// stays until something else selects — it is not saved and restored around
    /// a call, because the game's own bytecode selects once and writes several
    /// words later.
    pub(crate) fn select_screen(&mut self, handle: u32) {
        self.display.set_current(handle);
        // In the per-screen scheme the number `ACTDESC` stored now names a
        // descriptor of this screen; see [`Engine::select_descriptor`].
        if self.profile.per_screen_descriptors
            && let Some(h) = self.scene.selected_handle
        {
            self.scene.selected = self
                .scene
                .descriptors
                .iter()
                .position(|d| d.screen == handle && d.handle == h);
        }
    }

    /// `GSCRX`: the horizontal scroll of the active screen — the register
    /// `SCRPOS`, `SCRX` and `->SCRX` write, `+0x24` of the record hanging off
    /// the screen on the 32-bit machine and `scr+0` on the 16-bit one, which
    /// is the base every `GSCRX`-relative placement (the verb strip's clamp,
    /// `MOUSEINFO`, the 16-bit conversation) adds to.
    pub(crate) fn screen_origin_x(&mut self) -> i32 {
        self.display
            .current_mut()
            .map(|s| i32::from(s.pos.0))
            .unwrap_or(0)
    }

    /// The 32-bit `SCRX` and `SCRY`: one half of the scroll register, and a
    /// whole redraw when it changes.
    ///
    /// `SCRY` (R78 `0x5ee40`) compares the new value with `+0x26`, stores it
    /// and calls the screen's redraw marker (`0x58db0`) only when it differs;
    /// `SCRX` is its twin on `+0x24`. The marker sets bits `0x40` and `0x10`
    /// of the screen's flags, which the next drawer pass reads as "draw every
    /// active descriptor under the view again" — here, the view's rectangle
    /// goes onto the rebuild list, and the map is marked from the bottom, so
    /// the pass repaints what the moved window now shows.
    pub(crate) fn set_screen_scroll(&mut self, vertical: bool, v: i32) {
        let Some(s) = self.display.current_mut() else {
            return;
        };
        let slot = if vertical { &mut s.pos.1 } else { &mut s.pos.0 };
        if *slot == cell::short(v) {
            return;
        }
        *slot = cell::short(v);
        let handle = s.handle;
        self.redraw_view(handle);
    }

    /// The 32-bit redraw marker (`0x58db0`) on one screen: everything under
    /// its view is drawn again on the next pass.
    pub(crate) fn redraw_view(&mut self, handle: u32) {
        let Some(s) = self.display.screen_mut(handle) else {
            return;
        };
        let (x, y) = (i32::from(s.pos.0), i32::from(s.pos.1));
        let (w, h) = (i32::from(s.view.0), i32::from(s.view.1));
        s.mark(x, y, w, h, i32::from(i16::MIN));
        self.rebuild.push((handle, (x, y, w, h)));
        self.dirty = true;
    }

    /// `GSCRY`: the vertical scroll. See [`Engine::screen_origin_x`].
    pub(crate) fn screen_origin_y(&mut self) -> i32 {
        self.display
            .current_mut()
            .map(|s| i32::from(s.pos.1))
            .unwrap_or(0)
    }

    /// `GSCRVSIZE`: the visible window onto the surface, as `(width, height)`.
    ///
    /// The counterpart to `SCRVSIZE`, reading back the same two fields at +0x1c
    /// and +0x1e. The word pushes the height first so the width ends up on top,
    /// which is how `DOORDER` reads it — adding the first value it pops to an x
    /// coordinate. That ordering belongs to the stack ABI and stays in the arm.
    pub(crate) fn screen_view_size(&mut self) -> (i32, i32) {
        let (w, h) = self.display.current_mut().map(|s| s.view).unwrap_or((0, 0));
        (i32::from(w), i32::from(h))
    }
}
