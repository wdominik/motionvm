//! What *Bi-Fi 2*'s `XP01.EXE` hands the `parsec1994` runtime: where
//! this build keeps what each part of the runtime reads, one map per part,
//! with the instruction each value is read from beside it.
//!
//! The maps' types are the generation's (`titles/parsec1994/`); their
//! values are this program's and stand here. Every address in the comments
//! is an image offset of the unpacked `XP01.EXE` (164576 bytes, `DGROUP`
//! `250Ah`, image `0x250A0`), linear (`seg * 16 + off`). Where this build's
//! library does a thing differently from Dunkle Schatten's and Captain
//! Zins's, the runtime reads it off the routine
//! ([`crate::titles::parsec1994::Profile`]).

use super::xp01data::ExeText;
use super::{hook, logic};
use crate::titles::parsec1994::program::{Build, Names, PanelReading, Reading, Sound};
use crate::titles::parsec1994::{
    Block, DsMap, GateMap, KeyMap, Kinds, LibraryMap, MusicMap, OptionInks, OptionsMap, PointerMap,
    SaveMap,
};
use crate::titles::xp1994::Locations;

/// `DS:176Eh`, the player's walker record: the save's fourth block copies
/// `30h` bytes from `push 176Eh` (`0x0F336`), the restore copies them back
/// (`0x0F3B6`), and the logic hands it to the walker as `push ds ; push
/// 176Eh` (`0x10E64`–`0x10E68`, before `call far 05DCh:05C8h`).
pub(crate) const WALKER: u16 = 0x176E;

/// `2374h:0000h`, the in-memory saved game: `mov word [bp-2], 2374h` at
/// the start's load (`0x0EEDE`), the write (`0x0F0C1`), the save
/// (`0x0F2D6`) and the restore (`0x0F391`).
pub(crate) const SAVE_BUFFER: u16 = 0x2374;

/// `XP01.EXE`, as the generation's runtime would open it.
pub(crate) static BUILD: Build = Build {
    // The file on disk is packed; unpacked it is 164576 bytes.
    program: ("XP01.EXE", 164_576),
    names: Names {
        // `0x00A27`: `push 0B6Ah`, `RESOURCE\BFROOM.DAT`.
        rooms: "BFROOM.DAT",
        // `0x073CD`: `push 0B92h`, `RESOURCE\PANELS.DAT`, in the strip's
        // loader `073Ch:0007h`, whose one caller is the start (`0x00657`):
        // the name has no `%` and the loader puts no digit into it.
        panel: "PANELS.DAT",
        // `0x01370`: `push 0B12h`, `RESOURCE\CMBMOU.DAT`.
        pointers: "CMBMOU.DAT",
        // `0x0B75F`: `push 0BA6h`, `RESOURCE\PLAYER.DAT`, in the figure's
        // loader `0B73h:0003h`; no `%`, no digit (see the profile's
        // `figure_set`: the visit's setup pushes `-2`, a keep-if-loaded
        // flag that the loader tests with `jl`, `0x0B74E`).
        player: "PLAYER.DAT",
        // `0x009D5`: `push 0BBAh`, `RESOURCE\DIALOG.DAT`.
        dialog: "DIALOG.DAT",
        // `0x0E5EE`: `push 0B3Ah`, `RESOURCE\PSMP2L.DAT`.
        music: "PSMP2L.DAT",
        // `DS:0AFEh`, the drivers' RIB (`push 0AFEh`, `0x011A6`); the music
        // driver is its resource 5 (`push 5`, `0x0129E`, then `call far
        // 1975h:0283h`, `0x012AE`).
        drivers: "PSMDRV.DAT",
    },
    // The options' dissolve loop is
    // `0x0CC25` (`mov word [1CF6h], 0`) to `0x0CC77` (`jc` back after
    // `cmp word [1CF6h], 64h`, `0x0CC72`); each pass draws once from
    // `203Dh:0024h` with `push 5000h ; push 0` (`0x0CC2B`–`0x0CC30`) and
    // keeps the draw in `[bp-2]` (`0x0CC38`). The generator's seed is the
    // code-segment pair `203Dh:0002h` (low) / `203Dh:0004h` (high), set by
    // `203Dh:0006h` (`0x203E8`, `0x203ED`). Count the steps between the
    // seed before and after the loop of the options `XP01` opens at its
    // start, on DOSBox-X `core = normal`, `cycles = fixed 25000`, the game
    // started the way `BIFI.BAT` starts it, `XP00` and then `XP01` after
    // a new game (clicked by the resident `DOSINPUT.COM`, with which
    // `XP01` started on its own takes 158 523 against 158 526 without):
    // `0000:0000` before, `336Eh:4008h` after, 158 808 steps
    // of the generator. Nothing hands `XP01` a seed: the only callers of
    // `203Dh:0006h` are the settings' and the saved game's readers and
    // the saver (`0x0EE86`, `0x0EFF3`, `0x0F0EB`), and the seed's two
    // words ship as zero (`0x203D2`).
    dissolve_draws: 158_808,
    library: LIBRARY,
    kinds: KINDS,
    save: SAVE,
    options: OPTIONS,
    pointer: POINTER,
    music: MUSIC,
    sound: Sound {
        // `0x01306`: `push 6`, the channels the digital driver is
        // installed with (`call far 1975h:0469h`, `0x0131F`).
        channels: 6,
        // `0x0064C`: `push 3Ch`, then `call far 1975h:044Ah` (`0x0064E`),
        // the effects' volume after the init (the init's own `push 50h`,
        // `0x01346`, is the driver's default before it).
        effect_volume: 0x3C,
    },
    logic: logic::TABLE,
    hook: hook::answer,
    // The visit's setup opens with `push -2` (`0x09B64`), not with the
    // two stores that put a callback back, and the entry has no loop that
    // asks through one: the build keeps no callback.
    callbacks: &[],
    // After its main loop the program writes the settings (`call far
    // 0EE6h:0371h`, `0x007AD`) and goes to the developer's switches
    // (`test word [1F38h], 8`, `0x007B2`) or ends: it shows nothing. The
    // game's end is a location's: the Kaffee's entry 6 calls `0D84h:0008h`
    // (`0x10942`) and returns with the location word at `FFFFh`, which no
    // other path leaves; the runtime plays that sequence here.
    ending: super::ending::open,
    // The Kaffee's handler shows the end (`0x10942`) inside the main loop;
    // `_main` writes the settings after it (`0x007AD`).
    settings_after_ending: true,
};

