//! The four global rules of *Dunkle Schatten* that speak a hotspot's own
//! text: `Betrachte`, `Sprich mit`, `Nimm` and `Benutze` on anything.
//!
//! Each first asks the hotspot the object was chosen through for its line
//! (`[28B3h]`, the hotspot's `+16h + 4k`) and plays it with `0CFDh:18C2h`;
//! only without one does it fall back on a scene. A hotspot is not
//! reachable through [`Api`](crate::titles::parsec1994::Api), so these are
//! read over the runtime itself, as [`super::hook`] is; what is the game's
//! is the choice of the text slot, the scenes and the item bound.

use crate::num;
use crate::titles::parsec1994::{Adventure, Event, OBJECT_LIVE};

/// The first id past the items, `17h` (`0x15A5D`, `cmp word [2891h], 17h`).
const ITEMS_END: u16 = 0x17;

/// `1586h:0125h`, `Betrachte` on anything: the hotspot's `texts[3]`
/// (`0x15996`, `+22h`), else bank 2's place for an item
/// (`~(id + 4000h)`, `0x159C7`), else bank 999's place 16.
pub(crate) fn look(adv: &mut Adventure) -> i16 {
    if let Some(text) = adv.own_text(3) {
        adv.hotspot_text(&text);
    } else if (1..ITEMS_END).contains(&adv.object1) {
        let n = !(num::signed(adv.object1) + 0x4000);
        adv.scene_load(n);
    } else {
        adv.scene_load(!16);
    }
    10
}

/// `1586h:017Ch`, `Sprich mit`: `texts[4]` (`+26h`), else place 4 for
/// something carried and 3 for anything else.
pub(crate) fn talk(adv: &mut Adventure) -> i16 {
    if let Some(text) = adv.own_text(4) {
        adv.hotspot_text(&text);
    } else if adv.carried(num::low8(adv.object1)) {
        adv.scene_load(!4);
    } else {
        adv.scene_load(!3);
    }
    10
}

/// `1586h:01CCh`, `Nimm`: `texts[5]` (`+2Ah`); else an item that is
/// there and live goes out of the room — its live bit cleared (`0x15A8F`),
/// its hotspot following it (`14DDh:074Bh`, `0x15A98`) — and into the
/// player's hands (`0x15AB2`); else one of places 9, 10 and 11, rolled.
pub(crate) fn take(adv: &mut Adventure) -> i16 {
    if let Some(text) = adv.own_text(5) {
        adv.hotspot_text(&text);
        return 10;
    }
    if adv.object1 < ITEMS_END
        && let Some(i) = adv.live_object(num::low8(adv.object1))
    {
        adv.objects[i].flags &= !OBJECT_LIVE;
        let id = num::low8(adv.object1);
        adv.events.push(Event::ObjectHidden(id));
        adv.hotspot_follows(i);
        adv.give_item(id);
        return 10;
    }
    let n = !num::signed(adv.roll(9, 11));
    adv.scene_load(n);
    10
}

/// `1586h:0273h`, `Benutze`: place 7 for something carried; else
/// `texts[2]` (`+1Eh`), else place 6.
pub(crate) fn use_(adv: &mut Adventure) -> i16 {
    if adv.carried(num::low8(adv.object1)) {
        adv.scene_load(!7);
    } else if let Some(text) = adv.own_text(2) {
        adv.hotspot_text(&text);
    } else {
        adv.scene_load(!6);
    }
    10
}

#[cfg(test)]
mod tests {
    use super::*;

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
