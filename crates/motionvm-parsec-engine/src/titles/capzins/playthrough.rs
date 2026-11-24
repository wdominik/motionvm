//! Captain Zins played from its first scene to its end, through the host's
//! own roads: the verb buttons clicked, the carried items clicked in the
//! strip (scrolled to with its arrows), the hotspots clicked where the
//! pointer finds them, the dialogue menus answered with Tab and Enter, a
//! text clicked short as a player would. Nothing of the runtime is set by
//! hand.
//!
//! The path is the one the game's logic allows, derived from its rules:
//! the Kontoauszug won through the bank's machines, the trap in Saldo's
//! room, the eight Bits on the Codepoint lit as 141, Sharky and Madame
//! Extra-Sparen, the safe, the robot, I-GOR, Money; then the real world's
//! four things to the Professor and the way back to Money, whose third
//! scene ends the game — event 12h writes `FFFFh` to the location word and
//! the main loop leaves on a location of −1.
//!
//! It runs for minutes of game time: `#[ignore]`d, so `just check` skips
//! it; `just playthrough` runs it, in release.

use super::{CaptainZins, capzins_or_skip};
use crate::titles::Runtime;
use crate::titles::parsec1994::{Adventure, Api};

/// The verbs as the strip's buttons carry them, their ids' low bytes.
const LOOK: u8 = 1;
const TALK: u8 = 2;
const TAKE_GIVE: u8 = 3;
const USE: u8 = 4;
const COMBINE: u8 = 5;

/// What a choice in a dialogue menu is picked by.
type Choose = Box<dyn Fn(&Adventure, &[usize]) -> usize>;

/// An object of an action: a carried item, or a hotspot of the room.
#[derive(Debug, Clone, Copy)]
enum Thing {
    Item(u8),
    Spot(u16),
}

/// The game and how its menus are answered.
struct Player {
    g: CaptainZins,
    choose: Choose,
    /// The location an exit is taken from, and whether the game has
    /// stood anywhere else since.
    leaving: Option<i32>,
    left: bool,
}

impl Player {
    fn tick(&mut self, n: usize) {
        for _ in 0..n {
            if !self.g.tick() {
                return;
            }
            if self
                .leaving
                .is_some_and(|from| self.g.game.inner.location() != from)
            {
                self.left = true;
            }
        }
    }

    fn busy(&self) -> bool {
        let a = &self.g.game.inner;
        matches!(a.tasks.t[0].kind, 3..=5) || matches!(a.mode, 4..=8) || a.effects.busy()
    }

    /// Plays on until nothing is going on: a menu answered with Tab to
    /// the choice wanted and Enter, a text clicked short, the rest waited
    /// for.
    fn settle(&mut self) {
        for t in 0..40_000 {
            if self.g.game.over || !self.busy() {
                return;
            }
            let a = &self.g.game.inner;
            if a.tasks.t[0].kind == 5 && a.menu.up {
                let pos = (self.choose)(a, &a.tasks.choices);
                let want = u16::try_from(pos + 1).unwrap_or(1);
                for _ in 0..=a.tasks.choices.len() {
                    if self.g.game.inner.menu.highlight == want {
                        break;
                    }
                    self.g.key(9);
                }
                self.g.key(13);
            } else if matches!(a.tasks.t[0].kind, 3 | 4) && t % 30 == 0 {
                self.g.click(true);
            }
            self.tick(1);
        }
    }

    /// The pointer to `(x, y)` and a click there.
    fn click_at(&mut self, x: i32, y: i32) {
        self.g.mouse_move(x, y);
        self.tick(1);
        self.g.click(true);
        self.tick(1);
    }

    /// A strip widget's middle.
    fn widget(&self, id: u16) -> (i32, i32) {
        let w = self
            .g
            .game
            .inner
            .panel_widgets
            .widgets
            .iter()
            .find(|w| w.id == id)
            .unwrap_or_else(|| panic!("no strip widget {id:#x}"));
        (
            i32::from(w.x) + i32::from(w.w) / 2,
            i32::from(w.y) + i32::from(w.h) / 2,
        )
    }

