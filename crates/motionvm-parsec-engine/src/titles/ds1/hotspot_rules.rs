//! The four global rules of *Dunkle Schatten* that speak a hotspot's own
//! text: `Betrachte`, `Sprich mit`, `Nimm` and `Benutze` on anything.
//!
//! Each first asks the hotspot the object was chosen through for its line
//! (`[28B3h]`, the hotspot's `+16h + 4k`) and plays it with `0CFDh:18C2h`;
//! only without one does it fall back on a scene. What is the game's is
//! the choice of the text slot, the scenes and the item bound; they run
//! against [`Api`] like every other rule of the program.

use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// `[2891h]`, the first object.
const OBJECT: u16 = 0x2891;

/// The first id past the items, `17h` (`0x15A5D`, `cmp word [2891h], 17h`).
const ITEMS_END: u16 = 0x17;

/// The chosen hotspot's line in slot `k`, played when there is one
/// (`les bx, [28B3h]`, the far pointer at `+16h + 4k` tested for null, then
/// `0CFDh:18C2h`).
fn own_line(api: &mut dyn Api, k: u16) -> bool {
    let text = api.hotspot_own_text(k);
    if text == Far::NULL {
        return false;
    }
    api.say_text(text);
    true
}

/// `1586h:0125h`, `Betrachte` on anything: the hotspot's line 3 (`+22h`,
/// `0x15996`); else for an item, `0 < id < 17h`, bank 2's place
/// `~(id + 4000h)` (`0x159C7`); else place 16 (`push -11h`, `0x159CF`).
pub(crate) fn look(api: &mut dyn Api) -> i16 {
    if !own_line(api, 3) {
        let id = api.word(OBJECT);
        let n = if (1..ITEMS_END).contains(&id) {
            !(num::signed(id).wrapping_add(0x4000))
        } else {
            !16
        };
        api.scene_play(n); // 0x159D1
    }
    0x0A
}

/// `1586h:017Ch`, `Sprich mit`: line 4 (`+26h`, `0x159ED`); else place 4
/// for something carried (`0C3Fh:0B06h`, `0x15A11`) and 3 for anything
/// else.
pub(crate) fn talk(api: &mut dyn Api) -> i16 {
    if !own_line(api, 4) {
        let n = if api.inv_has(api.word(OBJECT)) != 0 {
            !4
        } else {
            !3
        };
        api.scene_play(n); // 0x15A21
    }
    0x0A
}

/// `1586h:01CCh`, `Nimm`: line 5 (`+2Ah`, `0x15A3D`); else an item below
/// `17h` that has a record and is live (`0BB6h:052Bh`, `0x15A68`; bit 0 of
/// `+2`, `0x15A7B`) leaves the room — the live bit cleared between two of
/// the drawer's marks (`0883h:000Fh`, `0x15A84`, `0x15AA6`), its hotspot
/// following it (`14DDh:074Bh`, `0x15A98`) — and goes into the player's
/// hands (`0C3Fh:095Ch`, `0x15AB2`); else one of places 9 to 11, rolled
/// (`1EB5h:0028h`, `0x15ABD`).
pub(crate) fn take(api: &mut dyn Api) -> i16 {
    if own_line(api, 5) {
        return 0x0A;
    }
    let id = api.word(OBJECT);
    if num::signed(id) < num::signed(ITEMS_END) {
        let rec = api.obj_find(id);
        let flags = Far {
            off: rec.off.wrapping_add(2),
            seg: rec.seg,
        };
        if rec != Far::NULL && api.far_byte(flags) & 1 != 0 {
            api.obj_invalidate(rec);
            let b = api.far_byte(flags);
            api.set_far_byte(flags, b & 0xFE);
            api.hotspot_follow(rec);
            api.obj_invalidate(rec);
            api.inv_give(id);
            return 0x0A;
        }
    }
    let n = !num::signed(api.random(9, 0x0B));
    api.scene_play(n); // 0x15AC8
    0x0A
}

