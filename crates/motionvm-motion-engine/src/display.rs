//! The game's screens and the display they composite onto.
//!
//! A screen is the original's screen record read whole: the five `SCR*` words
//! recorded verbatim, the controller, the freeze bit, the damage map, and the
//! drawing surface underneath — a [`Framebuffer`], which is all the renderer
//! is asked for. How the recorded values map onto compositing is a
//! *hypothesis*, spelled out at [`Display::compose`].

use motionvm_motion_forth::cell;
use motionvm_playable::Size;
use motionvm_render::Palette;
use motionvm_render::{Framebuffer, Rect};

/// One of the game's screens: a drawing surface with a size, a window onto it,
/// and a place on the display.
///
/// The five words that configure a screen are recorded verbatim. How they map
/// onto compositing is a *hypothesis*, spelled out at [`Display::compose`] —
/// the values the game passes are known exactly, their interpretation is what
/// the reference screenshots have to settle.
#[derive(Debug, Clone)]
pub struct Screen {
    /// What `NEWSCREEN` handed back and every other `SCR*` word names it by.
    pub handle: u32,
    /// `SCRSIZE`: the drawing surface.
    pub size: (u16, u16),
    /// `SCRFVSIZE`: the full view size.
    pub full_view: (u16, u16),
    /// `SCRVSIZE`: the visible window onto the surface.
    pub view: (u16, u16),
    /// `SCRVPOS`.
    pub view_pos: (i16, i16),
    /// Where the view sits over the surface — the scroll register.
    ///
    /// One pair on each machine, written by every word that scrolls. On the
    /// 32-bit one it is `+0x24`/`+0x26` of the record hanging off the screen:
    /// `SCRPOS` writes both (R109 `0x7098b`/`0x70995`, R78 `0x5cb1e`/
    /// `0x5cb28`), `SCRX` and `SCRY` one each, `->SCRX`/`->SCRY` slide it,
    /// and `GSCRX`/`GSCRY` read it back. On the 16-bit one it is the pair at
    /// `scr+0`/`+2`, which `SCRPOS`, `SCRX` and `SCRY` write. The composer
    /// takes the view out of the surface from here ([`Display::window`]),
    /// and the damage map is clipped against the same window.
    pub pos: (i16, i16),
    /// `SCRCTRL`: the word id this screen runs every frame, as the handler
    /// stores it — a raw id, resolved only when a frame comes to run it, and
    /// negative for none. It belongs to the screen and not to the engine
    /// because the handler writes it through the current-screen accessor
    /// (`0104:1663` in `LL.EXE`, `+0x14` of the record).
    pub controller: i32,
    /// `FREEZESCR` sets bit 2 of the screen's flag byte at +0x13 and
    /// `UNFREEZESCR` clears it again.
    pub frozen: bool,
    /// Whether the screen is composited at all. `SCRACTIVE` and `SCRINACTIVE`
    /// switch it; an inactive screen keeps its surface and its configuration.
    pub active: bool,
    /// The drawing surface itself, `size` big.
    ///
    /// **It persists.** The original's drawer (0x6915b) never clears it: it
    /// resets [`Screen::damage`] and then repaints only the descriptors that
    /// are marked, so what nobody repaints stays exactly as it was. That is
    /// what makes `SDINACTIVE` leave a picture standing, and the in-game
    /// mailbox is built on it — see [`Screen::mark`].
    pub buffer: Framebuffer,
    /// What was painted straight onto the surface past the drawer, in the
    /// order it was painted — `WHITEBOX`'s box, `FADEIN` mode 2's — and
    /// after which pass. The surface holds the pixels; this is what lets a
    /// rectangle rebuilt from the descriptor list hold them still, see
    /// [`crate::paint`]. Wiped with the surface.
    pub(crate) paints: Vec<crate::paint::Paint>,
    /// One entry per 8x8 tile of [`Screen::view`]: the lowest level that has to
    /// be redrawn there, or [`Screen::UNDAMAGED`] for nothing.
    ///
    /// The original keeps the same array at `screen+0x41A`, with
    /// `(view_w * view_h) >> 6` entries of two bytes each, and its drawer
    /// refills it with `0x7FFF` at the start of every pass (0x69248).
    pub damage: Vec<i16>,
}