/// Where `XP01.EXE` keeps the library routines the runtime reads its build
/// off, as `(segment, offset)`, with the image offset and a caller beside
/// each. Pforte's handler (`0x10D56`–`0x10EB1`) names the visit's five.
const LIBRARY: LibraryMap = LibraryMap {
    // `or word [2D46h], 2 ; push 4 ; call far 0751h:057Dh`.
    input_lock: (0x09B6, 0x17D4), // 0x0B334; called at 0x0FE4D, 0x1064A, … and near at 0x0AC6A
    // `and word [2D46h], FFFDh ; push 4 ; call far 0751h:0655h`.
    input_unlock: (0x09B6, 0x17E2), // 0x0B342; the door gate calls it (0x0FF10)
    // `mov word [1ED6h], 0 ; call far 1819h:0B02h ; call far 1819h:0B9Ah`.
    hover_off: (0x09B6, 0x1826), // 0x0B386; called at 0x1087D, 0x109ED, …
    // `push bp ; mov bp, sp`, task `[bp+6]`: `and word [bx+1E32h], 7FFFh`,
    // `or word [bx+1E2Eh], 1`, near call, `pop bp ; retf`, with no armed
    // test before it (Captain Zins `test word [bx+182Ah], 8000h ; jz`,
    // `0x09EBB`).
    task_start: (0x080C, 0x1950), // 0x09A10; called at 0x0B5FB, 0x1203D, …
    scene_bind: (0x080C, 0x0579), // 0x08639
    // `push bp ; mov bp, sp ; mov word [2D68h], 0FE3h ; mov word [2D66h],
    // 0848h …`, Captain Zins's shape.
    script_exit: (0x0FE3, 0x088A), // 0x106BA; called at 0x10BB5, 0x12086, …
    visit_finish_entry: (0x09B6, 0x00BB), // 0x09C1B; Pforte 0x10E49
    // `0x09C5F`, `0x09C68`: the two effects the entry's end calls.
    entry_reveal: (0x0B7E, 0x039F), // 0x0BB7F
    entry_slide: (0x0B7E, 0x0C80),  // 0x0C460
    visit_leave: (0x09B6, 0x018E),  // 0x09CEE; Pforte 0x10EAC
    // `mov word [1ED6h], 1` and three far calls, Captain Zins's shape; no
    // far call names it in the program (nor in Captain Zins's).
    hover_on: (0x09B6, 0x1837), // 0x0B397
    // Kind 4 answers bit 1 of `+2`, every other kind 3 (the profile's
    // `ObjectState::Bit1`).
    obj_state: (0x0F48, 0x074B), // 0x0FBCB; called at 0x1008E, 0x1011D, …
    // The program has no walk-to-node routine: the logic writes the walk
    // target itself and calls the walker directly (`mov [2D4Ah], ax`,
    // `0x16FE9`, then `call far 05DCh:05C8h`, `0x16FF1`).
    walk_to_node: None,
    obj_anim: (0x03F7, 0x1CF9),        // 0x05C69; Pforte 0x10DB6
    objs_save_state: (0x0F48, 0x0611), // 0x0FA91; Pforte 0x10EA4
    objs_load_state: (0x0F48, 0x0260), // 0x0F6E0, from the setup at 0x09B99
    hotspot_follow: (0x0F48, 0x06B9),  // 0x0FB39
    obj_show: (0x0F48, 0x0784),        // 0x0FC04
    obj_hide: (0x0F48, 0x08AA),        // 0x0FD2A
    say_text: (0x080C, 0x17D9),        // 0x09899
    options_gate: (0x09B6, 0x0465),    // 0x09FC5
    dispatch: (0x09B6, 0x0FB3),        // 0x0AB13; Pforte 0x10E88
    // Reached by near calls only (`0x09CF3`, `0x0A016`, …).
    mode_reset: (0x09B6, 0x0042),    // 0x09BA2
    second_wanted: (0x09B6, 0x0768), // 0x0A2C8
    // Called by the visit's setup (`0x09B70`).
    room_loader: (0x02D5, 0x010A), // 0x02E5A
    // Called by the room loader (`0x02ECA`), the entry's end (`0x09C49`),
    // the leaving (`0x09D13`).
    place_pair: (0x0F48, 0x082B),  // 0x0FCAB
    slot_unload: (0x02D5, 0x0D62), // 0x03AB2; Pforte 0x10EB1
    visit_enter: (0x09B6, 0x0212), // 0x09D72; Pforte 0x10D66
    // Called at 0x0B402, 0x0FF25, …; the shared hook's code 4 (0x0B402).
    inv_take: (0x0751, 0x099D),    // 0x07EAD
    inv_give: (0x0751, 0x090E),    // 0x07E1E
    visit_setup: (0x09B6, 0x0004), // 0x09B64; Pforte 0x10D56
    // `and bx, 0Fh` at `0x08374`; reached by near calls only (`0x08594`,
    // `0x087EA`, `0x089E5`); it has no horizontal field.
    slot_resolve: (0x080C, 0x02A4), // 0x08364
    // Called by the room loader (`0x02F24`, `0x03A82`).
    effects_load: (0x0EC8, 0x00FA), // 0x0ED7A
    // Called by the room loader (`0x02F16`, `0x03A66`).
    slot_song: (0x0C59, 0x002E), // 0x0C5BE
    // Called once, from the start (`0x00657`); no carried range.
    strip_load: (0x073C, 0x0007), // 0x073C7
    // Its one caller is `0x087D7` (near).
    text_scan: (0x080C, 0x01B6), // 0x08276
    // The dispatcher's second far call (`0x0AB1E`), as Captain Zins's
    // (`0x0B021`).
    task_query: (0x080C, 0x0F22), // 0x08FE2
    // Called by the entry's end (`0x09C2E`), the options (`0x0CA3C`), and
    // `0x09D5D`; its band is held dark 48h rows high.
    status_paint: (0x0751, 0x0067), // 0x07577
    status_gate: (0x09B6, 0x1588),  // 0x0B0E8
    mode_step: (0x09B6, 0x098D),    // 0x0A4ED
    // Its one caller is task 0's menu paint (`0x09614`); it clears
    // the line and holds the band dark.
    menu_strip: (0x0751, 0x04A2),    // 0x079B2
    anim_signal: (0x09B6, 0x1A75),   // 0x0B5D5
    task_arm: (0x080C, 0x193F),      // 0x099FF
    page_release: (0x080C, 0x1A86),  // 0x09B46
    play_sample: (0x09B6, 0x184D),   // 0x0B3AD
    interaction: (0x09B6, 0x16C4),   // 0x0B224
    figure_bind: (0x0F48, 0x009F),   // 0x0F51F
    animator_step: (0x03F7, 0x011C), // 0x0408C
};

