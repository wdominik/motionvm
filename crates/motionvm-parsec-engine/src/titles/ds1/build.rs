//! What *Dunkle Schatten*'s `XP01.EXE` hands the `parsec1994` runtime:
//! where this build keeps what each part of the runtime reads, one map per
//! part, with the instruction each value is read from beside it.
//!
//! The maps' types are the generation's (`titles/parsec1994/`); their
//! values are this program's and stand here, so that the generation names
//! no title.

use super::hook;
use super::logic::{self, SAVE_BUFFER, WALKER};
use super::xp01data::{self, ExeText};
use crate::num;
use crate::titles::parsec1994::program::{Build, Names, Reading, Sound};
use crate::titles::parsec1994::{
    Block, DsMap, GateMap, Kinds, MusicMap, OptionsMap, PointerMap, SaveMap,
};
use crate::titles::xp1994::Locations;

/// `XP01.EXE`, as the generation's runtime opens it.
pub(crate) static BUILD: Build = Build {
    program: ("XP01.EXE", 150_096),
    names: Names {
        rooms: "DSROOM.DAT",
        panel: "PANELS.DAT",
        pointers: "CMBMOU.DAT",
        player: "PLAY00.DAT",
        dialog: "DIALOG.DAT",
        music: "PSMP2L.DAT",
        // `DS:0A06h`, the drivers' RIB (`0x5B7B`).
        drivers: "PSMDRV.DAT",
    },
    kinds: KINDS,
    save: SAVE,
    options: OPTIONS,
    pointer: POINTER,
    music: MUSIC,
    // `0x5BD3`–`0x5C15`: six channels; `0x4E85`, the effects' volume
    // `3Ch` after the driver's default.
    sound: Sound {
        channels: 6,
        effect_volume: 0x3C,
    },
    logic: logic::TABLE,
    rule: logic::rule_handler,
    hook: hook::answer,
    location_hook: logic::location_hook,
    anim_hook: logic::anim_hook,
};

/// What the runtime takes from this title's own reading of its program.
pub(crate) fn reading(text: &ExeText, locations: &Locations) -> Reading {
    Reading {
        ds_map: ds_map(locations, text.inventory_at),
        templates: (text.template_row, xp01data::VERB_TEMPLATES),
        // The substitution row as the image ships it. Nothing is drawn
        // into it at the start: the roll at `0x1189` that fills entry 0
        // sits inside the Wohnzimmer's handler, for a wrong number on the
        // telephone.
        substitutions: text
            .substitutions
            .iter()
            .map(|s| motionvm_parsec_formats::to_cp437(s))
            .collect(),
        initial_items: text.initial_items.clone(),
        initial_scroll: text.initial_scroll,
        panel_curves: text.panel_curves,
        gray_ramp: text.gray_ramp.clone(),
        start: (
            num::low16(text.start_location),
            num::unsigned(text.start_entry),
        ),
    }
}

/// What the kinds of `XP01.EXE`'s location table mean.
pub(crate) const KINDS: Kinds = Kinds {
    // `0x04FA1`: the main loop walks on for kind 0; `0x053C7`, the walk
    // steps over it; `0x15919`, a door does not lead there.
    passage: 0,
    // `0x077F2`, the room loader; `0x083F6`, the unload that keeps the
    // slot; `0x1571B`, `14DDh:08F1h`; `0x0EC68`, the entry effect;
    // `0x0ED34`, the leave.
    forest: 3,
    // `0x15720`, `14DDh:08F1h`; `0x0EC6D`, the entry effect.
    reveal: 5,
    // `0x053CB`: the walk on steps over kind 4, location 60's.
    restore: 4,
    // `0x077F7`, `0x083FB`: `mov si, 14h`.
    forest_slot: 0x14,
};

