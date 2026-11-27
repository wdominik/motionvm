//! What *Captain Zins*'s `XP01.EXE` hands the `parsec1994` runtime: where
//! this build keeps what each part of the runtime reads, one map per part,
//! with the instruction each value is read from beside it.
//!
//! The maps' types are the generation's (`titles/parsec1994/`); their
//! values are this program's and stand here, so that the generation names
//! no title. Every address in the comments is an image offset of the
//! unpacked `XP01.EXE` (138752 bytes, `DGROUP` `1FC4h`), linear
//! (`seg * 16 + off`).

use super::hook;
use super::logic::{self, SAVE_BUFFER, WALKER};
use super::xp01data::ExeText;
use crate::titles::parsec1994::program::{Build, Names, PanelReading, Reading, Sound};
use crate::titles::parsec1994::{
    Block, DsMap, GateMap, KeyMap, Kinds, LibraryMap, MusicMap, OptionInks, OptionsMap, PointerMap,
    SaveMap,
};
use crate::titles::xp1994::Locations;

/// `XP01.EXE`, as the generation's runtime opens it.
pub(crate) static BUILD: Build = Build {
    // The file on disk is packed; unpacked it is 138752 bytes.
    program: ("XP01.EXE", 138_752),
    names: Names {
        // `0x00A62`: `push 9EEh`, `RESOURCE\CZROOM.DAT`.
        rooms: "CZROOM.DAT",
        // `DS:0A02h`, `RESOURCE\PANEL%.DAT` (`push 0A02h`, `0x076D6`): the
        // strip's loader `0764h:0001h(n)` puts `'0' + n` at the `%`
        // (`0x0770A`–`0x07712`), and its one caller is the figure's loader
        // (`0x0BEAB`), which hands it its own `n`.
        panel: "PANEL%.DAT",
        // `0x0138B`: `push 996h`, `RESOURCE\CMBMOU.DAT`.
        pointers: "CMBMOU.DAT",
        // `DS:0A16h`, `RESOURCE\PLAY0%.DAT` (`push 0A16h`, `0x0BE4B`), the
        // digit `'0' + n` (`0x0BEA0`); which `n` a visit takes is the
        // library's ([`crate::titles::parsec1994::Profile::figure_set`]).
        player: "PLAY0%.DAT",
        // `0x00A10`: `push 0A2Ah`, `RESOURCE\DIALOG.DAT`.
        dialog: "DIALOG.DAT",
        // `0x0F33E`: `push 9BEh`, `RESOURCE\PSMP2L.DAT`.
        music: "PSMP2L.DAT",
        // `DS:0982h`, the drivers' RIB (`push 982h`, `0x011C1`), opened at
        // `0x012C9`.
        drivers: "PSMDRV.DAT",
    },
    // The options' dissolve loop is
    // `0x0D3BB` (`mov word [16F4h], 0`) to `0x0D40D` (`jc` back after
    // `cmp word [16F4h], 64h`, `0x0D408`); each pass draws once from
    // `1D77h:0026h` with `push 5000h ; push 0` (`0x0D3C1`–`0x0D3C4`) and
    // keeps the draw in `[bp-2]` (`0x0D3CE`), where Dunkle Schatten's keeps
    // it in a register. The generator's seed is the code-segment pair
    // `1D77h:0004h` (low) / `1D77h:0006h` (high), set by `1D77h:0008h`
    // (`0x1D78A`, `0x1D78F`). Count the steps between the seed before and
    // after the loop of the options `XP01` opens at its start, on DOSBox-X
    // `core = normal`, `cycles = fixed 25000`, the game started the way
    // `ZINS.BAT` starts it, `XP00` and then `XP01` after a new game
    // (clicked by the resident `DOSINPUT.COM`, with which `XP01` started on
    // its own takes 157 491 against 157 493 without): `0000:0000` before,
    // `C799h:5296h` after, 158 282 steps of the generator. Nothing hands
    // `XP01` a seed: `XP00` leaves it its exit code and deletes
    // `SAVEGAME.SAV` for a new game (its `0x625`–`0x649`, as Dunkle
    // Schatten's does), and has no generator of its own; the only callers
    // of `1D77h:0008h` are the settings' check value
    // (`0x0FC17`), which the start's reader and the settings' writer call
    // (`0x0FCFB`, `0x0FFD3`), the start's reader of the saved game
    // (`0x0FD84`) and the saver (`0x0FE7C`); and the seed's two words ship
    // as zero (`0x1D774`).
    dissolve_draws: 158_282,
    library: LIBRARY,
    kinds: KINDS,
    save: SAVE,
    options: OPTIONS,
    pointer: POINTER,
    music: MUSIC,
    sound: Sound {
        // `0x01321`: `push 6`, the channels the digital driver is
        // installed with.
        channels: 6,
        // `0x0064C`: `push 28h`, then `call far 1798h:044Ah` (`0x0064E`),
        // the effects' volume after the init.
        effect_volume: 0x28,
    },
    logic: logic::TABLE,
    hook: hook::answer,
    callbacks: logic::CALLBACKS,
    ending: super::ending::open,
    // `_main` writes the settings (`0x007AD`) and then shows the end
    // (`0x007CB`).
    settings_after_ending: false,
};