/// What the runtime takes from this title's own reading of its program.
///
/// The panel's variants go to the upper half of the palette, where the
/// band held dark shows them; its owners' words (`[57Ah]`–`[588h]`) are
/// worked as Dunkle Schatten's (`[4E8h]`–`[4F6h]`) by the same routines
/// (`0751h:057Dh`, `05D6h`, `0635h`, `0655h`). The entry restores no gray
/// ramp.
pub(crate) fn reading(text: &ExeText, locations: &Locations) -> Reading {
    Reading {
        ds_map: ds_map(locations, text.inventory_at),
        templates: text.templates,
        // The row at `DS:1768h` as the image ships it: one far pointer,
        // `"Lukas"`.
        substitutions: text
            .substitutions
            .iter()
            .map(|s| motionvm_parsec_formats::to_cp437(s))
            .collect(),
        initial_items: text.initial_items.clone(),
        initial_scroll: text.initial_scroll,
        panel: PanelReading {
            curves: text.panel_curves,
            place: crate::titles::palette1994::PanelPlace::Upper,
            ramp: None,
            strip_row: Some(text.strip_row),
        },
        start: text.start,
    }
}

/// What the kinds of `XP01.EXE`'s location table mean.
///
/// The walk on (`0060h:04BFh`, `0x00ABF`–`0x00B3F`) is this program's own,
/// not the generation's, and re-expressed in its own module
/// ([`super::onward`]).
pub(crate) const KINDS: Kinds = Kinds {
    // `0x0075E`: `cmp word es:[bx+7], 0 ; jnz`, the main loop walks on for
    // kind 0 (`0x00768`); `0x0FF0A`: `cmp word [bp-2], 0`, a door does not
    // lead there.
    passage: 0,
    // Inferred, not an immediate: row 91 (`5Bh`), the row the load sends
    // the main loop to (`mov word [200Ch], 5Bh`, `0x0F376`), has kind 6 in
    // the table (`2181h:1844h + 7`, image `0x2305B`), and its handler is the
    // restore (`0EE6h:0523h`, `0x0F383`). The walk on steps over it with
    // `dec bx ; cmp bx, 4 ; ja` (`0x00AD4`–`0x00AD8`) — and over kinds 4 and
    // 7 too, which `Kinds` cannot say.
    restore: 6,
};

