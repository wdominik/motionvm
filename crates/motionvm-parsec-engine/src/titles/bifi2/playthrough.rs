//! *Bi-Fi 2* played from a fresh game to its end, through the host's own
//! roads: the verb buttons clicked, the carried items clicked in the strip
//! (scrolled to with its arrows), the hotspots clicked where the pointer
//! finds them, the dialogue menus answered with Tab and Enter, a text
//! clicked short as a player would. Nothing of the runtime is set by hand.
//!
//! The path is the one the game's logic allows, derived from its rules
//! and its dialogue banks: the Pförtner's clip and pass, the Kantine's
//! Autogramm, Apfel and Bi-Fi-Roll, the Apfel that sends the secretary
//! away, the Vorzimmer's computer programming the clip for the second
//! floor's doors; Rambowsky's course through the Denkfabrik, the Kantine
//! and Schlönstadt's Büro to the costume from the Maske's Vitrine with the
//! Bi-Fi-Roll's tattoo, worn in Rambowsky's scene; the Dino Zoo's
//! Wagenheber and Gagbrille, the Pförtner's Brille, the Maschinenraum's
//! Tonspule for the Tontechniker's Nachricht, both for the Techniker, whose
//! Zeitung gives a key and the VIP-Karte; the Saboteur's Perücke and key,
//! the Requisiteur's Leuchtfarbe, the VIP-Lounge's Buch; Öliens's Geld,
//! the 08-15's photo strip, the Ausweis for the SFX's Pyrotool, Jones 1 and
//! Jones 2 painted; the Vorführraum's crate moved and the labyrinth behind
//! its Luke, the Skelett's Stadtplan made the Labyrinth-Karte with
//! Rambowsky's hot Eimer, the lock at location 46 and Schlönstadt freed;
//! then the Bi-Fi room's last scene takes the Drehbuch, the Kaffee's entry
//! 6 writes `FFFFh` to the location word and the main loop leaves on a
//! location of −1, into the build's ending.
//!
//! It runs for minutes of game time: `#[ignore]`d, so `just check` skips
//! it; `just playthrough` runs it, in release.

use super::BiFi2;
use crate::titles::parsec1994::{Adventure, Api, Event, LOCKED};
use crate::titles::{Runtime, Start};

/// The verbs as the strip's six buttons carry them, their ids' low bytes
/// (`DS:116Ah`). The others are a rule's: `Benutze` on an item becomes
/// `Benutze … mit`, `Gib … an` or `Iß` by the rule the item and its class
/// find, on a door `Gehe durch`.
const LOOK: u8 = 1;
const TAKE: u8 = 2;
const TALK: u8 = 3;
const USE: u8 = 4;
const OPEN: u8 = 6;

/// The corridors by floor, ground floor first.
const FLOORS: [i32; 8] = [18, 19, 31, 32, 33, 34, 35, 30];
/// The Lift.
const LIFT: i32 = 36;

/// An object of an action: a carried item, or a hotspot of the room.
#[derive(Debug, Clone, Copy)]
enum Thing {
    Item(u8),
    Spot(u16),
}

/// How the dialogue menus are answered: by the records wanted (an id the
/// hook is told about), reached by the shortest way that passes no record
/// whose id is to be avoided; else by a line's text; else by the choice
/// whose dialogue ends soonest.
#[derive(Default)]
struct Answers {
    want: Vec<u16>,
    avoid: Vec<u16>,
    texts: Vec<&'static str>,
}

