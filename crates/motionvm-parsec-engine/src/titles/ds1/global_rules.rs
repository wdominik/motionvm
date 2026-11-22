//! *Dunkle Schatten*'s global rules: the handlers the table at
//! `2145h:0000h` (image `0x21450`) names, which the rule search asks after
//! a location's own (`13D0h:009Ch`).
//!
//! Most of them live in the segment `1586h` beside the location code and
//! are game code like it: they choose a verb, a scene, an item. Those are
//! re-expressed here against [`Api`], so that they run against the same
//! recording stand-in as the locations and are held to the original's
//! traces ([`api_rule`]). Four of them speak the text the chosen hotspot
//! carries, which [`Api`] does not reach; they are read here over the
//! runtime itself, in [`super::hotspot_rules`], and so is the eighth
//! verb's rule, which reads the object's record and which only this
//! program has. The rest are the generation's own machinery — the walk
//! verb's rule, the seventh verb's door, the options' gate — and are the
//! runtime's methods; this module only says under which address this
//! program links them ([`rule`]).

use super::hotspot_rules as hotspot;
use crate::num;
use crate::titles::parsec1994::{Adventure, Api, OBJECT_LIVE};

/// `[288Dh]`, the mode block's verb.
const VERB: u16 = 0x288D;
/// `[2891h]`, the first object.
const OBJECT: u16 = 0x2891;
/// `[2893h]`, the first object's class.
const OBJECT_CLASS: u16 = 0x2893;
/// `[2895h]`, the second object.
const SECOND: u16 = 0x2895;
/// `[2897h]`, the second object's class.
const SECOND_CLASS: u16 = 0x2897;
/// `[270Eh]`, the story byte the chapter turn sets (`1586:06B1`).
const TURNED: u16 = 0x270E;

/// The segment of the global rule handlers.
const RULES: u16 = 0x1586;

/// A global rule read against [`Api`], by its handler's far address;
/// `None` for one that is not among them. `flags` is the phase the search
/// runs with, which none of them reads.
pub(crate) fn api_rule(api: &mut dyn Api, far: (u16, u16), _flags: u16) -> Option<i16> {
    if far.0 != RULES {
        return None;
    }
    Some(match far.1 {
        // 0x15933: `Gib … an`. A carried second object rolls one of places
        // 0 to 2 (0x15948); a class-2 one is named in slot 9 (0x1596C) and
        // place 14 plays; anything else place 5.
        0x00D3 => {
            let n = if api.inv_has(api.word(SECOND)) != 0 {
                !num::signed(api.random(0, 2))
            } else if api.word(SECOND_CLASS) == 2 {
                let rec = api.obj_find(api.word(SECOND)); // 0x15960
                api.slot_set_object(9, 0, rec);
                !14
            } else {
                !5
            };
            api.scene_play(n); // 0x1597A
            0x0A
        }
        // 0x15B23: `Benutze … mit` — place 15 when both are class 2, 12
        // when the second is, 8 otherwise.
        0x02C3 => {
            let n = if api.word(SECOND_CLASS) != 2 {
                !8
            } else if api.word(OBJECT_CLASS) == 2 {
                !15
            } else {
                !12
            };
            api.scene_play(n); // 0x15B3E
            0x0A
        }
        // The hover rules, phase 1 (0x15BB6, 0x15C2E, 0x15C6E, 0x15C7C,
        // 0x15C8A): the verb a class of object means when none is picked.
        0x0356 => verb(api, 1, 1),
        0x03CE | 0x040E => verb(api, 0x0A, 1),
        0x041C => verb(api, 4, 1),
        0x042A => verb(api, 2, 1),
        // The phase-2 rules of `Benutze` on the classes 5, 2 and 4
        // (0x15C3C, 0x15C4A, 0x15E80): the verb becomes the walk verb — or
        // 7 for an object not live, 8 for a live class 5.
        0x03DC => verb(api, 0x0A, 5),
        0x03EA => {
            let live = api.obj_state(api.word(OBJECT)) != 0; // 0x15C51
            verb(api, if live { 0x0A } else { 7 }, 5)
        }
        0x0620 => {
            let live = api.obj_state(api.word(OBJECT)) != 0; // 0x15E87
            verb(api, if live { 8 } else { 7 }, 5)
        }
        // 0x15D03: the third button's rule, `Gib` for something carried,
        // `Nimm` otherwise.
        0x04A3 => {
            let carried = api.inv_has(api.word(OBJECT)) != 0; // 0x15D0A
            verb(api, if carried { 9 } else { 0x0C }, 5)
        }
        // 0x15D27: items 9 and 2 given up for item 3.
        0x04C7 => {
            api.inv_take(9);
            api.inv_take(2);
            api.inv_give(3);
            1
        }
        // 0x15D47: place 22.
        0x04E7 => scene(api, !22),
        // 0x15D57: place 33 once the chapter has turned, 21 before.
        0x04F7 => {
            let n = if api.byte(TURNED) != 0 { !33 } else { !21 };
            scene(api, n)
        }
        // 0x15D72: place 25 for a carried second object, 24 for a class-2
        // one, 23 otherwise.
        0x0512 => {
            let n = if api.inv_has(api.word(SECOND)) != 0 {
                !25
            } else if api.word(SECOND_CLASS) == 2 {
                !24
            } else {
                !23
            };
            scene(api, n)
        }
        // 0x15D9F to 0x15DFE: one place each, 27 to 32.
        0x053F => scene(api, !27),
        0x054F => scene(api, !28),
        0x055F => scene(api, !29),
        0x056F => scene(api, !30),
        0x057F => scene(api, !31),
        0x058F => scene(api, !32),
        _ => return None,
    })
}