impl Screen {
    /// An empty screen under `handle`, with everything at zero.
    ///
    /// The game configures a screen with the `SCR*` words immediately after
    /// making one, so nothing here is a default anyone relies on.
    pub fn new(handle: u32) -> Self {
        Self {
            handle,
            size: (0, 0),
            full_view: (0, 0),
            view: (0, 0),
            view_pos: (0, 0),
            pos: (0, 0),
            controller: -1,
            frozen: false,
            active: true,
            buffer: Framebuffer::new(0, 0),
            paints: Vec::new(),
            damage: Vec::new(),
        }
    }

    /// Resizes the drawing surface, keeping nothing.
    pub fn set_size(&mut self, w: u16, h: u16) {
        self.size = (w, h);
        self.buffer = Framebuffer::new(w, h);
    }

    /// `SCRVSIZE`: the visible window, and with it the damage map's shape.
    ///
    /// The map is sized from the view rather than from the surface because
    /// that is what the original measures it by — `(+0x1C * +0x1E) >> 6` at
    /// 0x69231, and the clipping in 0x6e701 uses the same two fields.
    pub fn set_view(&mut self, w: u16, h: u16) {
        self.view = (w, h);
        let (tw, th) = self.tiles();
        self.damage = vec![Self::UNDAMAGED; tw * th];
    }

    /// Nothing in this tile wants redrawing. `0x7FFF`, as the original fills it.
    pub const UNDAMAGED: i16 = 0x7fff;

    /// The damage map's shape: the view in whole 8-pixel tiles.
    ///
    /// Truncating, as the original's `sar $3` is. Every screen the game builds
    /// is a multiple of eight in both directions, so nothing is lost — and a
    /// screen that was not would leave its last strip unmarkable in the
    /// original too.
    pub fn tiles(&self) -> (usize, usize) {
        (usize::from(self.view.0) >> 3, usize::from(self.view.1) >> 3)
    }

    /// `0x6e701`: everything in this rectangle has to be redrawn from `level` up.
    ///
    /// The map holds a *minimum* per tile, so two marks in one frame keep the
    /// lower level — the deeper repaint wins, which is the whole point of
    /// storing a level instead of a flag.
    ///
    /// Coordinates are on the surface; the screen's origin is taken off first,
    /// exactly as the handler does with `+0x24` and `+0x26`.
    pub fn mark(&mut self, x: i32, y: i32, w: i32, h: i32, level: i32) {
        let (tw, th) = self.tiles();
        if tw == 0 || th == 0 || w <= 0 || h <= 0 {
            return;
        }
        let level = cell::short(level.clamp(i32::from(i16::MIN), i32::from(Self::UNDAMAGED)));
        for (tx, ty) in self.span(x, y, w, h) {
            let slot = &mut self.damage[ty * tw + tx];
            if *slot > level {
                *slot = level;
            }
        }
    }

    /// Whether anything in the rectangle is waiting to be redrawn at or below
    /// `level` — the test `0x6e8c8` makes for every descriptor (0x6eb04).
    ///
    /// A screen with no map answers yes: nothing can be ruled out, and drawing
    /// too often costs time where drawing too seldom freezes the picture.
    pub fn damaged(&self, x: i32, y: i32, w: i32, h: i32, level: i32) -> bool {
        let (tw, th) = self.tiles();
        if tw == 0 || th == 0 {
            return true;
        }
        self.span(x, y, w, h)
            .any(|(tx, ty)| i32::from(self.damage[ty * tw + tx]) <= level)
    }