/// Where `XP01.EXE` keeps the library routines the runtime reads its build
/// off, as `(segment, offset)`, with the image offset beside each. Each is
/// the far target the program's own calls name: the location handlers'
/// (the Wohnung's at `0x1140A`–`0x114B9`), the save's (`objs_save_state`,
/// `0x1004E`), the start's and the room loader's.
const LIBRARY: LibraryMap = LibraryMap {
    input_lock: (0x0A01, 0x185E),         // 0x0B86E
    input_unlock: (0x0A01, 0x186C),       // 0x0B87C
    hover_off: (0x0A01, 0x18B0),          // 0x0B8C0
    task_start: (0x0849, 0x1A20),         // 0x09EB0
    scene_bind: (0x0849, 0x05BC),         // 0x08A4C
    script_exit: (0x10C3, 0x05A6),        // 0x111D6
    visit_finish_entry: (0x0A01, 0x00FF), // 0x0A10F
    // The entry's end slides (`0x0A134`); the reveal is the effects
    // library's other entry, with Dunkle Schatten's prologue (`0x0C2E1`).
    entry_reveal: (0x0BF4, 0x03A1),       // 0x0C2E1
    entry_slide: (0x0BF4, 0x0C82),        // 0x0CBC2
    visit_leave: (0x0A01, 0x01B4),        // 0x0A1C4
    hover_on: (0x0A01, 0x18C1),           // 0x0B8D1
    obj_state: (0x1021, 0x07B0),          // 0x109C0
    walk_to_node: Some((0x1021, 0x09FD)), // 0x10C0D
    obj_anim: (0x03E8, 0x1EDE),           // 0x05D5E
    objs_save_state: (0x1021, 0x0653),    // 0x10863
    objs_load_state: (0x1021, 0x029B),    // 0x104AB, from the setup at 0x0A099
    hotspot_follow: (0x1021, 0x071E),     // 0x1092E
    obj_show: (0x1021, 0x0813),           // 0x10A23
    obj_hide: (0x1021, 0x0912),           // 0x10B22
    say_text: (0x0849, 0x18A3),           // 0x09D33
    options_gate: (0x0A01, 0x04DA),       // 0x0A4EA
    dispatch: (0x0A01, 0x1006),           // 0x0B016
    mode_reset: (0x0A01, 0x0092),         // 0x0A0A2
    second_wanted: (0x0A01, 0x07F1),      // 0x0A801
    room_loader: (0x02DB, 0x010A),        // 0x02EBA
    place_pair: (0x1021, 0x08BA),         // 0x10ACA
    slot_unload: (0x02DB, 0x0C3E),        // 0x039EE
    visit_enter: (0x0A01, 0x023C),        // 0x0A24C
    inv_take: (0x078A, 0x09E6),           // 0x08286
    inv_give: (0x078A, 0x0961),           // 0x08201
    visit_setup: (0x0A01, 0x003B),        // 0x0A04B
    // The resolver the slot setters' codes go through, `and bx, 0Fh` at
    // `0x08741`; it is reached by near calls only.
    slot_resolve: (0x0849, 0x02A1), // 0x08731
    // Called by the room loader (`0x02F5B`, `0x039BB`).
    effects_load: (0x0F9D, 0x00F7), // 0x0FAC7
    // Called by the room loader (`0x02F4D`, `0x0399F`).
    slot_song: (0x0CCF, 0x0030), // 0x0CD20
    // Its one caller is the figure's loader (`0x0BEAB`).
    strip_load: (0x0764, 0x0001), // 0x07641
    // Its one caller is a page's start (`0x08C6B`, as `0x0D79E` in DS1).
    text_scan: (0x0849, 0x01B3), // 0x08643
    // Called once a frame by the dispatcher (`0x0B021`).
    task_query: (0x0849, 0x0FE6), // 0x09476
    // Called as `078Ah:0069h` (Dunkle Schatten's `0C3Fh:005Ch`).
    status_paint: (0x078A, 0x0069), // 0x07909
    status_gate: (0x0A01, 0x1612),  // 0x0B622
    mode_step: (0x0A01, 0x09E0),    // 0x0A9F0
    // Its one caller is task 0's menu paint (`0x09AA8`).
    menu_strip: (0x078A, 0x04DF),    // 0x07D7F
    anim_signal: (0x0A01, 0x1D48),   // 0x0BD58
    task_arm: (0x0849, 0x1A0F),      // 0x09E9F
    page_release: (0x0849, 0x1B63),  // 0x09FF3
    play_sample: (0x0A01, 0x18D7),   // 0x0B8E7
    interaction: (0x0A01, 0x174E),   // 0x0B75E
    figure_bind: (0x1021, 0x00A0),   // 0x102B0
    animator_step: (0x03E8, 0x0127), // 0x03FA7
};

