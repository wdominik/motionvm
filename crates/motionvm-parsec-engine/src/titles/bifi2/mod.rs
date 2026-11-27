//! *Bi-Fi 2 - Action in Hollywood* (1994).
//!
//! The third title on the `parsec1994` generation, and the oldest build of
//! its library: the dispatch, the record fields, the rule format and the
//! handler's shape are Dunkle Schatten's, but the record is `44h` bytes
//! with sixteen exits, the kinds mean other things, and many library
//! routines are an earlier version, which the runtime reads off the
//! program ([`crate::titles::parsec1994::Profile`]). Each place of the
//! location table has its handler's logic ([`logic`]), one module per
//! location, and the global rules and the shared hook are this program's
//! own.
//!
//! Play begins in the title program `XP00.EXE` ([`xp00`]) as `BIFI.BAT`
//! runs it, and its menu's choice starts `XP01`.

mod bi_fi;
mod buero;
pub(crate) mod build;
mod denkfabrik;
mod dino_zoo;
mod ending;
mod enterhaken_bruecke;
mod enterhaken_maschinenraum;
mod flur;
mod gefaengnis;
mod global_rules;
mod hook;
mod interlude_13;
mod interlude_2;
mod interlude_3;
mod interlude_39;
mod interlude_6;
mod jones1;
mod jones2;
mod kaffee;
mod kantine;
mod labyrinth;
mod lift;
mod location_46;
mod logic;
mod maske;
mod nullachtfuenfzehn;
mod oeliens;
mod oeliens_halle;
mod onward;
mod pforte;
#[cfg(test)]
mod playthrough;
mod rambowsky;
mod requisite;
mod restore;
mod sfx;
mod technik;
mod tontechnik;
mod vip_lounge;
mod vorfuehrraum;
mod vorzimmer;
pub(crate) mod xp00;
mod xp01data;

use std::path::Path;

use motionvm_parsec_formats::Title;

use crate::error::Result;
use crate::titles::parsec1994::install::Install;
use crate::titles::parsec1994::program::Program;

/// One loaded install of *Bi-Fi 2*.
pub(crate) type BiFi2 = Install<xp00::Xp00>;

impl Install<xp00::Xp00> {
    /// Opens the install at `dir`. Nothing is played yet: the start, which
    /// reads the save directory, is [`crate::titles::Runtime::start`].
    ///
    /// The program is read twice over: by the generation, for its tables
    /// and its library's build, and by this title's own reading
    /// ([`xp01data::ExeText`]) for what only it finds.
    pub(crate) fn open(dir: &Path) -> Result<Self> {
        let image = crate::exe::Image::open(dir, build::BUILD.program.0, build::BUILD.program.1)?;
        let locations = crate::titles::xp1994::Locations::read(&image)?;
        let text = xp01data::ExeText::read(&image, &locations)?;
        let game = Program::open(dir, &build::BUILD, |l| Ok(build::reading(&text, l)))?;
        Ok(Install::new(Title::BiFi2, game, xp00::Xp00::open))
    }
}

#[cfg(test)]
mod tests {
    use crate::titles::parsec1994::program::Program;

    /// What the visit's routines do with the objects, the tasks, Escape,
    /// the verbs, the entry, the strip, the scenes and the options' gate.
    #[test]
    fn the_visit_s_routines_are_read() {
        use crate::titles::parsec1994::profile::{
            OptionsGate, OptionsHeld, Profile, RecordCodes, SecondWanted,
        };
        let Some(dir) = motionvm_parsec_testutil::gamedata_bifi2() else {
            return;
        };
        let b = &super::build::BUILD;
        let img = crate::exe::Image::open(&dir, b.program.0, b.program.1).expect("XP01.EXE");
        let p = Profile::read(&img, &b.library).expect("reads");
        assert_eq!(p.tri_state_kinds, 0b10_0010);
        assert!(!p.task_start_needs_armed);
        assert_eq!(p.escape_verb, 0x0E);
        assert!(!p.verb6_arm);
        assert_eq!(p.second_wanted, SecondWanted::Rules);
        assert!(!p.place_falls_back);
        assert!(!p.show_hides_parts && !p.hide_clears_head);
        assert!(!p.own_line_sets_speakers);
        assert_eq!(p.status_after, 2);
        assert_eq!((p.interaction.items_end, p.interaction.verb), (0x30, 3));
        assert!(!p.kind8_records);
        assert_eq!(p.anim_arms, 0x0A);
        {
            // `09B6h:1A75h`, codes 1 to 9 through `cs:1BC0h` (`0x0B720`), the
            // samples by the object's id through `cs:1BACh`, `cs:1B98h` and
            // `cs:1B84h` and the chain at `0x0B645`; code 8 runs on into
            // code 2's arm (`0x0B63C`).
            use crate::titles::parsec1994::signal::SignalAct::{
                Release, Sample, TaskArm, TaskStart,
            };
            let s = |k: &[u16]| k.iter().map(|&k| Sample(k)).collect::<Vec<_>>();
            assert_eq!(
                crate::titles::parsec1994::signal::tests::listed(&p.signal),
                [
                    (1, vec![Release(1)], vec![]),
                    (
                        2,
                        vec![],
                        vec![(0x72, s(&[0x11])), (0x73, s(&[0x11])), (0x7B, s(&[0x12]))]
                    ),
                    (
                        3,
                        vec![],
                        vec![
                            (0x72, s(&[0x11, 0x14])),
                            (0x73, s(&[0x11, 0x14])),
                            (0x79, s(&[0x18, 0x0E])),
                            (0x7B, s(&[0x12])),
                        ]
                    ),
                    (4, vec![], vec![]),
                    (5, vec![TaskArm(0)], vec![]),
                    (6, vec![TaskStart(0)], vec![]),
                    (7, vec![], vec![]),
                    (
                        8,
                        s(&[3]),
                        vec![
                            (0x72, s(&[0x10, 3, 0x11])),
                            (0x73, s(&[0x10, 3, 0x11])),
                            (0x79, s(&[0x1F])),
                            (0x7B, s(&[0x12, 3, 0x12])),
                        ]
                    ),
                    (
                        9,
                        s(&[4]),
                        vec![
                            (0x72, s(&[0x11, 0x10, 4])),
                            (0x73, s(&[0x10, 4])),
                            (0x79, vec![]),
                            (0x7B, s(&[0x12, 4])),
                        ]
                    ),
                ]
            );
        }
        let held = OptionsHeld {
            bit: 8,
            here_word: super::logic::WORDS.here,
            silent_at: 0x24,
            scene: !0x4B,
            wait: 0x10,
        };
        assert_eq!(
            p.options_gate,
            OptionsGate {
                drops_ticks: false,
                held: Some(held)
            }
        );
        assert_eq!(p.inventory_rows, Some(4));
        let sc = p.scenes;
        assert_eq!(
            (sc.item_base, sc.bounded, sc.falls_back),
            (0x4000, false, false)
        );
        assert_eq!(p.hotspot_kept, 0b1_1110);
        let r = p.records;
        assert_eq!(
            (r.codes, r.read, r.fresh),
            (RecordCodes::Plain, [Some(1), None, None], Some(4))
        );
    }