/// Where `XP01.EXE` keeps its saved game and settings, and their files' shape.
pub(crate) const SAVE: SaveMap = SaveMap {
    // `push 0D15h` at the clear (`0x0EEE8`) and the write (`0x0F19F`),
    // `cmp ax, 0D15h` at the size check (`0x0EFBF`) and after the write
    // (`0x0F1B1`).
    len: 0xD15,
    // `add dx, 14h` (`0x0F022`, `0x0F148`); `mov si, 685h` (`0x0F02B`) and
    // `mov cx, 685h` (`0x0F151`): `0D0Ah` bytes, nine past the file's end;
    // the nine bytes after the buffer (`0x24455`–`0x2445D`) are zero in the
    // image.
    sum: (0x14, 0x685),
    // `BiFiSAV\0`: the four words `0x0F124`–`0x0F135` store and
    // `0x0F067`–`0x0F07D` compare.
    sav_magic: [0x6942, 0x6946, 0x4153, 0x0056],
    // `BiFiCFG\0`: `0x0F1F3`–`0x0F202` and `0x0EF45`–`0x0EF5C`.
    cfg_magic: [0x6942, 0x6946, 0x4643, 0x0047],
    // `cmp ax, 1Dh` at the size check (`0x0EF1F`), `push 1Dh` at the write
    // (`0x0F274`); the check value walks five words from `+12h` (`add dx,
    // 12h`, `0x0EE94`; `mov si, 5`, `0x0EE9D`); the write puts `0` at
    // `+1Ch` (`mov byte [bp-60h], 0`, `0x0F226`) and no walk speed, and the
    // one store to `[1F2Eh]` (`0x0CF09`) is not in the walk-speed arm
    // (`0x0CF12`–`0x0CF2B`).
    cfg_len: 0x1D,
    cfg_words: 5,
    cfg_speed: false,
    // `0x0F376`: `mov word [200Ch], 5Bh`, with `[200Ah] := 1` (`0x0F37C`).
    restore_row: 0x5B,
    blocks: [
        // 0x0F2E0: the logic's state bytes (Pforte's `[2B98h]`, `0x10D6E`).
        Block {
            buf: 0x014,
            ds: 0x2B98,
            out: 0x4E,
            back: 0x4E, // 0x0F3E8
        },
        // 0x0F2F8: 92 object rows of 32 bytes.
        Block {
            buf: 0x062,
            ds: 0x2018,
            out: 0xB80,
            back: 0xB80, // 0x0F400
        },
        // 0x0F311: the location stack.
        Block {
            buf: 0xBE2,
            ds: 0x2010,
            out: 8,
            back: 8, // 0x0F39B
        },
        // 0x0F329: the player's record.
        Block {
            buf: 0xBEA,
            ds: 0x176E,
            out: 0x30,
            back: 0x30, // 0x0F3B3
        },
        // 0x0F341: FAh bytes saved, of which 0x0F419 restores 3Ah.
        Block {
            buf: 0xC1A,
            ds: 0x1662,
            out: 0xFA,
            back: 0x3A,
        },
    ],
    // `0x0F2C5`: `inc word [2010h]`; the restore copies the stack and the
    // player first and the rest after the pop (`0x0F39B`–`0x0F42E`).
    stack: 0x2010,
    // `0x0F0A4`: `cmp word [180Eh], 0`; `0x0F35E` sets it.
    saved: 0x180E,
    // `0x0F091`, `0x0F364` set it; `0x0F36F`, `0x0F387` read it.
    loaded: 0x1810,
    // `0x0F13B`, `0x0F207`: `mov ax, [17E4h]` (`100h` in the image).
    version: 0x17E4,
    // `0x0F1D6`: `cmp word [1F2Eh], 0`; the options set it (`0x0CF09`).
    cfg_dirty: 0x1F2E,
    // `0x0EF7D` (`mov [17CEh], ax`), `0x0F20D`, `0x0F433`.
    mouse: 0x17CE,
    // `0x0F443`: `mov word [47Ch], 0Ah`, `0x0F449`: `mov word [47Ah],
    // 0Ah`, the table at `cs:0624h` with five cases and the fallback
    // (`0x0F471`).
    divisors: (0x047C, 0x047A),
    // `0x0EF88`, `0x0F21B`: `push 175Ch`, eight bytes.
    text: 0x175C,
    // `[1782h]` is the walk-speed byte the options set (`mov [1782h], al`,
    // `0x0D1CE`, from `[bx+154Ah]`); neither the settings' read nor their
    // write touches it.
    speed: 0x1782,
    // `0x0EFA0`, `0x0F0B6`: `push 0B5Ch`, `*SAVEGAME.SAV`.
    sav_name: 0x0B5C,
    // `0x0EF03`, `0x0F1E8`: `push 0B4Eh`, `*SAVEGAME.CFG`.
    cfg_name: 0x0B4E,
    // `0x0F194`, `0x0F269`: `push 8B4h`.
    msg_create: 0x08B4,
    // `0x0F1BC`, `0x0F28F`: `push 8DCh`.
    msg_write: 0x08DC,
    // `0x0F086`: `push 17E6h`.
    msg_invalid: 0x17E6,
};

