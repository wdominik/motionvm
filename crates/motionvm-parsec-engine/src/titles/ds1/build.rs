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
use crate::titles::parsec1994::program::{Build, Names, PanelReading, Reading, Sound};
use crate::titles::parsec1994::{
    Block, DsMap, GateMap, KeyMap, Kinds, LibraryMap, MusicMap, OptionInks, OptionsMap, PointerMap,
    SaveMap,
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
    // The options' dissolve loop is
    // `0x12086` (`mov word [1C82h], 0`) to `0x120D3` (`jc` back after
    // `cmp word [1C82h], 64h`, `0x120CE`); each pass draws once from
    // `1EB5h:0028h` with `push 5000h ; push 0` (`0x1208C`–`0x1208F`) and
    // keeps the draw in `di` (`0x12099`). The generator's seed is the
    // code-segment pair `1EB5h:0006h` (low) / `1EB5h:0008h` (high), set by
    // `1EB5h:000Ah` (`0x1EB6C`, `0x1EB71`). Count the steps between the
    // seed before and after the loop of the options `XP01` opens at its
    // start, on DOSBox-X `core = normal`, `cycles = fixed 25000`, the game
    // started the way `SCHATTEN.BAT` starts it, `XP00` and then `XP01`
    // after a new game (clicked by the resident `DOSINPUT.COM`, with which
    // `XP01` started on its own takes 157 517 against 157 519 without):
    // `0000:0000` before, `A414h:AD4Bh` after, 158 215 steps
    // of the generator. Nothing hands `XP01` a seed: `XP00` leaves it its
    // exit code and deletes `SAVEGAME.SAV` for a new game (its
    // `0x625`–`0x649`), and has no generator of its own; the only callers
    // of `1EB5h:000Ah` are the settings' check value (`0x147BF`), which the
    // start's reader and the settings' writer call (`0x148A6`, `0x14B8C`),
    // the start's reader of the saved game (`0x14935`) and the saver
    // (`0x14A2D`); and the seed's two words ship as zero (`0x1EB56`).
    dissolve_draws: 158_215,
    library: LIBRARY,
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
    hook: hook::answer,
    ending: super::ending::open,
    // `_main` writes the settings (`0x4FF0`) and then shows the end
    // (`0x5014`).
    settings_after_ending: false,
    callbacks: &[],
};

