//! The `parsec1994` generation's runtime: the library Dunkle Schatten's
//! `XP01` is linked against, apart from what a location's logic does.
//!
//! **The location table** is [`crate::titles::xp1994`]: the dispatch shape
//! the program's main loop reads, with every figure in it read out of the
//! program.
//!
//! **Loading the shipped files** is `motionvm-parsec-formats`. A room is an
//! FGROUP slot, a strip is resource 0 of a panel file, a palette bank is
//! resource 1 — that is a fact about the format.
//!
//! **Where the strip goes** is read out of the program, by
//! [`panel_area`].
//!
//! Everything else — where the game opens, which strip is on screen, what
//! a click does in a given room — stays in [`crate::titles::ds1`],
//! next to the listing it is read from.

use crate::num;
use std::path::Path;
use std::sync::Arc;

use motionvm_parsec_formats::fgroup::{Dat, Slot, open_dat};
use motionvm_parsec_formats::{
    Animations, Bank, Font, ObjectMap, P2l, Picture, Rib, RoomFile, find_ci,
};
use motionvm_render::{Framebuffer, Palette};

use crate::dac::Apply as _;

use crate::error::Result;
use crate::exe::Image;
use crate::titles::caption::Caption;
use crate::titles::panel::Panel;
use crate::titles::rng::LcRng;
use crate::titles::subject::{Subject, Subjects, align_point, scaled};
use crate::titles::walker::{Gait, Walker};
use crate::titles::xp1994::{Flatten, Locations, Rule, Style, Timing};

mod anim;
mod api;
pub(crate) mod clock;
mod cursor;
mod dialogue;
mod effects;
mod frame;
mod input;
pub(crate) mod install;
pub(crate) mod logic;
mod menu;
mod mode;
mod movers;
mod music;
mod objects;
pub(crate) mod onward;
mod options;
mod player;
pub(crate) mod profile;
pub(crate) mod program;
mod render;
pub(crate) mod restore;
mod rules;
mod savegame;
mod script;
mod tasks;
pub(crate) mod title;
#[cfg(test)]
pub(crate) mod trace;

pub(crate) use anim::{AniRun, Body};
pub(crate) use clock::{BiosTick, Clock};
pub(crate) use cursor::PointerMap;
pub(crate) use dialogue::Event;
pub(crate) use effects::{RETRACE_DEN, RETRACE_NUM};
use logic::LogicTable;
#[cfg(test)]
pub(crate) use menu::prepare_menu_text;
pub(crate) use menu::{MenuKey, MenuShape};
pub(crate) use mode::{Gate, GateMap, TWIN};
pub(crate) use movers::MoverRt;
pub(crate) use music::{MusicMap, Request};
pub(crate) use objects::{HotspotRt, LIVE as OBJECT_LIVE, ObjectRt};
pub(crate) use options::{Input as OptionsInput, KEY_ESCAPE, KEY_TAB, Opened, OptionsMap};
pub(crate) use profile::{LibraryMap, Profile};
#[cfg(test)]
pub(crate) use savegame::sav_check;
pub(crate) use savegame::{Block, Cfg, SaveMap, cfg_read, sav_valid};
pub(crate) use tasks::{BankId, DialogueRef};

use std::collections::BTreeMap;

use tasks::Tasks;

/// A routine of the title's a callback can be bent to, by its far
/// address: what it answers for an object id.
pub(crate) type Callback = ((u16, u16), fn(&mut dyn Api, u16) -> i16);

/// The far hook the game's logic installs at `[1E32h]`: `(code, arg, task)`,
/// read against [`Api`] as the location logic is.
pub(crate) type Hook = fn(&mut dyn Api, u16, u16, u16);

/// The script globals the location hook works with — Dunkle Schatten's
/// `[1E50h]` to `[1E5Ah]` and `[1ED0h]`, each title's where its
/// [`DsMap::script`] says — cleared by its code-0 arm (`0x10515`). The logic reaches
/// them through [`Api::word`] at the addresses [`DsMap::script`] and
/// [`DsMap::script_start`] name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Script {
    /// `[1E50h]`: the figure's animation an animation code 11 or 12 takes
    /// up next (`0x10988`), −1 none.
    pub pending_animation: i16,
    /// `[1E52h]`: an animation for the figure, -1 none, -2 pending.
    pub figure_animation: i16,
    /// `[1E54h]`: a node to walk to once a text ends, -1 none.
    pub walk_after_text: i16,
    /// `[1E56h]`: cleared with the others; what reads it is not read.
    pub unread_word: i16,
    /// `[1E58h]`: the node last walked to.
    pub last_walk_node: i16,
    /// `[1E5Ah]`: a turn pending, 1 or 2.
    pub turn_pending: i16,
    /// `[1ED0h]`: event 111 armed task 0 and asked for it to start once
    /// the figure is linked (`0x0FB69`).
    pub start_task_asked: u16,
}

impl Script {
    /// The index [`Self::word`] takes for `start_task_asked`; 0 to 5 are
    /// `pending_animation` to `turn_pending`.
    const START: usize = 6;

    /// Word `i`, as the data segment holds it.
    fn word(&self, i: usize) -> u16 {
        match i {
            0 => num::unsigned(self.pending_animation),
            1 => num::unsigned(self.figure_animation),
            2 => num::unsigned(self.walk_after_text),
            3 => num::unsigned(self.unread_word),
            4 => num::unsigned(self.last_walk_node),
            5 => num::unsigned(self.turn_pending),
            _ => self.start_task_asked,
        }
    }

    /// Word `i`, written.
    fn set_word(&mut self, i: usize, v: u16) {
        match i {
            0 => self.pending_animation = num::signed(v),
            1 => self.figure_animation = num::signed(v),
            2 => self.walk_after_text = num::signed(v),
            3 => self.unread_word = num::signed(v),
            4 => self.last_walk_node = num::signed(v),
            5 => self.turn_pending = num::signed(v),
            _ => self.start_task_asked = v,
        }
    }
}

pub(crate) use api::{Api, DsMap, Far};

/// Rows of the playfield, from the `RDV1.00` header every room carries.
const PLAYFIELD_H: i32 = 128;

/// Rows of the whole screen, from the mode the program sets.
const SCREEN_H: i32 = 200;

/// Pixels per row, likewise.
const SCREEN_W: u16 = 320;

/// Which resource of the pointer file holds the font: resource 15 of
/// `CMBMOU.DAT`.
const FONT_RESOURCE: usize = 15;

/// The ink the status line is set in (`0x0C750`, `mov byte es:[bx+4],
/// 0F4h` on the font record).
const STATUS_INK: u8 = 0xF4;

/// The status painter's own controls, the table at `cs:04C2h` (image
/// `0x0C8B2`) and its arms: `09h` sets the ink to `0FAh` (`0x0C788`),
/// `0Ah` to `0FCh` (`0x0C793`), `0Dh` back to `0F4h` (`0x0C79E`); `00h`
/// ends the line (`0x0C7D2`).
const STATUS_INKS: [(u8, u8); 3] = [(0x09, 0xFA), (0x0A, 0xFC), (0x0D, 0xF4)];

/// The row the status band starts on, and its height (`0x0C876`).
const STATUS_Y: i32 = 128;

/// The entry twin a room carries for a door the player should walk in
/// through (`0x0EF4F`).
const ENTRY_TWIN: i32 = 0x7D0;

/// The node a room falls back to when neither the twin nor the entry id
/// names one (`0x0EF8E`). It is why `7Fh` is the entry a fresh game
/// starts with.
const FALLBACK_NODE: i32 = 0x7F;

/// The alignment the player's walker record carries (`+3` of `DS:1693h`).
///
/// Read rather than assumed for the drawing already: the figure is centered
/// on its `x` and stands on its `y`. The resolver's arm for it moves the
/// point differently from the drawer's — it only lifts by the height — so
/// the two are written out separately (`0x0D3B7`).
const WALKER_ALIGN: u8 = 2;

/// Which shipped files the game keeps its rooms and its strip in.
///
/// There is no rule to derive the names from — `DSROOM`, `PANELS`,
/// `PLAY00` — so the caller names them.
pub(crate) struct Files {
    /// The FGROUP whose slots are the locations' playfields.
    pub rooms: &'static str,
    /// The RIB whose resource 0 is the verb strip and 1 its palette bank.
    pub panel: &'static str,
    /// The RIB holding the pointers, their bank and the font.
    pub pointers: &'static str,
    /// The RIB holding the player's cels and their bank.
    pub player: &'static str,
    /// The RIB holding the two global dialogue banks and the figure's
    /// animations.
    pub dialog: &'static str,
    /// Where in the program the status-line templates are, and how many.
    /// The row and its length are this title's own reading.
    pub templates: (usize, usize),
    /// The music library, and where the build keeps the music's words.
    pub music: (&'static str, MusicMap),
}

/// What a build's location table means by the kind word at `+7` of a row,
/// where the generation's routines tell rows apart by it. The numbers are
/// immediates of those routines and differ from title to title.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Kinds {
    /// A row that is not a place: the main loop does not dispatch it but
    /// walks on past it, and a door does not lead to it.
    pub passage: u16,
    /// The restore row's kind, which the walk on steps over too.
    pub restore: u16,
}