/// What the runtime takes from this title's own reading of its program.
pub(crate) fn reading(text: &ExeText, locations: &Locations) -> Reading {
    Reading {
        ds_map: ds_map(locations, text.inventory_at),
        templates: (text.template_row, super::xp01data::VERB_TEMPLATES),
        // The row at `DS:11D2h` as the image ships it: two far pointers,
        // `"9990122233300"` and `"39452"`.
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
    // `0x0075E`: `cmp word es:[bx+7], 0`, the main loop walks on for kind
    // 0; `0x00B15`, the walk steps over it; `0x10CEA`, a door does not lead
    // there (Dunkle Schatten `0x04FA1`, `0x053C7`, `0x15919`).
    passage: 0,
    // `0x00B19` and `0x00B40`: `cmp ax, 3`, the walk on steps over kind 3,
    // the kind of row 19, the restore row's (Dunkle Schatten kind 4, `0x053CB`).
    restore: 3,
};

/// Where `XP01.EXE` keeps its saved game and settings, and their files' shape.
pub(crate) const SAVE: SaveMap = SaveMap {
    // `push 377h` at the clear (`0x0FC79`) and the write (`0x0FF30`), `cmp
    // ax, 377h` at the size check (`0x0FD50`).
    len: 0x377,
    // `add dx, 14h` (`0x0FDB3`, `0x0FED9`); `mov si, 1B6h` (`0x0FDBC`) and
    // `mov cx, 1B6h` (`0x0FEE2`): `36Ch` bytes, nine past the file's end,
    // as in Dunkle Schatten; the nine bytes after the buffer (`0x1F727`–`0x1F72F`) are
    // zero in the image.
    sum: (0x14, 0x1B6),
    // `ZinsSAV\0`: the four words `0x0FEB5`–`0x0FEC6` store and
    // `0x0FDF8`–`0x0FE0E` compare.
    sav_magic: [0x695A, 0x736E, 0x4153, 0x0056],
    // `ZinsCFG\0`: `0x0FF84`–`0x0FF93` and `0x0FCD6`–`0x0FCED`.
    cfg_magic: [0x695A, 0x736E, 0x4643, 0x0047],
    // `cmp ax, 1Dh` at the size check (`0x0FCB0`), `push 1Dh` at the write
    // (`0x10005`); the check value walks five words from `+12h` (`mov si,
    // 5`, `0x0FC2E`), and the file carries no walk speed (see `speed`).
    cfg_len: 0x1D,
    cfg_words: 5,
    cfg_speed: false,
    // `0x10107`: `mov word [1A5Ch], 13h`, with `[1A5Ah] := 1` (`0x1010D`).
    restore_row: 0x13,
    blocks: [
        // 0x10071: the story bytes.
        Block {
            buf: 0x014,
            ds: 0x1CEA,
            out: 0x1C,
            back: 0x1C, // 0x10179
        },
        // 0x10089: 20 object rows of 32 bytes.
        Block {
            buf: 0x030,
            ds: 0x1A68,
            out: 0x280,
            back: 0x280, // 0x10191
        },
        // 0x100A2: the location stack.
        Block {
            buf: 0x2B0,
            ds: 0x1A60,
            out: 8,
            back: 8, // 0x1012C
        },
        // 0x100BA: the player's record.
        Block {
            buf: 0x2B8,
            ds: 0x11DC,
            out: 0x30,
            back: 0x30, // 0x10144
        },
        // 0x100D2: 8Eh bytes saved, of which 0x101AA restores 26h.
        Block {
            buf: 0x2E8,
            ds: 0x1138,
            out: 0x8E,
            back: 0x26,
        },
    ],
    // `0x10056`: `inc word [1A60h]`; the restore copies the stack and the
    // player first and the rest after the pop, as Dunkle Schatten's (`0x1012C`–
    // `0x101BA`).
    stack: 0x1A60,
    // `0x0FE35`: `cmp word [128Ch], 0`; `0x100EF` sets it.
    saved: 0x128C,
    // `0x0FE22`, `0x100F5` set it; `0x10100` reads it.
    loaded: 0x128E,
    // `0x0FECC`, `0x0FF98`: `mov ax, [1262h]` (`100h` in the image).
    version: 0x1262,
    // `0x0FF67`: `cmp word [1984h], 0`; the options set it (`0x0D69F`).
    cfg_dirty: 0x1984,
    // `0x0FD0E`, `0x0FF9E`, `0x101C4`.
    mouse: 0x123C,
    // `0x101D4`: `mov word [372h], 0Ah`, `0x101DA`: `mov word [370h],
    // 0Ah`, the table at `cs:0625h` with Dunkle Schatten's five cases and its
    // fallback (`0x10202`).
    divisors: (0x0372, 0x0370),
    // `0x0FD19`, `0x0FFAC`: `push 11C6h`, eight bytes.
    text: 0x11C6,
    // `[11F0h]` is the walk-speed byte the options set (`mov byte [11F0h],
    // al`, `0x0D964`); neither the settings' read (`0x0FD0B`–`0x0FD26`) nor
    // their write (`0x0FF9E`–`0x0FFB7`) touches it, and the options do not
    // mark the settings changed for it.
    speed: 0x11F0,
    // `0x0FD31`, `0x0FE47`: `push 9E0h`, `*SAVEGAME.SAV`.
    sav_name: 0x09E0,
    // `0x0FC94`, `0x0FF79`: `push 9D2h`, `*SAVEGAME.CFG`.
    cfg_name: 0x09D2,
    // `0x0FF25`, `0x0FFFA`: `push 738h`.
    msg_create: 0x0738,
    // `0x0FF4D`, `0x10020`: `push 760h`.
    msg_write: 0x0760,
    // `0x0FE17`: `push 1264h`.
    msg_invalid: 0x1264,
};

/// Where `XP01.EXE` keeps what the game options read and write.
pub(crate) const OPTIONS: OptionsMap = OptionsMap {
    // 0x0CDB3: `les bx, [bx+0FDEh]`.
    rows: 0x0FDE,
    // 0x0CD9A: `push 0B36h`, `Spieloptionen`.
    title: 0x0B36,
    // 0x0D1FE: `push 2 ; push 0C0h ; push ds ; push 1002h`.
    colors: (0x1002, 0xC0),
    // 0x0D931: `mov ax, [bx+1008h]`, the row `8 · [0FA6h]` (`0x0D926`).
    text_speeds: 0x1008,
    // 0x0D960: `mov al, [bx+1020h]`, indexed by `[0FB0h]` (`0x0D95C`).
    walk_speeds: 0x1020,
    // 0x0D6CF, 0x0D6F1, 0x0D716, 0x0D728.
    confirms: [0x0F5E, 0x0F6A, 0x0F76, 0x0F82],
    // 0x0D63D: `xor word [1A8h], 1`.
    music: 0x01A8,
    // 0x0D6B2: `test word [198Eh], 10h`.
    switches: 0x198E,
    save: SAVE,
    inks: OptionInks {
        head: 0x0CD94,
        question: 0x0CFEF,
        yes: 0x0D0A6,
        no: 0x0D0CC,
        hover: 0x0D54C,
    },
    // 0x0CD67: `push 0C1h`, the playfield (`push 80h ; push 140h ; push 0
    // ; push 0 ; call far`); 0x0CD7C: `push 0C0h`, the band from row 0Fh,
    // 62h rows high.
    grounds: [0x0CD67, 0x0CD7C],
    // 0x0CF8F: `push 0FEh`, 0x0CFB5: `push 0FFh`, 0x0D06C (and 0x0D08F):
    // `push 0EEh`.
    box_inks: [0x0CF8F, 0x0CFB5, 0x0D06C],
    // The hold comes before the picture is presented.
    paint_before_hold: false,
};

/// Where `XP01.EXE` keeps the pointer's words and tables.
pub(crate) const POINTER: PointerMap = PointerMap {
    // `0105h:064Bh`, byte for byte Dunkle Schatten's `0590h:064Dh` but for the
    // addresses: `push 1CAh` (0x016BF), `20Ah` (0x016D1), `24Ah`
    // (0x016DB), `26Ah` (0x016E5), each copied to `push 2CAh` (0x016E1)
    // by `163Ch:0BE0h`.
    modes: [0x01CA, 0x020A, 0x024A, 0x026A],
    slot: 0x02CA,
    // `0x01616`: `mov word [34Ah], 1EAh`, after `mov [34Ch], ds`.
    regions: 0x01EA,
    // `0x01548` (`[13D0h]+4`, stored at `[1EEh]`, `0x01557`), `0x01595`
    // (`+14h`, `[24Eh]`, `0x015A4`), `0x015BC` (`+18h`, `[26Eh]`,
    // `0x015CB`), `0x015E3` (`+8`, `[30Eh]`, `0x015F2`): descriptor `+4`.
    centered: [(0x01EA, 1), (0x024A, 5), (0x026A, 6), (0x030A, 2)],
    // `0x1755D`: `mul word [13BAh]`.
    seed: 0x13BA,
    // `0x1649E`: `mov ax, [358h] ; xchg [35Ah], ax`.
    timeout: 0x0358,
    // `0x0162E`: `push word [2AAh]`.
    interval: 0x02AA,
    // `0x17231` (`mov cx, [2A0h]`), `0x17239` (`[2A4h]`); Dunkle Schatten `0x185D1`.
    limits: 0x02A0,
    // `0x1722B`, `0x17246`.
    position: 0x037C,
    // `0x016A8`: `mov [13D4h], ax`.
    mode: 0x13D4,
    // `0x1766F`, `0x17661`, `0x1750B`.
    dirty: 0x028C,
    state: 0x02AC,
    drawn: 0x0384,
    // `0x09158`: `push 1F16h ; push 2Ah ; call far 163Ch:0210h`. The
    // region's bytes are Dunkle Schatten's (`… 84 00 40 01 44 00`).
    menu_region: 0x1F16 * 16 + 0x2A,
    keys: Some(KeyMap {
        // `0x165E`–`0x1662`: `push ds ; push 1BEh ; call far 163Ch:055Fh`.
        list: 0x01BE,
        // `cs:04C2h`, `0x16982`: `cmp byte [cs:4C2h], 0`.
        switch: 0x163C0 + 0x04C2,
        // `0x169B6`: `mov di, 138Ah`.
        speeds: 0x138A,
        // `0x169C1`–`0x169C8`: `[35Eh]`, `[362h]`, `[360h]`.
        limits: 0x035E,
        // `0x1716C`: `mul word [36Eh]`.
        acceleration: 0x036E,
        // `0x17219`: `idiv word [370h]`; `0x171D9`: `idiv word [372h]`.
        divisors: 0x0370,
    }),
};

/// Where `XP01.EXE` keeps the music's and the effects' words and tables.
pub(crate) const MUSIC: MusicMap = MusicMap {
    // 0x0D63D (1 in the image).
    switch: 0x01A8,
    // 0x01716, 0x01816: `mov [1AAh], ax`.
    loops: 0x01AA,
    // 0x0D644: `mov ax, [1ACh]`.
    kept: 0x01AC,
    // 0x02F53, 0x02F57: `mov [1E2Ah], dx ; mov [1E28h], ax`.
    room_song: 0x1E28,
    // 0x0CD2F: `cmp word [bx+0F38h], 0`, for slots below `13h` (`cmp ax,
    // 13h`, `0x0CD26`), which the library's reading takes.
    slot_songs: 0x0F38,
    // `mov ax, 1F1Dh` (0x0FA8B, 0x0FAE3): the segment the global rules sit
    // in too, from `0080h` on.
    effect_seg: 0x1F1D,
    // 0x0FA90: `mov al, es:[si+4Ch]`.
    effect_rows: 0x004C,
    // The masks from `1F1Dh:0000h`, two words a room slot: `shl bx, 2`
    // (`0x0FAE0`), `mov ax, es:[bx+2] ; mov dx, es:[bx]` (`0x0FAE8`,
    // `0x0FAED`); how many slots and rooms, and the mask's width, are the
    // library's reading of the loader.
    effect_masks: 0x0000,
};

/// Where `XP01.EXE` keeps what the logic shares with the engine.
pub(crate) fn ds_map(locations: &Locations, inventory: (u16, u16, u16)) -> DsMap {
    DsMap {
        // The main loop's three words around the location the dispatch
        // reads (`0x0077D`): `[1A5Ah]`, `[1A5Ch]`, `[1A5Eh]`.
        entry: locations.location.wrapping_sub(2),
        location: locations.location,
        here: locations.location.wrapping_add(2),
        walker: WALKER,
        // `[16F4h]`, the hundredths the options' dissolve waits on (`cmp
        // word [16F4h], 64h`, `0x0D408`; Dunkle Schatten `0x120CE`), and `[16E8h]`,
        // the ticks not yet taken, which the frame pass reads first (`mov
        // dx, [16E8h]`, `0x0A223`; Dunkle Schatten `0x0ED6E`).
        hundredths: 0x16F4,
        ticks_taken: 0x16E8,
        // `0x10049`: `add ax, 1A68h` after `shl ax, 5`, in the save.
        object_states: 0x1A68,
        // `0x08E06`: `push ds ; push 11D2h ; call far 0EEBh:0031h`; two far
        // pointers, then the word `03E8h` (see `reading`).
        substitutions: 0x11D2,
        substitution_count: 2,
        // The rule matcher `0F15h:009Ch` and `0F15h:0141h` (Dunkle Schatten
        // `13D0h:009Ch`, `13D0h:0141h`): `mov cx, [1E58h]` (`0x0F1F4`),
        // `mov bx, [1E5Ch]` (`0x0F203`), `mov bx, [1E5Eh]` (`0x0F20C`),
        // `cmp ax, [1E60h]` (`0x0F2CB`), `cmp ax, [1E62h]` (`0x0F2D3`).
        // The mode reset clears them in Dunkle Schatten's order (`0x0A0BA`–`0x0A0C6`).
        verb: 0x1E58,
        object: 0x1E5C,
        object_class: 0x1E5E,
        second: 0x1E60,
        second_class: 0x1E62,
        // `0x09006`: `cmp byte [bx+1D0Ah], 0`; `0x08DE2`: `mov byte
        // [bx+1D0Ah], 1` (Dunkle Schatten `0x0DB39`, `0x0D915`).
        flags: 0x1D0A,
        // `0x07F41`: `mov bx, [13A4h]` (Dunkle Schatten `0x0CA84`).
        input_event: 0x13A4,
        // `0x0A6FE`: `call far [1E86h]`, the gate the machine asks.
        gate: 0x1E86,
        // `0x0B1CE`: `mov word [1E5Ah], 7`; `0x0B1C8`: `and word [1E66h], 0FE0Fh` (Escape's arm).
        mode: 0x1E5A,
        bits: 0x1E66,
        // `0x0B0AA`: `mov [1E68h], ax ; mov [1E58h], ax` as a verb is chosen.
        verb_chosen: 0x1E68,
        two_object: 0x1E64,
        // `0x10C4F`: `mov word [1E58h], 0Ah` in the door's gate; `0x1119D` in
        // the script exit's.
        walk_verb: 0x0A,
        // `0x030FB`: `mov [1E1Ch], dx` in the room loader (Dunkle Schatten `0x079BC`).
        room_header: 0x1E1C,
        // `0x0A26B`: the entry clears it; the door gate reads it (`mov ax,
        // [1990h]`, `0x10FF3`; Dunkle Schatten `0x15B58`). The entry never sets it.
        entry_door: 0x1990,
        // `mov [1E6Ah], ax`, the walk verb's rule (`0x11037`) and the
        // entry (`0x0A407`).
        walk_target: 0x1E6A,
        // `mov word [1E8Ch], …`, the door rules (`0x1108B`, `0x1112E`).
        anim_wait: 0x1E8C,
        status_age: 0x1E8E,
        // `0x0AAB9`: `mov al, [si+049Fh]`; items 18 to 25, the Bits, are
        // class 5.
        item_classes: 0x049F,
        // `0x0821C` (`mov al, [1138h]`), `0x07F79` (`[1139h]`), `0x08226`
        // (`[bx+113Ch]`), as this title's reading takes them.
        inventory,
        save_buffer: SAVE_BUFFER,
        // `0x09506`: `idiv word [47Eh]`, the record's line height, after
        // `mov ax, [13B4h]` (`0x09502`; Dunkle Schatten `0x0E061`/`0x0E065`). The
        // record at `DS:0474h` (`0x200B4`) ships Dunkle Schatten's geometry: 5 lines,
        // x 5, y `84h`, width `136h`, line height `0Dh`.
        menu: 0x0474,
        // The shared hook's words: its code-0 arm sets `[18C6h]`, `[18C4h]`
        // to `FFFFh` and clears `[18C8h]`, `[18CCh]` (`0x0B924`–`0x0B936`),
        // Dunkle Schatten's `[1E54h]`, `[1E52h]`, `[1E56h]`, `[1E5Ah]` (`0x10515`–
        // `0x10527`); the row starts two bytes before.
        script: Some(hook::W18C2),
        // No word asks the dispatcher for a task: the hook's event table at
        // `cs:1CC0h` has 29 events and none that would set one (50–52,
        // 55–57, 59, 90–107, 110, 200–202), and the dispatcher has no head
        // that would read one.
        script_start: None,
        gates: GateMap {
            // `0x111D9`, `0x111DF`: a script exit installs `10C3h:0564h`.
            script_exit: (0x10C3, 0x0564),
            // `0x10FE7`, `0x10FED`: the walk verb's rule installs
            // `10C3h:000Ch`.
            door: Some((0x10C3, 0x000C)),
            // `0x0A55B`, `0x0A561` (the sixth verb's rule) and `0x0B1B6`,
            // `0x0B1BC` (Escape): `0A01h:04DAh`, the game options.
            options: (0x0A01, 0x04DA),
        },
        // The title's own rules clear the gate to `0:0` (`0x12200`,
        // `0x12AEB`, `0x12B29`), `Gate::None`; no other far pointer is
        // written into `[1E86h]`.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::titles::parsec1994::Profile;
    use crate::titles::parsec1994::onward::Table;
    use crate::titles::parsec1994::profile::{
        Callback, Effects, FigureSet, RecordCodes, SecondWanted, SlotStyle, StripRange, TextFlags,
    };

    /// The options' dissolve leaves the generator where the original's left
    /// it on the reference machine: `C799h:5296h` at `1D77h:0004h` after the
    /// loop, from `0000:0000` before it.
    #[test]
    fn the_dissolve_draws_are_the_reference_machine_s_count() {
        let mut rng = crate::titles::rng::LcRng::default();
        for _ in 0..BUILD.dissolve_draws {
            rng.inclusive(0, 0x5000);
        }
        assert_eq!((rng.hi, rng.lo), (0xC799, 0x5296));
    }

    /// Each of the options' inks is where its routine sets it: a
    /// `mov byte es:[bx+4], ink` (`26 C6 47 04`) on the font record, and the
    /// ink the build's own.
    #[test]
    fn the_options_inks_are_the_routines_immediates() {
        let Some(adv) = crate::titles::capzins::adventure_for_test() else {
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
        assert_eq!(inks, [0xFE, 0xFE, 0xF0, 0xF0, 0xFE]);
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

    /// The entry's end asks for the pair's kind and slides every room in
    /// with its song (`0x0A12F`, `0x0A134`).
    #[test]
    fn the_entry_slides_every_room_in() {
        let Some(adv) = crate::titles::capzins::adventure_for_test() else {
            return;
        };
        let p = Profile::read(&adv.image, &LIBRARY).expect("reads");
        let slide = crate::titles::parsec1994::profile::EntryLook {
            effect: Some(crate::titles::parsec1994::profile::EntryEffect::Slide),
            song: true,
        };
        assert_eq!(
            p.entry_end,
            crate::titles::parsec1994::profile::EntryEnd {
                location_word: None,
                locations: [None; 2],
                pairs: [None; 3],
                otherwise: slide,
            }
        );
    }

    /// The record hook is never installed: the entry copies it from the
    /// row's `+2Fh` into `[1E3Ch]` (`0x0A36F`–`0x0A37D`, the word's only
    /// store), and every row is null there. Nor does a room file hold a
    /// record the animator or the drawer would hand it — a state above
    /// the step's arms, or bit 4 of `+2` — so the arms past them are
    /// never taken.
    #[test]
    fn no_record_reaches_the_record_hook() {
        let Some(adv) = crate::titles::capzins::adventure_for_test() else {
            return;
        };
        use crate::titles::parsec1994::image_records::tests::{hooked_records, row_far_pointers};
        assert_eq!(row_far_pointers(&adv, 0x2F), []);
        assert_eq!(hooked_records(&adv, adv.profile.anim_arms), []);
    }

    /// What the visit's routines do with the objects, the tasks, Escape,
    /// the verbs, the entry and the strip, read off them.
    #[test]
    fn the_visit_s_routines_are_read() {
        let Some(adv) = crate::titles::capzins::adventure_for_test() else {
            return;
        };
        let p = Profile::read(&adv.image, &LIBRARY).expect("reads");
        assert_eq!(p.tri_state_kinds, 0b11_0010);
        assert!(p.task_start_needs_armed);
        assert_eq!(p.escape_verb, 6);
        assert!(p.verb6_arm);
        assert_eq!(p.second_wanted, SecondWanted::Verbs);
        assert!(p.place_falls_back);
        assert!(p.show_hides_parts && p.hide_clears_head);
        assert!(p.own_line_sets_speakers);
        assert_eq!(p.status_after, 2);
        assert_eq!((p.interaction.items_end, p.interaction.verb), (0x1a, 2));
        assert!(p.kind8_records);
        assert_eq!(p.anim_arms, 0x0B);
        {
            // `0A01h:1D48h`, the table at `cs:1DE7h` (`0x0BDF7`): no sample
            // for code 3, sample 14h for 9 (`0x0BDAF`), and the band let go
            // for owner 8 for 10 (`078Ah:0666h`, `0x0BDB9`).
            use crate::titles::parsec1994::signal::SignalAct::{
                BandRelease, FigureAnimation, Release, Sample, TaskArm, TaskStart,
            };
            let n = vec![];
            assert_eq!(
                crate::titles::parsec1994::signal::tests::listed(&p.signal),
                [
                    (1, vec![Release(1)], n.clone()),
                    (2, vec![], n.clone()),
                    (3, vec![], n.clone()),
                    (4, vec![], n.clone()),
                    (5, vec![TaskArm(0)], n.clone()),
                    (6, vec![TaskStart(0)], n.clone()),
                    (7, vec![], n.clone()),
                    (8, vec![Sample(3)], n.clone()),
                    (9, vec![Sample(0x14)], n.clone()),
                    (10, vec![BandRelease(8)], n.clone()),
                    (11, vec![FigureAnimation], n.clone()),
                    (12, vec![FigureAnimation], n),
                ]
            );
        }
        {
            use crate::titles::parsec1994::profile::{OptionsGate, OptionsHeld};
            assert_eq!(
                p.options_gate,
                OptionsGate {
                    drops_ticks: true,
                    held: Some(OptionsHeld {
                        bit: 8,
                        here_word: 0x1A5E,
                        silent_at: 0,
                        scene: !0x4B,
                        wait: 0x10
                    })
                }
            );
        }
        assert_eq!(p.inventory_rows, None);
        let sc = p.scenes;
        assert_eq!(
            (sc.item_base, sc.bounded, sc.falls_back),
            (0x4000, true, true)
        );
        assert_eq!(p.hotspot_kept, 0b1_1110);
        let r = p.records;
        let records = (r.codes, r.read, r.fresh);
        assert_eq!(
            records,
            (RecordCodes::Hidden, [Some(1), Some(4), Some(5)], None)
        );
    }

    /// The status line's inks and the band under a choice menu, read off
    /// the status painter and the menu's strip redraw.
    #[test]
    fn the_status_band_is_the_one_the_runtime_was_read_from() {
        let Some(adv) = crate::titles::capzins::adventure_for_test() else {
            return;
        };
        let p = Profile::read(&adv.image, &LIBRARY).expect("reads");
        assert_eq!(
            (p.status_inks.ink, p.status_inks.controls),
            (0xF4, [(0x09, 0xFA), (0x0A, 0xFC), (0x0D, 0xF4)]),
            "`0x07C0D`, `0x07D6F`"
        );
        assert_eq!(p.menu_band, None, "`0x07D7F`–`0x07E2D` holds nothing dark");
    }

    /// The library build `XP01.EXE` is linked against, read off its
    /// routines: a build without the hover's resets, the task start's hook
    /// call, the screen slot, the remembered door and the forest, with a
    /// callback, two sets of strip and figure, six carried items to a
    /// window, 45 effect slots and slot words in fields of their own.
    #[test]
    fn the_library_build_is_the_one_the_runtime_was_read_from() {
        let Some(adv) = crate::titles::capzins::adventure_for_test() else {
            return;
        };
        let p = Profile::read(&adv.image, &LIBRARY).expect("reads");
        assert!(!p.lock_forgets_hover);
        assert!(!p.unlock_forgets_hover);
        assert!(!p.hover_off_forgets_hover);
        assert!(!p.task_start_tells_hook);
        assert!(!p.script_exit_hovers_off);
        assert_eq!(p.leave_fade, 2000);
        assert_eq!((p.leave_spares, p.leave_clears_status), ([None; 3], true));
        assert!(!p.mode_step_ages_status, "`0x0AA62`");
        assert_eq!(p.entry_fade_in, Some(6000));
        assert!(!p.hover_on_forgets_hover);
        assert!(!p.hover_on_polls);
        assert_eq!(
            p.object_state,
            crate::titles::parsec1994::profile::ObjectState::Modes {
                when_mode_one: false
            }
        );
        assert!(!p.walk_to_node_remembers);
        assert!(!p.anim_checks_count);
        assert_eq!(p.anim_wakes_only_kind, None);
        assert!(!p.dispatch_starts_asked_task);
        assert!(p.forgotten_hover_is_nothing);
        assert!(!p.room_click_forgets_hover);
        assert!(!p.hover_off_only_notes);
        assert!(!p.verb6_refreshes_status);
        assert_eq!(p.verb_bound, 0x0D);
        assert!(!p.locked_stops_hover_text);
        assert!(!p.mode_reset_forgets_hover);
        assert_eq!(
            p.rooms,
            crate::titles::parsec1994::profile::SharedRooms {
                sound_flags: p.rooms.sound_flags,
                ..Default::default()
            }
        );
        assert_eq!(p.screen_slot, None);
        let d = p.entry_door;
        let door = (d.ids, d.kind, d.remembered, d.frame, d.more);
        assert_eq!(door, ((100, 109), 4, false, -1, [None; 3]));
        assert_eq!(p.inventory_window, 6);
        assert_eq!((p.status_row, p.status_band), (130, 12));
        assert_eq!(
            p.callback,
            Some(Callback {
                word: 0x1980,
                default: (0x0A01, 0x0001),
                ids: (0x12, 0x19),
                skips_location: 6,
            })
        );
        assert_eq!(
            p.slot_style,
            Some(SlotStyle {
                vertical: [(0, 0), (0x10, 2), (0x20, 1), (0x30, 3)],
                horizontal: Some([(0x40, 4), (0x80, 8)]),
            })
        );
        assert_eq!(
            p.effects,
            Effects {
                slots: 45,
                room_first: 14,
                rooms: 0x13,
                wide_masks: true,
            }
        );
        assert_eq!(p.song_slots, 0x13);
        assert_eq!(
            p.figure_set,
            FigureSet::AfterLocation {
                word: 0x1A5C,
                above: 2
            }
        );
        assert_eq!(
            p.figure_set.digits(),
            [0, 1],
            "two sets of strip and figure"
        );
        assert_eq!(p.figure_set.digit_for(2), 0);
        assert_eq!(p.figure_set.digit_for(3), 1);
        assert_eq!(p.figure_first, 1, "the palette stepped past, `0x0BF02`");
        assert_eq!(p.strip, StripRange::Split(6));
        assert_eq!(
            p.text_flags,
            TextFlags {
                hold: 1,
                firm: 2,
                quick: None
            }
        );
        assert_eq!(adv.profile, p);
    }

    /// The numbers the generation's routines compare with, which the
    /// runtime takes from what it read rather than from here: each is
    /// still the immediate this program has at that place.
    #[test]
    fn the_counts_the_runtime_reads_are_the_program_s_immediates() {
        let Some(adv) = crate::titles::capzins::adventure_for_test() else {
            return;
        };
        let byte = |at: usize| adv.image.bytes()[at];
        // `cmp …, 1Ah` where an item is told from a room's object (25
        // names and one); the immediate is the instruction's last byte.
        for at in [0x08207, 0x10E68, 0x10EE4] {
            assert_eq!(u16::from(byte(at)), adv.items_end(), "{at:#x}");
        }
        // `cmp …, 14h` where a location word is held inside the table.
        for at in [0x0074F, 0x00B21, 0x00B26] {
            assert_eq!(usize::from(byte(at)), adv.locations.count, "{at:#x}");
        }
        // `cmp ax, 3`, the kind the walk on steps over beside 0.
        let imm = |at: usize| adv.image.word(at + 1).unwrap();
        for at in [0x00B19, 0x00B40] {
            assert_eq!(imm(at), KINDS.restore, "{at:#x}");
        }
        // `mov word [1A5Ch], 13h`, the load sending the main loop on.
        assert_eq!(adv.image.word(0x1010B).unwrap(), SAVE.restore_row);
    }

    /// The walk on reads the same table the dispatch was read from.
    #[test]
    fn the_walk_on_reads_the_dispatch_s_table() {
        let Some(adv) = crate::titles::capzins::adventure_for_test() else {
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

    /// The choice menu's geometry, read out of the record at `DS:0474h`:
    /// Dunkle Schatten's.
    #[test]
    fn the_menu_s_geometry_is_read_from_its_record() {
        let Some(adv) = crate::titles::capzins::adventure_for_test() else {
            return;
        };
        let m = adv.menu_shape;
        assert_eq!(
            (m.lines, m.x, m.y, m.width, m.line_h),
            (5, 5, 0x84, 0x136, 0x0D)
        );
    }
}