/// `1586h:0273h`, `Benutze`: place 7 for something carried (`0C3Fh:0B06h`,
/// `0x15ADB`); else line 2 (`+1Eh`, `0x15AF6`), else place 6.
pub(crate) fn use_(api: &mut dyn Api) -> i16 {
    if api.inv_has(api.word(OBJECT)) != 0 {
        api.scene_play(!7);
    } else if !own_line(api, 2) {
        api.scene_play(!6); // 0x15B18
    }
    0x0A
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::titles::parsec1994::{Adventure, Event, OBJECT_LIVE};

    /// The game started and in its first location, the Zimmer, with no
    /// hotspot under the pointer.
    fn zimmer() -> Option<Adventure> {
        let mut adv = crate::titles::ds1::adventure_for_test()?;
        assert_eq!(adv.here_word, 2, "the game starts in the Zimmer");
        adv.hover_spot = None;
        adv.events.clear();
        Some(adv)
    }

    /// The scenes the rule asked for.
    fn scenes(adv: &Adventure) -> Vec<i16> {
        adv.events
            .iter()
            .filter_map(|e| match e {
                Event::SceneBound { scene, .. } => Some(*scene),
                _ => None,
            })
            .collect()
    }

    /// An item the player does not carry, for the rules' other branch.
    fn not_carried(adv: &Adventure) -> u16 {
        (1..ITEMS_END)
            .find(|&id| !adv.carried(num::low8(id)))
            .expect("an item not carried")
    }

    /// An item the player carries.
    fn carried(adv: &Adventure) -> u16 {
        u16::from(*adv.items.first().expect("something carried at the start"))
    }

    #[test]
    fn talking_to_something_carried_is_place_4_and_to_anything_else_place_3() {
        let Some(mut adv) = zimmer() else { return };
        adv.object1 = carried(&adv);
        assert_eq!(talk(&mut adv), 10);
        assert_eq!(scenes(&adv), [!4]);
        adv.events.clear();
        adv.object1 = not_carried(&adv);
        assert_eq!(talk(&mut adv), 10);
        assert_eq!(scenes(&adv), [!3]);
    }

    #[test]
    fn using_something_carried_is_place_7_and_anything_without_a_text_place_6() {
        let Some(mut adv) = zimmer() else { return };
        adv.object1 = carried(&adv);
        assert_eq!(use_(&mut adv), 10);
        assert_eq!(scenes(&adv), [!7]);
        adv.events.clear();
        adv.object1 = not_carried(&adv);
        assert_eq!(use_(&mut adv), 10);
        assert_eq!(scenes(&adv), [!6]);
    }

    /// The Zimmer's toolbox, item 1, lies there live under a hotspot of its
    /// own (mask `C001h`). Taken, it leaves the room and its hotspot with
    /// it, and it is carried.
    #[test]
    fn taking_the_toolbox_hides_it_and_its_hotspot_and_carries_it() {
        let Some(mut adv) = zimmer() else { return };
        assert!(!adv.carried(1), "the toolbox is not carried yet");
        let i = adv.live_object(1).expect("the toolbox lies in the Zimmer");
        let spot = adv
            .room()
            .and_then(|r| r.hotspots.spots.iter().position(|h| h.verb_mask == 0xC001))
            .expect("the toolbox's hotspot");
        assert!(adv.hotspots_rt[spot].active);
        adv.object1 = 1;
        assert_eq!(take(&mut adv), 10);
        assert_eq!(adv.objects[i].flags & OBJECT_LIVE, 0);
        assert!(adv.events.contains(&Event::ObjectHidden(1)));
        assert!(
            !adv.hotspots_rt[spot].active,
            "the hotspot follows the toolbox"
        );
        assert!(adv.carried(1));
        assert!(scenes(&adv).is_empty());
    }

    /// Something not lying there to be taken: one of places 9 to 11.
    #[test]
    fn taking_what_is_not_there_rolls_one_of_places_9_to_11() {
        let Some(mut adv) = zimmer() else { return };
        adv.object1 = carried(&adv);
        assert_eq!(take(&mut adv), 10);
        let s = scenes(&adv);
        assert!(
            matches!(s.as_slice(), [n] if (9..=11).contains(&!n)),
            "{s:?}"
        );
    }

    /// An item with no text of its own looks up its own place in bank 2,
    /// `~(id + 4000h)`; anything else past the items is place 16.
    #[test]
    fn looking_at_an_item_is_its_own_place_and_at_anything_else_place_16() {
        let Some(mut adv) = zimmer() else { return };
        adv.object1 = 5;
        assert_eq!(look(&mut adv), 10);
        assert_eq!(scenes(&adv), [!(5 + 0x4000)]);
        adv.events.clear();
        adv.object1 = 0xC8;
        assert_eq!(look(&mut adv), 10);
        assert_eq!(scenes(&adv), [!16]);
    }
}