    /// The strip's 60-row picture goes on row 8Ch, below the band
    /// (`0x0761F`); the entry pass blacks a bank's first color
    /// (`0x0F980`); the options fill with 51h and 50h, the two entries
    /// their colors go to, and frame the confirmation with 7Eh, 7Fh and
    /// 6Eh, each a `push imm8`.
    #[test]
    fn the_strip_and_the_options_take_this_build_s_immediates() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_bifi2() else {
            return;
        };
        let b = &super::build::BUILD;
        let img = crate::exe::Image::open(&dir, b.program.0, b.program.1).expect("XP01.EXE");
        let locations = crate::titles::xp1994::Locations::read(&img).expect("the table");
        let text = super::xp01data::ExeText::read(&img, &locations).expect("the program's text");
        assert_eq!(text.strip_row, 0x8C);
        let o = &super::build::OPTIONS;
        let pushed: Vec<u8> = o
            .grounds
            .into_iter()
            .chain(o.box_inks)
            .map(|at| {
                assert_eq!(img.bytes()[at], 0x6A, "{at:#x}");
                img.bytes()[at + 1]
            })
            .collect();
        assert_eq!(pushed, [0x51, 0x50, 0x7E, 0x7F, 0x6E]);
    }

    /// What the runtime reads off this build's library, each value as
    /// the routine named beside it in `build.rs` does it.
    #[test]
    fn the_library_build_is_the_one_the_runtime_was_read_from() {
        use crate::titles::parsec1994::profile::{
            EntryEffect, ExitForm, FigureSet, MapExits, MenuBand, ObjectState, Profile,
            SharedRooms, StripRange,
        };
        let Some(dir) = motionvm_parsec_testutil::gamedata_bifi2() else {
            return;
        };
        let b = &super::build::BUILD;
        let img = crate::exe::Image::open(&dir, b.program.0, b.program.1).expect("XP01.EXE");
        let p = Profile::read(&img, &b.library).expect("reads");
        assert_eq!(p.object_state, ObjectState::Bit1);
        assert_eq!(
            p.rooms,
            SharedRooms {
                pairs: [Some(2), Some(4), Some(7)],
                shared: [Some((2, 0x25)), Some((4, 0x26))],
                sound_kept: Some(7),
                sound_flags: 0x0C,
                exits: Some(MapExits {
                    kind: 4,
                    first: 9,
                    form: ExitForm::Offset(0x8C),
                }),
            }
        );
        let d = p.entry_door;
        let more = [Some((0x78, true)), Some((0x79, false)), Some((0x7B, true))];
        assert_eq!(
            (d.ids, d.kind, d.remembered, d.frame),
            ((0x6E, 0x77), 4, true, -1)
        );
        assert_eq!(d.more, more);
        assert_eq!(p.slot_style.map(|s| s.horizontal), Some(None));
        assert_eq!(p.strip, StripRange::Unranged);
        assert_eq!(p.figure_set, FigureSet::Kept);
        assert_eq!(p.figure_first, 0, "no step past a palette, `0x0B7A8`");
        assert_eq!((p.status_row, p.status_band), (0x80, 0x48));
        assert_eq!(
            (p.status_inks.ink, p.status_inks.controls),
            (0x74, [(0x09, 0x7A), (0x0A, 0x7C), (0x0D, 0x74)]),
            "`0x07843`, `0x079A2`"
        );
        assert_eq!(
            p.menu_band,
            Some(MenuBand {
                row: 0x80,
                cleared: 0x0C,
                dark: 0x48
            }),
            "`0x07A62`, `0x07A7C`"
        );
        assert_eq!(p.leave_fade, 2000);
        assert_eq!(
            (p.leave_spares, p.leave_clears_status),
            ([Some(2), Some(4), Some(7)], false)
        );
        assert!(!p.mode_step_ages_status, "`0x0A55F`");
        let e = p.entry_end;
        let look = |loc, pair| {
            let l = e.look(loc, pair);
            (l.effect, l.song)
        };
        assert_eq!(e.location_word, Some(0x200C));
        assert_eq!(look(1, 0), (Some(EntryEffect::Reveal), true));
        assert_eq!(look(0x24, 2), (None, false));
        assert_eq!(look(5, 2), (Some(EntryEffect::Reveal), false));
        assert_eq!(look(5, 4), (Some(EntryEffect::Slide), false));
        assert_eq!(look(5, 7), (Some(EntryEffect::Reveal), false));
        assert_eq!(look(5, 0), (Some(EntryEffect::Slide), true));
        assert!(p.mode_reset_forgets_hover);
        assert_eq!(p.inventory_window, 8);
        let crate::titles::xp1994::Flatten::Shade(shade) =
            crate::titles::xp1994::Flatten::read(&img).expect("reads")
        else {
            panic!("Bi-Fi 2 darkens by the pixel's bit 7");
        };
        // Ramp 0, from 0 with the divisor 14h: one step a value up to 20,
        // then two a value (`0x010D3`).
        assert_eq!(
            shade.ramps[..24],
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 21,
                22
            ]
        );
        assert_eq!(shade.ramps[63], 38);
    }

    use crate::titles::parsec1994::Event;

    /// The game opened and started fresh, its start's options closed with
    /// Escape.
    fn fresh_game(saves: &str) -> Option<Program> {
        let Some(dir) = motionvm_parsec_testutil::gamedata_bifi2() else {
            eprintln!("skipping: no Bi-Fi 2 gamedata directory");
            return None;
        };
        let img = crate::exe::Image::open(&dir, "XP01.EXE", 164_576).expect("XP01.EXE");
        let locations = crate::titles::xp1994::Locations::read(&img).expect("the table");
        let text = super::xp01data::ExeText::read(&img, &locations).expect("the program's text");
        let mut g = Program::open(&dir, &super::build::BUILD, |l| {
            Ok(super::build::reading(&text, l))
        })
        .expect("opens");
        g.set_saves(&motionvm_parsec_testutil::saves_dir(saves))
            .expect("saves");
        g.boot(crate::titles::Start::New);
        for _ in 0..400 {
            g.tick();
        }
        assert!(g.inner.options_up(), "the start's options");
        g.key(crate::titles::parsec1994::KEY_ESCAPE);
        Some(g)
    }

    /// Under a fresh choice menu the strip redraw `0751h:04A2h` fills the
    /// line's 0Ch rows from 80h with color 0 (`0x07A62`) and holds the 48h
    /// rows from 80h dark (`0x07A7C`): every pixel of the band has bit 7
    /// on, but the menu's lines, set over it afterwards in their own ink
    /// (`0D22h:0345h`, `0x09652`).
    #[test]
    fn a_choice_menu_holds_the_band_dark_under_its_lines() {
        let Some(mut g) = fresh_game("bifi2-menu-band") else {
            return;
        };
        let mut n = 0;
        while !(g.inner.location_word == 4 && g.inner.menu.up) {
            if n % 300 == 299 {
                g.click(true);
            }
            g.tick();
            n += 1;
            assert!(n < 30_000, "no choice at the Pforte");
        }
        let fb = g.inner.render_scene();
        // A line's ink and the highlighted choice's (`0x0D5FE`, `add dx, 3`
        // at `0x0D613`), 78h and 68h, each with the glyph's second shade.
        let [a, b] = [g.inner.menu_shape.inks[0], g.inner.menu_shape.inks[3]];
        assert_eq!((a, b), (0x78, 0x68));
        let inks = [a, a + 1, b, b + 1];
        let (mut dark, mut lines) = (0, 0);
        for y in 0x80..0xC8 {
            for x in 0..320 {
                let p = fb.get(x, y).expect("on the screen");
                if inks.contains(&p) {
                    lines += 1;
                } else {
                    assert_eq!(p & 0x80, 0x80, "({x}, {y}) is held dark");
                    dark += 1;
                }
                if y < 0x8C && !inks.contains(&p) {
                    assert_eq!(p, 0x80, "({x}, {y}): color 0, held dark");
                }
            }
        }
        assert!(
            lines > 0 && dark > lines,
            "{lines} line pixels, {dark} dark"
        );
    }

    /// At the Pforte the player walks up to the gatekeeper, object 81h, and
    /// rule `00C9h`'s first scene of its sequence plays under mode 7
    /// (`0x10E54`–`0x10E82`): the conversation opens on a choice of three.
    /// A line chosen, its answer plays and the next choice comes up.
    #[test]
    fn the_pforte_opens_its_conversation() {
        use crate::titles::parsec1994::Event;
        let Some(mut g) = fresh_game("bifi2-pforte") else {
            return;
        };
        let mut n = 0;
        while !(g.inner.location_word == 4 && g.inner.menu.up) {
            if n % 300 == 299 {
                g.click(true);
            }
            g.tick();
            n += 1;
            assert!(n < 30_000, "no choice at the Pforte");
        }
        let lines: Vec<Vec<u8>> = g.inner.menu.lines.iter().map(|l| l.text.clone()).collect();
        let cp = |s: &str| motionvm_parsec_formats::to_cp437(s);
        assert_eq!(
            lines,
            [
                cp("Sind Sie kurzsichtig?"),
                cp("Sind Sie der Pförtner?"),
                cp("Ähh - \"junge Frau?\""),
            ]
        );
        g.inner.events.clear();
        // The menu's record at `DS:058Ah`: from row 84h, 0Dh a line. The
        // second line, "Sind Sie der Pförtner?", clicked.
        g.inner.mouse_move(100, 0x84 + 0x0D + 5);
        assert_eq!(g.inner.menu.highlight, 2);
        assert!(g.inner.click());
        assert!(!g.inner.menu.up, "the choice taken");
        // The answer is pages of the same dialogue, and the next choice
        // follows it.
        let mut next = None;
        for _ in 0..2_000 {
            g.tick();
            for e in g.inner.events.drain(..) {
                if let Event::MenuUp { lines } = e {
                    next = Some(lines);
                }
            }
            if next.is_some() {
                break;
            }
        }
        let next: Vec<String> = next
            .expect("the next choice")
            .into_iter()
            .map(|(_, l)| l)
            .collect();
        assert_eq!(
            next,
            [
                "Ich bin vom Kurierdienst.",
                "Ich soll hier was abgeben.",
                "Ein berühmter Schauspieler."
            ]
        );
    }

    /// "Betrachte" on the item the game starts with, 27h: row 44
    /// (`0FE3h:0404h`) plays `~(27h + 3FFFh)` (`0x10276`), which the scene
    /// lookup takes as the inventory bank's place 26h (`add ax, 0C000h`,
    /// `0x08657`) — the item's own line.
    #[test]
    fn looking_at_an_item_names_it() {
        let Some(mut g) = fresh_game("bifi2-look") else {
            return;
        };
        for _ in 0..200 {
            g.tick();
        }
        let name = g.inner.item_name(0x27).expect("item 27h").to_owned();
        let line = g
            .inner
            .scene_text_for_test(!(0x27 + 0x3FFF))
            .expect("a line");
        assert_eq!(name, "Drehbuch");
        assert!(line.starts_with("Das Drehbuch"), "{line}");
    }

    /// Ticks until the location `here` runs, clicking the Kaffee's opening
    /// scene through as `the_pforte_opens_its_conversation` does, and fifty
    /// more; every event on the way is kept in `seen`.
    fn run_until(g: &mut Program, here: u16, what: &str, seen: &mut Vec<Event>) {
        let mut n = 0;
        while g.inner.here_word != here || g.inner.location_word != here {
            if n % 300 == 299 {
                g.click(true);
            }
            g.tick();
            seen.append(&mut g.inner.events);
            n += 1;
            assert!(n < 30_000, "{what}");
        }
        for _ in 0..50 {
            g.tick();
            seen.append(&mut g.inner.events);
        }
    }

    /// A fresh game in the ground floor's corridor, location 18, left the
    /// Pforte for by its exit `6Eh` as an exit rule leaves it — the
    /// conversation that opens the gate is not played through here.
    fn on_the_ground_floor(saves: &str, seen: &mut Vec<Event>) -> Option<Program> {
        use crate::titles::parsec1994::Api;
        let mut g = fresh_game(saves)?;
        run_until(&mut g, 4, "the Pforte", seen);
        let w = super::logic::WORDS;
        Api::set_word(&mut g.inner, w.entry, 0x6E);
        Api::set_word(&mut g.inner, w.location, 18);
        run_until(&mut g, 18, "the ground floor", seen);
        Some(g)
    }

    /// Loading in game sends the main loop to row 91 (`0x0F376`), whose
    /// handler `0EE6h:0523h` copies the held game back (`0x0F383`–
    /// `0x0F42E`): a story byte changed after the save is the saved one
    /// again, and the main loop goes on in the corridor the game was saved
    /// in.
    #[test]
    fn loading_in_game_goes_back_to_the_saved_location() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = on_the_ground_floor("bifi2-restore", &mut seen) else {
            return;
        };
        let story = 0x2B98 + 0x20;
        g.inner.sav_set(story, 0x5A);
        g.inner
            .save_044e(&super::build::SAVE, 0x0001_0203)
            .expect("in a location, a save is written");
        g.inner.sav_set(story, 0);
        g.inner.load_0518(&super::build::SAVE);
        let w = super::logic::WORDS;
        assert_eq!(
            (
                Api::word(&g.inner, w.location),
                Api::word(&g.inner, w.entry)
            ),
            (91, 1)
        );
        run_until(&mut g, 18, "back in the corridor", &mut seen);
        assert!(
            !seen.iter().any(|e| matches!(e, Event::Unbuilt(..))),
            "every row on the way has its logic"
        );
        assert_eq!(g.inner.sav_byte(story), 0x5A);
    }

    /// The pointer on the hotspot whose id is `id`, and a click.
    fn click_spot(g: &mut Program, id: u16) {
        let room = g.inner.room().expect("a room");
        let i = room
            .hotspots
            .spots
            .iter()
            .position(|s| s.verb_mask & 0xFF == id)
            .unwrap_or_else(|| panic!("hotspot {id:#x}"));
        let h = &g.inner.hotspots_rt[i];
        let (x, y) = (h.x + h.w / 2, h.y + h.h / 2);
        g.inner.mouse_move(x, y);
        assert_eq!(
            g.inner.hotspot_index_under_pointer(),
            Some(i),
            "on hotspot {id:#x}"
        );
        g.click(true);
    }

    /// The pointer on the panel's button for verb `v`, and a click.
    fn click_verb(g: &mut Program, v: u8) {
        let w = g
            .inner
            .panel_widgets
            .widgets
            .iter()
            .find(|w| crate::titles::panel::Panel::verb_of(w) == Some(v))
            .unwrap_or_else(|| panic!("verb {v}"));
        let (x, y) = (
            i32::from(w.x) + i32::from(w.w) / 2,
            i32::from(w.y) + i32::from(w.h) / 2,
        );
        g.inner.mouse_move(x, y);
        g.click(true);
        g.tick();
        assert_eq!(g.inner.verb, v, "verb {v} picked");
    }

    /// A fresh game run on into `loc` by the exit `door`, as an exit rule
    /// leaves the ground floor for it.
    fn entered(saves: &str, door: u16, loc: u16, seen: &mut Vec<Event>) -> Option<Program> {
        use crate::titles::parsec1994::Api;
        let mut g = on_the_ground_floor(saves, seen)?;
        let w = super::logic::WORDS;
        Api::set_word(&mut g.inner, w.entry, door);
        Api::set_word(&mut g.inner, w.location, loc);
        run_until(&mut g, loc, "the location", seen);
        Some(g)
    }

    /// Ticks until `done`, at most `limit` ticks, clicking every 150 to
    /// move scenes on.
    fn tick_until(
        g: &mut Program,
        seen: &mut Vec<Event>,
        limit: usize,
        what: &str,
        done: impl Fn(&Program, &[Event]) -> bool,
    ) {
        let mut n = 0;
        while !done(g, seen) {
            if n % 150 == 149 {
                g.click(true);
            }
            g.tick();
            seen.append(&mut g.inner.events);
            n += 1;
            assert!(n < limit, "{what}");
        }
    }

    /// The Technik entered in state 0: object `91h` stands on animation 3
    /// (`0x146CC`). `Sprich mit` on it, without the `Nachricht`, runs rule
    /// `006Eh`: state 1 and scene 1 (`0x14649`, `0x1465D`).
    #[test]
    fn talking_in_the_technik_moves_it_on() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-technik", 0x6E, 15, &mut seen) else {
            return;
        };
        let i = g.inner.object_index(0x91).expect("91h");
        assert_eq!(g.inner.objects[i].flags & 1, 1, "91h shown");
        assert_eq!(Api::word(&g.inner, 0x2EEC), 3, "its rest");
        click_verb(&mut g, 3);
        click_spot(&mut g, 0x91);
        tick_until(&mut g, &mut seen, 5_000, "scene 1", |g, seen| {
            Api::byte(&g.inner, 0x2BB6) == 1
                && seen
                    .iter()
                    .any(|e| matches!(e, Event::SceneBound { scene: 1, .. }))
        });
        nothing_unbuilt(&seen);
    }

    /// The Denkfabrik in state 1, as Rambowsky leaves it (`0x11DC0`): the
    /// entry plays scene 3 as an interaction with object `94h` and moves on
    /// to state 2 (`0x14E7D`, `0x14E85`); `94h` stands (`0x14EB2`). While
    /// it speaks the hook's code 8 puts it on animation `arg + 1`
    /// (`0x14F77`), and the scene opens a conversation.
    #[test]
    fn the_denkfabrik_greets_once_rambowsky_has_sent_you() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = on_the_ground_floor("bifi2-denkfabrik", &mut seen) else {
            return;
        };
        Api::set_byte(&mut g.inner, 0x2BBE, 1);
        let w = super::logic::WORDS;
        Api::set_word(&mut g.inner, w.entry, 0x70);
        Api::set_word(&mut g.inner, w.location, 17);
        tick_until(&mut g, &mut seen, 5_000, "the Denkfabrik", |g, _| {
            g.inner.here_word == 17
        });
        assert_eq!(Api::byte(&g.inner, 0x2BBE), 2, "state 2");
        assert_eq!(g.inner.verb, 3, "the interaction's verb");
        let i = g.inner.object_index(0x94).expect("94h");
        assert_eq!(g.inner.objects[i].flags & 1, 1, "94h shown");
        tick_until(&mut g, &mut seen, 5_000, "the conversation", |g, _| {
            g.inner.menu.up
        });
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 3, .. })),
            "scene 3"
        );
        assert!(
            seen.iter().any(|e| matches!(
                e,
                Event::ObjectAnim {
                    object: 0x94,
                    anim: 2
                }
            )),
            "94h speaking"
        );
        let cp = |s: &str| motionvm_parsec_formats::to_cp437(s);
        let lines: Vec<Vec<u8>> = g.inner.menu.lines.iter().map(|l| l.text.clone()).collect();
        assert_eq!(
            lines,
            [
                cp("Ich bin vom Schnelldienst."),
                cp("Wer? Wo? Ich?"),
                cp("Wo bin ich denn hier?"),
            ]
        );
        nothing_unbuilt(&seen);
    }

    /// The Denkfabrik in state 2, `[2BA6h]` not yet 2: the entry leaves the
    /// state (`0x14E93`) and `94h` stands. `Sprich mit` on it runs rule
    /// `0027h`: one of scenes 4 to 6 (`0x14E1B`).
    #[test]
    fn talking_in_the_denkfabrik_plays_one_of_three() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = on_the_ground_floor("bifi2-denkfabrik-2", &mut seen) else {
            return;
        };
        Api::set_byte(&mut g.inner, 0x2BBE, 2);
        let w = super::logic::WORDS;
        Api::set_word(&mut g.inner, w.entry, 0x70);
        Api::set_word(&mut g.inner, w.location, 17);
        run_until(&mut g, 17, "the Denkfabrik", &mut seen);
        assert_eq!(Api::byte(&g.inner, 0x2BBE), 2, "state 2 kept");
        click_verb(&mut g, 3);
        click_spot(&mut g, 0x94);
        tick_until(
            &mut g,
            &mut seen,
            5_000,
            "one of scenes 4 to 6",
            |_, seen| {
                seen.iter()
                    .any(|e| matches!(e, Event::SceneBound { scene: 4..=6, .. }))
            },
        );
        nothing_unbuilt(&seen);
    }

    /// The Requisite before the `Perücke` is given (`[2BBBh]` clear):
    /// object `92h` stands on animation 0 (`0x149C1`). `Sprich mit` on it
    /// the first time runs rule `0020h`: the input taken over, scene 0, and
    /// `[2BBDh]` set (`0x1487A`–`0x14887`).
    #[test]
    fn the_requisite_s_first_talk_takes_the_input() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-requisite", 0x6F, 16, &mut seen) else {
            return;
        };
        let i = g.inner.object_index(0x92).expect("92h");
        assert_eq!(g.inner.objects[i].flags & 1, 1, "92h shown");
        click_verb(&mut g, 3);
        click_spot(&mut g, 0x92);
        tick_until(&mut g, &mut seen, 5_000, "scene 0", |g, seen| {
            Api::byte(&g.inner, 0x2BBD) == 1
                && seen
                    .iter()
                    .any(|e| matches!(e, Event::SceneBound { scene: 0, .. }))
        });
        nothing_unbuilt(&seen);
    }

    /// The Tontechnik once the Enterhaken - Maschinenraum has set
    /// `[2BB4h]` (`0x1385F`): object `96h` stands and the state moves from
    /// 0 to 1 (`0x1531B`–`0x1533E`). `Sprich mit` on it runs rule `0053h`:
    /// scene 0 and the location's gate `1523h:0009h` installed
    /// (`0x1529C`–`0x152A8`); the scene opens a conversation. The gate
    /// asked with 0, as the conversation's end asks it, moves the state to
    /// 4 (scene 2) and clears itself (`0x15249`–`0x15279`).
    #[test]
    fn the_tontechnik_talks_through_its_gate() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = on_the_ground_floor("bifi2-tontechnik", &mut seen) else {
            return;
        };
        Api::set_byte(&mut g.inner, 0x2BB4, 1);
        let w = super::logic::WORDS;
        Api::set_word(&mut g.inner, w.entry, 0x6E);
        Api::set_word(&mut g.inner, w.location, 20);
        run_until(&mut g, 20, "the Tontechnik", &mut seen);
        assert_eq!(Api::byte(&g.inner, 0x2BC0), 1, "state 1");
        let i = g.inner.object_index(0x96).expect("96h");
        assert_eq!(g.inner.objects[i].flags & 1, 1, "96h shown");
        click_verb(&mut g, 3);
        click_spot(&mut g, 0x96);
        tick_until(&mut g, &mut seen, 5_000, "scene 0", |g, seen| {
            Api::word(&g.inner, 0x2D66) == 9
                && seen
                    .iter()
                    .any(|e| matches!(e, Event::SceneBound { scene: 0, .. }))
        });
        tick_until(&mut g, &mut seen, 5_000, "the conversation", |g, _| {
            g.inner.menu.up
        });
        let cp = |s: &str| motionvm_parsec_formats::to_cp437(s);
        let lines: Vec<Vec<u8>> = g.inner.menu.lines.iter().map(|l| l.text.clone()).collect();
        assert_eq!(
            lines,
            [
                cp("Ist dies das Tonstudio?"),
                cp("Sind das viele Geräte!"),
                cp("Sind Sie der Tonmeister?"),
            ]
        );
        nothing_unbuilt(&seen);
        assert_eq!(
            super::tontechnik::gate(&mut g.inner, 9, 0),
            Some(0),
            "the gate's answer"
        );
        assert_eq!(Api::byte(&g.inner, 0x2BC0), 4, "state 4");
        assert_eq!(Api::word(&g.inner, 0x2D66), 0, "the gate cleared");
    }

    /// The SFX without the `Sicherheitsausweis` (item 8): node `3E8h` of
    /// the player's graph blocked (`0x155D3`). `Sprich mit` object `97h`
    /// runs rule `0059h`: the first of scenes `0Ah` to `0Dh` in turn by the
    /// counter `[2BC2h]` (`0x155A0`).
    #[test]
    fn the_sfx_talks_in_turn() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-sfx", 0x70, 21, &mut seen) else {
            return;
        };
        assert_eq!(Api::byte(&g.inner, 0x2BC3), 0, "the node not opened");
        click_verb(&mut g, 3);
        click_spot(&mut g, 0x97);
        tick_until(&mut g, &mut seen, 5_000, "scene 0Ah", |g, seen| {
            Api::byte(&g.inner, 0x2BC2) == 1
                && seen
                    .iter()
                    .any(|e| matches!(e, Event::SceneBound { scene: 0x0A, .. }))
        });
        nothing_unbuilt(&seen);
    }

    /// The Maske before Schlönstadt's Büro has set `[2BC6h]`: every entry
    /// makes the state 4, object `95h` closed on frame 2
    /// (`0x159A2`–`0x159AC`). `Öffne` on it runs rule `0052h`: state 3,
    /// frame 1 and sample 3 (`0x15759`–`0x1574E`).
    #[test]
    fn the_maske_opens_what_it_keeps_closed() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-maske", 0x6F, 22, &mut seen) else {
            return;
        };
        assert_eq!(Api::byte(&g.inner, 0x2BBF), 4, "state 4");
        seen.clear();
        click_verb(&mut g, 6);
        click_spot(&mut g, 0x95);
        tick_until(&mut g, &mut seen, 5_000, "state 3", |g, seen| {
            Api::byte(&g.inner, 0x2BBF) == 3 && seen.iter().any(|e| matches!(e, Event::Sample(3)))
        });
        nothing_unbuilt(&seen);
    }

    /// The Enterhaken - Brücke while the Tontechnik is short of state 4:
    /// the four figures stay, nodes `3E8h` and `3E9h` are blocked, and
    /// scenes 0 to 4 roll on task 1, repeated (`0x134DD`–`0x134E3`). The
    /// task-1 scene reaches the location hook, which moves the figures
    /// (`0x13531`).
    #[test]
    fn the_bruecke_s_crew_moves_with_its_scene() {
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-bruecke", 0x6E, 10, &mut seen) else {
            return;
        };
        tick_until(&mut g, &mut seen, 5_000, "the crew's scene", |_, seen| {
            seen.iter()
                .any(|e| matches!(e, Event::LocationHook { task: 1, .. }))
        });
        nothing_unbuilt(&seen);
    }

    /// The Enterhaken - Maschinenraum on the first visit (`[2BB4h]` clear):
    /// nodes `3E8h` to `3EBh` blocked, the location's own interaction with
    /// node `0FA0h` and its scene 0 (`0x13857`), `[2BB4h]` set (`0x1385F`), which
    /// is what lets the Tontechnik's man stand (`0x1531B`), and object
    /// `F9h` shown on frame 0 with its state written 0 (`0x13875`).
    #[test]
    fn the_maschinenraum_s_first_visit_opens_its_scene() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(g) = entered("bifi2-maschinenraum", 0x6F, 11, &mut seen) else {
            return;
        };
        assert_eq!(Api::byte(&g.inner, 0x2BB4), 1, "the first visit marked");
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 0, .. })),
            "the interaction's scene 0"
        );
        let i = g.inner.object_index(0xF9).expect("F9h");
        assert_eq!(g.inner.objects[i].flags & 1, 1, "F9h shown");
        assert_eq!(g.inner.objects[i].state, 0, "F9h's state");
        nothing_unbuilt(&seen);
        // The scene over, the script's event 0Ah while the play runs:
        // F9h's state 1 (`0x13BCF`), and the animator steps its four cels
        // round, one each interval `+14h` (30 ticks in the file).
        let mut g = g;
        tick_until(&mut g, &mut seen, 20_000, "the scene over", |g, _| {
            g.inner.mode == 0 && g.inner.bits & 2 == 0 && g.inner.tasks.t[0].kind == 0
        });
        assert_eq!(g.inner.here_word, 11, "still in the Maschinenraum");
        Api::set_word(&mut g.inner, 0x18AA, 1);
        super::enterhaken_maschinenraum::hook(&mut g.inner, 7, 0x0A, 0);
        assert_eq!(g.inner.objects[i].state, 1, "F9h runs");
        let interval = i32::from(g.inner.objects[i].interval);
        let mut cels = std::collections::BTreeSet::new();
        for _ in 0..4 {
            g.inner.animator_tick(interval);
            cels.insert(g.inner.objects[i].cel);
        }
        assert_eq!(cels.len(), 4, "its four cels: {cels:?}");
    }

    /// The Dino Zoo on the first visit: scene 0 and state 1
    /// (`0x141E6`–`0x141EE`). The gate rule `00AEh` installs, asked with
    /// 0, makes the state 4 with scene 9 and the input taken over
    /// (`0x140D7`–`0x14115`), asking `080Ch:1A39h` whether task 1 speaks
    /// first. The script's event `11h` takes the player out of the picture
    /// (`0x14532`) and the animation code `67h` puts it back (`0x14315`).
    #[test]
    fn the_dino_zoo_opens_and_throws() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-dino-zoo", 0x6F, 14, &mut seen) else {
            return;
        };
        assert_eq!(Api::byte(&g.inner, 0x2BB5), 1, "state 1");
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 0, .. })),
            "scene 0"
        );
        nothing_unbuilt(&seen);
        tick_until(&mut g, &mut seen, 20_000, "the scene over", |g, _| {
            g.inner.mode == 0 && g.inner.bits & 2 == 0 && g.inner.tasks.t[0].kind == 0
        });
        assert_eq!(super::dino_zoo::gate(&mut g.inner, 0x67, 0), Some(0));
        assert_eq!(Api::byte(&g.inner, 0x2BB5), 4, "state 4");
        assert_ne!(g.inner.bits & 2, 0, "the input taken over");
        let walker = super::build::WALKER + 2;
        super::dino_zoo::hook(&mut g.inner, 7, 0x11, 0);
        assert_eq!(Api::byte(&g.inner, walker) & 1, 0, "the player out");
        let rec = Api::ds(&g.inner, super::build::WALKER);
        super::dino_zoo::anim_hook(&mut g.inner, rec, 0x67);
        assert_eq!(Api::byte(&g.inner, walker) & 1, 1, "the player back");
    }

    /// Rambowsky on the first visit (course 0): the input taken over,
    /// phase 1, the script's interaction with node `0FA1h` and the first of
    /// its two scenes by the counter `[2BA3h]` (`0x11D75`), object `F3h`
    /// shown, and the Denkfabrik's state moved from 0 to 1 (`0x11DC0`).
    #[test]
    fn rambowsky_opens_and_sends_you_to_the_denkfabrik() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(g) = entered("bifi2-rambowsky", 0x6E, 7, &mut seen) else {
            return;
        };
        assert_eq!(Api::byte(&g.inner, 0x2BBE), 1, "the Denkfabrik's state");
        assert_eq!(Api::byte(&g.inner, 0x2BA3), 1, "the sequence counted");
        let i = g.inner.object_index(0xF3).expect("F3h");
        assert_eq!(g.inner.objects[i].flags & 1, 1, "F3h shown");
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 0, .. })),
            "scene 0"
        );
        nothing_unbuilt(&seen);
    }

    /// Rambowsky in course 3 with the `Theaterkostüm` `18h`: the player is
    /// sent to node `0FA0h` (phase 3, `0x11E31`); standing there, the
    /// walking record `F2h` takes the costume (phase 6, `0x11F70`), is
    /// bound to graph 1 on node `0BB8h` and walks to `0BB9h`; there the
    /// next of scenes 4 to 6 plays (phase 7, `0x11FF1`).
    #[test]
    fn rambowsky_s_walker_takes_the_costume() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = on_the_ground_floor("bifi2-rambowsky-3", &mut seen) else {
            return;
        };
        Api::set_byte(&mut g.inner, 0x2BA6, 3);
        Api::inv_give(&mut g.inner, 0x18);
        let w = super::logic::WORDS;
        Api::set_word(&mut g.inner, w.entry, 0x6E);
        Api::set_word(&mut g.inner, w.location, 7);
        tick_until(&mut g, &mut seen, 20_000, "phase 7", |g, _| {
            Api::word(&g.inner, 0x2DD2) == 7
        });
        assert_eq!(Api::inv_has(&mut g.inner, 0x18), 0, "the costume taken");
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 4..=6, .. })),
            "one of scenes 4 to 6"
        );
        nothing_unbuilt(&seen);
    }

    /// The Bi-Fi room once the VIP-Lounge has set `[2BA8h]` to 1: the
    /// input taken over, object `8Dh` shown, the location's own
    /// interaction with node `0FA0h` and scene 4 (`0x13D26`), and
    /// `[2BA8h]` made 2 (`0x13D2E`).
    #[test]
    fn the_bi_fi_room_opens_its_scene() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = on_the_ground_floor("bifi2-bi-fi", &mut seen) else {
            return;
        };
        Api::set_byte(&mut g.inner, 0x2BA8, 1);
        let w = super::logic::WORDS;
        Api::set_word(&mut g.inner, w.entry, 0x6E);
        Api::set_word(&mut g.inner, w.location, 12);
        run_until(&mut g, 12, "the Bi-Fi room", &mut seen);
        assert_eq!(Api::byte(&g.inner, 0x2BA8), 2, "[2BA8h] made 2");
        let i = g.inner.object_index(0x8D).expect("8Dh");
        assert_eq!(g.inner.objects[i].flags & 1, 1, "8Dh shown");
        tick_until(&mut g, &mut seen, 5_000, "scene 4", |_, seen| {
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 4, .. }))
        });
        nothing_unbuilt(&seen);
    }

    /// The Kantine in course 0: `Sprich mit` object `98h` runs rule
    /// `0006h`, scene 0 and course 1 (`0x15A6E`).
    #[test]
    fn the_kantine_s_first_talk_moves_rambowsky_s_course() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-kantine", 0x6F, 23, &mut seen) else {
            return;
        };
        seen.clear();
        click_verb(&mut g, 3);
        click_spot(&mut g, 0x98);
        tick_until(&mut g, &mut seen, 8_000, "scene 0", |g, seen| {
            Api::byte(&g.inner, 0x2BA6) == 1
                && seen
                    .iter()
                    .any(|e| matches!(e, Event::SceneBound { scene: 0, .. }))
        });
        nothing_unbuilt(&seen);
    }

    /// The Kantine below course 3 once Schlönstadt's Büro has set
    /// `[2BC7h]` to 1: course 3, scene 17h (`0x15B81`), and the walking
    /// record `98h` sent out through the door, which leads the visit on to
    /// the Büro with entry `9Ah` (`0x15C6E`).
    #[test]
    fn the_kantine_s_man_walks_out_to_the_buero() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = on_the_ground_floor("bifi2-kantine-2", &mut seen) else {
            return;
        };
        Api::set_byte(&mut g.inner, 0x2BA6, 2);
        Api::set_byte(&mut g.inner, 0x2BC7, 1);
        let w = super::logic::WORDS;
        Api::set_word(&mut g.inner, w.entry, 0x6F);
        Api::set_word(&mut g.inner, w.location, 23);
        tick_until(&mut g, &mut seen, 30_000, "the Kantine", |g, _| {
            g.inner.here_word == 23
        });
        assert_eq!(Api::byte(&g.inner, 0x2BA6), 3, "course 3");
        assert_eq!(Api::byte(&g.inner, 0x2BC7), 2, "[2BC7h] made 2");
        tick_until(&mut g, &mut seen, 40_000, "the Büro", |g, _| {
            g.inner.here_word == 24
        });
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 0x17, .. })),
            "scene 17h"
        );
        nothing_unbuilt(&seen);
    }

    /// Jones 1 before the story's stage 3 (`[2BA8h]` below 3): the input
    /// taken over, figure `F1h` alone shown, and the first of scenes 0 to 2
    /// in turn by the counter `[2BA9h]` (`0x125C6`).
    #[test]
    fn jones_1_opens_with_one_figure() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(g) = entered("bifi2-jones1", 0x6F, 8, &mut seen) else {
            return;
        };
        assert_eq!(Api::byte(&g.inner, 0x2BA9), 1, "the sequence counted");
        let shown = |id| {
            g.inner
                .object_index(id)
                .map(|i| g.inner.objects[i].flags & 1)
        };
        assert_eq!(shown(0xF1), Some(1), "F1h shown");
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 0, .. })),
            "scene 0"
        );
        nothing_unbuilt(&seen);
    }

    /// Jones 2 dark, on the first visit: scene 0 (`0x12EA5`), the room's
    /// first 20h colors through ramp 1 of the shade into the DAC
    /// (`02AFh:0217h`, `0x12EDB`; `0104h:090Eh`, `0x12F39`) and their
    /// darker copy through ramp 0 at 80h on (`0x0199A`). The Leuchtfarbe's
    /// event `0Ah` (`0x1335C`) starts the painting: the colors step towards
    /// the lit ones, the room's with every green doubled (`0x12DC2`).
    #[test]
    fn jones_2_is_dark_until_it_is_painted() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-jones2", 0x6E, 9, &mut seen) else {
            return;
        };
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 0, .. })),
            "scene 0"
        );
        assert_eq!(Api::byte(&g.inner, 0x2BAE), 1, "the first visit counted");
        let rgb = g.inner.room().expect("a room").room.palette_rgb.clone();
        // The DAC writes wait behind the entry's effect.
        tick_until(&mut g, &mut seen, 5_000, "the effect over", |g, _| {
            !g.inner.effects.busy()
        });
        let ramps = *g.inner.flatten.ramps().expect("the shade's ramps");
        let dark: Vec<u8> = rgb[..0x60]
            .iter()
            .map(|&v| ramps[0x40 + usize::from(v)])
            .collect();
        assert_eq!(&g.inner.dac_for_test().raw[..0x60], &dark[..], "ramp 1");
        let darker: Vec<u8> = dark.iter().map(|&v| ramps[usize::from(v)]).collect();
        assert_eq!(
            &g.inner.dac_for_test().raw[0x180..0x1E0],
            &darker[..],
            "ramp 0 at 80h on"
        );
        nothing_unbuilt(&seen);
        // The painting begun: within a while the first colors are lighter.
        seen.clear();
        super::jones2::hook(&mut g.inner, 7, 0x0A, 0);
        assert_eq!(
            Api::inv_has(&mut g.inner, 0x14),
            1,
            "the Eimer for the Leuchtfarbe"
        );
        for _ in 0..2_000 {
            g.tick();
            seen.append(&mut g.inner.events);
        }
        let now = &g.inner.dac_for_test().raw[..0x60];
        assert!(
            now.iter().zip(&dark).any(|(n, d)| n > d),
            "some color lighter than the dark one"
        );
        nothing_unbuilt(&seen);
    }

    /// The Öliens-Halle entered by another way than `72h`: object `72h`
    /// shown on frame 1 (`0x16E59`). `Betrachte` on it before an entry by
    /// `72h` plays scene 0 (rule `0059h`, `0x16DD3`).
    #[test]
    fn the_oeliens_halle_shows_its_door() {
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-oeliens-halle", 0x7B, 28, &mut seen) else {
            return;
        };
        let i = g.inner.object_index(0x72).expect("72h");
        assert_eq!(g.inner.objects[i].flags & 1, 1, "72h shown");
        seen.clear();
        click_verb(&mut g, 1);
        click_spot(&mut g, 0x72);
        tick_until(&mut g, &mut seen, 5_000, "scene 0", |_, seen| {
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 0, .. }))
        });
        nothing_unbuilt(&seen);
    }

    /// 08-15: figures `F2h`, `F1h` and `F0h` shown and scenes 0 and 1 rolled
    /// on task 1, repeated (`0x16F56`), whose pages reach the hook.
    #[test]
    fn the_08_15_talks_in_the_background() {
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-08-15", 0x6F, 29, &mut seen) else {
            return;
        };
        for id in [0xF0u8, 0xF1, 0xF2] {
            let i = g.inner.object_index(id).expect("a figure");
            assert_eq!(g.inner.objects[i].flags & 1, 1, "{id:#x} shown");
        }
        tick_until(&mut g, &mut seen, 5_000, "task 1's scene", |_, seen| {
            seen.iter()
                .any(|e| matches!(e, Event::LocationHook { task: 1, .. }))
        });
        nothing_unbuilt(&seen);
    }

    /// Öliens on the first visit (state 0): `[2BB8h]` set, state 1, object
    /// `A1h` shown and the script's interaction with it and scene 0
    /// (`0x16B37`–`0x16B6A`).
    #[test]
    fn oeliens_opens_with_its_man() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(g) = entered("bifi2-oeliens", 0x6E, 27, &mut seen) else {
            return;
        };
        assert_eq!(Api::byte(&g.inner, 0x2BCC), 1, "state 1");
        assert_eq!(Api::byte(&g.inner, 0x2BB8), 1, "[2BB8h] set");
        let i = g.inner.object_index(0xA1).expect("A1h");
        assert_eq!(g.inner.objects[i].flags & 1, 1, "A1h shown");
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 0, .. })),
            "scene 0"
        );
        nothing_unbuilt(&seen);
    }

    /// The interludes from the Gefängnis to the Öliens-Halle, each a row of
    /// kind 7 that locks the input and goes on by its animation hook: 6
    /// walks the player to node 7Fh (`0x11897`) and its code 6Ch goes on
    /// to 39 (`0x11ACB`); 39's code 64h raises the effects' volume and 65h
    /// goes on to 13 (`0x17409`); 13's code 65h goes on to the Öliens-Halle
    /// by entry 72h (`0x1405F`), which marks it (`[2BD2h]`, `0x16E39`).
    #[test]
    fn the_interludes_lead_to_the_oeliens_halle() {
        use crate::titles::parsec1994::{Api, Far};
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-interludes", 0x6E, 6, &mut seen) else {
            return;
        };
        assert_ne!(g.inner.bits & 2, 0, "the input locked");
        super::interlude_6::anim_hook(&mut g.inner, Far::NULL, 0x6C);
        tick_until(&mut g, &mut seen, 5_000, "location 39", |g, _| {
            g.inner.here_word == 39
        });
        super::interlude_39::anim_hook(&mut g.inner, Far::NULL, 0x64);
        assert_eq!(Api::word(&g.inner, 0x2F92), 1, "loud");
        super::interlude_39::anim_hook(&mut g.inner, Far::NULL, 0x67);
        super::interlude_39::anim_hook(&mut g.inner, Far::NULL, 0x65);
        tick_until(&mut g, &mut seen, 5_000, "location 13", |g, _| {
            g.inner.here_word == 13
        });
        super::interlude_13::anim_hook(&mut g.inner, Far::NULL, 0x64);
        super::interlude_13::anim_hook(&mut g.inner, Far::NULL, 0x65);
        tick_until(&mut g, &mut seen, 5_000, "the Öliens-Halle", |g, _| {
            g.inner.here_word == 28
        });
        assert_eq!(Api::byte(&g.inner, 0x2BD2), 1, "entered by 72h");
        nothing_unbuilt(&seen);
    }

    /// Location 2, between the Vorführraum and location 3: object `F0h` on
    /// animation 0 unless the visit came from the Vorführraum (`0x109B6`),
    /// and the Requisite's `[2BCEh]` raised to 2 (`0x109E3`).
    #[test]
    fn location_2_raises_the_requisite_s_state() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(g) = entered("bifi2-interlude-2", 0x7F, 2, &mut seen) else {
            return;
        };
        assert_eq!(Api::byte(&g.inner, 0x2BCE), 2, "[2BCEh] raised");
        let i = g.inner.object_index(0xF0).expect("F0h");
        assert_eq!(g.inner.objects[i].flags & 1, 1, "F0h shown");
        nothing_unbuilt(&seen);
    }

    /// The labyrinth laid out on its first entry (`0x176A0`–`0x17682`):
    /// every row's byte at `2446h:0B48h + n` names the first of its exits
    /// that leads one step closer to Schlönstadt's Gefängnis, so following
    /// them from row 40 ends there.
    #[test]
    fn the_labyrinth_s_ways_lead_to_the_gefaengnis() {
        use crate::titles::parsec1994::{Api, Far};
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-labyrinth", 0x70, 40, &mut seen) else {
            return;
        };
        let a = &mut g.inner;
        let mut byte = |seg: u16, off: u16| Api::far_byte(a, Far { off, seg });
        let mut n: u16 = 40;
        let mut steps = 0;
        while n != 41 {
            let way = u16::from(byte(0x2446, 0x0B48 + n));
            assert!(way < 4, "row {n} has a way");
            n = u16::from(byte(0x2181, n * 0x44 + 0x19 + way));
            steps += 1;
            assert!(steps < 60, "the ways end");
        }
        nothing_unbuilt(&seen);
    }

    /// Location 46 before the Gefängnis: `Benutze` on the Schloß with the
    /// Labyrinth-Karte opens it (`[2BD6h]`, animation 1) with scene 0Bh
    /// (rule `004Ah`, `0x17AE9`–`0x17B14`).
    #[test]
    fn location_46_opens_its_lock_with_the_map() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-location-46", 0x6E, 46, &mut seen) else {
            return;
        };
        Api::inv_give(&mut g.inner, 0x1B);
        seen.clear();
        click_verb(&mut g, 4);
        click_spot(&mut g, 0xF9);
        tick_until(&mut g, &mut seen, 5_000, "scene 0Bh", |g, seen| {
            Api::byte(&g.inner, 0x2BD6) == 1
                && seen
                    .iter()
                    .any(|e| matches!(e, Event::SceneBound { scene: 0x0B, .. }))
        });
        let i = g.inner.object_index(0x73).expect("73h");
        assert_eq!(g.inner.objects[i].kind, 4, "the Eisengitter a door");
        nothing_unbuilt(&seen);
    }

    /// Schlönstadt's Gefängnis on the first visit: `[2BD0h]` set and the
    /// script's interaction with node `0FA0h` and scene 2 (`0x177F2`); its
    /// grille `72h` a door for the visit (`0x17832`).
    #[test]
    fn the_gefaengnis_opens_its_scene() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(g) = entered("bifi2-gefaengnis", 0x73, 41, &mut seen) else {
            return;
        };
        assert_eq!(Api::byte(&g.inner, 0x2BD0), 1, "the first visit marked");
        let i = g.inner.object_index(0x72).expect("72h");
        assert_eq!(g.inner.objects[i].kind, 4, "the grille a door");
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 2, .. })),
            "scene 2"
        );
        nothing_unbuilt(&seen);
    }

    /// The Skelett (row 79) the first time: the mode block set up as
    /// `Betrachte` on object `40h` with the two-object word 3 and the player
    /// walked there (`0x176B9`–`0x176F1`); arrived, rule `006Eh` plays scene
    /// 0, gives the input back and marks it (`[2BCFh]`, `0x174E5`).
    #[test]
    fn the_skelett_is_looked_at_on_the_way_in() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-skelett", 0x6F, 79, &mut seen) else {
            return;
        };
        tick_until(&mut g, &mut seen, 10_000, "rule 006Eh", |g, seen| {
            Api::byte(&g.inner, 0x2BCF) == 1
                && seen
                    .iter()
                    .any(|e| matches!(e, Event::SceneBound { scene: 0, .. }))
        });
        nothing_unbuilt(&seen);
    }

    /// The record of the Vorführraum's roll, `2446h:0960h` (`0x1687C`).
    const ROLL_RECORD: crate::titles::parsec1994::Far = crate::titles::parsec1994::Far {
        seg: 0x2446,
        off: 0x0960,
    };

    /// The Vorführraum's row alone names a record hook, `+47h`, which the
    /// entry copies into `[2D1Ch]` (`0x09E95`–`0x09EA3`, the word's only
    /// store): `167Bh:015Bh`, the one [`super::logic`]'s table hands on.
    /// No room file holds a record the animator or the drawer would hand
    /// a hook — a state above `0Ah` or bit 4 of `+2`: the roll's record
    /// is the logic's own.
    #[test]
    fn the_vorfuehrraum_alone_names_a_record_hook() {
        use crate::titles::parsec1994::image_records::tests::{hooked_records, row_far_pointers};
        let Some(g) = fresh_game("bifi2-record-hook") else {
            return;
        };
        assert_eq!(row_far_pointers(&g.inner, 0x47), [(26, (0x167B, 0x015B))]);
        for loc in 0..u16::try_from(g.inner.locations.count).unwrap() {
            let hook = (super::logic::TABLE.record_hook)(loc);
            assert_eq!(hook.is_some(), loc == 26, "location {loc}");
        }
        assert_eq!(hooked_records(&g.inner, g.inner.profile.anim_arms), []);
    }

    /// The Vorführraum entered from the VIP-Lounge by its door `79h`: the
    /// entry lays the roll out, puts the record's cels in bundle 4's slot
    /// and links the record at the head of layer 0's chain (`0x16868`–
    /// `0x16882`), live and shown (`0x1689B`).
    #[test]
    fn the_vorfuehrraum_links_its_roll() {
        use crate::titles::parsec1994::{Api, Body};
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-vorfuehrraum", 0x79, 26, &mut seen) else {
            return;
        };
        assert_eq!(g.inner.chains[0].first(), Some(&Body::Data(ROLL_RECORD)));
        assert!(g.inner.room_roll.is_some(), "the roll laid out");
        // Where it ends, as the game's end lays the same lines out, read
        // off the program's own `push 0BCEh` (`0x029A5`).
        let lines = super::ending::lines(&g.inner.image).expect("the lines");
        let roll = crate::titles::parsec1994::title::roll::Roll::layout(
            &lines,
            g.inner.font_for_test(),
            super::ending::ROLL,
        );
        assert_eq!(
            Api::word(&g.inner, 0x2F6E),
            crate::num::low16(roll.end()),
            "[2F6Eh], where it ends"
        );
        assert_eq!(
            [0x2D34, 0x2D36].map(|a| Api::word(&g.inner, a)),
            [0x099A, 0x2446]
        );
        let flags = Api::far_byte(
            &mut g.inner,
            crate::titles::parsec1994::Far {
                off: ROLL_RECORD.off + 2,
                ..ROLL_RECORD
            },
        );
        assert_eq!(flags & 0x13, 0x11, "live, shown, the hook's to draw");
        nothing_unbuilt(&seen);
    }

    /// The roll's record in state `FFh`, past the animator's arms: every
    /// four ticks (`+14h`) the record hook is called with code 0
    /// (`0x041E9`), which moves `[2F70h]` on by one or counts a pause
    /// down; every frame the drawer calls it with code 1 (`0x0725B`), and
    /// the roll is drawn at the record's place from the position the
    /// cursor was taken to.
    #[test]
    fn the_vorfuehrraum_s_roll_moves_on() {
        use crate::titles::parsec1994::{Api, Body};
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-vorfuehrraum-roll", 0x79, 26, &mut seen) else {
            return;
        };
        let pos = |g: &Program| Api::word(&g.inner, 0x2F70);
        let pause = |g: &Program| Api::word(&g.inner, 0x2F6C);
        let start = pos(&g);
        assert!(start > 0, "the roll moved on during the entry's ticks");
        // Code 0: one step every four ticks while no pause is counted.
        let mut steps = Vec::new();
        for n in 0..400 {
            let (p, w) = (pos(&g), pause(&g));
            g.tick();
            seen.append(&mut g.inner.events);
            if pos(&g) != p {
                assert_eq!(pos(&g), p.wrapping_add(1), "a step of one");
                assert_eq!(w, 0, "no step while a pause is counted");
                steps.push(n);
            }
        }
        assert!(steps.len() > 10, "{steps:?}");
        let gaps: Vec<i32> = steps.windows(2).map(|w| w[1] - w[0]).collect();
        assert_eq!(gaps.iter().min(), Some(&4), "{steps:?}");
        assert!(gaps.iter().all(|&d| d % 4 == 0), "{steps:?}");
        assert!(pos(&g) > start);
        // Code 1: drawn at the record, from the kept position.
        let roll = g.inner.room_roll.as_ref().expect("the roll");
        assert_eq!(roll.shown.len(), 1);
        assert_eq!(roll.shown[0].0, Body::Data(ROLL_RECORD));
        assert_eq!(roll.shown[0].2, i32::from(pos(&g)));
        nothing_unbuilt(&seen);
    }

    /// Nothing on the way asked for code this runtime does not have.
    fn nothing_unbuilt(seen: &[Event]) {
        for e in seen {
            assert!(
                !matches!(
                    e,
                    Event::Unbuilt(..)
                        | Event::UnreadRule { .. }
                        | Event::SceneRefused(_)
                        | Event::FarWrite { .. }
                ),
                "{e:?}"
            );
        }
    }

    /// The ground floor's corridor, entered from anywhere but the Lift, is
    /// on floor 0, the lift's floor and goal both (`0x150C3`, `0x150C6`).
    /// Its lift door, `64h`, clicked: the global door gate opens it and the
    /// Lift is entered by it, floor 0 the one it came from (`0x171F9`). In
    /// the Lift the options are held back without a word (`0x09FE2`). Its
    /// button for the first floor, `65h`, clicked: rule `0087h` makes the
    /// verb `0Dh`, rule `0007h` leaves by the exit to location 19, and the
    /// goal is floor 1 (`0x172AB`). Entered from the Lift, the corridor
    /// runs the ride (`[2F0Eh]` from 3): `A4h` steps to floor 1 with sample
    /// `16h`, the door opens, the figure walks to it, and the corridor's
    /// gate closes the door and gives the input back.
    #[test]
    fn the_lift_rides_from_the_ground_floor_to_the_first() {
        use crate::titles::parsec1994::{Api, KEY_ESCAPE};
        let mut seen = Vec::new();
        let Some(mut g) = on_the_ground_floor("bifi2-lift", &mut seen) else {
            return;
        };
        let (state, goal, floor) = (0x2F0E, 0x2F8C, 0x2F8E);
        let words = |g: &Program| [state, goal, floor].map(|a| Api::word(&g.inner, a));
        assert_eq!(words(&g), [0, 0, 0]);
        click_spot(&mut g, 0x64);
        run_until(&mut g, 36, "the Lift", &mut seen);
        assert_eq!(Api::word(&g.inner, floor), 0);
        assert_ne!(g.inner.bits & 8, 0, "the Lift's hold on the options");
        g.key(KEY_ESCAPE);
        for _ in 0..50 {
            g.tick();
            seen.append(&mut g.inner.events);
        }
        assert!(!g.inner.options_up(), "no options in the Lift");
        click_spot(&mut g, 0x65);
        let mut n = 0;
        while g.inner.here_word != 19 {
            g.tick();
            seen.append(&mut g.inner.events);
            n += 1;
            assert!(n < 5_000, "the first floor");
        }
        assert_eq!(Api::word(&g.inner, goal), 1, "the goal floor");
        let mut states = vec![Api::word(&g.inner, state)];
        let from = seen.len();
        for _ in 0..5_000 {
            g.tick();
            seen.append(&mut g.inner.events);
            let s = Api::word(&g.inner, state);
            if states.last() != Some(&s) {
                states.push(s);
            }
            if s == 0 {
                break;
            }
        }
        assert_eq!(states, [3, 2, 1, 0], "the ride's states");
        assert_eq!(Api::word(&g.inner, floor), 1, "arrived");
        assert!(
            seen[from..]
                .iter()
                .any(|e| matches!(e, Event::Sample(0x16))),
            "a step's sample"
        );
        let node = g
            .inner
            .walker
            .as_ref()
            .and_then(|w| w.node)
            .and_then(|k| g.inner.room().and_then(|r| r.walk()).map(|w| w.nodes[k].id));
        assert_eq!(node, Some(0x65), "at the lift door");
        nothing_unbuilt(&seen);
    }

    /// The Vorzimmer's door, `6Fh`, clicked on the ground floor: the room
    /// opens with the first of its two scenes as the script's interaction
    /// with object `85h` (`0x11516`, `09B6h:164Dh`), its counter `[2BA0h]`
    /// one on, in this build's verb 3, `Sprich mit`; the scene plays into
    /// the conversation's first choice.
    #[test]
    fn the_vorzimmer_opens_a_conversation() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = on_the_ground_floor("bifi2-vorzimmer", &mut seen) else {
            return;
        };
        click_spot(&mut g, 0x6F);
        run_until(&mut g, 5, "the Vorzimmer", &mut seen);
        assert_eq!(
            Api::byte(&g.inner, 0x2BA0),
            1,
            "the entry's sequence counted"
        );
        assert_eq!(g.inner.verb, 3, "the interaction's verb");
        let mut n = 0;
        while !g.inner.menu.up {
            if n % 150 == 149 {
                g.click(true);
            }
            g.tick();
            seen.append(&mut g.inner.events);
            n += 1;
            assert!(n < 20_000, "the conversation's choice");
        }
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 8, .. })),
            "the first scene of the sequence"
        );
        let cp = |s: &str| motionvm_parsec_formats::to_cp437(s);
        let lines: Vec<Vec<u8>> = g.inner.menu.lines.iter().map(|l| l.text.clone()).collect();
        assert_eq!(
            lines,
            [
                cp("Haben Sie sich weh getan?"),
                cp("Warum ächzen Sie dann so?"),
                cp("Glaub' ich aber doch!"),
            ]
        );
        nothing_unbuilt(&seen);
    }

    /// Schlönstadt's Büro entered by its door `6Eh`, as the Vorzimmer's
    /// exit leaves it, before the story has brought him there (`[2BA6h]`
    /// not 2): object `F0h` hidden (`0x15FE7`), no scene, the input free.
    #[test]
    fn the_buero_is_empty_at_first() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = on_the_ground_floor("bifi2-buero", &mut seen) else {
            return;
        };
        let w = super::logic::WORDS;
        Api::set_word(&mut g.inner, w.entry, 0x6E);
        Api::set_word(&mut g.inner, w.location, 24);
        run_until(&mut g, 24, "the Büro", &mut seen);
        let f0 = g
            .inner
            .object_index(0xF0)
            .map(|i| g.inner.objects[i].flags & 1);
        assert_eq!(f0, Some(0), "F0h hidden");
        assert_eq!(g.inner.bits & 2, 0, "the input free");
        nothing_unbuilt(&seen);
    }

    /// Into the VIP-Lounge by door `78h`, as the ground floor's door rule
    /// leaves it: the first visit opens with the script's interaction with
    /// door `79h` and scene 6 (`0x163C9`), in this build's verb 3
    /// (`0x0B2F8`), and `[2BD7h]` and `[2BA8h]` are set.
    #[test]
    fn the_vip_lounge_opens_with_its_door() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = on_the_ground_floor("bifi2-vip-lounge", &mut seen) else {
            return;
        };
        let w = super::logic::WORDS;
        Api::set_word(&mut g.inner, w.entry, 0x78);
        Api::set_word(&mut g.inner, w.location, 25);
        let mut n = 0;
        while g.inner.here_word != 25 {
            g.tick();
            seen.append(&mut g.inner.events);
            n += 1;
            assert!(n < 5_000, "the VIP-Lounge");
        }
        assert_eq!(g.inner.verb, 3, "the interaction's verb");
        assert_eq!([0x2BD7, 0x2BA8].map(|a| Api::byte(&g.inner, a)), [1, 1]);
        for _ in 0..50 {
            g.tick();
            seen.append(&mut g.inner.events);
        }
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::SceneBound { scene: 6, .. })),
            "scene 6"
        );
        nothing_unbuilt(&seen);
    }

    /// Rule `0003h` of the ground floor, verb 6 on the VIP-Lounge's door
    /// without the VIP-Karte, plays scene 13h (`0x14FBA`) out of the row's
    /// own slot's bank, which the loader takes after the shared slot's
    /// (`0x03A88`).
    #[test]
    fn the_ground_floor_s_door_speaks_from_its_own_bank() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = on_the_ground_floor("bifi2-vip", &mut seen) else {
            return;
        };
        assert_eq!(Api::inv_has(&mut g.inner, 3), 0, "no VIP-Karte");
        g.inner.events.clear();
        assert_eq!(super::flur::rule(&mut g.inner, 0x0003, 0), Some(0x0A));
        let bound = g.inner.events.iter().find_map(|e| match e {
            Event::SceneBound { scene, place } => Some((*scene, place.map(|p| p.bank))),
            _ => None,
        });
        assert_eq!(
            bound,
            Some((0x13, Some(crate::titles::parsec1994::BankId::Own)))
        );
    }

    /// The rows of the two kinds that share a slot, 2 (the corridors and
    /// the lift) and 4 (the maze), carry an object map, and no other row
    /// does: the loader keeps the shared picture between two of them and
    /// binds the next row's map onto it, which it can only do with one.
    #[test]
    fn the_shared_rows_carry_object_maps() {
        let Some(g) = fresh_game("bifi2-maps") else {
            return;
        };
        for n in 0..0x5C {
            let kind = g.inner.locations.kind(&g.inner.image, n);
            let shared = matches!(kind, Some(2 | 4));
            assert_eq!(g.inner.has_object_map(n), shared, "row {n}, kind {kind:?}");
        }
    }

    /// A fresh game: the options at the start, closed with Escape, then
    /// the Kaffee's opening scene, scene 0, clicked through. The four faces
    /// take each speaker's frame (the hook's code 9), and when the scene is
    /// over the gate lets go and the story goes on to the Pforte, location
    /// 4, the row's entry for entry 0.
    #[test]
    fn the_opening_scene_leads_to_the_pforte() {
        use crate::titles::parsec1994::Event;
        let Some(dir) = motionvm_parsec_testutil::gamedata_bifi2() else {
            eprintln!("skipping: no Bi-Fi 2 gamedata directory");
            return;
        };
        let img = crate::exe::Image::open(&dir, "XP01.EXE", 164_576).expect("XP01.EXE");
        let locations = crate::titles::xp1994::Locations::read(&img).expect("the table");
        let text = super::xp01data::ExeText::read(&img, &locations).expect("the program's text");
        let mut g = Program::open(&dir, &super::build::BUILD, |l| {
            Ok(super::build::reading(&text, l))
        })
        .expect("opens");
        g.set_saves(&motionvm_parsec_testutil::saves_dir("bifi2-opening"))
            .expect("saves");
        g.boot(crate::titles::Start::New);
        for _ in 0..400 {
            g.tick();
        }
        assert!(g.inner.options_up(), "the start's options");
        g.key(crate::titles::parsec1994::KEY_ESCAPE);
        let (mut scenes, mut frames) = (Vec::new(), Vec::new());
        let mut onward = None;
        for n in 0..20_000 {
            if n % 300 == 299 {
                g.click(true);
            }
            g.tick();
            // The tick that leaves is the Pforte's entry too.
            if g.inner.location_word != 1 {
                onward = Some(g.inner.location_word);
                break;
            }
            for e in g.inner.events.drain(..) {
                match e {
                    Event::SceneBound { scene, .. } => scenes.push(scene),
                    Event::LocationHook { code: 9, arg, .. } => frames.push(arg),
                    _ => {}
                }
            }
        }
        assert_eq!(scenes, [0], "scene 0, once");
        assert!(frames.iter().all(|f| (0..4).contains(f)), "{frames:?}");
        assert!(frames.len() > 4, "the speakers change");
        assert_eq!(onward, Some(4), "on to the Pforte");
    }
    /// The Kaffee entered by entry 6 plays its scene, and its row names
    /// `FFFFh` (`DS:182Ch`): the faces back, the music faded, the frame
    /// pass alone for `C8h` hundredths (`0x10923`–`0x1092E`), and then the
    /// main loop is over and the end's sequence `0D84h:0008h` plays until
    /// an event lets it fade out; the program ends with it.
    /// What the end's sequence sends the drivers, the music switch left
    /// on or off: the requests from the sequence's start to the program's
    /// end.
    fn the_end_s_requests(
        save: &str,
        switch: u16,
    ) -> Option<Vec<crate::titles::parsec1994::Request>> {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let mut g = entered(save, 6, 1, &mut seen)?;
        let at = g.inner.music.map.switch;
        Api::set_word(&mut g.inner, at, switch);
        tick_until(&mut g, &mut seen, 31_000, "the end's sequence", |g, _| {
            g.ending()
        });
        g.inner.music.sent.clear();
        let mut n = 0;
        while !g.over {
            if n % 300 == 299 {
                g.click(true);
            }
            g.tick();
            n += 1;
            assert!(n < 20_000, "the program ends");
        }
        Some(g.inner.music.sent.clone())
    }

    /// The settings are written after the end's sequence: the Kaffee's
    /// handler shows it inside the main loop (`0x10942`), and `_main`
    /// writes them once the loop is over (`0x007AD`).
    #[test]
    fn the_settings_are_written_after_the_end_s_sequence() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-end-settings", 6, 1, &mut seen) else {
            return;
        };
        let map = super::build::BUILD.save;
        let file = g.save_path(map.cfg_name).expect("a saves directory");
        let _ = std::fs::remove_file(&file);
        Api::set_word(&mut g.inner, map.cfg_dirty, 1);
        tick_until(&mut g, &mut seen, 31_000, "the end's sequence", |g, _| {
            g.ending()
        });
        let mut n = 0;
        while !g.over {
            assert!(!file.exists(), "written while the sequence plays");
            if n % 300 == 299 {
                g.click(true);
            }
            g.tick();
            n += 1;
            assert!(n < 20_000, "the program ends");
        }
        assert!(file.exists(), "written once it is over");
    }

    /// The end's sequence calls the library's routines (`0104h`), which
    /// play, fade and sound an effect only with the switch `[2B2h]` on
    /// (`0x016D6`, `0x0179E`, `0x0180E`, `0x01837`) and stop whatever it
    /// says (`0x017E9`): with the switch off it plays no song and no
    /// effect, and still stops the music.
    #[test]
    fn the_end_s_sequence_keeps_to_the_music_switch() {
        use crate::titles::parsec1994::Request;
        let Some(on) = the_end_s_requests("bifi2-end-on", 1) else {
            return;
        };
        let off = the_end_s_requests("bifi2-end-off", 0).expect("the same game");
        let sounds = |r: &Request| {
            matches!(
                r,
                Request::Play { .. }
                    | Request::PlayFade { .. }
                    | Request::FadeOut(_)
                    | Request::Effect { .. }
                    | Request::StopEffects
            )
        };
        assert!(
            on.iter()
                .any(|r| matches!(r, Request::Play { piece: 0x0B, .. }))
        );
        assert!(on.iter().any(|r| matches!(r, Request::Effect { .. })));
        assert!(!off.iter().any(sounds), "{off:?}");
        assert!(off.contains(&Request::Stop));
    }

    #[test]
    fn the_kaffee_s_entry_6_ends_the_game() {
        use crate::titles::parsec1994::Api;
        let mut seen = Vec::new();
        let Some(mut g) = entered("bifi2-end", 6, 1, &mut seen) else {
            return;
        };
        tick_until(&mut g, &mut seen, 30_000, "the end's wait", |g, _| {
            g.inner.location_word == 0xFFFF
        });
        assert!(
            g.visit.is_some() && !g.ending(),
            "the wait runs in the visit"
        );
        let start = Api::word(&g.inner, 0x1CF6);
        assert!(start < 0xC8, "the hundredths set to 0: {start}");
        tick_until(&mut g, &mut seen, 1_000, "the end's sequence", |g, _| {
            g.ending()
        });
        assert!(g.visit.is_none());
        let mut n = 0;
        while !g.over {
            if n % 300 == 299 {
                g.click(true);
            }
            g.tick();
            n += 1;
            assert!(n < 20_000, "the program ends");
        }
    }

    /// The host's pointer, moved while the title program runs, does not
    /// move `XP01`'s library, which begins where its image has it, 160,
    /// 100 (`[486h]`/`[488h]`).
    #[test]
    fn xp01_begins_with_its_library_s_pointer() {
        use crate::titles::{Runtime, Start};
        let Some(dir) = motionvm_parsec_testutil::gamedata_bifi2() else {
            return;
        };
        let mut g = super::BiFi2::open(&dir).expect("opens");
        g.set_saves(&motionvm_parsec_testutil::saves_dir("bifi2-xp01-pointer"))
            .expect("saves");
        g.start(Start::New).expect("start");
        for n in 0..3000 {
            if n == 600 {
                g.click(true);
            }
            assert!(g.tick());
        }
        assert!(g.xp00.program.is_some(), "the title program runs");
        // "Neues Spiel beginnen", the menu's first line at row 9Ah.
        g.mouse_move(160, 0x9A + 4);
        g.click(true);
        for _ in 0..400 {
            assert!(g.tick());
            if g.xp00.program.is_none() {
                break;
            }
        }
        assert!(g.xp00.program.is_none(), "XP01 runs");
        assert_eq!(g.game.inner.mouse, (160, 100));
    }

    /// The figure's cels count from the file's resource 0: the loader keeps
    /// the file's start (`0x0B79E`–`0x0B7A8`), so cel 0 is resource 0 and
    /// every resource is a cel.
    #[test]
    fn the_figure_s_cels_count_from_the_file_s_start() {
        let Some(g) = fresh_game("bifi2-figure-cels") else {
            return;
        };
        let dir = motionvm_parsec_testutil::gamedata_bifi2().expect("data");
        let res = motionvm_parsec_formats::detect::resource_dir(&dir);
        let path = motionvm_parsec_formats::find_ci(&res, "PLAYER.DAT").expect("PLAYER.DAT");
        let rib = motionvm_parsec_formats::Rib::parse(&std::fs::read(path).expect("reads"))
            .expect("a RIB");
        let first = motionvm_parsec_formats::Picture::parse(rib.get(0).expect("resource 0"))
            .expect("resource 0 is a cel");
        let cel = &g.inner.figure_cels[0];
        assert_eq!((cel.width, cel.height), (first.width, first.height));
        assert_eq!(cel.pixels, first.pixels);
        assert_eq!(g.inner.figure_cels.len(), rib.resources.len());
    }
}