    /// A carried item clicked in the strip, scrolled to first.
    fn click_item(&mut self, id: u8) {
        for _ in 0..40 {
            let a = &self.g.game.inner;
            let slots = a.panel_widgets.grid.len();
            if let Some(n) = (0..slots).find(|&n| a.item_in_slot(n) == Some(id)) {
                let wid = a.panel_widgets.slots.start() + u16::try_from(n).unwrap_or(0);
                let (x, y) = self.widget(wid);
                self.click_at(x, y);
                return;
            }
            let at = a.items.iter().position(|&i| i == id);
            let up = at.is_some_and(|i| i < a.scroll_for_test());
            let arrows = a.panel_widgets.arrows;
            let (x, y) = self.widget(if up { arrows.up } else { arrows.down });
            self.click_at(x, y);
        }
        panic!(
            "item {id} is not in the strip: {:?}",
            self.g.game.inner.items
        );
    }

    /// A hotspot clicked where the pointer finds it.
    fn click_spot(&mut self, id: u16) {
        let a = &self.g.game.inner;
        let room = a.room().expect("a room is up");
        let i = room
            .hotspots
            .spots
            .iter()
            .enumerate()
            .rposition(|(i, h)| {
                (h.verb_mask & 0xFF) == id && a.hotspots_rt.get(i).is_some_and(|rt| rt.active)
            })
            .unwrap_or_else(|| panic!("no hotspot {id} in location {}", a.location()));
        let h = a.hotspots_rt[i].clone();
        let point = (h.y..h.y + h.h)
            .step_by(2)
            .flat_map(|y| (h.x..h.x + h.w).step_by(2).map(move |x| (x, y)))
            .find(|&(x, y)| {
                self.g.game.inner.mouse_move(x, y);
                self.g.game.inner.hotspot_index_under_pointer() == Some(i)
            })
            .unwrap_or_else(|| panic!("hotspot {id} is under the pointer nowhere"));
        self.click_at(point.0, point.1);
    }

    fn click_thing(&mut self, t: Thing) {
        match t {
            Thing::Item(id) => self.click_item(id),
            Thing::Spot(id) => self.click_spot(id),
        }
    }

    /// A verb button clicked, then its object, and the second if it takes
    /// one; then the game played on until it rests.
    fn act(&mut self, verb: u8, first: Thing, second: Option<Thing>) {
        self.settle();
        // The verb stays chosen after an action, and its button clicked
        // again parks it (`0x0FC6D`): clicked only when it changes.
        let a = &self.g.game.inner;
        if !(a.mode == 2 && a.verb == verb) {
            let (x, y) = self.widget(0x100 | u16::from(verb));
            self.click_at(x, y);
        }
        self.click_thing(first);
        if let Some(s) = second {
            self.click_thing(s);
        }
        self.tick(3);
        self.settle();
        self.tick(200);
        self.settle();
    }

    /// Through the exits named, one after the other. A visit may send the
    /// player straight back (Sharky's do): what is asked is that the exit
    /// led out.
    fn go(&mut self, exits: &[u16]) {
        for &e in exits {
            let from = self.g.game.inner.location();
            self.leaving = Some(from);
            self.left = false;
            for _ in 0..3 {
                self.act(USE, Thing::Spot(e), None);
                if self.left {
                    break;
                }
            }
            for _ in 0..4 {
                self.tick(100);
                self.settle();
            }
            self.leaving = None;
            assert!(self.left, "exit {e} leads out of location {from}");
        }
    }

    fn talk(&mut self, spot: u16) {
        self.act(TALK, Thing::Spot(spot), None);
    }

    /// A hotspot taken until the item is carried.
    fn take(&mut self, id: u8) {
        for _ in 0..3 {
            self.act(TAKE_GIVE, Thing::Spot(u16::from(id)), None);
            if self.g.game.inner.items.contains(&id) {
                return;
            }
        }
        panic!(
            "item {id} could not be taken in location {}",
            self.g.game.inner.location()
        );
    }

    fn rest(&mut self, n: usize) {
        for _ in 0..n {
            self.tick(100);
            self.settle();
        }
    }

    /// Menus answered with the records in `list` where offered, else with
    /// the choice whose dialogue ends soonest.
    fn prefer(&mut self, list: &'static [usize]) {
        self.choose = Box::new(move |a, choices| {
            if let Some(p) = choices.iter().position(|k| list.contains(k)) {
                return p;
            }
            let Some(d) = a.tasks.t[0].dialogue.and_then(|r| a.dialogue_of(r)) else {
                return 0;
            };
            let steps = |start: usize| -> usize {
                let mut seen = vec![false; d.len()];
                let mut queue = std::collections::VecDeque::from([(start, 0usize)]);
                while let Some((k, n)) = queue.pop_front() {
                    if k >= d.len() || seen[k] {
                        continue;
                    }
                    seen[k] = true;
                    let Some(rec) = d.record(k) else { continue };
                    if rec.next.is_empty() {
                        return n;
                    }
                    queue.extend(rec.next.iter().map(|&x| (usize::from(x), n + 1)));
                }
                usize::MAX
            };
            (0..choices.len())
                .min_by_key(|&i| steps(choices[i]))
                .unwrap_or(0)
        });
    }