/// Where `XP01.EXE` keeps what the game options read and write.
///
/// The menu darkens the playfield by setting bit 7 of every pixel
/// (`0DE4h:0123h`, `or byte es:[di], 80h` at `0x0DF88`) against a second
/// palette half the palette loader fills with a converted copy of the first
/// (`0104h:090Eh`, `0x0194E`–`0x019B0`) — not through a flattening table —
/// and draws in the first half: its inks are Dunkle Schatten's less 80h,
/// its grounds the two entries its colors go to, 51h and 50h
/// ([`crate::titles::xp1994::Shade`]).
pub(crate) const OPTIONS: OptionsMap = OptionsMap {
    // 0x0C635: `les bx, [bx+1506h]`; eight rows, the ninth null.
    rows: 0x1506,
    // 0x0C61C: `push 0F0Ah`, `Spieloptionen`.
    title: 0x0F0A,
    // No `push 2 ; push 0C0h ; push ds ; push rgb`: the
    // entry loads the record `DS:152Ah` through the palette loader (`push
    // ds ; push 152Ah ; call far 0104h:090Eh`, `0x0CA6D`–`0x0CA71`): first
    // `50h`, count 2, then the six bytes at `DS:152Ch`; the loader also
    // writes their converted copies at `D0h`, `D1h`. The runtime
    // loads the six bytes at `50h` from this pair as it loads Dunkle
    // Schatten's, and the shade's mirror writes the copies at `D0h`, `D1h`.
    colors: (0x152C, 0x50),
    // 0x0D19B: `mov ax, [bx+1532h]`, the row `8 · [14CEh]` (`0x0D190`–
    // `0x0D199`).
    text_speeds: 0x1532,
    // 0x0D1CA: `mov al, [bx+154Ah]`, indexed by `[14D8h]` (`0x0D1C6`).
    walk_speeds: 0x154A,
    // 0x0CF38, 0x0CF5A, 0x0CF7F, 0x0CF91: `push 1486h`, `1492h`, `149Eh`,
    // `14AAh`.
    confirms: [0x1486, 0x1492, 0x149E, 0x14AA],
    // 0x0CEA7: `xor word [2B2h], 1`.
    music: 0x02B2,
    // 0x0CF1C: `test word [1F38h], 10h`.
    switches: 0x1F38,
    save: SAVE,
    inks: OptionInks {
        // `26 C6 47 04 74`: 74h (Dunkle Schatten F4h).
        head: 0x0C616,
        // `… 7E`: 7Eh (FEh).
        question: 0x0C86E,
        // `… 78`: 78h (F8h).
        yes: 0x0C922,
        // `… 7A`: 7Ah (FAh).
        no: 0x0C948,
        // `… 7E`: 7Eh (FEh).
        hover: 0x0CDB6,
    },
    // 0x0C5EB: `push 51h`, the playfield (`push 80h ; push 140h ; push 0 ;
    // push 0 ; call far 1B72h:00D2h`); 0x0C5FF: `push 50h`, the band from
    // row 0Fh, 62h rows high — the two entries `DS:152Ah` loads.
    grounds: [0x0C5EB, 0x0C5FF],
    // 0x0C811: `push 7Eh`, 0x0C836: `push 7Fh`, 0x0C8EA (and 0x0C90C):
    // `push 6Eh`; the box's own grounds are the menu's, `push 51h`
    // (0x0C857) and `push 50h` (0x0C8C2).
    box_inks: [0x0C811, 0x0C836, 0x0C8EA],
    // 0x0CA3C: the status painter, then 0x0CA43 the hold.
    paint_before_hold: true,
};