/// One loaded install of a parsec1994 game: the program, its locations,
/// and its pictures.
pub(crate) struct Adventure {
    /// The unpacked `XP01.EXE`, kept because a record is read on demand.
    pub image: Image,
    /// The location table this program dispatches on.
    pub locations: Locations,
    /// What the table's kinds mean to this build.
    pub kinds: Kinds,
    /// One entry per room-file slot; `None` where the slot carries no
    /// playfield.
    pub rooms: Vec<Option<RoomFile>>,
    /// The verb strip.
    pub panel: Picture,
    /// The palette banks laid over a location's own palette, in order.
    pub banks: Vec<Vec<u8>>,
    /// The row the strip is drawn on, read out of the program.
    pub panel_y: i32,
    /// The strip's widgets: the verb buttons, the arrows, the slots.
    pub panel_widgets: Panel,
    /// The font the status line is set in — `CMBMOU.DAT` resource 15.
    font: Font,
    /// The verb the player picked, the program's `[288Dh]`. Zero is the
    /// empty template, which is what a fresh location leaves it at
    /// (`0x0EDA0` clears the block it heads).
    pub verb: u8,
    /// The status-line templates, one per verb.
    pub templates: Vec<String>,
    /// The cel drawn for the pointer while the pointer library is not
    /// installed ([`Self::install_pointer`]): the pointer file's first
    /// cel.
    pub pointer: Option<Picture>,
    /// The pointer file's cels from resource 1, the fourteen the loader
    /// visits (`0x05D45`–`0x05D86`, `cmp cx, 0Eh`): the table the
    /// descriptors index, `[195Eh]`.
    pointer_cels: Vec<Option<Picture>>,
    /// The pointer library, `179Eh`, once the game's logic has installed it.
    pub(crate) cursor: Option<cursor::Cursor>,
    /// The choice menu's geometry, read out of the program.
    pub menu_shape: MenuShape,
    /// The player's cels: bundle 1, which is the figure file from its
    /// resource **1** — the loader advances the slot by four bytes, one
    /// far pointer, past the palette (`0x10AD0`).
    pub figure_cels: Vec<Picture>,
    /// The gait the player walks with: the interval and the phase count
    /// out of its own walker record, and the reach out of the leg setup.
    pub gait: Gait,
    /// The player, once a location has put it down.
    pub walker: Option<Walker>,
    /// The current room's people: its 30h-byte records, in table order
    /// ([`movers`]).
    pub movers: Vec<MoverRt>,
    /// Each layer's chain of walking records, head first: the order the
    /// mover pass, the animator and the drawer take them in ([`movers`]).
    pub chains: Vec<Vec<Body>>,
    /// The two global dialogue banks, 999 and 2 — `DIALOG.DAT`'s first two
    /// resources, which is where `0x525F` puts them.
    dialogues: (Bank, Bank),
    /// The interaction rules that hold everywhere.
    pub global_rules: Vec<Rule>,
    /// The speaker styles a caption can be drawn in.
    pub styles: Vec<Style>,
    /// The style a hover text is set in, the three bytes before the row.
    pub hover_style: Style,
    /// How a caption without a panel flattens what is under it.
    pub flatten: Flatten,
    /// The weights a page's own size is turned into a duration with.
    timing: Timing,
    /// What every item is called, by id, counting from one.
    item_names: Vec<String>,
    /// One cel per item id, resolved through the executable's mapping.
    /// Several ids can refer to the same panel resource. `None` for an id
    /// the current strip file has no cel for, in a build with several.
    item_cels: Vec<Option<Picture>>,
    /// Every set of strip and figure files the build can load, by digit.
    figure_sets: Vec<(u16, FigureFiles)>,
    /// The set loaded now.
    pub(crate) figure_digit: u16,
    /// `[1600h]`, `[1601h]`: the carried list's range the strip shows and
    /// takes clicks in, from `inv_lo` up to but not including `inv_hi`.
    pub(crate) inv_lo: usize,
    /// See [`Self::inv_lo`].
    pub(crate) inv_hi: usize,
    /// What the player is carrying, in the order the strip shows it.
    /// `DS:1602h`, one byte an item.
    pub(crate) items: Vec<u8>,
    /// How far the strip is scrolled — `DS:15FFh`.
    scroll: usize,
    /// Whether the item cels reach the picture: from the first room's
    /// binding pass on ([`Adventure::set_location`]).
    pub(crate) items_drawn: bool,
    /// The event of the click being taken, `[184Eh]`:
    /// [`input::BUTTON_FIRST`] or [`input::BUTTON_SECOND`].
    pub(crate) button: u16,
    /// What the scenes of this location are about: the program's own
    /// array of subject slots at `DS:1D56h`, filled by the entry function.
    pub subjects: Subjects,
    /// `[288Fh]`, the interaction mode — the whole of what a click means.
    ///
    /// Nine values and five arms on the strip's side (`CS:148Bh`), and on
    /// the picture's side the object road answers only for **0, 2 and 3**
    /// (`0x0F5F4`): mode 1 and modes 4 upwards fall straight to the exit
    /// of that routine, so a click on a hotspot does nothing at all in
    /// them. That is the "parked" state a second click on the same verb
    /// button reaches.
    ///
    /// A fresh location is mode 0, because the entry function clears the
    /// `38h` bytes the verb index heads and this word is inside them
    /// (`0x0EDA8`).
    pub mode: u16,
    /// `[2899h]`, the two-object flag a verb sets when it wants a second.
    pub two_object: u16,
    /// `[2891h]` and `[2893h]`, the object that has been chosen and its
    /// class. Cleared whenever a verb is taken.
    pub object1: u16,
    /// See [`Adventure::object1`].
    pub object2: u16,
    /// The title's location logic, which the runtime asks for the
    /// location's own hooks and for the handler a rule names
    /// ([`LogicTable`]). Without one no hook does anything and no handler
    /// is read.
    pub logic_table: Option<LogicTable>,
    /// Each location's own dialogue bank, by slot.
    ///
    /// It is resource **3** of the slot: the room loader takes the far
    /// pointer at `+0Ch` of the loaded slot — the fourth of six — and keeps
    /// it in `[2855h]` (`0x7B0D`, `0x7B14`), counted into `[1EEAh]` by the
    /// routine that counts bank 999 (`0x7B1A`). A scene number past that
    /// count is what the resolver cannot find, and that has an answer of
    /// its own.
    pub own_banks: Vec<Option<Bank>>,
    /// Every slot's object map, resource 2 — see [`ObjectMap`]. Only the
    /// Wald's and the maze's slots carry one.
    object_maps: Vec<Option<ObjectMap>>,
    /// The current row's object map as the loader bound it, while the row
    /// is a forest row — see [`objects::BoundMap`].
    pub(crate) bound_map: Option<objects::BoundMap>,
    /// The loaded slot's file with the walk graphs as the logic edited
    /// them this visit ([`Api::node_block`], [`Api::node_open`]): the
    /// original writes the edges in the room's memory, and the next load
    /// of the slot brings the file's again. `None` while nothing was
    /// edited, which is what the rooms read at the start then answer.
    edited_room: Option<RoomFile>,
    /// Whether the slot unload kept the forest picture for the next row,
    /// two forest rows in a row (`0x83A3`–`0x83FB`): the loader then loads
    /// only the next row's own slot and binds its map onto the records as
    /// they are (`0x7806`).
    slot_kept: bool,
    /// Whether the current location's setup kept the last row's picture
    /// rather than load its slot: what the loader skips then — the slot's
    /// song (`0x780D`) and its dialogue bank (`0x7B0D`) — stays as the last
    /// row left it.
    pub(crate) picture_kept: bool,
    /// The objects of the current location as the program has them now
    /// — the 32-byte records it writes into the loaded room, which this
    /// engine keeps beside the parsed room and rebuilds on every arrival
    /// the loader loads the room for.
    pub objects: Vec<ObjectRt>,
    /// The hotspots of the current location as they stand now: the active
    /// bit an object's live bit sets, and a rectangle an animation may
    /// move.
    pub hotspots_rt: Vec<HotspotRt>,
    /// The working state of every `AN` entry the location can play: the
    /// room block's in directory order, then the global block's.
    pub anis: Vec<AniRun>,
    /// The room's own `AN` block, `[286Dh]`, shared with the runs that
    /// walk it.
    pub room_animations: Option<Arc<Animations>>,
    /// `DIALOG.DAT`'s resource 2, the block the figure animates from
    /// (`[1ED4h]`, `0x0525F`).
    pub global_animations: Option<Arc<Animations>>,
    /// The code that let a held page go (`0CFDh:1BA2h`; Dunkle Schatten's
    /// `[1E48h]`).
    pub page_release_code: u16,
    /// The sixty-one 32-byte records at `DS:1F56h`, one per location row:
    /// what a visit leaves of the objects' state for the next
    /// (`14DDh:068Ah`, `14DDh:0293h`). Zero in the image.
    pub location_records: Vec<[u8; 32]>,
    /// The generator the refusals are drawn with (`1EB5:0028`). Its seed
    /// is zero in the image, and it advances only when something rolls.
    rng: LcRng,
    /// How many draws the options' dissolve takes from that generator each
    /// time it plays. A property of the machine the program runs on, not
    /// of the game: the title's [`program::Build::dissolve_draws`] unless set
    /// otherwise.
    pub(crate) dissolve_draws: u32,
    /// The two 74-byte task records and the globals around them.
    pub tasks: Tasks,
    /// The record the timer counts into, `DS:1C74h`.
    pub clock: Clock,
    /// The substitution row at `DS:1689h`: entry 0 what a location's own
    /// handler draws into it — *Dunkle Schatten*'s Wohnzimmer, for a
    /// wrong number — entry 1 the word the program keeps beside it.
    pub substitutions: Vec<Vec<u8>>,
    /// What the scene machine did, in order, for the game's wrapper and
    /// for the tests.
    pub events: Vec<Event>,
    /// The far hook at `[1E32h]`. The default does nothing.
    pub hook: Hook,
    /// The runtime's copy of the program's `DGROUP`, taken from the image:
    /// every global the location logic owns lives here at its address.
    pub ds: Vec<u8>,
    /// `DGROUP`'s segment, for far pointers into it.
    pub dgroup: u16,
    /// Where this build keeps the globals the logic shares with the engine.
    pub ds_map: DsMap,
    /// The screen effects queued or playing (`10B8h`).
    pub(crate) effects: effects::Effects,
    /// The picture as the last frame pass presented it — what the screen
    /// holds when an effect starts drawing over it.
    pub(crate) presented: Framebuffer,
    /// The music library and the driver's calls.
    pub(crate) music: music::Music,
    /// `si` of `0EB9h:00B8h`: whether the entry plays the room's song
    /// (`0x0EC60`–`0x0EC74`), decided before the entry effect and acted on
    /// after it (`0x0ECA9`).
    entry_music: bool,
    /// The entry's tail is owed: the entry effect was queued, and what
    /// `0EB9h:00B8h` does after its call returns (`0x0EC84`–`0x0ED02`)
    /// waits for the effects to be over.
    entry_tail: bool,
    /// `[1C76h]` as a handler writes it behind a blocking effect: the
    /// effect plays here after the handler's code has run on, and what it
    /// writes into the tick count lands once the effect is over, where the
    /// original writes it (`0xC5B`, `0x2F8F`).
    pub(crate) ticks_after_effect: Option<u16>,
    /// The running handler's own locals that live across its frame loop.
    pub handler_locals: [u16; 8],
    /// The location whose hooks the entry function installed.
    pub hook_location: i32,
    /// The main loop's entry word (`[1F48h]` in Dunkle Schatten): the entry id a visit
    /// arrives by, cleared after entry, the exit id once one is asked for.
    pub entry_word: u16,
    /// The main loop's location word (`[1F4Ah]`): where the next visit goes.
    pub location_word: u16,
    /// The running handler's own location (`[1F4Ch]`).
    pub here_word: u16,
    /// The script globals the hook keeps.
    pub script: Script,
    /// `[04ECh]`: owners tinting the panel and flattening the status band.
    /// Bit 0 is a text's (`0x0D85F`); Dunkle Schatten releases owners after refresh.
    pub status_dim: u16,
    /// The measured panel state machine, once opted into with its curves.
    panel_colors: Option<crate::titles::palette1994::PanelColors>,
    /// The post-entry ramp, restored after transition effects (`0x0EC84`).
    entry_ramp: Option<(u8, Vec<u8>)>,
    /// Flag words the machine wrote into a loaded dialogue, by record.
    ///
    /// Ordered, so that nothing which ever walks it — a save, a trace —
    /// takes its order from a hash.
    pub record_flag_writes: BTreeMap<(DialogueRef, usize), u16>,
    /// `[289Dh]`, the verb as it was chosen, before a rule changed it.
    pub verb_chosen: u16,
    /// `[2895h]` and `[2897h]`, the second object and its class.
    pub second_object: (u16, u16),
    /// `[289Bh]`, the mode machine's bit field; [`mode`] names the bits.
    pub bits: u16,
    /// `[289Fh]`, the node the machine is walking the figure to.
    pub walk_target: i16,
    /// `[28BBh]`/`[28BDh]`, the gate the machine asks before it goes on.
    pub gate: Gate,
    /// `[28C3h]`, the pending status refresh age. `0x101D5` tests it,
    /// `0x101E7` increments it, and a previous value above 3 calls the
    /// status composer `0x10051`, which clears it at `0x101B3`
    /// ([`Adventure::status_gate`]). This word does not select a cursor
    /// cel.
    pub status_refresh_age: u16,
    /// The status line as it was last set — the buffer at `2115h:0132h`
    /// that `0C3Fh:0B3Eh` fills and the band painter reads every frame
    /// (`0x0C730`). The composer writes it only through the gate, and
    /// the leaving empties it after the exit effect (`0x0ED60`); between
    /// the two it holds.
    pub status_line: Vec<u8>,
    /// `[1EDEh]`: the door the player came in through, until the figure
    /// has left its node (`0x0EF22`, `0x0F573`).
    pub entry_door: i16,
    /// Dunkle Schatten's `[28BFh]`: ticks a rule asked the step to wait
    /// (bit `20h`), counted down by the frame pass (`0x0F53A`).
    pub wait_ticks: u16,
    /// Dunkle Schatten's `[28C1h]`, Captain Zins's `[1E8Ch]`
    /// ([`DsMap::anim_wait`]): the animation code the step waits for (bit
    /// `100h`, `0x0F549`), cleared when an animation signals it
    /// (`0x109A8`).
    pub anim_wait: u16,
    /// `[1F48h]` and `[1F4Ah]` as the door's gate leaves them: the exit
    /// taken and the location it leads to, applied when the frame is over.
    pub leave: Option<(u8, i32)>,
    /// `[28B3h]`/`[28B5h]`: the hotspot record of the object chosen, as an
    /// index into the room's hotspots, and `[28B7h]`/`[28B9h]` the second
    /// object's.
    pub hover_spot: Option<usize>,
    pub second_spot: Option<usize>,
    /// `[1E64h]`: whether the pointer is followed over the room. Set by
    /// the visit's setup (`0EB9h:0001h`) and `0EB9h:1917h`, cleared by
    /// `0EB9h:18F4h` while a script has the figure.
    pub hover_enabled: bool,
    /// `[28A3h]`/`[28A5h]`: what the pointer was over last frame.
    pub last_hover: mode::Hover,
    /// What the library build the program is linked against does
    /// differently, read off its routines before the runtime opens.
    pub profile: Profile,
    /// The callback's far pointer as the program last wrote it, when the
    /// build keeps one ([`Profile::callback`]).
    pub callback: Far,
    /// The title's own routines a callback may be bent to, by far address.
    pub callbacks: &'static [Callback],
    /// The choice menu, `DS:04F8h`.
    pub menu: menu::Menu,
    /// `[1E4Eh]`: the choice taken, 1-based, 0 for none yet.
    pub choice_taken: u16,
    /// Where the pointer is, in game coordinates.
    pub(crate) mouse: (i32, i32),
    /// The game options while they are up.
    pub(crate) options: Option<options::Options>,
    /// Where this build keeps the options' records, when it has them.
    pub(crate) options_map: Option<OptionsMap>,
    /// The BIOS tick (`0:046Ch`), counted on by the interrupt; a save and
    /// the settings file carry it.
    pub(crate) bios: BiosTick,
    /// A saved game the options wrote, for the host to put into its file.
    pub(crate) save_written: Option<Vec<u8>>,
    /// A message the original ends the program with, shown instead.
    pub(crate) notice: Option<String>,
    /// Where the player is.
    loc: i32,
    /// The composed DAC for the current location.
    dac: Palette,
}