    /// Menus answered with the choice whose text raises script event `ev`.
    fn prefer_event(&mut self, ev: u8) {
        self.choose = Box::new(move |a, choices| {
            let Some(d) = a.tasks.t[0].dialogue.and_then(|r| a.dialogue_of(r)) else {
                return 0;
            };
            choices
                .iter()
                .position(|&k| {
                    d.record(k).is_some_and(|rec| {
                        motionvm_parsec_formats::to_cp437(rec.text)
                            .windows(2)
                            .any(|w| w == [3, ev])
                    })
                })
                .unwrap_or(0)
        });
    }
}

/// The Kontoauszug.
fn kontoauszug(p: &mut Player) {
    p.talk(18);
    p.take(18);
    p.go(&[101]); // Telefon
    p.talk(200);
    p.take(23);
    p.go(&[100]); // Wucher: the ServiceCard
    p.go(&[100, 101, 102]); // Telefon, Kreuzung, PC
    p.prefer(&[1, 11, 74, 93, 109, 111, 114]);
    p.talk(200);
    p.take(19);
    p.prefer(&[]);
    p.go(&[102, 101]); // Kreuzung, Telefon
    p.talk(200);
    p.go(&[101, 100]); // Kreuzung, Bits
    p.go(&[100, 102, 100]); // Kreuzung, PC, Schranke
    for ev in [200, 201, 201, 200, 200, 201, 201, 200] {
        p.prefer_event(ev);
        p.talk(200);
    }
    p.prefer(&[]);
    p.tick(200);
    p.take(20);
    p.go(&[102]); // Sparplan
    p.talk(200);
    p.take(22);
    p.go(&[102, 100, 102, 101]); // Schranke, PC, Kreuzung, Telefon
    p.talk(200);
    p.talk(200);
    p.go(&[101]); // Kreuzung
    p.act(TAKE_GIVE, Thing::Item(6), Some(Thing::Spot(129)));
    p.rest(3);
    assert!(p.g.game.inner.items.contains(&8), "the Kontoauszug");
}

/// Saldo's trap; the Robots' Bit and the Bau's.
fn saldo_robots_bau(p: &mut Player) {
    p.go(&[102, 100, 102, 100]); // PC, Schranke, Sparplan, Saldo
    p.act(TAKE_GIVE, Thing::Item(8), Some(Thing::Spot(128)));
    p.talk(11);
    p.act(USE, Thing::Spot(200), None);
    p.go(&[101]); // Robots
    p.prefer(&[4]);
    p.talk(200);
    p.take(25);
    p.prefer(&[]);
    p.go(&[100]); // Bau
    p.take(21);
    p.go(&[100, 101, 100, 102, 101]); // Robots, Saldo, Sparplan, Schranke, Formel
    p.prefer(&[3]);
    p.talk(200);
    p.prefer(&[]);
}

/// Wucher's way to Igor, then the eight Bits on the Codepoint.
fn wucher_and_the_bits(p: &mut Player) {
    p.go(&[101, 100, 102, 101, 100]); // Schranke, PC, Kreuzung, Telefon, Wucher
    p.act(TAKE_GIVE, Thing::Item(11), Some(Thing::Spot(200)));
    p.rest(3);
    p.take(24);
    p.go(&[100, 101, 100]); // Telefon, Kreuzung, Bits
    for id in 18..=25 {
        p.act(COMBINE, Thing::Item(id), Some(Thing::Spot(200)));
    }
    for id in [18, 20, 21, 25] {
        p.prefer_event(201);
        p.talk(id);
    }
    p.prefer(&[]);
}