/// `mov word [288Dh], verb ; mov ax, answer`.
fn verb(api: &mut dyn Api, verb: u16, answer: i16) -> i16 {
    api.set_word(VERB, verb);
    answer
}

/// One scene, then the answer `0Ah`.
fn scene(api: &mut dyn Api, n: i16) -> i16 {
    api.scene_play(n);
    0x0A
}

/// `1586h:0438h`: the eighth verb's rule — verb 8, any object, flag 4
/// (`0x21564`) — the seventh's mirror, which `Benutze`'s phase-2 rule
/// on class 5 hands an object that is live (`0x15E83`–`0x15E9D`).
///
/// The object's record (`0BB6h:052Bh`, `0x15CA0`) of kind 4 is a door
/// (`0x15CB4`): it plays its animation 0 (`0883h:1F82h`, `0x15CC1`),
/// `[28C1h] = 3` waits for the animation's code 3 (`0x15CC9`),
/// and the answer is 12 (`0x15CCF`). Anything else has bit 0 of `+2`
/// cleared, no longer live (`0x15CE5`), between two of the drawer's
/// marks (`0883h:000Fh`, `0x15CDA`, `0x15CEE`; this engine draws every
/// frame whole), asks for sample 13 (`0EB9h:1949h`, `0x15CF8`), and the
/// answer is 1.
///
/// An object without a record reads its kind from a null pointer in the
/// original (`0x15CAF`); this engine answers 1 for it, as the seventh's
/// does.
fn eighth_verb(adv: &mut Adventure) -> i16 {
    let Ok(id) = u8::try_from(adv.object1) else {
        return 1;
    };
    let Some(i) = adv.object_index(id) else {
        return 1;
    };
    if adv.objects[i].kind == 4 {
        adv.object_anim(id, 0);
        adv.anim_code_wait(3);
        12
    } else {
        adv.objects[i].flags &= !OBJECT_LIVE;
        adv.effect_1949(13);
        1
    }
}