/// Where `XP01.EXE` keeps its saved game and settings, and their files' shape.
pub(crate) const SAVE: SaveMap = SaveMap {
    // `push 8B1h` at the clear (`0x14821`), the write (`0x14AE1`) and
    // the size check (`0x14901`).
    len: 0x8B1,
    // `add dx, 14h` (`0x14A8A`, `0x14964`); `mov cx, 453h` (`0x14A93`)
    // and `mov si, 453h` (`0x1496D`): `8A6h` bytes, nine past the
    // file's end, into the segment's padding — which is zero, and which
    // nothing but this segment's own code reaches.
    sum: (0x14, 0x453),
    // `DSchSAV\0`: the four words `0x14A66`–`0x14A77` store and
    // `0x149A9`–`0x149BF` compare.
    sav_magic: [0x5344, 0x6863, 0x4153, 0x0056],
    // `DSchCFG\0`: `0x14B35`–`0x14B44` and `0x14881`–`0x14898`.
    cfg_magic: [0x5344, 0x6863, 0x4643, 0x0047],
    // `0x14CBF`: `mov word [1F4Ah], 3Ch`.
    restore_row: 0x3C,
    blocks: [
        // 0x14C2A: the story bytes.
        Block {
            buf: 0x014,
            ds: 0x26F6,
            out: 0x45,
            back: 0x45,
        },
        // 0x14C42: 61 object rows of 32 bytes.
        Block {
            buf: 0x059,
            ds: 0x1F56,
            out: 0x7A0,
            back: 0x7A0,
        },
        // 0x14C5B: the location stack.
        Block {
            buf: 0x7F9,
            ds: 0x1F4E,
            out: 8,
            back: 8,
        },
        // 0x14C73: the player's record.
        Block {
            buf: 0x801,
            ds: 0x1693,
            out: 0x30,
            back: 0x30,
        },
        // 0x14C8B: 7Fh bytes saved, of which 0x14D62 restores 23h.
        Block {
            buf: 0x831,
            ds: 0x15FE,
            out: 0x7F,
            back: 0x23,
        },
    ],
    stack: 0x1F4E,
    saved: 0x1742,
    loaded: 0x1744,
    version: 0x1718,
    cfg_dirty: 0x1ED2,
    mouse: 0x16F3,
    divisors: (0x03DC, 0x03DA),
    text: 0x167D,
    speed: 0x16A7,
    sav_name: 0x0A64,
    cfg_name: 0x0A56,
    msg_create: 0x07BC,
    msg_write: 0x07E4,
    msg_invalid: 0x171A,
};

/// Where `XP01.EXE` keeps what the game options read and write.
pub(crate) const OPTIONS: OptionsMap = OptionsMap {
    // 0x11A65: `les bx, [bx+14A4h]`.
    rows: 0x14A4,
    // 0x11A51: `push 0E60h`.
    title: 0x0E60,
    // 0x11EE0: `push 2 ; push 0C0h ; push ds ; push 14C8h`.
    colors: (0x14C8, 0xC0),
    // 0x125E5: `mov ax, [bx+14CEh]`, the row `8 · [146Ch]`.
    text_speeds: 0x14CE,
    // 0x1261D: `mov al, [bx+14E6h]`.
    walk_speeds: 0x14E6,
    // 0x12393, 0x123B7, 0x123DE, 0x123F0.
    confirms: [0x1424, 0x1430, 0x143C, 0x1448],
    // 0x12304: `xor word [212h], 1`.
    music: 0x0212,
    // 0x12379: `test word [1EDCh], 10h`.
    switches: 0x1EDC,
    save: SAVE,
};