    /// Every tile a surface rectangle touches, clipped to the view.
    ///
    /// The view's base in surface coordinates is the scroll register,
    /// [`Screen::pos`]: the 32-bit clip takes `+0x24`/`+0x26` off first
    /// (`0x6e701`), the 16-bit one the pair at `scr+0`/`+2` (`016a:19d8`).
    fn span(&self, x: i32, y: i32, w: i32, h: i32) -> impl Iterator<Item = (usize, usize)> + use<> {
        let (tw, th) = self.tiles();
        let (ox, oy) = (i32::from(self.pos.0), i32::from(self.pos.1));
        let x0 = ((x - ox) >> 3).clamp(0, cell::count(tw));
        let y0 = ((y - oy) >> 3).clamp(0, cell::count(th));
        let x1 = ((x - ox + w + 7) >> 3).clamp(x0, cell::count(tw));
        let y1 = ((y - oy + h + 7) >> 3).clamp(y0, cell::count(th));
        // Clamped at zero above, so the conversions cannot come up short.
        let span = |lo: i32, hi: i32| cell::at(lo).unwrap_or(0)..cell::at(hi).unwrap_or(0);
        let cols = span(x0, x1);
        span(y0, y1).flat_map(move |ty| cols.clone().map(move |tx| (tx, ty)))
    }

    /// `0x69248`: the drawer empties the map at the start of every pass.
    pub fn reset_damage(&mut self) {
        self.damage.fill(Self::UNDAMAGED);
    }
}

/// The set of screens plus the palette in force.
#[derive(Debug)]
pub struct Display {
    /// The size of the picture the screens are composited onto: 640×480 for
    /// the 32-bit engine's game, 320×200 for the 16-bit engine's.
    pub size: Size,
    /// Every screen the game has made, in the order it made them — which is
    /// also the order they are composited in, before `level` is considered.
    pub screens: Vec<Screen>,
    /// The palette in force. `SETPAL` replaces it wholesale, so a frame's
    /// meaning depends on this as much as on its indices.
    pub palette: Palette,
    /// The screen `SCR*` words configure and new descriptors attach to.
    /// `NEWSCREEN` makes its screen current; `ACTSCR` selects an existing one.
    pub current: Option<u32>,
    next_handle: u32,
}

impl Display {
    /// Whether a screen is frozen, which is what stops the per-frame walk from
    /// reaching the descriptors on it.
    pub fn frozen(&self, handle: u32) -> bool {
        self.screens.iter().any(|s| s.handle == handle && s.frozen)
    }

    /// A display of the given size, with no screens and an all-black palette.
    pub fn with_size(size: Size) -> Self {
        Self {
            size,
            screens: Vec::new(),
            palette: Palette::from_6bit(&[0; Palette::BYTES]),
            current: None,
            next_handle: 1,
        }
    }

    /// `NEWSCREEN`: creates a screen and hands back its handle.
    ///
    /// Handles start at 1, which is what the original returns for the first
    /// screen of a run.
    pub fn new_screen(&mut self) -> u32 {
        let handle = self.next_handle;
        self.next_handle += 1;
        self.screens.push(Screen::new(handle));
        self.current = Some(handle);
        handle
    }

    /// The screen under `handle`, to be changed.
    pub fn screen_mut(&mut self, handle: u32) -> Option<&mut Screen> {
        self.screens.iter_mut().find(|s| s.handle == handle)
    }

    /// The screen the `SCR*` words configure.
    pub fn current_mut(&mut self) -> Option<&mut Screen> {
        let h = self.current?;
        self.screen_mut(h)
    }

    /// `ACTSCR`: makes an existing screen current.
    pub fn set_current(&mut self, handle: u32) {
        if self.screens.iter().any(|s| s.handle == handle) {
            self.current = Some(handle);
        }
    }