impl Answers {
    fn choose(&self, a: &Adventure, choices: &[usize]) -> usize {
        let Some(d) = a.tasks.t[0].dialogue.and_then(|r| a.dialogue_of(r)) else {
            return 0;
        };
        let steps = |start: usize, goal: &dyn Fn(u16, bool) -> bool| -> usize {
            let mut seen = vec![false; d.len()];
            let mut queue = std::collections::VecDeque::from([(start, 0usize)]);
            while let Some((k, n)) = queue.pop_front() {
                if k >= d.len() || seen[k] {
                    continue;
                }
                seen[k] = true;
                let Some(rec) = d.record(k) else { continue };
                if self.avoid.contains(&rec.id) {
                    continue;
                }
                if goal(rec.id, rec.next.is_empty()) {
                    return n;
                }
                queue.extend(rec.next.iter().map(|&x| (usize::from(x), n + 1)));
            }
            usize::MAX
        };
        let best = |goal: &dyn Fn(u16, bool) -> bool| {
            (0..choices.len())
                .map(|i| (steps(choices[i], goal), i))
                .filter(|&(n, _)| n != usize::MAX)
                .min()
                .map(|(_, i)| i)
        };
        if let Some(i) = best(&|id, _| id != 0 && self.want.contains(&id)) {
            return i;
        }
        for t in &self.texts {
            if let Some(i) = choices
                .iter()
                .position(|&k| d.record(k).is_some_and(|r| r.text.contains(t)))
            {
                return i;
            }
        }
        best(&|_, end| end).unwrap_or(0)
    }
}

/// The game and how its menus are answered.
struct Player {
    g: BiFi2,
    answers: Answers,
    /// What the runtime said it has not got, on the way.
    unbuilt: Vec<String>,
    /// The location an exit is taken from, and whether the game has
    /// stood anywhere else since.
    leaving: Option<i32>,
    left: bool,
}

impl Player {
    fn a(&self) -> &Adventure {
        &self.g.game.inner
    }

    fn here(&self) -> i32 {
        self.a().location()
    }

    fn byte(&self, addr: u16) -> u8 {
        self.a().mem_byte(addr)
    }

    fn has(&self, id: u8) -> bool {
        self.a().items.contains(&id)
    }

    fn tick(&mut self, n: usize) {
        for _ in 0..n {
            if !self.g.tick() {
                return;
            }
            let here = self.here();
            for e in std::mem::take(&mut self.g.game.inner.events) {
                match e {
                    // A record's id, told the hooks as code 4: what was
                    // wanted of the dialogue is had.
                    Event::LocationHook { code: 4, arg, .. } | Event::Hook { code: 4, arg, .. } => {
                        let id = crate::num::unsigned(arg);
                        self.answers.want.retain(|&w| w != id);
                    }
                    Event::Unbuilt(..)
                    | Event::UnreadRule { .. }
                    | Event::SceneRefused(_)
                    | Event::FarWrite { .. }
                    | Event::EndingUnread(_) => {
                        self.unbuilt.push(format!("in location {here}: {e:?}"));
                    }
                    _ => {}
                }
            }
            if self.leaving.is_some_and(|from| here != from) {
                self.left = true;
            }
        }
    }

    fn busy(&self) -> bool {
        let a = self.a();
        matches!(a.tasks.t[0].kind, 3..=5)
            || matches!(a.mode, 4..=8)
            || a.effects.busy()
            || a.bits & LOCKED != 0
    }