/// Where `XP01.EXE` keeps the pointer's words and tables.
pub(crate) const POINTER: PointerMap = PointerMap {
    // `0590h:064Dh`'s table at `cs:06A1h`: `push 234h` (0x05F71),
    // `274h` (0x05F83), `2B4h` (0x05F8D), `2D4h` (0x05F97), each
    // copied to `push 334h` by `179Eh:0960h`.
    modes: [0x0234, 0x0274, 0x02B4, 0x02D4],
    slot: 0x0334,
    // `0x05EC4`: `mov [3B6h], ds ; mov word [3B4h], 254h`.
    regions: 0x0254,
    // `0x05DFA` (`[195Eh]+4`), `0x05E47` (`+14h`), `0x05E6E`
    // (`+18h`), `0x05E95` (`+8`).
    centered: [(0x0254, 1), (0x02B4, 5), (0x02D4, 6), (0x0374, 2)],
    // `0x188FD`: `mul word [1864h]`.
    seed: 0x1864,
    // `0x17ABE`: `mov ax, [3C2h] ; xchg [3C4h], ax`.
    timeout: 0x03C2,
    // `0x05EE0`: `push word [314h]`.
    interval: 0x0314,
    // `0x185B2`, `0x185BA`, `0x185F1`, `0x185F9`.
    limits: 0x030A,
    position: 0x03E6,
    // `0x05F5A`: `mov [1962h], ax`.
    mode: 0x1962,
    // `0x18A0F`, `0x18A01`, `0x188CB`.
    dirty: 0x02F6,
    state: 0x0316,
    drawn: 0x03EE,
    // `0x0DC8B`: `push 2139h ; push 2Ah ; call far 179Eh:0210h`.
    menu_region: 0x2139 * 16 + 0x2A,
};

/// Where `XP01.EXE` keeps the music's and the effects' words and tables.
pub(crate) const MUSIC: MusicMap = MusicMap {
    switch: 0x0212,
    loops: 0x0214,
    kept: 0x0216,
    room_song: 0x285D,
    slot_songs: 0x13E8,
    effect_seg: 0x2140,
    effect_rows: 0x0034,
    effect_masks: 0x0000,
};