/// Where `XP01.EXE` keeps the pointer's words and tables. The pointer
/// library `1819h` is Captain Zins's `163Ch` with every site `1DD0h`
/// further on.
pub(crate) const POINTER: PointerMap = PointerMap {
    // `0104h:0640h` (`0x01680`), the table at `cs:0694h`: `push 2D4h`
    // (0x016A4), `314h` (0x016B6), `354h` (0x016C0), `374h` (0x016CA), each
    // copied to `push 3D4h` (0x016A0) by `1819h:0BE0h` (0x016A7).
    modes: [0x02D4, 0x0314, 0x0354, 0x0374],
    slot: 0x03D4,
    // `0x015FB`: `mov word [454h], 2F4h`, after `mov [456h], ds` (0x015F7).
    regions: 0x02F4,
    // `0x01531` (`[19D2h]+4`, stored at `[2F8h]`, `0x0153C`), `0x0157E`
    // (`+14h`, `[358h]`, `0x01589`), `0x015A5` (`+18h`, `[378h]`,
    // `0x015B0`), `0x015CC` (`+8`, `[418h]`, `0x015D7`): descriptor `+4`.
    centered: [(0x02F4, 1), (0x0354, 5), (0x0374, 6), (0x0414, 2)],
    // `0x1932D`: `mul word [19BCh]`.
    seed: 0x19BC,
    // `0x1826E`: `mov ax, [462h] ; xchg [464h], ax`.
    timeout: 0x0462,
    // `0x01613`: `push word [3B4h]`, before `call far 19C8h:1402h`.
    interval: 0x03B4,
    // `0x19001` (`mov cx, [3AAh]`), `0x19009` (`[3AEh]`).
    limits: 0x03AA,
    // `0x18FFB`, `0x19016`.
    position: 0x0486,
    // `0x0168D`: `mov [19D6h], ax`.
    mode: 0x19D6,
    // `0x1943F`, `0x19431`, `0x192DB`.
    dirty: 0x0396,
    state: 0x03B6,
    drawn: 0x048E,
    // `0x08CC4`: `push 232Fh ; push 2Ah ; call far 1819h:0210h`. The
    // region's bytes (`0x2331A`) are Captain Zins's (`… 84 00 40 01 44 00`).
    menu_region: 0x232F * 16 + 0x2A,
    keys: Some(KeyMap {
        // `0x01643`–`0x01647`: `push ds ; push 2C8h ; call far 1819h:055Fh`.
        list: 0x02C8,
        // `cs:04C2h`, `0x18752`: `cmp byte [cs:4C2h], 0`.
        switch: 0x18190 + 0x04C2,
        // `0x18786`: `mov di, 198Ch`.
        speeds: 0x198C,
        // `0x18791`–`0x18798`: `[468h]`, `[46Ch]`, `[46Ah]`.
        limits: 0x0468,
        // `0x18F3C`: `mul word [478h]`.
        acceleration: 0x0478,
        // `0x18FE9`: `idiv word [47Ah]`; `0x18FA9`: `idiv word [47Ch]`.
        divisors: 0x047A,
    }),
};