    /// Flattens the screens into one image of the display's size.
    ///
    /// **Hypothesis, to be checked against the reference screenshots.** The
    /// game lays out three screens: the main picture 640x400 with `SCRVPOS`
    /// (0,0), a status bar 640x80 with `SCRVPOS` (0,400), and a dialogue
    /// surface 384x480 with `SCRVPOS` (640,0). The first two tile the display
    /// exactly and the third falls outside it, which is what suggests `SCRVPOS`
    /// is the screen's place on the display and `SCRPOS` the scroll offset
    /// inside it. Every value here comes from the game; only this reading of
    /// them is provisional.
    ///
    /// **A running curtain needs no exception here**, although the flags
    /// suggest one: `FADEOUT` clears the active flag and `FADEIN` only sets it
    /// again at the end, so the screen a fade is revealing is invisible to this
    /// filter for the whole fade. It never has to be visible, because a curtain
    /// writes its bands straight into what is on screen
    /// (`Engine::advance_curtain`) — which is what the original does too — and
    /// nothing composes while one runs.
    pub fn compose(&self) -> Framebuffer {
        let mut out = Framebuffer::new(self.size.width, self.size.height);
        for s in self.screens.iter().filter(|s| s.active) {
            out.copy_from(
                &s.buffer,
                self.window(s),
                i32::from(s.view_pos.0),
                i32::from(s.view_pos.1),
            );
        }
        out
    }

    /// The part of a screen's surface that shows, in surface coordinates.
    ///
    /// `SCRPOS` scrolls the window over the surface and `SCRVSIZE` sizes it;
    /// a curtain has to map its bands the same way, so the rule lives here
    /// rather than twice.
    pub fn window(&self, s: &Screen) -> Rect {
        Rect {
            x: i32::from(s.pos.0),
            y: i32::from(s.pos.1),
            w: s.view.0.min(s.size.0),
            h: s.view.1.min(s.size.1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The damage span follows the scrolled window.
    ///
    /// A 16-bit screen scrolls `pos` over a surface wider than its view
    /// (`016a:19d8` clips marks against that window); a mark under the
    /// scrolled window must land in the map, and one left behind the
    /// window must not.
    #[test]
    fn a_mark_under_the_scrolled_window_lands_in_the_map() {
        let mut s = Screen::new(1);
        s.size = (960, 200);
        s.set_view(320, 200);
        s.pos = (640, 0);
        s.mark(700, 50, 16, 16, 3);
        assert!(s.damaged(700, 50, 16, 16, 3), "the mark under the window");
        assert!(
            !s.damaged(100, 50, 16, 16, 3),
            "nothing was marked behind the window"
        );
        let mut unscrolled = Screen::new(2);
        unscrolled.size = (960, 200);
        unscrolled.set_view(320, 200);
        unscrolled.mark(700, 50, 16, 16, 3);
        assert!(
            !unscrolled.damaged(100, 50, 16, 16, 3),
            "a mark beyond an unscrolled window is dropped"
        );
    }

    /// The layout the game actually asks for: a main picture and a status bar
    /// that together tile the display, plus a third screen parked outside it.
    #[test]
    fn screens_tile_the_display() {
        let mut d = Display::with_size(Size {
            width: 640,
            height: 480,
        });

        let main = d.new_screen();
        let s = d.screen_mut(main).unwrap();
        s.set_size(640, 400);
        s.view = (640, 400);
        s.view_pos = (0, 0);
        s.buffer.fill(1);

        let status = d.new_screen();
        let s = d.screen_mut(status).unwrap();
        s.set_size(640, 80);
        s.view = (640, 80);
        s.view_pos = (0, 400);
        s.buffer.fill(2);

        let dialogue = d.new_screen();
        let s = d.screen_mut(dialogue).unwrap();
        s.set_size(384, 480);
        s.view = (384, 480);
        s.view_pos = (640, 0);
        s.buffer.fill(3);

        let out = d.compose();
        assert_eq!(out.get(0, 0), Some(1), "main picture at the top");
        assert_eq!(out.get(639, 399), Some(1));
        assert_eq!(out.get(0, 400), Some(2), "status bar below it");
        assert_eq!(out.get(639, 479), Some(2));
        assert!(
            !out.pixels.contains(&3),
            "the third screen sits outside the display and must be clipped away"
        );
    }

    #[test]
    fn handles_start_at_one() {
        let mut d = Display::with_size(Size {
            width: 640,
            height: 480,
        });
        assert_eq!(d.new_screen(), 1);
        assert_eq!(d.new_screen(), 2);
    }
}