/// Every global rule of this program, by its handler's far address, for
/// the runtime; `None` for one this engine does not read.
pub(crate) fn rule(adv: &mut Adventure, far: (u16, u16), flags: u16) -> Option<i16> {
    if let Some(answer) = api_rule(adv, far, flags) {
        return Some(answer);
    }
    Some(match far {
        (RULES, 0x0125) => hotspot::look(adv),
        (RULES, 0x017C) => hotspot::talk(adv),
        (RULES, 0x01CC) => hotspot::take(adv),
        (RULES, 0x0273) => hotspot::use_(adv),
        // 0x15B49: the walk verb's rule.
        (RULES, 0x02E9) => adv.walk_rule(),
        // 0x15BC4: the seventh verb's rule, a door.
        (RULES, 0x0364) => adv.door_rule(),
        // 0x15C98: the eighth verb's rule.
        (RULES, 0x0438) => eighth_verb(adv),
        // `0EB9h:04BDh` (0x0F04D): the sixth verb's rule installs the
        // options' gate and waits for the figure.
        (0x0EB9, 0x04BD) => adv.options_rule(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::titles::parsec1994::Event;
    use crate::titles::subject::Subject;

    /// Every row of the global table names a handler this module reads:
    /// asked under each row's own phase, every handler answers and none is
    /// reported unread. The table is the program's own, read by shape.
    #[test]
    fn every_row_of_the_global_table_finds_its_handler() {
        let Some(mut adv) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        let table = adv.global_rules.clone();
        assert_eq!(table.len(), 33, "rows in the table at 2145h:0000h");
        for row in &table {
            adv.events.clear();
            assert!(
                rule(&mut adv, row.handler, row.phase).is_some(),
                "{:04X}h:{:04X}h is not read",
                row.handler.0,
                row.handler.1
            );
            assert!(
                !adv.events
                    .iter()
                    .any(|e| matches!(e, Event::UnreadRule { .. })),
                "{:04X}h:{:04X}h reports itself unread",
                row.handler.0,
                row.handler.1
            );
        }
    }

    /// The game started, in the Zimmer, nothing chosen.
    fn zimmer() -> Option<Adventure> {
        let mut adv = crate::titles::ds1::adventure_for_test()?;
        assert_eq!(adv.here_word, 2, "the game starts in the Zimmer");
        adv.hover_spot = None;
        adv.events.clear();
        Some(adv)
    }

    /// The scenes asked for since the events were last cleared.
    fn scenes(adv: &mut Adventure) -> Vec<i16> {
        let s = adv
            .events
            .iter()
            .filter_map(|e| match e {
                Event::SceneBound { scene, .. } => Some(*scene),
                _ => None,
            })
            .collect();
        adv.events.clear();
        s
    }

    /// The runtime's answer to the rule at `1586h:off`, asked in phase 4.
    fn ask(adv: &mut Adventure, off: u16) -> i16 {
        rule(adv, (RULES, off), 4).expect("a rule of the table")
    }

    /// An item the player carries, and one not carried.
    fn items(adv: &Adventure) -> (u16, u16) {
        let carried = u16::from(adv.items[0]);
        let not = (1..0x17).find(|&id| !adv.carried(num::low8(id))).unwrap();
        (carried, not)
    }

    /// The mode block's words the rules read through [`Api`] are the
    /// runtime's own fields, both ways.
    #[test]
    fn the_mode_block_s_words_are_the_runtime_s_fields() {
        let Some(mut adv) = zimmer() else { return };
        adv.object2 = 0x0102;
        adv.second_object = (0x0304, 0x0506);
        assert_eq!(Api::word(&adv, OBJECT_CLASS), 0x0102);
        assert_eq!(Api::word(&adv, SECOND), 0x0304);
        assert_eq!(Api::word(&adv, SECOND_CLASS), 0x0506);
        assert_eq!(Api::byte(&adv, SECOND_CLASS.wrapping_add(1)), 0x05);
        Api::set_word(&mut adv, OBJECT_CLASS, 2);
        Api::set_word(&mut adv, SECOND, 200);
        Api::set_word(&mut adv, SECOND_CLASS, 3);
        assert_eq!((adv.object2, adv.second_object), (2, (200, 3)));
        Api::set_byte(&mut adv, SECOND, 0xC9);
        assert_eq!(adv.second_object.0, 0xC9);
        Api::set_word(&mut adv, VERB, 0x0A);
        assert_eq!(adv.verb, 0x0A);
    }

    /// `Gib … an`: a carried second object rolls one of places 0 to 2; a
    /// class-2 one is named in slot 9 — or, with no such object in the
    /// room, slot 9 holds nothing — and place 14 plays; else place 5.
    #[test]
    fn giving_rolls_names_the_second_object_or_refuses() {
        let Some(mut adv) = zimmer() else { return };
        let (carried, not) = items(&adv);
        adv.second_object = (carried, 0);
        assert_eq!(ask(&mut adv, 0x00D3), 0x0A);
        assert!(matches!(scenes(&mut adv).as_slice(), [n] if (0..=2).contains(&!n)));

        adv.second_object = (200, 2);
        assert_eq!(ask(&mut adv, 0x00D3), 0x0A);
        assert_eq!(scenes(&mut adv), [!14]);
        let slot = adv.subjects.get(9).unwrap();
        assert_eq!((slot.code, slot.subject), (0x1000, Subject::Object(200)));

        // No object 250 in the Zimmer: the lookup answers null, and the
        // setter stamps 3000h.
        adv.second_object = (250, 2);
        assert_eq!(ask(&mut adv, 0x00D3), 0x0A);
        assert_eq!(scenes(&mut adv), [!14]);
        let slot = adv.subjects.get(9).unwrap();
        assert_eq!((slot.code, slot.subject), (0x3000, Subject::Nothing));

        adv.second_object = (not, 1);
        assert_eq!(ask(&mut adv, 0x00D3), 0x0A);
        assert_eq!(scenes(&mut adv), [!5]);
    }

    /// `Benutze … mit` by the two classes, and the rule of the pair.
    #[test]
    fn using_with_and_the_pair_rule_choose_their_place_by_the_second_object() {
        let Some(mut adv) = zimmer() else { return };
        let (carried, not) = items(&adv);
        for (classes, want) in [((2, 2), !15), ((1, 2), !12), ((2, 1), !8)] {
            adv.object2 = classes.0;
            adv.second_object = (not, classes.1);
            assert_eq!(ask(&mut adv, 0x02C3), 0x0A);
            assert_eq!(scenes(&mut adv), [want]);
        }
        for (second, want) in [((carried, 2), !25), ((not, 2), !24), ((not, 1), !23)] {
            adv.second_object = second;
            assert_eq!(ask(&mut adv, 0x0512), 0x0A);
            assert_eq!(scenes(&mut adv), [want]);
        }
    }

    /// The hover rules, `Benutze`'s phase 2 and the third button: each
    /// sets the verb and answers 1 or 5.
    #[test]
    fn the_verb_rules_set_the_verb_by_the_object() {
        let Some(mut adv) = zimmer() else { return };
        let (carried, not) = items(&adv);
        // Object 1, the toolbox, is live (kind 0); 203 is of kind 1 and
        // not live.
        for (off, object, verb, answer) in [
            (0x0356, 1, 1, 1),
            (0x03CE, 1, 0x0A, 1),
            (0x040E, 1, 0x0A, 1),
            (0x041C, 1, 4, 1),
            (0x042A, 1, 2, 1),
            (0x03DC, 1, 0x0A, 5),
            (0x03EA, 1, 0x0A, 5),
            (0x03EA, 203, 7, 5),
            (0x0620, 1, 8, 5),
            (0x0620, 203, 7, 5),
            (0x04A3, carried, 9, 5),
            (0x04A3, not, 0x0C, 5),
        ] {
            adv.verb = 0;
            adv.object1 = object;
            assert_eq!(ask(&mut adv, off), answer, "{off:#06x}");
            assert_eq!(adv.verb, verb, "{off:#06x} on {object}");
        }
    }

    /// Items 9 and 2 for item 3, and the rules that are one place each.
    #[test]
    fn the_item_and_place_rules() {
        let Some(mut adv) = zimmer() else { return };
        adv.give_item(9);
        adv.give_item(2);
        assert_eq!(ask(&mut adv, 0x04C7), 1);
        assert!(!adv.carried(9) && !adv.carried(2) && adv.carried(3));
        assert_eq!(ask(&mut adv, 0x04E7), 0x0A);
        assert_eq!(scenes(&mut adv), [!22]);
        Api::set_byte(&mut adv, TURNED, 0);
        assert_eq!(ask(&mut adv, 0x04F7), 0x0A);
        assert_eq!(scenes(&mut adv), [!21]);
        Api::set_byte(&mut adv, TURNED, 1);
        assert_eq!(ask(&mut adv, 0x04F7), 0x0A);
        assert_eq!(scenes(&mut adv), [!33]);
        for (off, n) in [
            (0x053F, 27),
            (0x054F, 28),
            (0x055F, 29),
            (0x056F, 30),
            (0x057F, 31),
            (0x058F, 32),
        ] {
            assert_eq!(ask(&mut adv, off), 0x0A);
            assert_eq!(scenes(&mut adv), [!n]);
        }
    }

    /// The eighth verb's rule in the Zimmer: a door plays its animation 0,
    /// waits for code 3 and answers 12; anything else is made not live,
    /// asks for sample 13 and answers 1 (`0x15C98`–`0x15CFE`).
    #[test]
    fn the_eighth_verb_s_rule_opens_a_door_or_plays_sample_13() {
        let Some(mut a) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        while a.effects.busy() {
            a.interrupt();
            a.effects_tick();
        }
        let door = a.objects.iter().find(|o| o.kind == 4).map(|o| o.id);
        let other = a
            .objects
            .iter()
            .find(|o| o.kind != 4 && o.id != 0)
            .map(|o| o.id);
        let (Some(door), Some(other)) = (door, other) else {
            panic!("the Zimmer has a door and an object besides");
        };
        a.events.clear();
        a.object1 = u16::from(door);
        assert_eq!(ask(&mut a, 0x0438), 12);
        assert_eq!(a.word_28c1, 3);
        assert!(a.events.contains(&Event::ObjectAnim {
            object: door,
            anim: 0
        }));
        // The door's animation 0 reaches its code 3, and the wait is over
        // (`0x109A8`–`0x109AE`).
        let mut frames = 0;
        while a.word_28c1 != 0 {
            a.interrupt();
            a.frame_pass();
            frames += 1;
            assert!(frames < 2000, "the door's animation never reports code 3");
        }
        assert!(
            a.events
                .iter()
                .any(|e| matches!(e, Event::AnimCode { code: 3, .. }))
        );
        a.events.clear();
        a.object1 = u16::from(other);
        let i = a.object_index(other).expect("the object");
        a.objects[i].flags |= OBJECT_LIVE;
        assert_eq!(ask(&mut a, 0x0438), 1);
        assert_eq!(a.objects[i].flags & OBJECT_LIVE, 0);
        assert!(a.events.contains(&Event::Sample(13)));
        assert!(
            !a.events
                .iter()
                .any(|e| matches!(e, Event::UnreadRule { .. })),
            "the rule is read"
        );
    }

    /// A handler that is not in the table is not answered here, so the
    /// rule search can say so.
    #[test]
    fn an_address_outside_the_table_is_not_answered() {
        let Some(mut adv) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        assert_eq!(rule(&mut adv, (RULES, 0x0001), 4), None);
        assert_eq!(rule(&mut adv, (0x0EB9, 0x04BE), 4), None);
    }
}