/// Where `XP01.EXE` keeps the music's and the effects' words and tables.
pub(crate) const MUSIC: MusicMap = MusicMap {
    // 0x0CEA7 (1 in the image).
    switch: 0x02B2,
    // 0x016FB: `mov [2B4h], ax`; the switch pushes it (`0x0CEF2`).
    loops: 0x02B4,
    // 0x0CEAE: `mov ax, [2B6h]`.
    kept: 0x02B6,
    // 0x02F1C, 0x02F20: `mov [2D0Ah], dx ; mov [2D08h], ax`.
    room_song: 0x2D08,
    // 0x0C5CD: `cmp word [bx+1436h], 0`, for slots below `28h` (`cmp ax,
    // 28h`, `0x0C5C4`).
    slot_songs: 0x1436,
    // `mov ax, 2336h` (0x0ED3B, 0x0ED96): the segment the global rules sit
    // in too, from `0130h` on.
    effect_seg: 0x2336,
    // 0x0ED40: `mov al, es:[si+0A0h]`.
    effect_rows: 0x00A0,
    // The masks from `2336h:0000h`, a double word a room slot: `shl bx, 2`
    // (`0x0ED93`), `mov ax, es:[bx+2] ; mov dx, es:[bx]` (`0x0ED9B`,
    // `0x0EDA0`).
    effect_masks: 0x0000,
};