/// Sharky, Madame Extra-Sparen, the Zinsen.
fn sharky_and_extra(p: &mut Player) {
    p.go(&[101]); // Schrank
    p.go(&[100]); // Sharky
    p.prefer(&[3]);
    p.go(&[100]);
    p.prefer(&[]);
    p.go(&[100]);
    p.go(&[101, 100, 102, 100, 102]); // Bits, Kreuzung, PC, Schranke, Sparplan
    p.talk(200);
    p.go(&[102, 100, 102, 100, 101, 100]); // Schranke, PC, Kreuzung, Bits, Schrank, Sharky
    p.go(&[101]); // Extra
    p.talk(200);
    p.go(&[101, 100, 101, 100, 101]); // Sharky, Schrank, Bits, Kreuzung, Telefon
    p.talk(200);
    p.go(&[101, 100, 101, 100, 101]); // Kreuzung, Bits, Schrank, Sharky, Extra
    p.talk(200);
}

/// The safe, the Backstein, the robot.
fn safe_and_robot(p: &mut Player) {
    p.go(&[101, 100, 101, 100, 102, 100, 102, 100, 101, 100]); // … to the Bau
    p.act(USE, Thing::Spot(200), None);
    p.prefer(&[10]);
    p.act(COMBINE, Thing::Item(12), Some(Thing::Spot(200)));
    p.prefer(&[]);
    p.rest(3);
    p.take(13);
    p.go(&[100, 101, 100, 102, 100, 102, 100, 101]); // … to the Schrank
    p.act(COMBINE, Thing::Item(13), Some(Thing::Spot(202)));
    p.act(USE, Thing::Spot(203), None);
    p.take(16);
    p.take(14);
    p.act(COMBINE, Thing::Item(14), Some(Thing::Item(16)));
    assert!(
        p.g.game.inner.items.contains(&17),
        "the robot with its battery"
    );
}

/// I-GOR, and Money's first scene sends the player back.
fn igor_and_money(p: &mut Player) {
    p.choose = Box::new(|_, choices| {
        let pick = [10usize, 22, 32, 40, 51, 62];
        choices.iter().position(|k| pick.contains(k)).unwrap_or(0)
    });
    p.go(&[101, 100, 101, 100, 101]); // Bits, Kreuzung, Telefon, Wucher, Igor
    p.prefer(&[]);
    for _ in 0..60 {
        p.tick(500);
        if p.g.game.inner.location() != 17 {
            break;
        }
    }
    p.rest(10);
    if p.g.game.inner.location() == 16 {
        p.go(&[101]);
    }
    p.go(&[100]); // Money
    p.rest(6);
}

/// The real world: four things for the Professor.
fn real_world_and_the_end(p: &mut Player) {
    p.go(&[100]); // Wohnung
    p.act(USE, Thing::Spot(201), None);
    p.act(LOOK, Thing::Spot(201), None);
    p.take(3);
    p.go(&[100]); // Prof
    p.act(LOOK, Thing::Spot(200), None);
    p.take(5);
    p.take(2);
    for id in 1..=4 {
        p.act(TAKE_GIVE, Thing::Item(id), Some(Thing::Spot(201)));
    }
    // The way back plays by itself, to Money's third scene.
    for _ in 0..40 {
        p.settle();
        p.tick(200);
        if p.g.game.ending() {
            break;
        }
    }
}

#[test]
#[ignore = "minutes of game time: `just playthrough`"]
fn captain_zins_can_be_played_to_its_end() {
    let Some(g) = capzins_or_skip() else {
        return;
    };
    let mut p = Player {
        g,
        choose: Box::new(|_, _| 0),
        leaving: None,
        left: false,
    };
    p.prefer(&[]);
    // The opening plays by itself: Wohnung, Prof, Kreuzung.
    p.rest(40);
    assert_eq!(
        p.g.game.inner.location(),
        3,
        "the opening ends at the Kreuzung"
    );

    kontoauszug(&mut p);
    saldo_robots_bau(&mut p);
    wucher_and_the_bits(&mut p);
    sharky_and_extra(&mut p);
    safe_and_robot(&mut p);
    igor_and_money(&mut p);
    real_world_and_the_end(&mut p);
    let a = &p.g.game.inner;
    assert!(
        p.g.game.ending(),
        "the main loop ended, in location {}",
        a.location()
    );
    assert_eq!(
        Api::word(a, 0x1A5C),
        0xFFFF,
        "on a location of −1, not by quitting"
    );
    assert_eq!(a.mem_byte(0x1CEC), 3, "Money's third scene");
    // The parade, until a click; then the program is over.
    p.tick(1000);
    assert!(p.g.game.ending() && !p.g.game.over, "the parade goes on");
    p.g.click(true);
    let over = (0..2000).any(|_| !p.g.tick());
    assert!(over, "the game is over");
}
