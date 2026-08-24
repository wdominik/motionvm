//! The screen words as methods: which surface is selected, and what it reads back.
//!
//! A screen is the original's drawing surface — a framebuffer with a size, a
//! view onto it and a position. `ACTSCR` selects one and every `SCR…` and
//! `GSCR…` word after it acts on that selection, which is the same shape the
//! descriptors have and for the same reason: the original keeps the selection in
//! a global and the bytecode relies on it standing.
//!
//! The screens themselves live in [`motionvm_render::Display`]; this is the
//! engine's side of them. `words/screens.rs` is the stack ABI over what is here.

use crate::Engine;

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
        if self.per_screen_descriptors
            && let Some(h) = self.selected_handle
        {
            self.selected = self
                .descriptors
                .iter()
                .position(|d| d.screen == handle && d.handle == h);
        }
    }

    /// `GSCRX`: the horizontal origin of the active screen.
    ///
    /// Two registers answer here, one per generation: the 32-bit `SCRX`
    /// writes `origin` (+0x24; that game only ever passes zero), the
    /// 16-bit `SCRX` writes `pos` (the scroll at scr+0, which the town
    /// views move). Each generation leaves the other's register at zero,
    /// so the sum is the right answer for both — the same base
    /// `Screen::span` uses for the damage map. Reading only `origin`
    /// made every native `GSCRX`-relative placement (the verb strip's
    /// clamp, `MOUSEINFO`, the 16-bit conversation) drop the scroll
    /// term.
    pub(crate) fn screen_origin_x(&mut self) -> i32 {
        self.display
            .current_mut()
            .map(|s| i32::from(s.origin.0) + i32::from(s.pos.0))
            .unwrap_or(0)
    }

    /// `SCRX`: writes the horizontal origin that [`Engine::screen_origin_x`]
    /// reads back.
    ///
    /// What it shifts is not established — the game only ever sets it to zero —
    /// so nothing composites with it yet. A non-zero value is counted so it
    /// cannot pass unnoticed if that ever changes.
    pub(crate) fn set_screen_origin_x(&mut self, v: i32) {
        if v != 0 {
            self.note_unhandled("SCRX (non-zero)".into());
        }
        if let Some(s) = self.display.current_mut() {
            s.origin.0 = v as i16;
        }
    }

    /// `GSCRY`: the vertical origin. See [`Engine::screen_origin_x`].
    pub(crate) fn screen_origin_y(&mut self) -> i32 {
        self.display
            .current_mut()
            .map(|s| i32::from(s.origin.1) + i32::from(s.pos.1))
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
        (w as i32, h as i32)
    }
}