/// Where `XP01.EXE` keeps the library routines the runtime reads its build
/// off, as `(segment, offset)`, with the image offset beside each. Each is
/// the far target a call names: hover off at `0x15E44`
/// (the script's exit gate calls it first), the task start at `0x0FB7F`
/// (the dispatcher's head), the script's exit gate in the global rules'
/// segment, and the two visit routines every location handler calls —
/// the Zimmer's at `0x009B8` and `0x00A02`.
const LIBRARY: LibraryMap = LibraryMap {
    input_lock: (0x0EB9, 0x188A),         // 0x1041A
    input_unlock: (0x0EB9, 0x18A4),       // 0x10434
    hover_off: (0x0EB9, 0x18F4),          // 0x10484
    task_start: (0x0CFD, 0x1A3F),         // 0x0EA0F
    scene_bind: (0x0CFD, 0x059E),         // 0x0D56E
    script_exit: (0x1586, 0x05E1),        // 0x15E41
    visit_finish_entry: (0x0EB9, 0x00B8), // 0x0EC48
    // `0x0EC76`, `0x0EC7F`: the two effects the entry's end calls.
    entry_reveal: (0x10B8, 0x03BA),       // 0x10F3A
    entry_slide: (0x10B8, 0x0CAB),        // 0x1182B
    visit_leave: (0x0EB9, 0x017A),        // 0x0ED0A
    hover_on: (0x0EB9, 0x1917),           // 0x104A7
    obj_state: (0x14DD, 0x07E7),          // 0x155B7
    walk_to_node: Some((0x14DD, 0x0A56)), // 0x15826
    obj_anim: (0x0883, 0x1F82),           // 0x0A7B2
    objs_save_state: (0x14DD, 0x068A),    // 0x1545A
    // `0x0EBC6`: the visit's setup hands it the location's state row.
    objs_load_state: (0x14DD, 0x0293), // 0x15063
    hotspot_follow: (0x14DD, 0x074B),  // 0x1551B
    obj_show: (0x14DD, 0x084A),        // 0x1561A
    obj_hide: (0x14DD, 0x096B),        // 0x1573B
    say_text: (0x0CFD, 0x18C2),        // 0x0E892
    options_gate: (0x0EB9, 0x0489),    // 0x0F019
    dispatch: (0x0EB9, 0x0FCE),        // 0x0FB5E
    mode_reset: (0x0EB9, 0x003F),      // 0x0EBCF
    second_wanted: (0x0EB9, 0x0776),   // 0x0F306
    room_loader: (0x0766, 0x010A),     // 0x0776A
    place_pair: (0x14DD, 0x08F1),      // 0x156C1
    slot_unload: (0x0766, 0x0D34),     // 0x08394
    visit_enter: (0x0EB9, 0x0207),     // 0x0ED97
    inv_take: (0x0C3F, 0x09E1),        // 0x0CDD1
    inv_give: (0x0C3F, 0x095C),        // 0x0CD4C
    visit_setup: (0x0EB9, 0x0001),     // 0x0EB91
    slot_resolve: (0x0CFD, 0x02C0),    // 0x0D290
    effects_load: (0x1458, 0x00F7),    // 0x14677
    slot_song: (0x1197, 0x0035),       // 0x119A5
    strip_load: (0x0C1C, 0x000C),      // 0x0C1CC
    text_scan: (0x0CFD, 0x01BE),       // 0x0D18E
    task_query: (0x0CFD, 0x0FD9),      // 0x0DFA9
    status_paint: (0x0C3F, 0x005C),    // 0x0C44C
    status_gate: (0x0EB9, 0x163E),     // 0x101CE
    mode_step: (0x0EB9, 0x0965),       // 0x0F4F5
    // Its one caller is task 0's menu paint (`0x0E607`).
    menu_strip: (0x0C3F, 0x04D2),    // 0x0C8C2
    anim_signal: (0x0EB9, 0x1D97),   // 0x10927
    task_arm: (0x0CFD, 0x1A2E),      // 0x0E9FE
    page_release: (0x0CFD, 0x1BA2),  // 0x0EB72
    play_sample: (0x0EB9, 0x1949),   // 0x104D9
    interaction: (0x0EB9, 0x177A),   // 0x1030A
    figure_bind: (0x14DD, 0x0098),   // 0x14E68
    animator_step: (0x0883, 0x012A), // 0x0895A
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
        panel: PanelReading {
            curves: text.panel_curves,
            place: crate::titles::palette1994::PanelPlace::Over,
            ramp: Some(text.gray_ramp.clone()),
            strip_row: None,
        },
        start: text.start,
    }
}