impl Adventure {
    /// Opens what `files` names in `res`, on a program already unpacked.
    ///
    /// Where the game opens is **not** decided here. A caller that has
    /// read its title's opening passes it to [`Self::set_location`]; one
    /// that has not leaves the location at whatever the program's own
    /// globals hold, which is zero.
    #[expect(
        clippy::too_many_lines,
        reason = "one literal names every field the runtime opens with, which is \
                  more than half the length; the reads before it feed that literal"
    )]
    pub(crate) fn open(
        res: &Path,
        image: Image,
        locations: Locations,
        kinds: Kinds,
        ds_map: DsMap,
        files: Files,
        profile: Profile,
    ) -> Result<Self> {
        let rooms = load_rooms(res, files.rooms)?;
        let own_banks = load_own_banks(res, files.rooms)?;
        let object_maps = load_object_maps(res, files.rooms)?;
        // The music library. A copy without it, or with one that will not
        // read, plays no music rather than refusing the game: the program
        // would end at its init (`0x13F00`, `0x13F3D`), and everything but
        // the music is there to play.
        let name = files.music.0;
        let library = find_ci(res, name)
            .ok_or_else(|| format!("RESOURCE/{name}: not found"))
            .and_then(|p| std::fs::read(&p).map_err(|e| format!("RESOURCE/{name}: {e}")))
            .and_then(|b| P2l::parse(b).map_err(|e| format!("RESOURCE/{name}: {e}")));
        // A slot that holds a playfield and no row to dispatch on means
        // the two numberings are not the same numbering, and nothing that
        // follows would be safe.
        for (n, room) in rooms.iter().enumerate() {
            if room.is_some() && locations.handler(&image, n).is_none() {
                return Err(format!(
                    "{} slot {n} holds a playfield but location {n} has no handler in the \
                     table the dispatch at {:#x} names — a location number is not a slot \
                     number after all",
                    files.rooms, locations.at
                )
                .into());
            }
        }

        // The strip's and the figure's files, every set the build can
        // load ([`Profile::figure_set`]); the first is the one a visit
        // starts from.
        if let profile::FigureSet::AfterLocation { word, .. } = profile.figure_set
            && word != ds_map.location
        {
            return Err(format!(
                "the setup picks the figure by [{word:04X}h], not by the main loop's location"
            )
            .into());
        }
        let item_count = crate::titles::xp1994::item_names(&image)?.len();
        let digits = profile.figure_set.digits();
        let figure_sets = digits
            .iter()
            .map(|&d| {
                Ok((
                    d,
                    FigureFiles::load(res, &image, &files, item_count, d, digits.len() > 1)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let (figure_digit, first) = figure_sets
            .first()
            .cloned()
            .ok_or("the build loads no strip and figure files")?;
        let panel = first.panel.clone();
        // The picture goes on the area's first row: `PANELS.DAT`'s
        // resource 0 is the area's 72 rows, band and strip.
        let (panel_y, _) = panel_area(&image)?;
        let panel_widgets = Panel::read(&image)?;
        let templates = image.ptr_table(image.at(files.templates.0), files.templates.1)?;
        let pointers = rib(res, files.pointers)?;
        let font = Font::parse(pointers.get(FONT_RESOURCE).ok_or_else(|| {
            format!(
                "{} has no resource {FONT_RESOURCE}, the font",
                files.pointers
            )
        })?)?;
        let player = crate::titles::mover::movers(&image)?.0;
        let gait = Gait {
            speed: i32::from(player.interval),
            phases: player.phases,
            reach: crate::titles::mover::leg_dividend(&image)?,
        };
        let figure_cels = first.figure_cels.clone();
        // The first resource that reads as a cel. Resource 0 is a palette
        // bank, and which of the cels after it the program actually shows
        // is not read.
        let pointer = pointers
            .resources
            .iter()
            .skip(1)
            .find_map(|b| Picture::parse(b).ok());
        let pointer_cels = pointers
            .resources
            .iter()
            .skip(1)
            .take(14)
            .map(|b| Picture::parse(b).ok())
            .collect();
        let banks = [
            first.panel_bank.clone(),
            pointers.get(0).map(<[u8]>::to_vec),
            first.player_bank.clone(),
        ]
        .into_iter()
        .flatten()
        .filter(|b| motionvm_parsec_formats::pal::looks_like(b))
        .collect();

        // The two global banks, the rules that hold everywhere, and the
        // numbers a caption is drawn and timed with, all read out of the
        // program.
        let dialog = rib(res, files.dialog)?;
        let dialogues = (
            Bank::parse(
                dialog
                    .get(0)
                    .ok_or_else(|| format!("{} has no bank 999", files.dialog))?,
            )?,
            Bank::parse(
                dialog
                    .get(1)
                    .ok_or_else(|| format!("{} has no bank 2", files.dialog))?,
            )?,
        );
        // Resource 2 is the figure's `AN` block, loaded beside the banks
        // (`0x0525F`); without one no figure animation plays.
        let global_animations = match dialog.get(2).filter(|b| Animations::looks_like(b)) {
            Some(b) => Some(Arc::new(Animations::parse(b)?)),
            None => None,
        };
        let global_rules = Rule::read_table(&image)?;
        let styles = Style::read_row(&image)?;
        let hover_style = Style::read_hover(&image)?;
        let flatten = Flatten::read(&image)?;
        let timing = Timing::read(&image)?;
        let item_names = crate::titles::xp1994::item_names(&image)?;
        let item_cels = first.item_cels;

        let mut tasks = Tasks::new();
        tasks.cue_guard = crate::titles::xp1994::cursor_cue_guard(&image)?;
        let rows = locations.count;
        // The program's data segment as it ships; BSS past the image is zero.
        let dgroup = u16::try_from(image.at(0) / 16).unwrap_or(0);
        let ds = {
            let at = image.at(0);
            let bytes = image.bytes();
            let mut ds = vec![0u8; 0x10000];
            let n = bytes.len().saturating_sub(at).min(0x10000);
            ds[..n].copy_from_slice(&bytes[at..at + n]);
            ds
        };
        let menu_shape = MenuShape::read(&image, &ds, ds_map.menu)?;
        let mut me = Adventure {
            menu_shape,
            image,
            locations,
            kinds,
            rooms,
            panel,
            banks,
            panel_y,
            panel_widgets,
            font,
            verb: 0,
            templates,
            pointer,
            pointer_cels,
            cursor: None,
            figure_cels,
            gait,
            walker: None,
            movers: Vec::new(),
            chains: Vec::new(),
            dialogues,
            global_rules,
            styles,
            hover_style,
            flatten,
            timing,
            item_names,
            item_cels,
            figure_sets,
            figure_digit,
            items: Vec::new(),
            inv_lo: 0,
            inv_hi: 0,
            scroll: 0,
            items_drawn: false,
            button: input::BUTTON_FIRST,
            subjects: Subjects::default(),
            logic_table: None,
            own_banks,
            object_maps,
            bound_map: None,
            edited_room: None,
            slot_kept: false,
            picture_kept: false,
            objects: Vec::new(),
            hotspots_rt: Vec::new(),
            anis: Vec::new(),
            room_animations: None,
            global_animations,
            page_release_code: 0,
            location_records: vec![[0; 32]; rows],
            rng: LcRng::default(),
            dissolve_draws: 1,
            mode: 0,
            two_object: 0,
            object1: 0,
            object2: 0,
            tasks,
            clock: Clock::new(),
            substitutions: Vec::new(),
            events: Vec::new(),
            hook: |_, _, _, _| {},
            ds,
            dgroup,
            ds_map,
            effects: effects::Effects::default(),
            presented: crate::draw::display(),
            music: music::Music::new(library, files.music.1, profile.effects.slots),
            entry_music: false,
            entry_tail: false,
            ticks_after_effect: None,
            handler_locals: [0; 8],
            hook_location: 0,
            entry_word: 0,
            location_word: 0,
            here_word: 0,
            script: Script::default(),
            status_dim: 0,
            panel_colors: None,
            entry_ramp: None,
            record_flag_writes: BTreeMap::new(),
            verb_chosen: 0,
            second_object: (0, 0),
            bits: 0,
            walk_target: 0,
            gate: Gate::None,
            status_refresh_age: 0,
            status_line: Vec::new(),
            entry_door: 0,
            wait_ticks: 0,
            anim_wait: 0,
            leave: None,
            menu: menu::Menu::default(),
            choice_taken: 0,
            hover_spot: None,
            second_spot: None,
            hover_enabled: false,
            last_hover: mode::Hover::None,
            profile,
            callback: Far::NULL,
            callbacks: &[],
            mouse: (0, 0),
            options: None,
            options_map: None,
            bios: BiosTick::default(),
            save_written: None,
            notice: None,
            loc: 0,
            dac: Palette::default(),
        };
        me.rebuild_dac();
        Ok(me)
    }

    /// Where the player is.
    pub(crate) fn location(&self) -> i32 {
        self.loc
    }

    /// The pointer library as the loader leaves it (`0x05D45`–`0x05F0A`),
    /// at the addresses `map` names. The mouse starts where the library
    /// has it, `[3E6h]`/`[3E8h]`.
    pub(crate) fn install_pointer(&mut self, map: PointerMap) {
        let c = cursor::Cursor::load(map, &self.ds, self.image.bytes(), &self.pointer_cels);
        self.mouse = (i32::from(c.pos.0), i32::from(c.pos.1));
        self.cursor = Some(c);
    }

    /// The pointer file's cel `n`, counted from resource 1.
    pub(crate) fn pointer_cel(&self, n: usize) -> Option<&Picture> {
        self.pointer_cels.get(n).and_then(Option::as_ref)
    }

    /// `0590h:064Dh(mode)`, the pointer's mode set.
    pub(crate) fn pointer_mode(&mut self, mode: u16) {
        if let Some(c) = &mut self.cursor {
            c.set_mode(mode);
        }
    }

    /// `179Eh:1017h`.
    pub(crate) fn pointer_resync(&mut self) {
        if let Some(c) = &mut self.cursor {
            c.resync();
        }
    }

    /// `179Eh:0882h`.
    pub(crate) fn pointer_off(&mut self) {
        if let Some(c) = &mut self.cursor {
            c.hide();
        }
    }

    /// `179Eh:091Ah`.
    pub(crate) fn pointer_thaw(&mut self) {
        let mouse = self.mouse;
        if let Some(c) = &mut self.cursor {
            c.unfreeze(mouse);
        }
    }

    /// `179Eh:0830h`.
    pub(crate) fn pointer_on(&mut self) {
        let mouse = self.mouse;
        if let Some(c) = &mut self.cursor {
            c.show(mouse);
        }
    }

    /// `179Eh:01B0h`: what the poll does to the pointer (`0x17ABB`).
    pub(crate) fn pointer_poll(&mut self) {
        if let Some(c) = &mut self.cursor {
            c.poll();
        }
    }

    /// `0CFDh:056Eh(i)`'s pointer half (`0x0D555`–`0x0D567`): for task 0
    /// the menu's mouse region goes and the shape is taken again.
    pub(crate) fn dialogue_over_pointer(&mut self, i: usize) {
        if i != 0 {
            return;
        }
        if let Some(c) = &mut self.cursor {
            c.menu_region_remove();
            c.resync();
        }
    }

    /// The status font, for a test to measure with.
    #[cfg(test)]
    pub(crate) fn font_for_test(&self) -> &Font {
        &self.font
    }

    /// The four weights a page of text is timed with, as the data segment
    /// holds them now (`0x0D865`–`0x0D8BB` read them there each time).
    pub(crate) fn timing(&self) -> Timing {
        let w = |a: u16| {
            let a = usize::from(a);
            u16::from_le_bytes([self.ds[a], self.ds[a + 1]])
        };
        let s = self.timing.sites;
        Timing {
            base: w(s[0]),
            per_line: w(s[1]),
            per_char: w(s[2]),
            max: w(s[3]),
            sites: s,
        }
    }

    /// A byte of the program's state by its `DS` address: the location
    /// handlers' story bytes (`[2725h]`, `[270Fh]`, `[2727h]`, the day
    /// bytes from `[26F6h]`), the dialogue flags at `[273Fh]` — all of it
    /// lives in `DGROUP`, starting from what the image ships.
    pub(crate) fn sav_byte(&self, addr: usize) -> u8 {
        self.ds.get(addr).copied().unwrap_or(0)
    }

    /// The same, written.
    pub(crate) fn sav_set(&mut self, addr: usize, v: u8) {
        if let Some(b) = self.ds.get_mut(addr) {
            *b = v;
        }
    }

    /// Dialogue flag `n`, `[273Fh + n]`.
    pub(crate) fn flag(&self, n: usize) -> u8 {
        self.sav_byte(usize::from(self.ds_map.flags) + n)
    }

    /// The same, set (`0x0D915`).
    pub(crate) fn set_flag(&mut self, n: usize) {
        self.sav_set(usize::from(self.ds_map.flags) + n, 1);
    }

    /// `0EB9h:1691h(counter, scenes…, −1)`: the next scene of a sequence.
    /// A scene number past 50 is an error (`0x10241`) and the list ends at
    /// −1 (`0x1025D`). While the counter byte (`0x10266`) is below the last
    /// index, its scene plays and the counter counts on (`0x1026E`,
    /// `0x10275`); from there on the last scene plays every time
    /// (`0x1027C`), through `0CFDh:172Fh` (`0x1028A`).
    pub(crate) fn scene_sequence(&mut self, counter: usize, scenes: &[i16]) {
        let Some(&last) = scenes.last() else {
            return;
        };
        let k = usize::from(self.sav_byte(counter));
        let scene = if k + 1 < scenes.len() {
            self.sav_set(counter, num::byte(k + 1));
            scenes[k]
        } else {
            last
        };
        self.scene_load(scene);
    }

    /// What that location calls itself, when its record names anything.
    ///
    /// An empty name is not a name: a record can carry a real pointer to
    /// an empty string, and a caller wanting to say where the player is
    /// would rather have the number than nothing in brackets.
    pub(crate) fn location_name(&self) -> Option<&str> {
        usize::try_from(self.loc)
            .ok()
            .and_then(|n| self.locations.name(n))
            .filter(|n| !n.is_empty())
    }

    /// Go to a location, if the program could be in it.
    ///
    /// Two gates, both the program's own: the bound it checks before every
    /// dispatch, and whether that row names a handler at all. A number
    /// past either is ignored rather than entered — standing somewhere the
    /// original could not stand would be this engine's invention.
    pub(crate) fn set_location(&mut self, n: i32) {
        let known = usize::try_from(n).is_ok_and(|n| {
            n < self.locations.count && self.locations.handler(&self.image, n).is_some()
        });
        if known {
            // The loader keeps the forest picture when the unload kept it
            // and the place left and the place ahead are both forest rows
            // (`14DDh:08F1h`, `0x77DA`, `0x7802`).
            let kept = std::mem::take(&mut self.slot_kept)
                && self.bound_map.is_some()
                && u16::try_from(n).is_ok_and(|n| self.forest_row(n))
                && self
                    .profile
                    .forest
                    .is_some_and(|f| self.place_pair_kind() == f.kind);
            self.picture_kept = kept;
            self.loc = n;
            self.clear_mode_block();
            // The loader's fresh copy of the room's records — or, with
            // the picture kept, the records as the last row left them —
            // the row's object map bound to them, and the entry pass that
            // binds the block's entries to them (`14DDh:0293h`, from the
            // visit's setup `0EB9h:0001h`).
            if !kept {
                self.bound_map = None;
                self.edited_room = None;
                self.rebuild_objects();
            }
            self.bind_object_map();
            self.rebuild_movers();
            self.rebuild_animations();
            self.bind_animations();
            self.settle_objects();
            // The room's palette goes to the DAC as the entry pass loads
            // it (`14DDh:0293h`, `0x1535F`), and the visit's setup that
            // runs that pass comes after the exit effect has played
            // (`0x0ED4F`, synchronous, before `0EB9h:0001h`). An exit still
            // queued gets the room's colors behind it.
            if self.effects.busy() {
                self.load_room_palette();
            } else {
                self.rebuild_dac();
            }
            // The same pass points the item cels' blitter at the back
            // buffer (`1601h:10A4h` at `0x153C7`, its one caller). Until a
            // room has been bound, the blitter draws on the screen the
            // image names, `A000h:0000h`, and the next present copies the
            // back buffer over what it drew: the start's options show the
            // strip without its items.
            self.items_drawn = true;
            // A jump is not an arrival: the original only ever enters a
            // location through an entry id, and only that puts the player
            // anywhere. `--loc` alone leaves the figure's record out of
            // every chain, which is not drawn.
            if let Some(w) = &mut self.walker {
                w.unlink();
            }
            self.sync_links();
            self.leave = None;
        }
    }

    /// The entry function's first act (`0x0EDA8`): the `38h` bytes from
    /// `[288Dh]` cleared — verb, mode, objects, bits, walk target, gate
    /// and status refresh age — then `[28C3h] = 0Ah` and `[1EDEh] = 0`
    /// (`0x0EDB0`, `0x0EDB6`).
    fn clear_mode_block(&mut self) {
        self.verb = 0;
        self.mode = 0;
        self.object1 = 0;
        self.object2 = 0;
        self.second_object = (0, 0);
        self.two_object = 0;
        self.bits = 0;
        self.verb_chosen = 0;
        self.walk_target = 0;
        self.gate_clear();
        self.status_refresh_age = 0x0A;
        self.entry_door = 0;
        self.hover_spot = None;
        self.second_spot = None;
        self.last_hover = mode::Hover::None;
        // `0x0EC1B`, the visit's setup: `[1E64h] = 1`.
        self.hover_enabled = true;
    }

    /// The same, remembering the entry id it was reached through, and
    /// putting the player down the way that entry says.
    ///
    /// For a test that wants a location without its handler: driven the
    /// way the shortest handler runs — leave, set up, enter, finish.
    #[cfg(test)]
    pub(crate) fn set_location_via(&mut self, n: i32, entry: i16) {
        self.complete_entry();
        if self.tasks.enabled {
            // `0x9EF`: every handler keeps its objects' states in its
            // location's own row before it leaves.
            self.store_location_record(i32::from(num::signed(self.here_word)));
            self.visit_leave_impl();
        }
        self.location_word = num::low16(n);
        self.entry_word = num::unsigned(entry);
        if self.visit_setup_impl() {
            self.here_word = self.location_word;
            self.visit_enter_impl(n, entry);
            self.visit_finish_entry_impl();
            self.entry_word = 0;
        }
    }

    /// `0EB9h:017Ah`, leaving a location that was being visited: the
    /// machine reset, both tasks stopped, the band held, the frame pass
    /// run, and — for a location of any kind but 3 — the music faded and
    /// one of six transition effects rolled (`0x0ED42`,
    /// `1EB5h:0028h(1, 6)`) and played through `10B8h:0210h`, which is not
    /// read. The roll is taken so the generator stays in step.
    pub(super) fn visit_leave_impl(&mut self) {
        // `0x0ED0A`: the pointer off the screen first.
        self.pointer_off();
        let before = self.loc;
        self.mode_reset();
        self.task_stop(0);
        self.task_stop(1);
        self.panel_tint(4);
        // `0x0ED2C` runs the frame pass before the transition. This consumes the stop requests,
        // including task 0's panel release, before the destination
        // replaces the task records.
        self.frame_pass();
        let _ = before;
        // 0x0ED2F: unless both places are forest rows, the music faded
        // over 1500 ms (0x0ED39), one of six tints rolled (0x0ED42) for the
        // transition (0x0ED4F), and the music stopped once it has played
        // (0x0ED55) — a second into the fade, so what still sounds dies
        // away from the level the fade has reached.
        if Some(self.place_pair_kind()) != self.profile.leave_spares_kind {
            self.music_call(music::Call::Fade(num::signed(self.profile.leave_fade)));
            let effect = num::low8(self.roll(1, 6));
            self.events.push(Event::Transition(effect));
            let picture = self.render_scene();
            self.effects.exit(effect, &picture, &self.presented);
            self.music_call(music::Call::Stop);
        }
        // `0x0ED5A`–`0x0ED68`: after the effect the line is set to the
        // empty string at `DS:0546h` and the band painted with it.
        self.status_line.clear();
        self.leave = None;
    }

    /// `0EB9h:0001h`, the visit's own setup: the location the main loop's
    /// word names is loaded, afresh on every visit but between two forest
    /// rows — the leaving released the slot (`0766h:0D34h`). Answers
    /// whether that location could be loaded.
    /// The callback asked about object `id`: the library's own answer
    /// when it points where the visit's setup put it, else the title's
    /// routine at that address. One the title does not have is reported
    /// and answers −1, the answer for an object that is not there.
    pub(crate) fn callback(&mut self, id: u16) -> i16 {
        let Some(cb) = self.profile.callback else {
            return -1;
        };
        let at = (self.callback.seg, self.callback.off);
        if at == cb.default {
            return self.callback_default(id);
        }
        if let Some(&(_, f)) = self.callbacks.iter().find(|(a, _)| *a == at) {
            return f(self, id);
        }
        self.events.push(Event::Unbuilt(
            "callback",
            (u32::from(at.0) << 16) | u32::from(at.1),
        ));
        -1
    }

    /// The library's own callback (the routine the visit's setup installs):
    /// −1 for an object that is not there or not live, 1 for one whose
    /// frame is below 6, 0 otherwise — `obj_find`, `test byte [+2], 1`,
    /// `cmp byte [+9], 6`.
    fn callback_default(&self, id: u16) -> i16 {
        let Some(o) = self
            .object_index(num::low8(id))
            .and_then(|i| self.objects.get(i))
        else {
            return -1;
        };
        if o.flags & objects::LIVE == 0 {
            -1
        } else {
            i16::from(o.frame < 6)
        }
    }

    pub(super) fn visit_setup_impl(&mut self) -> bool {
        // A build that keeps a callback puts it back to the library's own
        // answer first, on every visit.
        if let Some(cb) = self.profile.callback {
            self.callback = Far {
                off: cb.default.1,
                seg: cb.default.0,
            };
        }
        // The strip and the figure for this visit (`0BE1h:0000h(1, n)` in
        // a build with several sets): a different digit swaps them.
        let digit = self
            .profile
            .figure_set
            .digit_for(num::signed(self.location_word));
        self.figure_set_load(digit);
        let n = i32::from(num::signed(self.location_word));
        self.set_location(n);
        if self.loc == n {
            self.room_music_load();
        }
        self.loc == n
    }

    /// `0EB9h:0207h(loc, entry)`, the shared entry function.
    pub(super) fn visit_enter_impl(&mut self, loc: i32, entry: i16) {
        // The location's own hooks are installed from the row it is handed
        // (`0x0EE6C`, `0x0EE96`), not from `[1F4Ch]`, which some handlers
        // write only later.
        self.hook_location = loc;
        self.place(i32::from(entry));
        self.entry_door(entry);
        // The entry function fills the subject slots before anything else
        // in the location runs (`0x0EDF7`).
        self.fill_subjects();
        // Its last call, `0CFDh:0F62h`, empties both tasks and points their
        // scratch at the buffer. The location's own bank came fresh out of
        // the file with the slot (`0x7B0D`), so what the machine wrote into
        // the old one is gone with it — but for the picture kept between
        // two forest rows, whose bank stays loaded (`0x8547`).
        self.tasks.t = Default::default();
        self.tasks.queue_len = 0;
        if !self.picture_kept {
            self.record_flag_writes
                .retain(|(r, _), _| r.bank != BankId::Own);
        }
        // `0x0F00B`: `push word [1E66h]`, so `[1E46h]` is whether the
        // figure file is loaded (`0x0DF3A`). Both tasks' dialogues end
        // (`0x0DF4E`, task 1 then 0), the default cue is asked for
        // (`0x0DF8C`, `0x0D69D`) and applied at once, not three passes on:
        // the wanted cue `[1E40h]` goes straight to `0590h:064Dh`
        // (`0x0DF8F`, `0x0DF93`), and the cue in flight `[1E3Eh]` and the
        // countdown `[1E44h]` are cleared behind it (`0x0DF99`, `0x0DF9F`).
        self.tasks.cue_figure = !self.figure_cels.is_empty();
        self.dialogue_over_pointer(1);
        self.dialogue_over_pointer(0);
        self.cue_default();
        self.tasks.cue_current = self.tasks.cue_wanted;
        self.pointer_mode(self.tasks.cue_wanted);
        self.tasks.cue_in = 0;
        self.tasks.enabled = false;
        // `0x0EEF0` acquires owner 4.
        self.panel_tint(4);
        // A build that keeps a callback asks it about a run of objects
        // last, but for one location: an answer of 0 or 1 is the animation
        // each starts on (`03E8h`-style `obj_anim_id`), anything else leaves
        // the object as it is.
        if let Some(cb) = self.profile.callback
            && loc != i32::from(cb.skips_location)
        {
            for id in cb.ids.0..=cb.ids.1 {
                if let k @ (0 | 1) = self.callback(id) {
                    Api::obj_anim_id(self, id, k);
                }
            }
        }
    }

    /// `0EB9h:00B8h`, the entry finished: `0x0ECF4` releases owner 4 (the
    /// release is deferred by the panel refresh machine), the entry ramp,
    /// and `0CFDh:1B89h(1)`: the tasks enabled and ticked once with
    /// nothing.
    pub(super) fn visit_finish_entry_impl(&mut self) {
        // `0x0EC49`: the pointer off the screen while the room comes up.
        self.pointer_off();
        // `0x0EC4E`–`0x0EC55`: one animator pass with no ticks, before the
        // room is drawn (`0BB6h:027Ah`) and the effect plays. A record
        // whose countdown is at zero — an animation just started — takes
        // its first step here, and its first wait is drawn here, ahead of
        // the effect rather than after it: `0883h:0305h` with a count of 0
        // (`0x0EC50`).
        self.animator_tick(0);
        self.panel_release(4);
        // 0x0EC63: the entry effect by the pair of places — the reveal
        // between two forest rows or two F rows, the slide otherwise — and
        // `si`, the room's song, cleared between two forest rows alone
        // (`0x0EC74`).
        let picture = self.render_scene();
        let pair = self.place_pair_kind();
        let reveals = self.profile.entry_reveals;
        self.entry_music = reveals.is_none_or(|(silent, _)| pair != silent);
        if reveals.is_some_and(|(silent, also)| pair == silent || pair == also) {
            self.effects.reveal(&picture, &self.presented);
        } else {
            self.effects.entry(&picture, &self.presented);
        }
        // 0x0EC84: the game's gray ramp, after the effect.
        if let Some((start, rgb)) = self.entry_ramp.clone() {
            self.effects.dac(&mut self.dac, start, &rgb);
        }
        // The effect is synchronous in the original: everything after its
        // call runs when it is over, and the tasks' first tick with it —
        // the first page goes up, and the hook hears it, after the effect,
        // with the clocks where the effect left them.
        if self.effects.busy() {
            self.entry_tail = true;
        } else {
            self.finish_entry_tail();
        }
    }

    /// A host's jump leaves a location the original only ever leaves
    /// once it is entered: an entry whose effect is still playing is
    /// finished first, as if the effect had played.
    pub(crate) fn complete_entry(&mut self) {
        if std::mem::take(&mut self.entry_tail) {
            self.finish_entry_tail();
        }
    }

    /// `0x0EC89`–`0x0ED02`, the rest of the entry once its effect has
    /// played: unfrozen, the keys flushed, the input polled twice, the
    /// pointer shown when hover is on, and `0CFDh:1B89h(1)` — the tasks
    /// enabled and ticked once with nothing. It comes after the effect has
    /// run to its end (`10B8h:0CABh`, `0x0EC7F`): the pointer first
    /// (`179Eh:091Ah`, `0x0EC89`), the tasks last (`0x0ED02`).
    fn finish_entry_tail(&mut self) {
        self.pointer_thaw();
        self.pointer_poll();
        self.pointer_poll();
        if self.hover_enabled {
            self.pointer_on();
        }
        // `0x0ECA9`–`0x0ECB7`: the room's song, endlessly.
        if self.entry_music {
            let at = self.music.map.room_song;
            let song = Far {
                off: self.word(at),
                seg: self.word(at.wrapping_add(2)),
            };
            self.music_call(match self.profile.entry_fade_in {
                Some(ms) => music::Call::PlayFade {
                    song,
                    loops: -1,
                    ms,
                },
                None => music::Call::Play { song, loops: -1 },
            });
        }
        // `0x0ECFA`: the ticks the entry waited out are dropped.
        self.clock.ticks = 0;
        self.tasks.enabled = true;
        self.tick_tasks(0);
    }

    /// `14DDh:08F1h` (`0x156C1`): the kind two places share when the
    /// handler's own place `[1F4Ch]` and the main loop's `[1F4Ah]` differ,
    /// both lie in the table past row 0 (in *Dunkle Schatten* 1 to 60) and
    /// both are forest rows or both of the other kind the entry reveals
    /// into (in *Dunkle Schatten* 3 and 5, the F rows); 0 otherwise.
    pub(crate) fn place_pair_kind(&self) -> u16 {
        let (a, b) = (self.here_word, self.location_word);
        let rows = 1..num::word(self.locations.count);
        if a == b || !rows.contains(&a) || !rows.contains(&b) {
            return 0;
        }
        let kind = |n: u16| {
            self.locations
                .kind(&self.image, usize::from(n))
                .unwrap_or(0)
        };
        let k = kind(b);
        if kind(a) == k
            && self
                .profile
                .forest
                .is_some_and(|f| k == f.kind || k == f.reveal)
        {
            k
        } else {
            0
        }
    }

    /// One tick while effects play: the clock went on in the interrupt,
    /// the effect takes its next step when its wait is over.
    pub(crate) fn effects_tick(&mut self) {
        let hundredths = self.clock.hundredths_1c82_mut();
        self.effects
            .tick(clock::TICK_PERIOD, &mut self.dac, hundredths);
        for call in self.effects.take_due() {
            self.music_run(call);
        }
        if !self.effects.busy()
            && let Some(v) = self.ticks_after_effect.take()
        {
            self.clock.ticks = v;
        }
        if self.entry_tail && !self.effects.busy() {
            self.entry_tail = false;
            self.finish_entry_tail();
        }
    }

    pub(crate) fn roll(&mut self, lo: u16, hi: u16) -> u16 {
        self.rng.inclusive(lo, hi)
    }

    /// The first visible item, for a test.
    #[cfg(test)]
    pub(crate) fn scroll_for_test(&self) -> usize {
        self.scroll
    }

    /// The generator's seed, `cs:[6]` and `cs:[8]`, for a test to count
    /// the rolls that led to it.
    #[cfg(test)]
    pub(crate) fn seed_for_test(&self) -> (u16, u16) {
        (self.rng.lo, self.rng.hi)
    }

    /// Where an entry id puts the player.
    ///
    /// Four ids are tried in the order `0x0EF4B` tries them, and the first
    /// that names a node in this location's graph wins: the entry plus
    /// `7D0h` — the twin a room carries when it wants the player to walk
    /// in — then the entry itself, then node `7Fh`, and last the first
    /// exit of the location's own row (`0x0EFF3`, the byte at `+9`). The
    /// third is asked with `0FFFEh` rather than `0FFFFh`, and both are
    /// below zero, so both take the facing out of the hotspot and turn it
    /// round (`0x0B6DA`).
    ///
    /// With the twin, `0x0EF57` snaps onto it, `0x0EF67` puts the entry
    /// into `[289Fh]` and `0x0EF6F` walks to the node with the plain id.
    /// When none of the four names a node, the figure's record stays
    /// where it was and out of every chain — which is a room without the
    /// player in it, as the Karte is.
    pub(super) fn place(&mut self, entry: i32) {
        // `0A9Fh:0C59h` and `0617h` both stop an animation on the figure
        // first (`14DDh:01D4h`).
        self.stop_player_anim();
        if entry == -1 {
            self.place_restored();
            return;
        }
        let mut w = self.take_player();
        w.unlink();
        let Some(room) = self.room() else {
            self.walker = Some(w);
            self.sync_links();
            return;
        };
        let Some(graph) = room.walk() else {
            self.walker = Some(w);
            self.sync_links();
            return;
        };
        let has = |id: i32| u16::try_from(id).is_ok_and(|id| graph.lookup(id).is_some());
        let first_exit = usize::try_from(self.loc)
            .ok()
            .and_then(|n| {
                self.locations
                    .exits(&self.image, n)
                    .first()
                    .map(|&(e, _)| i32::from(e))
            })
            .unwrap_or(0);

        let mut walk_target = 0;
        let mut snapped = false;
        if has(entry + ENTRY_TWIN) {
            let facing = self.facing_for(room, entry + ENTRY_TWIN);
            if let Ok(twin) = i16::try_from(entry + ENTRY_TWIN) {
                snapped = w.snap(graph, twin, i16::from(facing));
            }
            if let Ok(id) = i16::try_from(entry) {
                walk_target = id;
                w.walk_to(graph, id, -1);
            }
        } else if let Some(id) = [entry, FALLBACK_NODE, first_exit]
            .into_iter()
            .find(|&id| has(id))
        {
            let facing = self.facing_for(room, id);
            if let Ok(id) = i16::try_from(id) {
                snapped = w.snap(graph, id, i16::from(facing));
            }
        }
        self.walker = Some(w);
        if snapped {
            self.player_snapped();
        }
        self.sync_links();
        self.walk_target = walk_target;
    }

    /// `0x0EFBC`: the entry −1, which only a restored game makes, puts the
    /// figure back on the node its record names, facing the way its block
    /// says (`0A9Fh:10B5h`, `0x0BAA5`) — when there is a figure at all
    /// (`[1E66h]`, `0x0EFC1`) and the room has a walk graph (`[2865h]`,
    /// `0x0BAA8`). The routine is handed the walker record `DS:1693h`
    /// (`0x0EFCC`); the facing is the record's `+2Ch` over its phases
    /// `+15h` plus one (`0x0BAC2`, `0x0BACA`), and the node is its `+28h`
    /// (`0x0BAD6`), snapped onto by `0A9Fh:0C59h` (`0x0BADF`).
    ///
    /// Everything else the figure looks like comes with the record: the
    /// snap turns it only when the facing's block differs from `+2Ch`
    /// (`0x0ABF1`), so the cel stands on the restored `+0Ch` and mirror
    /// bit.
    fn place_restored(&mut self) {
        if self.figure_cels.is_empty() {
            return;
        }
        let mut w = self.take_player();
        let Some(graph) = self.room().and_then(|r| r.walk()) else {
            self.walker = Some(w);
            return;
        };
        // The room's graph header is new; nothing of the last one's walk
        // carries over.
        w.unlink();
        let (node, facing) = (w.dest_id, w.facing());
        let snapped = w.snap_or_unlink(graph, node, facing);
        self.walker = Some(w);
        if snapped {
            self.player_snapped();
        }
        self.sync_links();
        self.walk_target = 0;
    }

    /// `0x0EF0F`: an entry between 100 and 109 whose object is of kind 4
    /// is a door, remembered in `[1EDEh]` and shown open — `14DDh:0A06h`
    /// sets its live bit and puts it on frame 1 (`0x157F1`, `0x157FB`).
    fn entry_door(&mut self, entry: i16) {
        if !(100..=109).contains(&entry) {
            return;
        }
        let Ok(id) = u8::try_from(entry) else {
            return;
        };
        let door = self.profile.entry_door;
        if self
            .object_index(id)
            .is_some_and(|i| self.objects[i].kind == u16::from(door.kind))
        {
            if door.remembered {
                self.entry_door = entry;
            }
            self.object_show_frame(id, door.frame);
        }
    }

    /// Which way the player looks after being put on a node with a facing
    /// below zero.
    ///
    /// `0x0B6AA` walks the location's hotspots for one whose verb mask
    /// ends in this id and takes bits 12 and 13 of that mask, xored with
    /// 2 (`0x0B6E3`) — the figure faces away from the door it came
    /// through; a location with no such hotspot leaves facing 2, the
    /// front (`0x0B6D5`).
    pub(super) fn facing_for(&self, room: &RoomFile, id: i32) -> u8 {
        let Ok(id) = u8::try_from(id & 0xFF) else {
            return 2;
        };
        room.hotspots
            .spots
            .iter()
            .find(|h| num::low8(h.verb_mask) == id)
            .map_or(2, |h| num::low8((h.verb_mask >> 12) & 3) ^ 2)
    }

    /// `0A9Fh:0F42h(1693h)` (`0x0B932`): stop the figure's animation,
    /// mark its old tiles and remove its draw-chain link.
    ///
    /// `0x0B94C` stops the animation; `0x0B95A` marks the tiles. The
    /// final call at `0x0B99C` reaches `0x0AA1F`, which replaces a layer
    /// head with the record's successor (`0x0AA78`) or bypasses the record
    /// in a predecessor's next link (`0x0AB1A`). It does not insert the
    /// record. Insertion is the separate routine at `0x0AB4A`, called
    /// by the snap at `0x0B7E2`. Removing the link preserves position,
    /// node and movement state; leaving the room is a different operation.
    pub(crate) fn detach_player(&mut self) {
        self.stop_player_anim();
        if let Some(w) = &mut self.walker {
            w.detach_draw_chain();
        }
        self.sync_links();
    }

    /// Whether row `loc` is a forest row: in *Dunkle Schatten* the Wald
    /// and the maze.
    pub(crate) fn forest_row(&self, loc: u16) -> bool {
        let kind = self.locations.kind(&self.image, usize::from(loc));
        self.profile.forest.is_some_and(|f| kind == Some(f.kind))
    }

    /// The `DSROOM.DAT` slot the room loader (`0766h:010Ah`) takes for row
    /// `loc`: the forest picture's slot for a forest row (in *Dunkle
    /// Schatten* `0x077ED` reads the kind, `0x077F2` compares it with 3,
    /// `0x077F7` takes slot 14h), and the row's own slot otherwise
    /// (`0x077FC`). The picture, the walk
    /// graph, the animations and the row's dialogue bank all come out of
    /// that slot.
    pub(crate) fn room_slot(&self, loc: u16) -> u16 {
        if let Some(f) = self.profile.forest.filter(|_| self.forest_row(loc)) {
            f.slot
        } else {
            loc
        }
    }

    /// The slot the current location was loaded from.
    fn slot(&self) -> Option<usize> {
        u16::try_from(self.loc)
            .ok()
            .map(|n| usize::from(self.room_slot(n)))
    }

    /// The playfield of the current location, when it has one: for a
    /// forest row the picture with the row's map bound to it, for any
    /// other the slot's file.
    pub(crate) fn room(&self) -> Option<&RoomFile> {
        loaded_room(
            self.bound_map.as_ref(),
            self.edited_room.as_ref(),
            &self.rooms,
            self.slot(),
        )
    }

    /// `0766h:0D34h`, the slot of the location just left released. Between
    /// two forest rows the picture stays and the map's bindings are undone
    /// (`0x86CE`); the next visit's setup loads its slot over any other.
    pub(crate) fn slot_release(&mut self) {
        self.room_music_unload();
        if self.forest_kept() {
            self.slot_kept = self.unbind_object_map();
        }
    }

    /// The dialogue bank of the slot the current location was loaded from
    /// (`0x7B0D`: `+0Ch` of the slot, its resource 3).
    pub(crate) fn own_bank(&self) -> Option<&Bank> {
        self.slot()
            .and_then(|n| self.own_banks.get(n))
            .and_then(Option::as_ref)
    }

    /// The palette the current picture is indexed by.
    pub(crate) fn palette(&self) -> &Palette {
        &self.dac
    }

    /// Where the player's own record resolves to (`0x0D2DA`).
    ///
    /// The walker record is an object record like any other — the
    /// resolver does not know it is the player — so the same fields are
    /// read: the point at `+4`/`+6`, the cel's size scaled by `+0Eh` and
    /// `+10h`, and the alignment at `+3`. The walker record carries
    /// alignment 2, so a figure resolves to the middle of its
    /// feet lifted by its own height: the top of its head.
    ///
    /// Answers the point and the cel's scaled width and height.
    pub(crate) fn player_box(&self) -> Option<(i32, i32, i32, i32)> {
        let p = self.walker.as_ref()?;
        let (cel, _) = render::figure_cel(p);
        let cel = self.figure_cels.get(cel)?;
        let scale = num::small(p.scale);
        let w = scaled(i32::from(cel.width), scale);
        let h = scaled(i32::from(cel.height), scale);
        let (x, y) = align_point(WALKER_ALIGN, p.x, p.y, w, h);
        Some((x, y, w, h))
    }

    /// The same for an object of this location, found by the id it carries
    /// at `+13h` — which is how the two record setters name one.
    pub(crate) fn object_point(&self, id: u8) -> Option<(i32, i32)> {
        self.slot_object_box(id).map(|(x, y, _, _)| (x, y))
    }

    /// The same with the cel's scaled width and height beside it.
    pub(crate) fn slot_object_box(&self, id: u8) -> Option<(i32, i32, i32, i32)> {
        let o = self.objects.get(self.object_index(id)?)?;
        // An object whose cel this engine cannot find answers with the
        // bare point rather than one aligned against a cel it did not
        // find.
        let Some((w, h)) = self.object_cel_size(o.bundle, o.cel) else {
            return Some((o.x, o.y, 0, 0));
        };
        let w = scaled(w, o.scale.0);
        let h = scaled(h, o.scale.1);
        let (x, y) = align_point(o.align, o.x, o.y, w, h);
        Some((x, y, w, h))
    }

    /// What the entry function makes of a location's own object list
    /// (`0x0EDF7`–`0x0EE66`).
    ///
    /// Nine ids sit in the record from `+19h` on, and the loop hands each
    /// to the object lookup and puts what comes back in the **next** slot:
    /// `mov ax,di` then `inc di` before the push, so list entry `i` lands
    /// in slot `i + 1` and slot 0 is left to the location's own script
    /// (`0x0EDF9`). The id is the record's byte at `+19h` (`0x0EE01`); a
    /// zero clears the slot (`0x0EE0C`), any other is looked up with
    /// `0BB6h:052Bh` (`0x0EE0F`) and put in the slot with `0CFDh:1A8Ch`
    /// (`0x0EE20`), nine times over (`0x0EE53`).
    ///
    /// Before the loop, slot 0 takes the player — but only when there is a
    /// figure at all. `0x0EDE0` asks `[1E66h]`, which `0x0EBB2` sets to
    /// one exactly when the far pointer at `[2865h]`/`[2867h]` is not null,
    /// so it is the question "is the figure file loaded", not "did an
    /// entry find a node". After the loop, slot 10 takes a literal point:
    /// `push 40h, 0A0h, 53h, 0Ah` at `0x0EE58` is the raw setter with
    /// code `53h` and the point (160, 64) — the middle of the playfield.
    pub(super) fn fill_subjects(&mut self) {
        self.subjects = Subjects::default();
        if !self.figure_cels.is_empty() {
            self.subjects.set(0, 0, Subject::Player);
        }
        let Ok(here) = usize::try_from(self.loc) else {
            return;
        };
        for (i, id) in self
            .locations
            .object_ids(&self.image, here)
            .into_iter()
            .enumerate()
        {
            let slot = i + 1;
            if id == 0 {
                self.subjects.clear(slot, 0);
            } else if self.object_point(id).is_some() {
                self.subjects.set(slot, 0, Subject::Object(id));
            } else if let Some(subject) = self.mover_subject(id) {
                // No object by that id: `0BB6h:05B0h` and the figure
                // setter `0CFDh:1AD0h` take the person's record instead
                // (`0x0EE2E`–`0x0EE3E`).
                self.subjects.set(slot, 0, subject);
            } else {
                // The lookup found nothing, and both setters stamp
                // `3000h` when they are handed a null pointer.
                self.subjects.clear(slot, 0);
            }
        }
        if let Some(s) = self.profile.screen_slot {
            self.subjects.set_point(
                usize::from(s.slot),
                s.code,
                num::signed(s.x),
                num::signed(s.y),
            );
        }
    }

    /// Installs the panel color curves and the entry ramp. Dunkle Schatten
    /// opts in before entering its initial room.
    pub(crate) fn enable_panel_colors(
        &mut self,
        curves: crate::titles::palette1994::PanelCurves,
        ramp: (u8, Vec<u8>),
    ) -> Result<()> {
        let bank = self.banks.first().ok_or("missing panel bank")?;
        self.panel_colors = Some(crate::titles::palette1994::PanelColors::new(bank, curves)?);
        self.entry_ramp = Some(ramp);
        self.flatten.enable_updates();
        self.rebuild_dac();
        self.sync_panel_colors();
        Ok(())
    }

    /// `0x0C971`: normal dialogue/input ownership uses variant zero.
    pub(super) fn panel_tint(&mut self, mask: u16) {
        if let Some(p) = &mut self.panel_colors {
            p.tint(mask);
        } else {
            self.status_dim |= mask;
        }
        self.sync_panel_colors();
    }

    /// `0x0C9CA`: a choice menu dims the colors and freezes refresh.
    fn panel_dim(&mut self, mask: u16) {
        if let Some(p) = &mut self.panel_colors {
            p.dim(mask);
        }
        self.sync_panel_colors();
    }

    /// `0x0CA49`: release an owner after the pending panel refresh.
    fn panel_release(&mut self, mask: u16) {
        if let Some(p) = &mut self.panel_colors {
            p.release(mask);
        } else {
            self.status_dim &= !mask;
        }
        self.sync_panel_colors();
    }

    /// `0x0ED82`: called once after movement/animation, before task painting.
    /// The figure's loader with kind 1 (`0BE1h:0000h(1, n)`, Dunkle
    /// Schatten's `109Dh:000Eh`): nothing when `n` is the set already
    /// loaded (Dunkle Schatten's `0x10A01`); otherwise the strip and the
    /// figure of set `n`, `n` kept in the player's record `+1Eh`
    /// (`0x10A68`) — which the save carries — and the strip's loader on the
    /// new file (`0x10A79`), which splits the carried list anew.
    pub(crate) fn figure_set_load(&mut self, n: u16) {
        if n == self.figure_digit {
            return;
        }
        let Some((_, set)) = self.figure_sets.iter().find(|(d, _)| *d == n) else {
            return;
        };
        self.panel = set.panel.clone();
        self.item_cels = set.item_cels.clone();
        self.figure_cels = set.figure_cels.clone();
        self.figure_digit = n;
        let [lo, hi] = n.to_le_bytes();
        self.set_player_record_byte(player::FIGURE_SET, lo);
        self.set_player_record_byte(player::FIGURE_SET + 1, hi);
        self.strip_range();
    }

    /// The start's call of the figure's loader with `n = −1` (`0x0072D`,
    /// Dunkle Schatten's `0x04F7A`), before the options: the set the
    /// player's record remembers (Dunkle Schatten's `0x10A58`) — 0 as the
    /// image ships it,
    /// or what a saved game restored at the start carried. Then the strip
    /// has its range.
    pub(crate) fn figure_set_at_start(&mut self) {
        let n = u16::from_le_bytes([
            self.player_record_byte(player::FIGURE_SET),
            self.player_record_byte(player::FIGURE_SET + 1),
        ]);
        self.figure_set_load(n);
        self.strip_range();
    }

    fn panel_frame(&mut self) {
        if let Some(p) = &mut self.panel_colors {
            p.frame();
        }
        self.sync_panel_colors();
    }

    /// Palette and flatten-band ownership must observe the same pending
    /// releases; rendering itself must not advance a refresh countdown.
    fn sync_panel_colors(&mut self) {
        if let Some(p) = &self.panel_colors {
            self.status_dim = p.mask;
            p.apply(&mut self.dac);
        }
    }

    /// The room's own banks, as the entry pass's loop loads them
    /// (`0x15313`–`0x1535F`, `0590:0A1F` for each): written to the DAC
    /// behind the effects that are still to play, and to the flatten
    /// lookup at once — only a picture drawn after the load reads it.
    /// Everything else in the DAC — the strip's, the pointer's and the
    /// player's banks, the ramp and the panel's variant — is no room's
    /// and stays as it is.
    fn load_room_palette(&mut self) {
        let Some(room) = self.room() else {
            return;
        };
        let start = room.room.palette_start;
        let rgb = room.room.palette_rgb.clone();
        if self.flatten.table.is_some() {
            self.flatten.load_rgb(start, &rgb);
        }
        self.effects.dac(&mut self.dac, start, &rgb);
    }

    /// The DAC: the location's own palette, then the banks the strip, the
    /// pointer and the player load over it.
    pub(super) fn rebuild_dac(&mut self) {
        let mut dac = self.room().map(crate::dac::of_room).unwrap_or_default();
        if self.flatten.table.is_some()
            && let Some(room) = self.room()
        {
            let start = room.room.palette_start;
            let rgb = room.room.palette_rgb.clone();
            self.flatten.load_rgb(start, &rgb);
        }
        for bank in &self.banks {
            dac.apply(bank);
            if bank.len() >= 2 {
                self.flatten.load_rgb(bank[0], &bank[2..]);
            }
        }
        if let Some((start, rgb)) = &self.entry_ramp {
            dac.load(*start, rgb);
        }
        if let Some(panel) = &self.panel_colors {
            panel.apply(&mut dac);
        }
        self.dac = dac;
    }
}

/// The rectangle a program reserves for the strip, as `(y, height)`.
///
/// The program marks its full-width regions the same way — four words
/// pushed and a far call — and among them the one that starts where
/// the playfield ends and reaches the bottom of the screen is the strip's.
/// The words are the height, the width, the row and the column, and the
/// call goes to `1BBDh:2036h`.
///
/// Dunkle Schatten's is at `0x0D012`: 72 rows, 320 wide, at row 128 and
/// column 0, divided into a 12-row band and a 60-row strip.
///
/// Reading it beats computing it. The arithmetic that a 128-row playfield
/// and a 72-row picture leave one placement happens to give the same
/// answer, but it gives no answer at all for a picture shorter than its
/// area.
fn panel_area(img: &Image) -> Result<(i32, i32)> {
    let b = img.bytes();
    let width = SCREEN_W.to_le_bytes();
    let mut seen: Option<(i32, i32)> = None;
    for i in 4..b.len().saturating_sub(14) {
        // push imm16 of the screen width, with a height pushed before it.
        if b[i] != 0x68 || b[i + 1..i + 3] != width {
            continue;
        }
        let Some(h) = push_before(b, i) else { continue };
        let Some((y, n)) = push_at(b, i + 3) else {
            continue;
        };
        let Some((x, m)) = push_at(b, i + 3 + n) else {
            continue;
        };
        if b[i + 3 + n + m] != 0x9A || x != 0 || y != PLAYFIELD_H || y + h != SCREEN_H {
            continue;
        }
        match seen {
            None => seen = Some((y, h)),
            Some(prev) if prev == (y, h) => {}
            Some((py, ph)) => {
                return Err(format!(
                    "the program reserves both {ph} rows at {py} and {h} at {y} for the \
                     strip; which is the strip's is not decidable"
                )
                .into());
            }
        }
    }
    seen.ok_or_else(|| "the program never reserves a strip under the playfield".into())
}

/// The value and length of a `push imm8`/`push imm16` at `i`.
fn push_at(b: &[u8], i: usize) -> Option<(i32, usize)> {
    match b.get(i)? {
        0x6A => Some((i32::from(num::signed8(*b.get(i + 1)?)), 2)),
        0x68 => Some((
            i32::from(u16::from_le_bytes([*b.get(i + 1)?, *b.get(i + 2)?])),
            3,
        )),
        _ => None,
    }
}

/// What [`Adventure::room`] answers, borrowed from the fields it lives in
/// alone, for a caller that writes the records beside it while it reads.
fn loaded_room<'a>(
    bound: Option<&'a objects::BoundMap>,
    edited: Option<&'a RoomFile>,
    rooms: &'a [Option<RoomFile>],
    slot: Option<usize>,
) -> Option<&'a RoomFile> {
    match (bound, edited) {
        (Some(m), _) => Some(&m.room),
        (None, Some(r)) => Some(r),
        (None, None) => slot.and_then(|n| rooms.get(n)).and_then(Option::as_ref),
    }
}

/// The value of the push that ends where `i` begins.
fn push_before(b: &[u8], i: usize) -> Option<i32> {
    [2usize, 3]
        .into_iter()
        .find_map(|n| push_at(b, i.checked_sub(n)?).filter(|&(_, len)| len == n))
        .map(|(v, _)| v)
}

/// A location's own dialogue bank, out of its room slot.
///
/// The loaded slot is six far pointers in resource order, and the room
/// loader takes the fourth — `mov dx, es:[bx+0Ch]` at `0x7B0D` — into
/// `[2855h]` (`0x7B14`), then counts it into `[1EEAh]` with the routine
/// that counts bank 999 (`0x7B1A`, `0590h:1893h`). So the bank is resource
/// 3, and it has the same nested directory the global banks have. A slot
/// without one leaves the globals at zero (`0x7B27`), and so does this.
/// Resource 2 of every slot of `name`, where it is an object map.
fn load_object_maps(res: &Path, name: &str) -> Result<Vec<Option<ObjectMap>>> {
    let path = find_ci(res, name).ok_or_else(|| format!("{name} not found"))?;
    let bytes = std::fs::read(path)?;
    let Dat::Fgroup(group) = open_dat(&bytes)? else {
        return Err(format!("{name} is not an FGROUP").into());
    };
    group
        .slots
        .iter()
        .map(|slot| match slot {
            Slot::Rib(rib) => match rib.get(2).filter(|b| b.starts_with(b"OMP\0")) {
                Some(blob) => Ok(Some(ObjectMap::parse(blob)?)),
                None => Ok(None),
            },
            Slot::Null | Slot::Raw(_) => Ok(None),
        })
        .collect()
}

fn own_bank(rib: &Rib) -> Result<Option<Bank>> {
    let Some(blob) = rib.get(3) else {
        return Ok(None);
    };
    if !Bank::looks_like(blob) {
        return Ok(None);
    }
    Ok(Some(Bank::parse(blob)?))
}

fn load_rooms(res: &Path, name: &str) -> Result<Vec<Option<RoomFile>>> {
    let path = find_ci(res, name).ok_or_else(|| format!("{name} not found"))?;
    let bytes = std::fs::read(path)?;
    let Dat::Fgroup(group) = open_dat(&bytes)? else {
        return Err(format!("{name} is not an FGROUP").into());
    };
    group
        .slots
        .iter()
        .map(|slot| match slot {
            Slot::Rib(rib) => RoomFile::from_1994_slot(rib)
                .transpose()
                .map_err(Into::into),
            Slot::Null | Slot::Raw(_) => Ok(None),
        })
        .collect()
}

/// The own bank of every slot, in slot order.
fn load_own_banks(res: &Path, name: &str) -> Result<Vec<Option<Bank>>> {
    let path = find_ci(res, name).ok_or_else(|| format!("{name} not found"))?;
    let bytes = std::fs::read(path)?;
    let Dat::Fgroup(group) = open_dat(&bytes)? else {
        return Err(format!("{name} is not an FGROUP").into());
    };
    group
        .slots
        .iter()
        .map(|slot| match slot {
            Slot::Rib(rib) => own_bank(rib),
            Slot::Null | Slot::Raw(_) => Ok(None),
        })
        .collect()
}

/// One `RESOURCE/*.DAT` opened as a RIB.
/// One set of strip and figure files: the strip's picture and bank, the
/// carried items' cels, the figure's cels and bank.
#[derive(Debug, Clone)]
pub(crate) struct FigureFiles {
    panel: Picture,
    panel_bank: Option<Vec<u8>>,
    item_cels: Vec<Option<Picture>>,
    figure_cels: Vec<Picture>,
    player_bank: Option<Vec<u8>>,
}

impl FigureFiles {
    /// The set with digit `d`: the names' `%` replaced by it (a name
    /// without one is the only set). With `several` sets an item whose
    /// resource is not a cel in this strip file has none here; with one,
    /// that is an error.
    fn load(
        res: &Path,
        image: &Image,
        files: &Files,
        items: usize,
        d: u16,
        several: bool,
    ) -> Result<Self> {
        let digit = char::from_digit(u32::from(d), 10).unwrap_or('0');
        let named = |n: &str| n.replace('%', &digit.to_string());
        let panel_name = named(files.panel);
        let panels = rib(res, &panel_name)?;
        let panel = Picture::parse(
            panels
                .get(0)
                .ok_or_else(|| format!("{panel_name} has no resource 0, the verb strip"))?,
        )?;
        // `0x0C283`: ids address a byte mapping, not consecutive panel
        // resources. Every id is kept, aliases too.
        let item_cels = crate::titles::xp1994::item_resources(image, items)?
            .into_iter()
            .enumerate()
            .map(|(id, resource)| -> Result<Option<Picture>> {
                let cel = panels
                    .get(resource)
                    .ok_or_else(|| {
                        format!("item {}: missing panel resource {resource}", id + 1).into()
                    })
                    .and_then(|data| {
                        Picture::parse_prefix(data).map_err(|e| {
                            crate::error::Error::from(format!(
                                "item {}: panel resource {resource}: {e}",
                                id + 1
                            ))
                        })
                    });
                match cel {
                    Ok(c) => Ok(Some(c)),
                    Err(_) if several => Ok(None),
                    Err(e) => Err(e),
                }
            })
            .collect::<Result<_>>()?;
        let player = rib(res, &named(files.player))?;
        // Bundle 1 is the figure file from resource 1 on: the loader
        // stores the whole file and then advances the slot by four bytes,
        // one far pointer, which is the palette these files open with
        // (`0x10AD0`).
        let figure_cels = player
            .resources
            .iter()
            .skip(1)
            .filter_map(|b| Picture::parse(b).ok())
            .collect();
        Ok(FigureFiles {
            panel,
            panel_bank: panels.get(1).map(<[u8]>::to_vec),
            item_cels,
            figure_cels,
            player_bank: player.get(0).map(<[u8]>::to_vec),
        })
    }
}

fn rib(res: &Path, name: &str) -> Result<Rib> {
    let path = find_ci(res, name).ok_or_else(|| format!("{name} not found"))?;
    Ok(Rib::parse(&std::fs::read(path)?)?)
}
