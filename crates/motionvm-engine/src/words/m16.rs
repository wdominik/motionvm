//! The words of the 16-bit kernel that the 32-bit kernel does not have, or
//! has under another name — asked first on the 16-bit machine.
//!
//! `=>GET`, `=>ERASE` and `SCRCTRL` need the machine itself and are answered
//! in the 16-bit `Host` impl; what is here works on the stack and the engine
//! alone. Every arity below is read off the call sites in Die Enviro-Kids
//! greifen ein's modules; where a handler in `ENVIRO.EXE` has been read at
//! the instruction level, the arm says so.

use crate::Engine;
use crate::Fade;
use crate::Wipe;
use crate::stack::{pop_n, pop1};
use motionvm_forth::AddressSpace;
use motionvm_forth::Result;

impl Engine {
    pub(crate) fn words_m16(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match name {
            // `( n -- flag )`: whether save slot `n` exists — `706 701 DO I
            // =>EXIST LOOP` in `RUN`. The 32-bit kernel asks the same of a
            // file with `EXIST`; here it is the `=>` family's word, so the
            // answer is the same test on the same file.
            // `( mode duration step -- )`, same shape as the 32-bit pair —
            // but the 16-bit handlers (`FADEOUT` `05f1:2827`, `FADEIN`
            // `05f1:29e4`, both read) draw a box, not a band: the black
            // frame closes on the view's center, the picture opens back out
            // of it, ring by ring — see [`Wipe`]. Their bookkeeping around
            // it: `FADEIN` sets the screen active and composes it whole
            // (`016a:0821`) before the first ring, whatever the mode;
            // `FADEOUT` sets it inactive after the last, and unlike the
            // 32-bit handler it leaves the screen's surface untouched —
            // only the display goes black. Mode 0 is the same effect all at
            // once; the game passes `1 50 8` at every one of its sites.
            "FADEOUT" | "FADEIN" => {
                let a = pop_n(stack, 3, "FADEIN/FADEOUT")?;
                let (mode, duration, step) = (a[0], a[1], a[2]);
                let opening = name == "FADEIN";
                let screen = self.display.current.unwrap_or(0);
                let was = self
                    .display
                    .screens
                    .iter()
                    .find(|s| s.handle == screen)
                    .map(|s| s.active);
                let showing = self
                    .descriptors
                    .iter()
                    .filter(|d| d.active && d.screen == screen)
                    .count();
                let background = self
                    .descriptors
                    .iter()
                    .find(|d| d.active && d.screen == screen && d.shows.table().is_some())
                    .and_then(|d| d.shows.graphic());
                self.fades.push(Fade {
                    name: name.to_string(),
                    screen,
                    was_active: was.unwrap_or(false),
                    showing,
                    background,
                });
                if let Some(s) = self.display.current_mut() {
                    s.active = opening;
                }
                if opening {
                    // `SCRACT` (`05f1:094a`) sets bit 0 of the screen's
                    // flags — the same rebuild request `SCRPOS` makes — so
                    // the compose `FADEIN` then runs (`016a:0821`) takes
                    // its **full** branch: surface cleared, save-unders
                    // forgotten, every active descriptor drawn
                    // (`016a:092c`–`016a:09ef`). That clear is what takes
                    // the previous picture off a surface its descriptors no
                    // longer cover — the intro swaps its motifs under a
                    // FADEOUT/FADEIN pair and erases nothing itself, and
                    // the start-up page's teardown (`KILL_MENU`) leans on
                    // it the same way. (`SCRINACT`, `05f1:09b9`, freezes
                    // the screen and requests the same rebuild; its
                    // frozen-path clear, `016a:0880`, is subsumed here.)
                    self.rebuild_current_screen();
                    self.repaint_screen(screen);
                    self.draw_screen(screen);
                }
                if mode != 0 && mode != 1 {
                    // `05f1:284e`, `05f1:2a1c`: any other mode skips the
                    // effect; the flag work above has already happened.
                    self.note_unhandled(format!("{name} (mode {mode})"));
                    return Ok(Some(()));
                }
                let area = self
                    .display
                    .screens
                    .iter()
                    .find(|s| s.handle == screen)
                    .map(|s| {
                        (
                            s.view_pos.0 as i32,
                            s.view_pos.1 as i32,
                            s.view.0 as i32,
                            s.view.1 as i32,
                        )
                    })
                    .unwrap_or((0, 0, self.display.size.0 as i32, self.display.size.1 as i32));
                if mode == 0 {
                    // All at once: `FADEOUT` fills the view with black
                    // (`05f1:286b`), `FADEIN` copies it whole out of the
                    // surface (`05f1:2a39`) — one finished wipe does both.
                    let mut wipe = Wipe::new(opening, screen, area, duration, step);
                    wipe.walked = wipe.rings;
                    self.paint_wipe_now(&wipe);
                    return Ok(Some(()));
                }
                self.wipes
                    .push_back(Wipe::new(opening, screen, area, duration, step));
            }
            "=>EXIST" => {
                let n = pop1(stack, "=>EXIST")?;
                let found = self.save_path(n, "blk").is_some_and(|p| p.exists());
                stack.push(if found { -1 } else { 0 });
            }
            // `( handle -- )`: the intro's teardown releases the shadow font
            // it loaded — `_SHFONT @ -FONT`.
            "-FONT" => {
                let handle = pop1(stack, "-FONT")?;
                self.fonts.remove(&handle);
            }
            // `( n -- )`: `0 SFT` before the first `+FONT`, in `RUN` and in
            // the intro, and nowhere with another argument. Read as a reset
            // of the font stack — of a stack that starts out reset.
            "SFT" => {
                let n = pop1(stack, "SFT")?;
                if n != 0 {
                    self.note_unhandled(format!("SFT {n}"));
                }
            }
            // Called once, before `BUFON`, with nothing on the stack to take:
            // a reset of the animation system, which starts out reset.
            "NEWANIM" => {}
            // `( screen x step -- )` and `( screen y step -- )`: scroll a
            // screen's window over its surface to a new origin, `step`
            // pixels a tick. Read from `ENVIRO.EXE` (`->SCRX` at file
            // `0xc149`): the handler pops the step, the target and the
            // screen number, writes the screen's origin — the field `SCRPOS`
            // writes — and then blits its own frames, one per `DELAY` tick,
            // until the origin has arrived; the word does not return until
            // it has. Every call in the game is `_MS @ x 8 ->SCRX` inside
            // `SETBUSY`/`SETNOBUSY`. Run here as a transition: the slide is
            // queued, the interpreter is held the way a fade holds it, and
            // each frame moves one step and presents.
            "->SCRX" | "->SCRY" => {
                let a = pop_n(stack, 3, "->SCRX")?;
                let (screen, to, step) = (a[0], a[1], a[2]);
                self.scroll = Some(crate::Scroll {
                    screen: screen as u32,
                    vertical: name == "->SCRY",
                    target: to,
                    step,
                });
                self.dirty = true;
            }
            // `( x -- )` / `( y -- )`: the 16-bit `SCRX` (file `0x9809`) is
            // `SCRPOS` with the other coordinate kept — it writes the screen's
            // window origin, not the separate pair the 32-bit handler keeps —
            // and `GSCRX` (file `0x97da`) reads the same cell back. The
            // scripts scroll a wide location with it (`34 SCRX`, `152 SCRX`
            // in the location macros) and add it to the mouse for world
            // coordinates.
            "SCRX" | "SCRY" => {
                let v = pop1(stack, "SCRX")?;
                if let Some(s) = self.display.current_mut() {
                    if name == "SCRX" {
                        s.pos.0 = v as i16;
                    } else {
                        s.pos.1 = v as i16;
                    }
                }
                self.rebuild_current_screen();
            }
            // `( x y -- )`: the window's origin, and — read at file `0x979b` —
            // bit 0 of the screen's flags, which the frame loop answers by
            // clearing the surface and drawing every active descriptor
            // again (`016a:0821`). `SCRX`/`SCRY` above go through it.
            "SCRPOS" => {
                let a = pop_n(stack, 2, "SCRPOS")?;
                if let Some(s) = self.display.current_mut() {
                    s.pos = (a[0] as i16, a[1] as i16);
                }
                self.rebuild_current_screen();
            }
            "GSCRX" => {
                let v = self.display.current_mut().map_or(0, |s| s.pos.0 as i32);
                stack.push(v);
            }
            // The music, with this kernel's own stack effects — they differ
            // from the 32-bit engine's. `STARTTUNE ( loop n -- )` (file
            // `0xbf89`) pops the block number and the loop flag and answers
            // nothing; `ENDTUNE ( -- )` (file `0xbfed`) takes nothing — where
            // the 32-bit pair answers a handle and takes it back. The shared
            // arms leaked one cell per location change here.
            "STARTTUNE" => {
                let a = pop_n(stack, 2, "STARTTUNE")?;
                let (looping, tune) = (a[0], a[1]);
                self.start_tune(tune, looping);
            }
            "ENDTUNE" => {
                if let Some(music) = self.music.as_mut() {
                    music.stop(0);
                }
            }
            "GSCRY" => {
                let v = self.display.current_mut().map_or(0, |s| s.pos.1 as i32);
                stack.push(v);
            }
            // The text-console words, on a debug path in `CTRL`: `.` prints a
            // number, `EMIT` a character. There is no console behind a 320×200
            // game; both take their argument and print nothing.
            "." | "EMIT" => {
                pop1(stack, "console")?;
                self.note_unhandled(name.to_string());
            }
            // `( -- c )`: blocks for a key in the original, on the same debug
            // path. Answering the key that is waiting, or 0, is the reading
            // that does not stall a run; hypothesis.
            "KEY" => {
                self.polled();
                stack.push(self.key);
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}

impl Engine {
    /// The 16-bit rebuild flag: the next drawing pass builds the active
    /// screen's whole surface again over a cleared one, as the frame loop
    /// does when `SCRPOS` has set bit 0 of the screen's flags.
    pub(crate) fn rebuild_current_screen(&mut self) {
        if let Some(s) = self.display.current_mut() {
            let (handle, (w, h)) = (s.handle, s.size);
            self.rebuild.push((handle, (0, 0, w as i32, h as i32)));
        }
        self.dirty = true;
    }
}