/// What the kinds of `XP01.EXE`'s location table mean.
pub(crate) const KINDS: Kinds = Kinds {
    // `0x04FA1`: the main loop walks on for kind 0; `0x053C7`, the walk
    // steps over it; `0x15919`, a door does not lead there.
    passage: 0,
    // `0x053CB`: the walk on steps over kind 4, location 60's.
    restore: 4,
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
    // `push 1Fh` at the write (`0x14BBE`), `cmp ax, 1Fh` at the size
    // check (`0x1485B`); the check value takes six words from `+12h`
    // (`mov si, 6`, `0x147D6`), the last of them the speed the write
    // stores at `0x14B68`; the options mark it changed (`0x1263A`).
    cfg_len: 0x1F,
    cfg_words: 6,
    cfg_speed: true,
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
    inks: OptionInks {
        head: 0x11A4B,
        question: 0x11CA6,
        yes: 0x11D5D,
        no: 0x11D83,
        hover: 0x12214,
    },
    // 0x11A1E: `push 0C1h`, the playfield (`push 80h ; push 140h ; push 0
    // ; push 0 ; call far`); 0x11A33: `push 0C0h`, the band from row 0Fh,
    // 62h rows high.
    grounds: [0x11A1E, 0x11A33],
    // 0x11C46: `push 0FEh`, 0x11C6C: `push 0FFh`, 0x11D23 (and 0x11D46):
    // `push 0EEh`.
    box_inks: [0x11C46, 0x11C6C, 0x11D23],
    // The hold comes before the picture is presented.
    paint_before_hold: false,
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
    keys: Some(KeyMap {
        // `0x5F10`–`0x5F14`: `push ds ; push 228h ; call far 179Eh:1483h`.
        list: 0x0228,
        // `cs:13B2h`, `0x18EC6`: `cmp byte [cs:13B2h], 0`.
        switch: 0x179E0 + 0x13B2,
        // `0x18F16`: `mov di, 1868h`.
        speeds: 0x1868,
        // `0x18F05`–`0x18F0C`: `[3C8h]`, `[3CCh]`, `[3CAh]`.
        limits: 0x03C8,
        // `0x1850C`: `mul word [3D8h]`.
        acceleration: 0x03D8,
        // `0x185B9`: `idiv word [3DAh]`; `0x18579`: `idiv word [3DCh]`.
        divisors: 0x03DA,
    }),
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
        // `0x0FD50`: `mov word [288Fh], 7`; `0x0FD4A`: `and word [289Bh], 0FE0Fh` (Escape's arm).
        mode: 0x288F,
        bits: 0x289B,
        // `0x0FC78`: `mov [289Dh], ax ; mov [288Dh], ax` as a verb is chosen.
        verb_chosen: 0x289D,
        two_object: 0x2899,
        // `0x15878`: `mov word [288Dh], 0Ah` in the door's gate; `0x15E08` in
        // the script exit's.
        walk_verb: 0x0A,
        // `0x7A0C`: the room's header the loader keeps; F3 reads its
        // palette through it (`0x487B`).
        room_header: 0x2851,
        // `0x16B1`: the Straße waits for the door it came in by; `0x1803`
        // its hook sets one.
        entry_door: 0x1EDE,
        // `mov [289Fh], si` after a walk to a node that started
        // (`0x15840`).
        walk_target: 0x289F,
        // `mov word [28C1h], 3`, the eighth verb's door (`0x15CC9`).
        anim_wait: 0x28C1,
        status_age: 0x28C3,
        // `0x0F5C4`: `mov al, [si+0527h]`; all zero in the image.
        item_classes: 0x0527,
        // `0x0CD67`, `0x0CABC`, `0x0CD71`.
        inventory,
        // `0x14817`: `mov word [bp-2], 2163h`, the buffer the start's load
        // clears (`0x14821`) and the save and restore copy through.
        save_buffer: SAVE_BUFFER,
        // `0x0E061`: `[185Eh] / [502h]`, the record's line height; the
        // record at `DS:04F8h` (`0x223E8`) ships the menu's geometry.
        menu: 0x04F8,
        // The shared hook's words: `[1E50h]` to `[1E5Ah]`, which its code-0
        // arm clears (`0x10515`–`0x10527`) and the rest of it reads and
        // writes, and `[1ED0h]`, which its event 111 sets (`0x1082A`) and
        // the dispatcher's head reads (`0x0FB69`).
        script: Some(hook::W1E50),
        script_start: Some(hook::W1ED0),
        gates: GateMap {
            // `0x15E4F`: a script exit installs `1586h:059Fh`.
            script_exit: (0x1586, 0x059F),
            // `0x15B52`: the walk verb's rule installs `1586h:0000h`.
            door: Some((0x1586, 0x0000)),
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

    /// The options' dissolve leaves the generator where the original's left
    /// it on the reference machine: `A414h:AD4Bh` at `1EB5h:0006h` after
    /// the loop, from `0000:0000` before it.
    #[test]
    fn the_dissolve_draws_are_the_reference_machine_s_count() {
        let mut rng = crate::titles::rng::LcRng::default();
        for _ in 0..BUILD.dissolve_draws {
            rng.inclusive(0, 0x5000);
        }
        assert_eq!((rng.hi, rng.lo), (0xA414, 0xAD4B));
    }

    /// Each of the options' inks is where its routine sets it: a
    /// `mov byte es:[bx+4], ink` (`26 C6 47 04`) on the font record, and the
    /// ink the build's own.
    #[test]
    fn the_options_inks_are_the_routines_immediates() {
        let Some(adv) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        let i = OPTIONS.inks;
        let inks: Vec<u8> = [i.head, i.question, i.yes, i.no, i.hover]
            .into_iter()
            .map(|at| {
                let b = adv.image.bytes();
                assert_eq!(b[at..at + 4], [0x26, 0xC6, 0x47, 0x04], "{at:#x}");
                b[at + 4]
            })
            .collect();
        assert_eq!(inks, [0xF4, 0xFE, 0xF8, 0xFA, 0xFE]);
        // The grounds and the confirmation's inks, each `push imm16`.
        let b = adv.image.bytes();
        let pushed: Vec<u8> = OPTIONS
            .grounds
            .into_iter()
            .chain(OPTIONS.box_inks)
            .map(|at| {
                assert_eq!((b[at], b[at + 2]), (0x68, 0), "{at:#x}");
                b[at + 1]
            })
            .collect();
        assert_eq!(pushed, [0xC1, 0xC0, 0xFE, 0xFF, 0xEE]);
    }

    /// What the visit's routines do with the objects, the tasks, Escape,
    /// the verbs, the entry and the strip, read off them.
    #[test]
    fn the_visit_s_routines_are_read() {
        let Some(adv) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        use crate::titles::parsec1994::profile::{Profile, RecordCodes, SecondWanted};
        let p = Profile::read(&adv.image, &LIBRARY).expect("reads");
        assert_eq!(p.tri_state_kinds, 0b10_0010);
        assert!(p.task_start_needs_armed);
        assert_eq!(p.escape_verb, 6);
        assert!(p.verb6_arm);
        assert_eq!(p.second_wanted, SecondWanted::Verbs);
        assert!(p.place_falls_back);
        assert!(p.show_hides_parts && p.hide_clears_head);
        assert!(p.own_line_sets_speakers);
        assert_eq!(p.status_after, 3);
        assert_eq!((p.interaction.items_end, p.interaction.verb), (0x17, 2));
        assert!(p.kind8_records);
        assert_eq!(p.anim_arms, 0x0B);
        {
            // `0EB9h:1D97h`, the table at `cs:1E35h` (`0x109C5`).
            use crate::titles::parsec1994::signal::SignalAct::{
                FigureAnimation, Release, Sample, TaskArm, TaskStart,
            };
            let n = vec![];
            assert_eq!(
                crate::titles::parsec1994::signal::tests::listed(&p.signal),
                [
                    (1, vec![Release(1)], n.clone()),
                    (2, vec![], n.clone()),
                    (3, vec![Sample(4)], n.clone()),
                    (4, vec![], n.clone()),
                    (5, vec![TaskArm(0)], n.clone()),
                    (6, vec![TaskStart(0)], n.clone()),
                    (7, vec![], n.clone()),
                    (8, vec![Sample(3)], n.clone()),
                    (9, vec![], n.clone()),
                    (10, vec![], n.clone()),
                    (11, vec![FigureAnimation], n.clone()),
                    (12, vec![FigureAnimation], n),
                ]
            );
        }
        {
            use crate::titles::parsec1994::profile::OptionsGate;
            assert_eq!(
                p.options_gate,
                OptionsGate {
                    drops_ticks: true,
                    held: None
                }
            );
        }
        assert_eq!(p.inventory_rows, None);
        let sc = p.scenes;
        assert_eq!(
            (sc.item_base, sc.bounded, sc.falls_back),
            (0x4001, true, true)
        );
        assert_eq!(p.hotspot_kept, 0b10_0001_1110);
        assert_eq!(
            (p.records.codes, p.records.read, p.records.fresh),
            (RecordCodes::Hidden, [Some(1), Some(5), None], None)
        );
    }

    /// The record hook is never installed: the entry copies it from the
    /// row's `+37h` into `[2871h]` (`0x0EECB`–`0x0EED9`, the word's only
    /// store), and every row is null there. Nor does a room file hold a
    /// record the animator or the drawer would hand it — a state above
    /// the step's arms, or bit 4 of `+2` — so the arms past them are
    /// never taken.
    #[test]
    fn no_record_reaches_the_record_hook() {
        let Some(adv) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        use crate::titles::parsec1994::image_records::tests::{hooked_records, row_far_pointers};
        assert_eq!(row_far_pointers(&adv, 0x37), []);
        assert_eq!(hooked_records(&adv, adv.profile.anim_arms), []);
    }

    /// The status line's inks, the band under a choice menu and the age
    /// mode 8 sets, read off the status painter, the menu's strip redraw
    /// and the mode's step.
    #[test]
    fn the_status_band_is_the_one_the_runtime_was_read_from() {
        let Some(adv) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        let p = crate::titles::parsec1994::Profile::read(&adv.image, &LIBRARY).expect("reads");
        assert_eq!(
            (p.status_inks.ink, p.status_inks.controls),
            (0xF4, [(0x09, 0xFA), (0x0A, 0xFC), (0x0D, 0xF4)]),
            "`0x0C750`, `0x0C8B2`"
        );
        assert_eq!(p.menu_band, None, "`0x0C8C2`–`0x0C970` holds nothing dark");
        assert!(p.mode_step_ages_status, "`0x0F56D`");
    }

    /// The library build `XP01.EXE` is linked against, read off its
    /// routines: every capability as this build has it.
    #[test]
    fn the_library_build_is_the_one_the_runtime_was_read_from() {
        let Some(adv) = crate::titles::ds1::adventure_for_test() else {
            return;
        };
        let p = crate::titles::parsec1994::Profile::read(&adv.image, &LIBRARY).expect("reads");
        assert!(p.lock_forgets_hover);
        assert!(p.unlock_forgets_hover);
        assert!(p.hover_off_forgets_hover);
        assert!(p.task_start_tells_hook);
        assert!(p.script_exit_hovers_off);
        assert_eq!(p.leave_fade, 0x05DC);
        use crate::titles::parsec1994::profile::{ExitForm, MapExits, SharedRooms};
        assert_eq!(
            p.rooms,
            SharedRooms {
                pairs: [Some(3), Some(5), None],
                shared: [Some((3, 0x14)), None],
                sound_kept: None,
                sound_flags: 0x0C,
                exits: Some(MapExits {
                    kind: 3,
                    first: 0x0A,
                    form: ExitForm::Cleared,
                }),
            }
        );
        assert_eq!(
            (p.leave_spares, p.leave_clears_status),
            ([Some(3), None, None], true)
        );
        {
            use crate::titles::parsec1994::profile::{EntryEffect, EntryEnd, EntryLook};
            let look = |effect, song| EntryLook {
                effect: Some(effect),
                song,
            };
            assert_eq!(
                p.entry_end,
                EntryEnd {
                    location_word: None,
                    locations: [None; 2],
                    pairs: [
                        Some((3, look(EntryEffect::Reveal, false))),
                        Some((5, look(EntryEffect::Reveal, true))),
                        None
                    ],
                    otherwise: look(EntryEffect::Slide, true),
                }
            );
        }
        assert_eq!(p.entry_fade_in, None);
        assert!(p.hover_on_forgets_hover);
        assert!(p.hover_on_polls);
        assert_eq!(
            p.object_state,
            crate::titles::parsec1994::profile::ObjectState::Modes {
                when_mode_one: true
            }
        );
        assert!(p.walk_to_node_remembers);
        assert!(p.anim_checks_count);
        assert_eq!(p.anim_wakes_only_kind, Some(4));
        assert!(p.dispatch_starts_asked_task);
        assert!(!p.forgotten_hover_is_nothing);
        assert!(p.room_click_forgets_hover);
        assert!(p.hover_off_only_notes);
        assert!(p.verb6_refreshes_status);
        assert_eq!(p.verb_bound, 0x0F);
        assert!(p.locked_stops_hover_text);
        assert!(p.mode_reset_forgets_hover);
        let s = p.screen_slot.expect("this build fills a screen slot");
        assert_eq!((s.slot, s.code, s.x, s.y), (10, 0x53, 160, 64));
        let door = p.entry_door;
        assert_eq!((door.kind, door.remembered, door.frame), (4, true, 1));
        assert_eq!((door.ids, door.more), ((100, 109), [None; 3]));
        assert_eq!(p.inventory_window, 8);
        assert_eq!((p.status_row, p.status_band), (128, 12));
        assert_eq!(p.callback, None);
        assert_eq!(p.slot_style, None);
        let e = p.effects;
        assert_eq!(
            (e.slots, e.room_first, e.rooms, e.wide_masks),
            (21, 12, 0x1A, false)
        );
        assert_eq!(p.song_slots, 30);
        assert_eq!(p.figure_set.digits(), [0], "one set of strip and figure");
        assert_eq!(p.figure_set.digit_for(40), 0);
        assert_eq!(p.figure_first, 1, "the palette stepped past, `0x10AD0`");
        assert_eq!(
            p.strip,
            crate::titles::parsec1994::profile::StripRange::Whole
        );
        assert_eq!(
            p.text_flags,
            crate::titles::parsec1994::profile::TextFlags {
                hold: 0x100,
                firm: 0x200,
                quick: Some(0x400)
            }
        );
        assert_eq!(adv.profile, p);
    }

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
        // The forest's are the library's and read with its build: the
        // runtime's reading is the program's immediates too.
        let rooms = adv.profile.rooms;
        let (forest, slot) = rooms.shared[0].expect("this build has a forest");
        for at in [0x077F2, 0x083F6, 0x1571B, 0x0EC68, 0x0ED34] {
            assert_eq!(imm(at), forest, "{at:#x}");
        }
        for at in [0x15720, 0x0EC6D] {
            assert_eq!(Some(imm(at)), rooms.pairs[1], "{at:#x}");
        }
        assert_eq!(imm(0x053CB), KINDS.restore);
        for at in [0x077F7, 0x083FB] {
            assert_eq!(imm(at), slot, "{at:#x}");
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