/// Where `XP01.EXE` keeps what the logic shares with the engine.
pub(crate) fn ds_map(locations: &Locations, inventory: (u16, u16, u16)) -> DsMap {
    DsMap {
        // The main loop's three words sit together around the location
        // the dispatch reads (`0x4FC0`): `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`.
        entry: locations.location.wrapping_sub(2),
        location: locations.location,
        here: locations.location.wrapping_add(2),
        walker: WALKER,
        // `[1C82h]`, the hundredths the Zimmer's opening waits on
        // (`0x9D4`), and `[1C76h]`, the ticks not yet taken (`0xC5B`).
        hundredths: 0x1C82,
        ticks_taken: 0x1C76,
        // `0x9F5`: `add ax, 1F56h` after `shl ax, 5`.
        object_states: 0x1F56,
        // `0x11A1`: `mov [1689h], dx`, entry 0 of the row `02 n` reads.
        substitutions: 0x1689,
        substitution_count: 2,
        // `0x610`: the map's rule reads the object the click chose, and its
        // other two rules write the verb (`0x665`).
        verb: 0x288D,
        object: 0x2891,
        // `0x15B26`, `0x15B2D`: `Benutze … mit` reads both classes;
        // `0x15D75` the second object, which `0C3Fh:0B06h` is asked about.
        object_class: 0x2893,
        second: 0x2895,
        second_class: 0x2897,
        // `0x0DB39`, `0x0D915`: the dialogue's 256 flag bytes, `[273Fh + n]`.
        flags: 0x273F,
        // `0x757`: the map's frame loop reads the event record the poll
        // fills.
        input_event: 0x184E,
        // `0x0F1FE`: `call far [28BBh]`, the gate the machine asks; the
        // Wald's rule takes it down (`0x3F2A`).
        gate: 0x28BB,
        // `0x7A0C`: the room's header the loader keeps; F3 reads its
        // palette through it (`0x487B`).
        room_header: 0x2851,
        // `0x16B1`: the Straße waits for the door it came in by; `0x1803`
        // its hook sets one.
        entry_door: 0x1EDE,
        // `0x0CD67`, `0x0CABC`, `0x0CD71`.
        inventory,
        // `0x14817`: `mov word [bp-2], 2163h`, the buffer the start's load
        // clears (`0x14821`) and the save and restore copy through.
        save_buffer: SAVE_BUFFER,
        // `0x0E061`: `[185Eh] / [502h]`, the record's line height; the
        // record at `DS:04F8h` (`0x223E8`) ships the menu's geometry.
        menu: 0x04F8,
        gates: GateMap {
            // `0x15E4F`: a script exit installs `1586h:059Fh`.
            script_exit: (0x1586, 0x059F),
            // `0x15B52`: the walk verb's rule installs `1586h:0000h`.
            door: (0x1586, 0x0000),
            // `0x0F056` (the sixth verb's rule) and `0x0FD3E` (Escape):
            // `0EB9h:0489h`, the game options.
            options: (0x0EB9, 0x0489),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::titles::parsec1994::onward::Table;

    /// The numbers the generation's routines compare with, which the
    /// runtime takes from what it read rather than from here: each is
    /// still the immediate the program has at that place.
    #[test]
    fn the_counts_the_runtime_reads_are_the_program_s_immediates() {
        let Some(adv) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        let byte = |at: usize| adv.image.bytes()[at];
        // `cmp …, 17h` where an item is told from a room's object; the
        // immediate is the instruction's last byte.
        for at in [0x0CD52, 0x0F2DE, 0x0F5AE, 0x0F894, 0x10320] {
            assert_eq!(u16::from(byte(at)), adv.items_end(), "{at:#x}");
        }
        // `cmp …, 3Dh` where a location word is held inside the table.
        for at in [0x158FE, 0x083B0, 0x083BE, 0x156D5, 0x156E3] {
            assert_eq!(usize::from(byte(at)), adv.locations.count, "{at:#x}");
        }
        // `cmp ax, 3`, `cmp ax, 5`, `cmp ax, 4` and `mov si, 14h`, each
        // with its word immediate after the opcode.
        let imm = |at: usize| adv.image.word(at + 1).unwrap();
        for at in [0x077F2, 0x083F6, 0x1571B, 0x0EC68, 0x0ED34] {
            assert_eq!(imm(at), KINDS.forest, "{at:#x}");
        }
        for at in [0x15720, 0x0EC6D] {
            assert_eq!(imm(at), KINDS.reveal, "{at:#x}");
        }
        assert_eq!(imm(0x053CB), KINDS.restore);
        for at in [0x077F7, 0x083FB] {
            assert_eq!(imm(at), KINDS.forest_slot, "{at:#x}");
        }
        // `mov word [1F4Ah], 3Ch`, the load sending the main loop on.
        assert_eq!(adv.image.word(0x14CC3).unwrap(), SAVE.restore_row);
    }

    /// The walk on reads the same table the dispatch was read from.
    #[test]
    fn the_walk_on_reads_the_dispatch_s_table() {
        let Some(adv) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        let l = &adv.locations;
        assert_eq!(
            super::super::onward::TABLE,
            Table {
                seg: l.table,
                stride: u16::try_from(l.stride).unwrap(),
                kind: u16::try_from(l.kind).unwrap(),
                rows: u16::try_from(l.count).unwrap(),
            }
        );
    }

    /// The choice menu's geometry, read out of the record at `DS:04F8h`
    /// and the mouse object at `2139h:002Ah`, is what the listing shows.
    #[test]
    fn the_menu_s_geometry_is_read_from_its_record() {
        let Some(adv) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        let m = adv.menu_shape;
        assert_eq!(
            (m.lines, m.x, m.y, m.width, m.line_h),
            (5, 5, 0x84, 0x136, 0x0D)
        );
        assert_eq!(m.inks, [0xF8, 0xF8, 0xFA, 0xE8, 0xE8, 0xE8, 0xFE, 0xF8]);
        let rect = adv.cursor.as_ref().unwrap().menu_rect();
        assert_eq!(rect, (0, 0x84, 0x140, 0x44));
    }
}