/// Where `XP01.EXE` keeps what the logic shares with the engine.
pub(crate) fn ds_map(locations: &Locations, inventory: (u16, u16, u16)) -> DsMap {
    DsMap {
        // The main loop's three words around the location the dispatch
        // reads (`0x0077D`): `[200Ah]`, `[200Ch]`, `[200Eh]` (`0x0078E`:
        // `mov ax, [200Ch] ; cmp ax, [200Eh]`).
        entry: locations.location.wrapping_sub(2),
        location: locations.location,
        here: locations.location.wrapping_add(2),
        walker: WALKER,
        // `[1CF6h]`, the hundredths the options' dissolve waits on (`cmp
        // word [1CF6h], 64h`, `0x0CC72`), and `[1CEAh]`, the ticks not yet
        // taken, which the frame pass reads first (`mov dx, [1CEAh]`,
        // `0x09D49`).
        hundredths: 0x1CF6,
        ticks_taken: 0x1CEA,
        // `0x10E9F`: `add ax, 2018h` after `shl ax, 5` (Pforte), and
        // `0x09B94` in the visit's setup.
        object_states: 0x2018,
        // `0x08972`, `0x08C49`, `0x0985F`, `0x098AF`: `push ds ; push 1768h
        // ; call far 0E17h:002Dh`. The row holds one far pointer,
        // `250Ah:17D0h` (`Lukas`), then the word `03E8h` at `[176Ch]`, which
        // is read on its own (`mov ax, [176Ch]`, `0x08F76`) — as Captain
        // Zins's two pointers are followed by its own (`0x0940A`).
        substitutions: 0x1768,
        // One: the word after the pointer is a variable of its own;
        // `0E17h:002Dh` does not bound `n` (`0x0E1DC`–`0x0E1EE`).
        substitution_count: crate::num::word(super::xp01data::SUBSTITUTIONS),
        // The rule matcher `0E40h`: `mov cx, [2D38h]` (`0x0E4A4`), `mov bx,
        // [2D3Ch]` (`0x0E4B3`), `mov bx, [2D3Eh]` (`0x0E4BC`), `cmp ax,
        // [2D40h]` (`0x0E57B`), `cmp ax, [2D42h]` (`0x0E583`). The mode
        // reset clears them in Captain Zins's order (`0x09BA2`–`0x09BCC`).
        verb: 0x2D38,
        object: 0x2D3C,
        object_class: 0x2D3E,
        second: 0x2D40,
        second_class: 0x2D42,
        // `0x08B72`: `cmp byte [bx+2BEAh], 0`; `0x0894E`: `mov byte
        // [bx+2BEAh], 1`.
        flags: 0x2BEA,
        // `0x07BA0`: `mov bx, [19A6h]`; the dispatcher's event arm
        // (`0x0AC07`).
        input_event: 0x19A6,
        // `0x0A1C5`: `call far [2D66h]`, the gate the machine asks.
        gate: 0x2D66,
        // `0x0AC8B`: `mov word [2D3Ah], 7`; `0x0AC85`: `and word [2D46h], 0FE0Fh` (Escape's arm).
        mode: 0x2D3A,
        bits: 0x2D46,
        // `0x0ABA4`: `mov [2D48h], ax ; mov [2D38h], ax` as a verb is chosen.
        verb_chosen: 0x2D48,
        two_object: 0x2D44,
        // `0x0FE52`: `mov word [2D38h], 0Ch` in the door's gate; `0x10681`
        // in the script exit's. The walk verb's rule is verb 0Ch (row 46).
        walk_verb: 0x0C,
        // `0x030C4`: `mov [2CFCh], dx` in the room loader.
        room_header: 0x2CFC,
        // `0x09D91`: the entry clears it; `0x09F03` (`mov [1F3Ah], si`) sets
        // it for a door; the door gate reads it (`mov ax, [1F3Ah]`,
        // `0x10611`).
        entry_door: 0x1F3A,
        // `mov [2D4Ah], ax`: the entry (`0x09F46`), the door gate
        // (`0x0FE7F`), the script exit (`0x1069A`), and the logic itself.
        walk_target: 0x2D4A,
        // `mov word [2D6Ch], 3`, the door gate (`0x0FE96`).
        anim_wait: 0x2D6C,
        status_age: 0x2D6E,
        // `0x0A5B6`: `mov al, [si+05A1h]`.
        item_classes: 0x05A1,
        inventory,
        save_buffer: SAVE_BUFFER,
        // `0x09072`: `idiv word [594h]`, the record's line height, after
        // `mov ax, [19B6h]` (`0x0906E`). The record at `DS:058Ah`
        // (`0x2562A`) ships Captain Zins's geometry: 5 lines, x 5, y `84h`,
        // width `136h`, line height `0Dh`.
        menu: 0x058A,
        // The shared hook `09B6h:186Ch` (`0x0B3CC`–`0x0B566`,
        // registered by `push 9B6h ; push 186Ch ; call far 080Ch:1A5Bh`,
        // `0x09E87`) takes codes 4 to 8 only (`sub bx, 4 ; cmp bx, 4`,
        // `0x0B3DA`), has no code-0 arm and keeps two words of its own,
        // `[1ECAh]` and `[1ECCh]` — no row of six.
        script: None,
        // The dispatcher's head makes its next far call at once (`0x0AB1E`)
        // and the hook has no event that would ask for a task.
        script_start: None,
        gates: GateMap {
            // `0x106BD`, `0x106C3`: a script exit installs `0FE3h:0848h`.
            script_exit: (0x0FE3, 0x0848),
            // `0x10605`, `0x1060B`: global rule 46 (verb `0Ch`, class 4,
            // `0FE3h:07D2h`, image `0x2371E`) installs `0FE3h:000Eh`.
            // The door's gate `0FE3h:000Eh` is the program's own: its doors
            // 64h to 6Dh wait for an animation, and it trades item 2Dh for
            // 2Ch on the way out (`0x0FE88`, `0x0FF17`). The logic answers it
            // ([`super::global_rules::door_gate`]).
            door: None,
            // `0x0AC73`, `0x0AC79` (the key arm, mapped code `100h`,
            // `cs:139Ah` at `0x0AC24`): `09B6h:0465h`, the game options. No
            // verb's rule installs it.
            options: (0x09B6, 0x0465),
        },
        // The program writes four more far pointers into `[2D66h]`,
        // each a location's own routine, which `GateMap` cannot name:
        // `107Ah:0008h` (`0x10898`), `1407h:0067h` (`0x14136`), `14F9h:01B4h`
        // (`0x151AC`), `1523h:0009h` (`0x152A2`, `0x152B8`).
    }
}

#[cfg(test)]
mod tests {
    use super::BUILD;

    /// The options' dissolve leaves the generator where the original's left
    /// it on the reference machine: `336Eh:4008h` at `203Dh:0002h` after
    /// the loop, from `0000:0000` before it.
    #[test]
    fn the_dissolve_draws_are_the_reference_machine_s_count() {
        let mut rng = crate::titles::rng::LcRng::default();
        for _ in 0..BUILD.dissolve_draws {
            rng.inclusive(0, 0x5000);
        }
        assert_eq!((rng.hi, rng.lo), (0x336E, 0x4008));
    }
}