    /// Plays on until nothing is going on: a menu answered with Tab to
    /// the choice wanted and Enter, a text clicked short, the rest waited
    /// for.
    fn settle(&mut self) {
        for t in 0..40_000 {
            if self.g.game.over || self.g.game.ending() || !self.busy() {
                return;
            }
            let a = &self.g.game.inner;
            if a.tasks.t[0].kind == 5 && a.menu.up {
                let pos = self.answers.choose(a, &a.tasks.choices);
                let want = u16::try_from(pos + 1).unwrap_or(1);
                for _ in 0..=a.tasks.choices.len() {
                    if self.g.game.inner.menu.highlight == want {
                        break;
                    }
                    self.g.key(9);
                }
                // The menu's own table takes 101h, handed to the runtime
                // past the pointer library's hook.
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
            .a()
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
            let a = self.a();
            let slots = a.panel_widgets.grid.len();
            if let Some(n) = (0..slots).find(|&n| a.item_in_slot(n) == Some(id)) {
                let wid = a.panel_widgets.slots.start() + u16::try_from(n).unwrap_or(0);
                let (x, y) = self.widget(wid);
                self.click_at(x, y);
                return;
            }
            let at = a.items.iter().position(|&i| i == id);
            assert!(at.is_some(), "item {id:#x} is not carried: {:x?}", a.items);
            let up = at.is_some_and(|i| i < a.scroll_for_test());
            let arrows = a.panel_widgets.arrows;
            let (x, y) = self.widget(if up { arrows.up } else { arrows.down });
            self.click_at(x, y);
        }
        panic!("item {id:#x} is not in the strip: {:x?}", self.a().items);
    }

    /// A hotspot clicked where the pointer finds it.
    fn click_spot(&mut self, id: u16) {
        let a = self.a();
        let room = a.room().expect("a room is up");
        let i = room
            .hotspots
            .spots
            .iter()
            .enumerate()
            .rposition(|(i, h)| {
                (h.verb_mask & 0xFF) == id && a.hotspots_rt.get(i).is_some_and(|rt| rt.active)
            })
            .unwrap_or_else(|| panic!("no hotspot {id:#x} in location {}", a.location()));
        let h = a.hotspots_rt[i].clone();
        let point = (h.y..h.y + h.h)
            .step_by(2)
            .flat_map(|y| (h.x..h.x + h.w).step_by(2).map(move |x| (x, y)))
            .find(|&(x, y)| {
                self.g.game.inner.mouse_move(x, y);
                self.g.game.inner.hotspot_index_under_pointer() == Some(i)
            })
            .unwrap_or_else(|| panic!("hotspot {id:#x} is under the pointer nowhere"));
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
        // again parks it (`0x0ABBF`–`0x0ABC5`): clicked only when it
        // changes.
        let a = self.a();
        if !(a.mode == 2 && a.verb == verb) {
            let (x, y) = self.widget(0x100 | u16::from(verb));
            self.click_at(x, y);
        }
        self.click_thing(first);
        if let Some(s) = second {
            // The first object's rule may walk before it wants the second.
            self.tick(3);
            self.settle();
            self.click_thing(s);
        }
        self.tick(3);
        self.settle();
        self.tick(200);
        self.settle();
    }

    fn rest(&mut self, n: usize) {
        for _ in 0..n {
            self.tick(100);
            self.settle();
        }
    }

    /// Plays on until location `loc` stands and rests.
    fn reach(&mut self, loc: i32, what: &str) {
        for _ in 0..300 {
            if self.here() == loc && !self.busy() {
                self.rest(2);
                if self.here() == loc {
                    return;
                }
            }
            self.rest(1);
        }
        panic!("{what}: in location {} instead of {loc}", self.here());
    }

    /// Through the exits named, one after the other: what is asked is that
    /// each led out.
    fn go(&mut self, exits: &[u16]) {
        for &e in exits {
            let from = self.here();
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
            assert!(self.left, "exit {e:#x} leads out of location {from}");
        }
    }

    /// From a corridor to the corridor of floor `to`, by the Lift: its
    /// door is the corridor's exit `64h` plus the floor, and in the Lift
    /// the button for a floor carries the same id.
    fn ride(&mut self, to: usize) {
        let here = self.here();
        let from = FLOORS
            .iter()
            .position(|&l| l == here)
            .unwrap_or_else(|| panic!("location {here} is no corridor"));
        if from == to {
            return;
        }
        self.go(&[0x64 + u16::try_from(from).unwrap_or(0)]);
        assert_eq!(self.here(), LIFT, "the Lift");
        self.go(&[0x64 + u16::try_from(to).unwrap_or(0)]);
        self.reach(FLOORS[to], "the ride");
    }

    fn talk(&mut self, spot: u16) {
        self.act(TALK, Thing::Spot(spot), None);
    }

    /// A hotspot taken until the item is carried.
    fn take(&mut self, id: u8) {
        for _ in 0..3 {
            self.act(TAKE, Thing::Spot(u16::from(id)), None);
            if self.has(id) {
                return;
            }
        }
        panic!(
            "item {id:#x} could not be taken in location {}",
            self.here()
        );
    }

    fn answer(&mut self, want: &[u16], avoid: &[u16], texts: &[&'static str]) {
        self.answers = Answers {
            want: want.to_vec(),
            avoid: avoid.to_vec(),
            texts: texts.to_vec(),
        };
    }

    fn plain(&mut self) {
        self.answers = Answers::default();
    }

    fn state(&self) -> String {
        format!(
            "location {} items {:x?} block {:02x?}",
            self.here(),
            self.a().items,
            (0x2B98..0x2BE0u16)
                .map(|a| self.byte(a))
                .collect::<Vec<_>>()
        )
    }
}

/// The Pforte's first talk, the Kantine's two, the kid's trade, and the
/// Vorzimmer up to the programmed clip.
fn ground_floor_and_clip(p: &mut Player) {
    p.reach(4, "the opening ends at the Pforte");
    // The Pforte's entry talks to the gatekeeper by itself.
    p.answer(&[1], &[], &["weitergehen"]);
    p.rest(5);
    assert!(p.has(1) && p.has(2), "clip and pass: {}", p.state());
    p.plain();
    p.go(&[0x6E]);
    p.reach(18, "the ground floor");
    p.ride(5);
    p.go(&[0x6F]); // Kantine
    p.answer(&[0x1A], &[], &[]);
    p.talk(0x98);
    assert!(p.has(0x21), "the Autogramm: {}", p.state());
    p.answer(&[0x1B, 0x1C], &[], &[]);
    p.talk(0xF0);
    assert!(p.has(0x10) && p.has(0x22), "Apfel and Bi-Fi: {}", p.state());
    p.plain();
    p.go(&[0x6F]);
    p.ride(0);
    p.go(&[0x6E]); // Pforte
    p.answer(&[0x2B, 0x2C], &[], &[]);
    p.talk(0xA6);
    assert!(p.has(0x1E), "the Ball: {}", p.state());
    p.plain();
    p.go(&[0x6E, 0x6F]); // Flur EG, Vorzimmer
    p.reach(5, "the Vorzimmer");
    p.answer(&[8], &[], &[]);
    p.act(USE, Thing::Item(0x10), None);
    assert_eq!(p.byte(0x2BA1), 1, "the secretary gone: {}", p.state());
    p.plain();
    p.act(LOOK, Thing::Spot(0x82), None);
    assert!(p.has(0x0D), "the Haarnadel: {}", p.state());
    p.act(OPEN, Thing::Spot(0x84), None);
    p.answer(&[7], &[], &[]);
    p.act(LOOK, Thing::Spot(0x84), None);
    assert!(p.has(6), "the Sicherheitsausweis: {}", p.state());
    p.plain();
    p.go(&[0x6E]); // Büro
    p.act(LOOK, Thing::Spot(0x9D), None);
    assert_eq!(p.byte(0x2B9E), 1, "the Zettel: {}", p.state());
    p.go(&[0x6E]); // Vorzimmer
    p.act(USE, Thing::Spot(0x83), None);
    assert_eq!(p.byte(0x2B9D), 1, "the computer on: {}", p.state());
    p.answer(&[9], &[], &[]);
    p.act(USE, Thing::Spot(0x83), None);
    assert_eq!(p.byte(0x2B9D), 2, "the password: {}", p.state());
    p.plain();
    p.act(USE, Thing::Item(1), Some(Thing::Spot(0x83)));
    assert_eq!(p.byte(0x2BA2), 1, "the clip programmed: {}", p.state());
}

/// Rambowsky's course: sent to the Denkfabrik, the Regisseur in the
/// Kantine, Schlönstadt's meeting and the Sprechanlage's errand; the
/// costume from the Maske's Vitrine with the Bi-Fi-Roll's tattoo on it.
fn course_to_the_costume(p: &mut Player) {
    p.go(&[0x6F]); // Flur EG
    p.ride(4);
    p.go(&[0x6E]); // Rambowsky, who sends the player out again
    p.reach(33, "Rambowsky sends the player out");
    assert_eq!(p.byte(0x2BBE), 1, "{}", p.state());
    p.ride(1);
    p.go(&[0x70]); // Denkfabrik
    p.rest(5);
    assert_eq!(
        p.byte(0x2BBE),
        2,
        "the Denkfabrik's greeting: {}",
        p.state()
    );
    p.go(&[0x70]);
    p.ride(5);
    p.go(&[0x6F]); // Kantine
    p.talk(0x98);
    assert_eq!(p.byte(0x2BA6), 2, "the course: {}", p.state());
    p.go(&[0x6F]);
    p.ride(0);
    p.go(&[0x6F, 0x6E]); // Vorzimmer, Büro
    p.answer(&[], &[], &["Ich heiße Lukas und suche den Direktor!"]);
    p.rest(5);
    assert_eq!(p.byte(0x2BC6), 1, "the meeting: {}", p.state());
    p.plain();
    p.talk(0x9A);
    // The Sprechanlage sends the player to the Kantine, and its man back.
    for _ in 0..60 {
        p.rest(1);
        if p.byte(0x2BC7) == 3 && p.here() == 24 && !p.busy() {
            break;
        }
    }
    assert_eq!(p.byte(0x2BC7), 3, "the errand: {}", p.state());
    assert_eq!(p.byte(0x2BA6), 3, "the course: {}", p.state());
    p.go(&[0x6E, 0x6F]); // Vorzimmer, Flur EG
    p.ride(2);
    p.go(&[0x6F]); // Maske
    p.act(USE, Thing::Item(0x0D), Some(Thing::Spot(0x95)));
    assert_eq!(p.byte(0x2BBF), 1, "the Vitrine unlocked: {}", p.state());
    p.act(OPEN, Thing::Spot(0x95), None);
    p.take(0x17);
    p.act(LOOK, Thing::Spot(0x44), None);
    assert!(p.has(9), "the Kaugummi: {}", p.state());
    p.act(USE, Thing::Item(0x22), None);
    assert!(p.has(0x19), "the tattoo: {}", p.state());
    p.act(USE, Thing::Item(0x19), Some(Thing::Item(0x17)));
    assert!(p.has(0x18), "the costume: {}", p.state());
}

/// The Denkfabrik's Drehbuch looked at, the costume worn for Rambowsky;
/// the Dino Zoo's Wagenheber and Gagbrille.
fn costume_and_dino_zoo(p: &mut Player) {
    p.go(&[0x6F]);
    p.ride(1);
    p.go(&[0x70]); // Denkfabrik
    p.act(LOOK, Thing::Spot(0xF0), None);
    assert_eq!(p.byte(0x2BA7), 1, "the Drehbuch looked at: {}", p.state());
    p.go(&[0x70]);
    p.ride(4);
    p.go(&[0x6E]); // Rambowsky
    for _ in 0..80 {
        p.rest(1);
        if p.byte(0x2BA6) == 5 && p.here() == 7 && !p.busy() {
            break;
        }
    }
    assert_eq!(p.byte(0x2BA6), 5, "the scene shot: {}", p.state());
    p.go(&[0x6F]); // Dino Zoo
    p.answer(&[0x28], &[], &[]);
    p.talk(0x90);
    assert_eq!(p.byte(0x2BD9), 1, "the Wagenheber offered: {}", p.state());
    p.answer(&[0x0F], &[], &[]);
    p.talk(0x90);
    assert!(p.has(0x1F), "the Wagenheber: {}", p.state());
    p.plain();
    p.act(USE, Thing::Item(0x1E), None);
    p.rest(5);
    p.take(0x15);
}

/// The Gagbrille for the Pförtner's Brille; the Maschinenraum's Tonspule
/// for the Tontechniker's Nachricht; both for the Techniker, whose
/// Zeitung gives the keys and the VIP-Karte.
fn tonspule_brille_and_keys(p: &mut Player) {
    costume_and_dino_zoo(p);
    p.go(&[0x6F, 0x6E]); // Rambowsky, Flur 4
    p.ride(0);
    p.go(&[0x6E]); // Pforte
    p.act(USE, Thing::Item(0x15), Some(Thing::Spot(0x81)));
    p.rest(3);
    p.take(0x16);
    p.go(&[0x6E]);
    p.ride(3);
    p.go(&[0x6E, 0x6F]); // Brücke, Maschinenraum
    p.reach(
        32,
        "the Maschinenraum's scene leads back to the third floor",
    );
    assert!(p.has(0x11), "the Tonspule: {}", p.state());
    p.ride(2);
    p.go(&[0x6E]); // Tontechnik
    p.answer(&[0x15, 0x17], &[0x16], &[]);
    p.talk(0x96);
    p.rest(5);
    assert!(p.has(0x20), "the Nachricht: {}", p.state());
    p.plain();
    p.go(&[0x6E]);
    p.ride(1);
    p.go(&[0x6E]); // Technik
    p.answer(&[0x13], &[], &[]);
    p.talk(0x91);
    p.reach(15, "the Technik again");
    assert_eq!(p.byte(0x2BB6), 4, "{}", p.state());
    p.answer(&[0x11, 0x10], &[], &[]);
    p.act(LOOK, Thing::Spot(0x40), None);
    assert!(
        p.has(3) && p.has(5),
        "the VIP-Karte and a key: {}",
        p.state()
    );
    p.plain();
}

/// The VIP-Lounge opened with the card; the Saboteur in the Bi-Fi room
/// leaves the Perücke, and in the Maske gives the key; the Perücke for the
/// Requisiteur's Leuchtfarbe; the VIP-Lounge's door and its Buch.
fn peruecke_and_the_key(p: &mut Player) {
    p.go(&[0x6E]);
    p.ride(0);
    // The card opens the door (rule `0035h`): it plays its animation 1,
    // and the machine lets go once it reports; then the door is gone
    // through.
    p.act(USE, Thing::Item(3), Some(Thing::Spot(0x78)));
    p.go(&[0x78]);
    p.reach(25, "the VIP-Lounge");
    assert_eq!(p.byte(0x2BA8), 1, "{}", p.state());
    p.go(&[0x78]);
    p.ride(5);
    p.go(&[0x6E]); // Bi-Fi
    p.reach(12, "the Bi-Fi room again");
    assert_eq!(p.byte(0x2BA8), 2, "{}", p.state());
    p.take(0x12);
    p.go(&[0x6E]);
    p.ride(2);
    p.go(&[0x6F]); // Maske
    p.answer(&[0x19], &[], &[]);
    for _ in 0..6 {
        p.talk(0x8D);
        if p.has(4) {
            break;
        }
    }
    assert!(p.has(4), "the Saboteur's key: {}", p.state());
    p.plain();
    p.go(&[0x6F]);
    p.ride(1);
    p.go(&[0x6F]); // Requisite
    p.answer(&[0x32, 0x31, 0x14], &[], &[]);
    p.act(USE, Thing::Item(0x12), Some(Thing::Spot(0x92)));
    p.reach(16, "the Requisite again");
    assert!(p.has(0x13), "the Leuchtfarbe: {}", p.state());
    p.plain();
    p.go(&[0x6F]);
    p.ride(0);
    p.go(&[0x78]); // VIP-Lounge
    p.act(USE, Thing::Item(4), Some(Thing::Spot(0x79)));
    assert_eq!(p.byte(0x2BC9), 1, "the door unlocked: {}", p.state());
    p.rest(3);
    p.take(0x1C);
    p.act(LOOK, Thing::Item(0x1C), None);
    assert_eq!(p.byte(0x2BAC), 1, "the manual read: {}", p.state());
}

/// Öliens's money, the 08-15's photo strip, the Vorzimmer's scissors;
/// the Ausweis made with the chewing gum and the photo.
fn geld_and_ausweis(p: &mut Player) {
    p.go(&[0x78]);
    p.ride(7);
    p.go(&[0x6E]); // Öliens
    p.act(USE, Thing::Spot(0xF0), None);
    assert_eq!(p.byte(0x2BD3), 1, "the Knopf: {}", p.state());
    p.go(&[0x7B]); // Halle
    p.act(USE, Thing::Item(5), Some(Thing::Spot(0xA0)));
    assert_eq!(p.byte(0x2BCC), 2, "the Hebel: {}", p.state());
    p.answer(&[0x25], &[], &[]);
    p.go(&[0x7B]); // Öliens
    p.rest(3);
    assert!(p.has(0x26), "the Geld: {}", p.state());
    p.plain();
    p.go(&[0x6E, 0x6F]); // Flur 7, 08-15
    p.answer(&[0x27], &[], &[]);
    p.act(USE, Thing::Item(0x26), Some(Thing::Spot(0xA2)));
    p.rest(3);
    assert!(p.has(0x0E), "the Fotostreifen: {}", p.state());
    p.plain();
    p.go(&[0x6F]);
    p.ride(0);
    p.go(&[0x6F]); // Vorzimmer
    p.act(USE, Thing::Item(0x0E), Some(Thing::Spot(0x86)));
    assert!(p.has(0x0B), "the Passfoto: {}", p.state());
    p.act(USE, Thing::Item(9), None);
    assert!(p.has(0x0A), "the chewed gum: {}", p.state());
    p.act(USE, Thing::Item(0x0A), Some(Thing::Item(6)));
    assert!(p.has(7), "the gum on the Ausweis: {}", p.state());
    p.act(USE, Thing::Item(0x0B), Some(Thing::Item(7)));
    assert!(p.has(8), "the Ausweis with its photo: {}", p.state());
}

/// The SFX's Pyrotool for Jones 1, and Jones 2 painted with the
/// Leuchtfarbe; the Vorführraum's crate moved with the Wagenheber.
fn pyrotool_and_paint(p: &mut Player) {
    p.go(&[0x6F]);
    p.ride(2);
    p.go(&[0x70]); // SFX
    p.talk(0x97);
    assert_eq!(p.byte(0x2BC3), 1, "the Ausweis shown: {}", p.state());
    p.take(0x1D);
    p.go(&[0x70]);
    p.ride(6);
    p.go(&[0x6F]); // Jones 1
    for _ in 0..80 {
        p.rest(1);
        if p.byte(0x2BAC) == 2 && p.here() == 8 && !p.busy() {
            break;
        }
    }
    assert_eq!(p.byte(0x2BAC), 2, "the Pyrotool used: {}", p.state());
    p.rest(5);
    p.go(&[0x6E]); // Jones 2
    p.act(USE, Thing::Item(0x13), Some(Thing::Spot(0x80)));
    p.reach(9, "Jones 2 again");
    assert!(p.has(0x14), "the Eimer: {}", p.state());
    p.go(&[0x6E, 0x6F]); // Jones 1, Flur 6
    p.ride(0);
    p.go(&[0x78, 0x79]); // VIP-Lounge, Vorführraum
    p.act(USE, Thing::Item(0x1F), Some(Thing::Spot(0x9F)));
    assert_eq!(p.byte(0x2BCB), 1, "the crate moved: {}", p.state());
}

/// The labyrinth's ways from location 69 to the Skelett, 79.
const TO_THE_SKELETT: [u16; 11] = [
    0x70, 0x6E, 0x6F, 0x6E, 0x70, 0x6E, 0x70, 0x6E, 0x71, 0x6E, 0x6F,
];
/// And back.
const FROM_THE_SKELETT: [u16; 12] = [
    0x6F, 0x6E, 0x71, 0x6E, 0x70, 0x6E, 0x70, 0x6E, 0x6F, 0x6E, 0x70, 0x71,
];
/// From location 69 to location 46.
const TO_46: [u16; 10] = [0x6E, 0x6F, 0x70, 0x6F, 0x70, 0x6E, 0x70, 0x6F, 0x6E, 0x70];

/// Through the Luke to the labyrinth's first place, 69.
fn into_the_labyrinth(p: &mut Player) {
    p.go(&[0x7A]);
    p.reach(69, "the labyrinth");
}

/// The Skelett's Stadtplan; Rambowsky's Quelle and Flämmchen heat the
/// Eimer, and the hot Eimer makes the Stadtplan the Labyrinth-Karte.
fn the_labyrinth_s_map(p: &mut Player) {
    into_the_labyrinth(p);
    p.go(&TO_THE_SKELETT);
    p.rest(3);
    assert!(p.has(0x1A), "the Stadtplan: {}", p.state());
    p.go(&FROM_THE_SKELETT);
    p.reach(26, "back in the Vorführraum");
    p.go(&[0x79, 0x78]); // VIP-Lounge, Flur EG
    p.ride(4);
    p.go(&[0x6E]); // Rambowsky
    p.act(USE, Thing::Item(0x14), Some(Thing::Spot(0x87)));
    assert!(p.has(0x2C), "the Eimer filled: {}", p.state());
    p.act(USE, Thing::Item(0x2C), Some(Thing::Spot(0xF0)));
    assert!(p.has(0x2E), "the Eimer heated: {}", p.state());
    p.act(USE, Thing::Item(0x2E), Some(Thing::Item(0x1A)));
    assert!(p.has(0x1B), "the Labyrinth-Karte: {}", p.state());
}

/// The way to location 46, its lock opened with the map; Schlönstadt
/// freed in the Gefängnis; the way out to the Öliens-Halle, and the Bi-Fi
/// room's last scene, which ends the game.
fn schloenstadt_and_the_end(p: &mut Player) {
    p.go(&[0x6E]);
    p.ride(0);
    p.go(&[0x78, 0x79]); // VIP-Lounge, Vorführraum
    into_the_labyrinth(p);
    p.go(&TO_46);
    assert_eq!(p.here(), 46, "{}", p.state());
    p.act(USE, Thing::Spot(0xF9), None);
    assert_eq!(p.byte(0x2BD6), 1, "the lock opened: {}", p.state());
    // Opened, the Eisengitter plays its animation 1 and the machine lets
    // go once it reports (rule `0005h`); then it is gone through.
    p.act(OPEN, Thing::Spot(0x73), None);
    p.go(&[0x73]);
    p.reach(41, "the Gefängnis");
    p.rest(5);
    assert_eq!(p.byte(0x2BD0), 1, "Schlönstadt freed: {}", p.state());
    p.go(&[0x72]);
    p.reach(28, "the way out to the Öliens-Halle");
    p.go(&[0x7B, 0x6E]); // Öliens, Flur 7
    p.ride(5);
    p.go(&[0x6E]); // Bi-Fi
    for _ in 0..60 {
        p.settle();
        p.tick(200);
        if p.g.game.ending() {
            break;
        }
    }
}

#[test]
#[ignore = "minutes of game time: `just playthrough`"]
fn bi_fi_2_can_be_played_to_its_end() {
    let Some(dir) = motionvm_parsec_testutil::gamedata_bifi2() else {
        eprintln!("skipping: no Bi-Fi 2 gamedata directory");
        return;
    };
    let mut g = BiFi2::open(&dir).expect("open Bi-Fi 2");
    g.set_saves(&motionvm_parsec_testutil::saves_dir("bifi2-playthrough"))
        .expect("saves");
    g.start_past_title(Start::New).expect("start");
    for _ in 0..2_000 {
        if g.game.inner.options_up() && !g.game.inner.options_fading() {
            break;
        }
        g.tick();
    }
    g.key(27);
    g.tick();
    assert!(!g.game.inner.options_up(), "Escape closes the options");
    let mut p = Player {
        g,
        answers: Answers::default(),
        unbuilt: Vec::new(),
        leaving: None,
        left: false,
    };
    ground_floor_and_clip(&mut p);
    course_to_the_costume(&mut p);
    tonspule_brille_and_keys(&mut p);
    peruecke_and_the_key(&mut p);
    geld_and_ausweis(&mut p);
    pyrotool_and_paint(&mut p);
    the_labyrinth_s_map(&mut p);
    schloenstadt_and_the_end(&mut p);
    assert!(p.unbuilt.is_empty(), "{:#?}", p.unbuilt);
    let a = p.a();
    assert!(
        p.g.game.ending(),
        "the main loop ended, in location {}",
        a.location()
    );
    assert_eq!(
        Api::word(a, super::logic::WORDS.location),
        0xFFFF,
        "on a location of −1, not by quitting"
    );
    assert!(!p.has(0x27), "the Drehbuch handed over");
    // The end's sequence rolls its credits until an event; then it fades
    // out and the program is over.
    p.tick(1000);
    assert!(p.g.game.ending() && !p.g.game.over, "the credits roll");
    p.g.click(true);
    let over = (0..2000).any(|_| !p.g.tick());
    assert!(over, "the game is over");
}
