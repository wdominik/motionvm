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
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Error;
use motionvm_motion_forth::Result;
use motionvm_motion_forth::cell;

impl Engine {
    pub(crate) fn words_m16(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
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
            Word::FADEOUT_16 | Word::FADEIN_16 => {
                let a = pop_n(stack, 3, "FADEIN/FADEOUT")?;
                let (mode, duration, step) = (a[0], a[1], a[2]);
                let opening = word == Word::FADEIN_16;
                let screen = self.display.current.unwrap_or(0);
                let was = self
                    .display
                    .screens
                    .iter()
                    .find(|s| s.handle == screen)
                    .map(|s| s.active);
                let showing = self
                    .scene
                    .descriptors
                    .iter()
                    .filter(|d| d.active && d.screen == screen)
                    .count();
                let background = self
                    .scene
                    .descriptors
                    .iter()
                    .find(|d| d.active && d.screen == screen && d.shows.table().is_some())
                    .and_then(|d| d.shows.graphic());
                self.transitions.fades.push(Fade {
                    name: word.name().to_string(),
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
                    self.note_unhandled(word, Some(format!("mode {mode}")));
                    return Ok(Some(()));
                }
                let area = self
                    .display
                    .screens
                    .iter()
                    .find(|s| s.handle == screen)
                    .map(|s| {
                        (
                            i32::from(s.view_pos.0),
                            i32::from(s.view_pos.1),
                            i32::from(s.view.0),
                            i32::from(s.view.1),
                        )
                    })
                    .unwrap_or_else(|| {
                        (
                            0,
                            0,
                            i32::from(self.display.size.width),
                            i32::from(self.display.size.height),
                        )
                    });
                if mode == 0 {
                    // All at once: `FADEOUT` fills the view with black
                    // (`05f1:286b`), `FADEIN` copies it whole out of the
                    // surface (`05f1:2a39`) — one finished wipe does both.
                    let mut wipe = Wipe::new(opening, screen, area, duration, step);
                    wipe.walked = wipe.rings;
                    self.paint_wipe_now(&wipe);
                    return Ok(Some(()));
                }
                self.transitions
                    .wipes
                    .push_back(Wipe::new(opening, screen, area, duration, step));
            }
            Word::RES_EXIST => {
                let n = pop1(stack, "=>EXIST")?;
                let found = self.save_path(n, "blk").is_some_and(|p| p.exists());
                stack.push(if found { -1 } else { 0 });
            }
            // `( handle -- )`: the intro's teardown releases the shadow font
            // it loaded — `_SHFONT @ -FONT`.
            Word::MINUS_FONT => {
                let handle = pop1(stack, "-FONT")?;
                self.scene.fonts.remove(&handle);
            }
            // `( n -- )`: selects font reference table `n`. The handler
            // (`05f1:1813`) spells the number into the engine's file template
            // (`#F0R3i.frt` at `ds:0xf81`), fetches that item through the
            // container hook (`0362:0bf9`), and installs it as the table the
            // text drawer maps characters through (`14ee:10ae`, the pointer at
            // `ds:0x18d8`), freeing the one before. `0 SFT` before the first
            // `+FONT`, in `RUN` and in the intro, is the one call the games
            // make, and table 0 is the one the opener installed.
            Word::SFT => {
                let n = pop1(stack, "SFT")?;
                let table = self
                    .resources
                    .as_ref()
                    .and_then(|r| r.font_ref_table(cell::unsigned(n.max(0))))
                    .ok_or(Error::MissingResource {
                        kind: "font reference table",
                        id: n,
                        word: "SFT",
                        at: None,
                    })?;
                self.scene.font_refs = Some(table);
            }
            // Called once, before `BUFON`, with nothing on the stack to take:
            // a reset of the animation system, which starts out reset.
            Word::NEWANIM => {}
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
            Word::TO_SCRX_16 | Word::TO_SCRY_16 => {
                let a = pop_n(stack, 3, "->SCRX")?;
                let (screen, to, step) = (a[0], a[1], a[2]);
                self.transitions.scroll = Some(crate::Scroll {
                    screen: cell::unsigned(screen),
                    vertical: word == Word::TO_SCRY_16,
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
            Word::SCRX_16 | Word::SCRY_16 => {
                let v = pop1(stack, "SCRX")?;
                if let Some(s) = self.display.current_mut() {
                    if word == Word::SCRX_16 {
                        s.pos.0 = cell::short(v);
                    } else {
                        s.pos.1 = cell::short(v);
                    }
                }
                self.rebuild_current_screen();
            }
            // `( x y -- )`: the window's origin, and — read at file `0x979b` —
            // bit 0 of the screen's flags, which the frame loop answers by
            // clearing the surface and drawing every active descriptor
            // again (`016a:0821`). `SCRX`/`SCRY` above go through it.
            Word::SCRPOS_16 => {
                let a = pop_n(stack, 2, "SCRPOS")?;
                if let Some(s) = self.display.current_mut() {
                    s.pos = (cell::short(a[0]), cell::short(a[1]));
                }
                self.rebuild_current_screen();
            }
            Word::GSCRX_16 => {
                let v = self.display.current_mut().map_or(0, |s| i32::from(s.pos.0));
                stack.push(v);
            }
            // `( -- y x )`: the pair `SCRPOS` sets, read back. The two words
            // are mirrored rather than matched — `SCRPOS` takes x deepest and
            // this leaves x on top (`0104:13bf` in `LL.EXE` pushes the cell at
            // `+2` and then the one at `+0`) — so feeding one into the other
            // swaps them. Only Victor Loomes calls it.
            Word::GSCRPOS => {
                let (x, y) = self
                    .display
                    .current_mut()
                    .map_or((0, 0), |s| (i32::from(s.pos.0), i32::from(s.pos.1)));
                stack.push(y);
                stack.push(x);
            }
            // `( n -- )`: the two colors the system request box is drawn in,
            // and nothing else in either build reads them — `SYSBC`
            // (`0104:3597`) writes `ds:0x1b2`, `SYSFC` (`0104:358e`) writes
            // `ds:0x1b0`, and the only other mention of either is inside
            // `REQUEST`, which fills its box with the first and draws the
            // frame, the button outlines and the text in the second. Kept
            // rather than acted on until `REQUEST` is built.
            Word::SYSFC | Word::SYSBC => {
                let color = pop1(stack, "SYSFC/SYSBC")?;
                if word == Word::SYSFC {
                    self.system_fg = color;
                } else {
                    self.system_bg = color;
                }
            }
            // `( delay last first -- )`: the rotating palette, armed here and
            // turned once a frame by [`Engine::tick_palette_cycle`] — see
            // [`crate::cycle`] for the handler (`0104:5319`) and its tick.
            // With `first >= last` the handler only disarms the rotation,
            // leaving the DAC as the last turn left it.
            Word::SETCYCLE => {
                let a = pop_n(stack, 3, "SETCYCLE")?;
                let (delay, last, first) = (a[0], a[1], a[2]);
                self.palette_cycle =
                    (first < last).then(|| crate::cycle::PaletteCycle::new(first, last, delay));
            }
            // `( a b -- )`: two cells into two globals that nothing in either
            // build ever reads back — `0104:2824` stores them at `ds:0x150`
            // and `ds:0x152`, and a search over both binaries finds no other
            // mention of either address. `RUN` calls it once, `-1 16
            // SETSHADE`. Taking the two cells is the whole of it; anything
            // else would be inventing an effect the original does not have.
            Word::SETSHADE => {
                pop_n(stack, 2, "SETSHADE")?;
            }
            // `( room steps shadow routes aux -- )`: the step buffer between
            // where the walker is and where it is going. The later games
            // reach the same routine through `DOWALK`, which reads the five
            // pointers out of a person record; this game has no `DOWALK` and
            // pops them itself (`0104:4a45` in `LL.EXE`, in this order).
            // What this build's routine does differently — no default for a
            // zero shrink, a closing pass over the headings — is read off
            // the binary when the game opens, see
            // [`crate::Profile::walk_defaults_shrink`] and
            // [`crate::Profile::walk_smooths_headings`].
            Word::CROUTE => {
                let a = pop_n(stack, 5, "CROUTE")?;
                // `pop_n` hands them back deepest first, and the handler pops
                // the aux table first, so the aux table is the top of the
                // stack and the room is the deepest.
                let (room, steps, shadow, routes, aux) = (
                    a[0],
                    cell::unsigned(a[1]),
                    cell::unsigned(a[2]),
                    cell::unsigned(a[3]),
                    cell::unsigned(a[4]),
                );
                crate::walk::croute_with(self, _mem, shadow, steps, routes, aux, room)?;
            }
            // `( -- 0 )`: the last word of the domain table, four
            // instructions long, pushing a constant zero (`0104:0002`). A
            // placeholder in every build; no module calls it.
            Word::POOR => stack.push(0),
            // The music, with this kernel's own stack effects — they differ
            // from the 32-bit engine's. `STARTTUNE ( loop n -- )` (file
            // `0xbf89`) pops the block number and the loop flag and answers
            // nothing; `ENDTUNE ( -- )` (file `0xbfed`) takes nothing — where
            // the 32-bit pair answers a handle and takes it back. The shared
            // arms leaked one cell per location change here.
            //
            // Both go through the sound module's Play routine (`1696:02ce`;
            // `LL.EXE` `0e87:01f2`), which runs the stop routine first when
            // a tune is still playing — the fade, the half-second wait, the
            // stop — and only then hands the driver the new song. Victor
            // Loomes changes its music that way at every location, with no
            // `ENDTUNE` between: the new tune is queued behind the wait.
            Word::STARTTUNE_16 => {
                let a = pop_n(stack, 2, "STARTTUNE")?;
                let (looping, tune) = (a[0], a[1]);
                if self.sound.playing {
                    self.end_tune16();
                    if let Some(hold) = self.transitions.wipes.back_mut() {
                        hold.then_tune = Some((tune, looping));
                    }
                } else {
                    self.start_tune(tune, looping);
                }
            }
            // The stop routine (`1696:02fd`; the other three builds' are the
            // same shape) does nothing while no tune is playing. Otherwise it
            // starts the driver's 2000 ms fade-out and spins until the 200 Hz
            // tick has counted 100 — half a second — before it stops the
            // driver and returns. The audio side keeps that pair's timing on
            // its own thread; the spin is a [`Wipe::hold`], queued where the
            // script stands so that the room change waits for it as the
            // original's does.
            Word::ENDTUNE_16 => {
                if self.sound.playing {
                    self.end_tune16();
                }
            }
            Word::GSCRY_16 => {
                let v = self.display.current_mut().map_or(0, |s| i32::from(s.pos.1));
                stack.push(v);
            }
            // The text-console words, on a debug path in `CTRL`: `.` prints a
            // number, `EMIT` a character. There is no console behind a 320×200
            // game; both take their argument and print nothing.
            Word::DOT | Word::EMIT => {
                pop1(stack, "console")?;
                self.note_unhandled(word, None);
            }
            // `( -- c )`: blocks for a key. The handler (`12c8:061c`) loops on
            // the keyboard translator until it answers something other than
            // 0 — `LL.EXE`'s (file `0xd6f8`) also gives up when Ctrl-Break
            // has been pressed. The one place a shipped script reaches it is
            // Victor Loomes' `PRINT` (`DUP . 32 EMIT KEY DROP`), the hook
            // its assertions call, and this engine answers the key that is
            // waiting, or 0, rather than holding the game with no prompt on
            // the screen: a departure the ledger carries, with the question
            // it raises.
            Word::KEY => {
                self.polled();
                stack.push(self.input.key);
            }
            // `( handle -- )`: the handler (`ENVIRO.EXE` `05f1:08d4`; the same
            // shape at `05e1:0884` in `STERN.EXE`, `05f0:08d5` in `BMZ.EXE`
            // and `0104:14b6` in `LL.EXE`) pops the screen, frees its surface
            // where one is allocated (`+0x20`/`+0x22`) and clears the slot's
            // in-use flag (`+0x1e`); the current-screen variable it leaves
            // alone. Every game's intro tears its screen down with
            // `_MS @ REMSCR`, and a word that left the handle behind would
            // leave every later frame one cell deep — which Falsches Spiel mit
            // Eddie M.'s `CTRL` notices: it opens on `DUP 10 !=`, against the
            // ten constants `RUN` pushes before `TOGFX` and never pops, and
            // prints the top of the stack with `.` and `EMIT` when the check
            // fails. The 32-bit reading, which takes nothing, is
            // [`Word::REMSCR`].
            Word::REMSCR_16 => {
                let handle = pop1(stack, "REMSCR")?;
                self.remove_screen(cell::unsigned(handle.max(0)));
            }
            // `( block mode -- )`: a digital sound effect. Read in
            // `STERN.EXE` (`15e5:0349`, file `0x19399`), the one build whose
            // game calls it; `ENVIRO.EXE`'s (`1696:0357`) is the same
            // handler's first half alone, and no game of that build reaches
            // it. Both cells are popped and the mode is never read again.
            // What follows is two halves. **A tune that is playing is
            // stopped first**: the handler tests the driver's flag
            // (`ds:1af8`, the one `ENDTUNE` tests), resets the tick counter,
            // spins until it reads 100 (`cmp $0x64` — half a second), clears
            // the flag, calls the music driver's Stop entry and the digital
            // driver's StopAll (`1933:0276`, entry 18). That is `ENDTUNE`'s
            // stop routine (`1614:0003`) **without** the 2000 ms fade-out the
            // routine starts first: the tune plays on at full volume through
            // the half second and is cut. **Then the sample**: with a digital
            // driver installed (`ds:108c`) and the configuration's digital
            // flag read as 1 (`ds:7770` set to `0x71` at `1058:00ca`), the
            // block is loaded through the `#F0R3i.blk` template, copied whole
            // into the driver's buffer and handed to the driver's play entry
            // (`1933:0271`, entry 17) with a mode of 0 — always 0, whatever
            // the script passed. motionvm is a machine with an Ad Lib card
            // and a Sound Blaster, the configuration `SOUND.EXE` writes when
            // both are on, so both halves run: the cut, and after the half
            // second the sample, over the sink the way a tune goes.
            Word::PLAYSAMPLE => {
                let a = pop_n(stack, 2, "PLAYSAMPLE")?;
                let block = a[0];
                if self.sound.playing {
                    self.cut_tune16();
                    if let Some(hold) = self.transitions.wipes.back_mut() {
                        hold.then_sample = Some(block);
                    }
                } else {
                    self.play_sample(block);
                }
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
            self.rebuild
                .push((handle, (0, 0, i32::from(w), i32::from(h))));
        }
        self.dirty = true;
    }
}

impl Engine {
    /// The 16-bit stop routine (`1696:02fd`) with a tune playing: the
    /// driver's fade-out begins, and the script waits half a second
    /// ([`Wipe::hold`]) before the driver is stopped and the word returns.
    /// The audio side keeps the pair's own timing — the fade at once, the
    /// hard stop 500 ms in — so what is queued here is only the wait.
    pub(crate) fn end_tune16(&mut self) {
        self.sound.playing = false;
        if let Some(music) = self.sound.sink.as_mut() {
            music.stop(0);
        }
        self.transitions
            .wipes
            .push_back(Wipe::hold(Self::ENDTUNE_HOLD_TICKS));
    }

    /// `PLAYSAMPLE`'s stop of a playing tune (`STERN.EXE` `15e5:035d`): the
    /// stop routine's wait and its hard stop, without the fade-out that
    /// routine starts first. The script holds half a second
    /// ([`Wipe::hold`]) while the tune plays on at full volume; the audio
    /// side lands the driver's Stop entry when that half second is up.
    pub(crate) fn cut_tune16(&mut self) {
        self.sound.playing = false;
        if let Some(music) = self.sound.sink.as_mut() {
            music.cut(0);
        }
        self.transitions
            .wipes
            .push_back(Wipe::hold(Self::ENDTUNE_HOLD_TICKS));
    }
}
