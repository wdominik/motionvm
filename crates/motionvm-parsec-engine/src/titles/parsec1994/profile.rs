//! The library build a parsec1994 program is linked against.
//!
//! Every title of the generation is its own code linked with the same
//! library, but not always with the same build of it: a routine that plays
//! the same part in two programs can do a little more in one than in the
//! other. What the runtime does differently because of that is a
//! capability, named for the behavior, and it is read off the routine's
//! own body in the program the runtime opens — never from which title it
//! is. [`Profile`] holds every such capability; it is `Copy`, is read whole
//! before the runtime opens and has no method that changes it, the way
//! MOTION's engine holds its own.
//!
//! The title names where its program keeps each routine ([`LibraryMap`]),
//! as it names where its data segment keeps each word; what a body means is
//! the generation's. A body that is neither of the shapes a reader knows is
//! refused: the alternative is guessing which behavior a build has.

use crate::error::Result;
use crate::exe::Image;

/// Where a program keeps the library routines whose bodies
/// [`Profile::read`] tells apart, as `(segment, offset)`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LibraryMap {
    /// The input lock: the lock bit set, the band held.
    pub input_lock: (u16, u16),
    /// The input lock lifted: the lock bit cleared, the band let go.
    pub input_unlock: (u16, u16),
    /// Hover off: the hover word cleared, the status refreshed.
    pub hover_off: (u16, u16),
    /// A dialogue task started.
    pub task_start: (u16, u16),
    /// The script's exit gate installed.
    pub script_exit: (u16, u16),
    /// The entry finished: first picture, effect, music, tasks.
    pub visit_finish_entry: (u16, u16),
    /// Leaving a location.
    pub visit_leave: (u16, u16),
    /// Hover on again.
    pub hover_on: (u16, u16),
    /// An object's state as the logic asks it.
    pub obj_state: (u16, u16),
    /// The player walked to a node.
    pub walk_to_node: (u16, u16),
    /// An animation started on an object.
    pub obj_anim: (u16, u16),
    /// The location's objects kept in its state row.
    pub objs_save_state: (u16, u16),
    /// One frame of input dispatched.
    pub dispatch: (u16, u16),
    /// The mode machine reset.
    pub mode_reset: (u16, u16),
    /// The room loader: which slot a row's picture is taken from.
    pub room_loader: (u16, u16),
    /// The kind two places share.
    pub place_pair: (u16, u16),
    /// The slot just left released.
    pub slot_unload: (u16, u16),
    /// The shared entry: the location's objects, slots and hooks.
    pub visit_enter: (u16, u16),
    /// An item out of the player's hands.
    pub inv_take: (u16, u16),
    /// The visit's own setup.
    pub visit_setup: (u16, u16),
    /// Where a subject slot is on the screen: the resolver that reads the
    /// slot word.
    pub slot_resolve: (u16, u16),
    /// A room's effects fetched by its mask.
    pub effects_load: (u16, u16),
    /// A room slot's song loaded.
    pub slot_song: (u16, u16),
    /// The strip's loader, which sets the carried items' visible range.
    pub strip_load: (u16, u16),
    /// The scan of a page's controls, which sets task 0's flags.
    pub text_scan: (u16, u16),
    /// What task 0 is doing, asked once a frame; with a text up it takes
    /// the click or key that would cut the page short.
    pub task_query: (u16, u16),
}

/// The bits of task 0's flags `+1Ah` that a page's controls set, and what
/// a click makes of them. Bit 15, armed, and bits 0 and 1, read for tasks
/// other than 0, are the same in every build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextFlags {
    /// `03 01`: the page waits for a click; the tick parks its countdown.
    pub hold: u16,
    /// `03 02`: a click leaves the page as it is.
    pub firm: u16,
    /// `03 05`, when the build has it: the countdown waits while the bit
    /// is set, and a click ends the page at once. Without it, `03 05` is
    /// an event for the hook like any control the scan does not know.
    pub quick: Option<u16>,
}

/// Which of its strip and figure files a build loads for a visit: the
/// digit `n` the files' names take for their `%`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FigureSet {
    /// Always the same one.
    Fixed(u16),
    /// 1 once the location word is above `above`, else 0.
    AfterLocation {
        /// The location word the setup compares.
        word: u16,
        /// The location the second set starts after.
        above: u16,
    },
}

impl FigureSet {
    /// Every digit the build can load.
    pub(crate) fn digits(self) -> Vec<u16> {
        match self {
            FigureSet::Fixed(n) => vec![n],
            FigureSet::AfterLocation { .. } => vec![0, 1],
        }
    }

    /// The digit for a visit to `loc`.
    pub(crate) fn digit_for(self, loc: i16) -> u16 {
        match self {
            FigureSet::Fixed(n) => n,
            FigureSet::AfterLocation { above, .. } => u16::from(loc > crate::num::signed(above)),
        }
    }
}

/// The effect slots a build keeps and how a room claims some of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Effects {
    /// How many slots the table holds.
    pub slots: usize,
    /// The first slot a room's mask names; the ones below are fetched at
    /// the start and kept.
    pub room_first: usize,
    /// The room slots that have a mask: those below this.
    pub rooms: u16,
    /// Whether a mask is a double word rather than a word.
    pub wide_masks: bool,
}

/// How a build that writes the slot word in its own code takes the box's
/// style out of it: the code's two fields, each with the style it stands
/// for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SlotStyle {
    /// The vertical field (`and ax, 30h`), value by value.
    pub vertical: [(u16, u16); 4],
    /// The horizontal field (`and ax, 0C0h`), value by value.
    pub horizontal: [(u16, u16); 2],
}

impl SlotStyle {
    /// The style a code stands for, in the generation's own form: the
    /// vertical placement in bits 0–1, the horizontal in bits 2–3.
    pub(crate) fn style(&self, code: u16) -> u16 {
        let pick = |table: &[(u16, u16)], field: u16| {
            table
                .iter()
                .find(|(c, _)| *c == field)
                .map_or(0, |&(_, s)| s)
        };
        pick(&self.vertical, code & 0x30) | pick(&self.horizontal, code & 0xC0)
    }
}

/// A callback a build keeps in its data segment, through which the entry
/// asks a location about a run of objects, and which a location may bend
/// to a routine of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Callback {
    /// The far pointer's two words, at this offset and the next.
    pub word: u16,
    /// What the visit's setup puts it back to on every visit: the
    /// library's own answer, as `(segment, offset)`.
    pub default: (u16, u16),
    /// The ids the entry asks about, first and last.
    pub ids: (u16, u16),
    /// The location for which the entry does not ask.
    pub skips_location: u16,
}

/// A subject slot the entry fills with a point on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScreenSlot {
    /// The slot.
    pub slot: u16,
    /// The code it is filled with.
    pub code: u16,
    /// The point, in screen pixels.
    pub x: u16,
    /// See [`Self::x`].
    pub y: u16,
}

/// What the entry does with the door the player came in by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntryDoor {
    /// The object kind a door is.
    pub kind: u8,
    /// Whether the door is remembered until it has swung shut.
    pub remembered: bool,
    /// The frame it is shown on; below zero, hidden.
    pub frame: i16,
}

/// The forest a build has: rows of one kind that share one picture, kept
/// loaded between two of them, and a second kind the entry reveals into
/// from another of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Forest {
    /// The forest rows' kind.
    pub kind: u16,
    /// The room slot their picture is taken from.
    pub slot: u16,
    /// The other kind the pair of places is told by.
    pub reveal: u16,
}

/// The capabilities of the library build a program is linked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Profile {
    /// Whether the input lock forgets what the pointer was over (two
    /// `mov word [hover], FFFFh` after the lock bit is set). Without it,
    /// the hotspot under the pointer before the lock is still the one the
    /// dispatcher compares with after it.
    pub lock_forgets_hover: bool,
    /// Whether lifting the lock forgets it too.
    pub unlock_forgets_hover: bool,
    /// Whether hover off forgets it too, and puts the status refresh age
    /// at 1, before the status is refreshed.
    pub hover_off_forgets_hover: bool,
    /// Whether starting a dialogue task tells the location's hook, with
    /// code 0Ah and the task's script, through the far hook it installed
    /// (`push 0Ah ; call far [hook]`).
    pub task_start_tells_hook: bool,
    /// Whether the script's exit gate turns hover off before it installs
    /// itself (`call far hover_off` as its first instruction).
    pub script_exit_hovers_off: bool,
    /// The fade the music gets when a location is left, in milliseconds
    /// (`push imm16` before the fade's far call).
    pub leave_fade: u16,
    /// The kind two places must share for the leaving to spare the music
    /// and the transition (`cmp ax, k ; je` after the pair's kind is
    /// asked), when the build tests one.
    pub leave_spares_kind: Option<u16>,
    /// The two kinds a pair of places shares for the entry to reveal the
    /// room rather than slide it in, the first of them without the room's
    /// song (`cmp ax, a ; je … ; cmp ax, b ; je …`), when the build tests
    /// them.
    pub entry_reveals: Option<(u16, u16)>,
    /// The fade-in the room's song starts with, in milliseconds, when the
    /// build starts it with one (`push imm16` before the song's far
    /// pointer).
    pub entry_fade_in: Option<u16>,
    /// Whether hover on forgets what the pointer was over and puts the
    /// refresh age at 1 (two `mov word [m16], FFFFh` and one `…, 1` after
    /// the hover word is set).
    pub hover_on_forgets_hover: bool,
    /// Whether hover on polls the input twice before it shows the
    /// pointer (four far calls after its stores, not three).
    pub hover_on_polls: bool,
    /// Whether a kind-4 object is live when the two low bits of its flags
    /// are 1 (`and ax, 3 ; cmp ax, 1 ; jne` to the "not live" answer) —
    /// or, the other way round, when they are anything else (`je`).
    pub kind4_live_when_mode_one: bool,
    /// Whether a walk to a node that started is put on record as the walk
    /// target (`mov [m16], si` before the return).
    pub walk_to_node_remembers: bool,
    /// Whether starting an animation first checks that the object has an
    /// animation block and that the number is below its count, and clears
    /// the object's state when not.
    pub anim_checks_count: bool,
    /// The only kind a dormant object is woken for by an animation's
    /// start (`cmp byte es:[bx], k ; jne` to the end), when the build
    /// tests one; otherwise every kind wakes.
    pub anim_wakes_only_kind: Option<u8>,
    /// The kinds whose objects the state row keeps as dormant, hidden or
    /// a frame, one bit per kind.
    pub tri_state_kinds: u8,
    /// Whether the dispatcher's head starts task 0 when a script asked
    /// for it and the figure is linked (`cmp word [m16], 0 ; je` after
    /// its first far call), before anything else.
    pub dispatch_starts_asked_task: bool,
    /// Whether what the dispatcher forgets the hover with is the same as
    /// nothing under the pointer: a build that forgets with `0:0` finds
    /// "nothing" again the same as last time, one that forgets with
    /// `FFFFh:FFFFh` finds it changed.
    pub forgotten_hover_is_nothing: bool,
    /// Whether a click in the room forgets the hover before it takes the
    /// hotspot under the pointer (the forgetting stores right before
    /// `mov ax, [cur+2] ; mov dx, [cur] ; mov [bp-2], ax`).
    pub room_click_forgets_hover: bool,
    /// Whether the dispatcher's tail, with hover off, only takes note of
    /// the hotspot under the pointer and stops task 0 (`cmp word [m16], 0 ;
    /// jne` right after the hotspot is found the same or not), instead of
    /// hovering as with hover on.
    pub hover_off_only_notes: bool,
    /// Whether choosing verb 6 asks for the status line again with an age
    /// of 0Ah (`mov word [m16], 0Ah`).
    pub verb6_refreshes_status: bool,
    /// The verbs below this are the strip's (`not si ; cmp si, n ; jge`).
    pub verb_bound: u16,
    /// Whether a frame under the lock stops task 0 when it shows a hover
    /// text, kind 2, which the query answers −1 for.
    pub locked_stops_hover_text: bool,
    /// Whether the mode reset forgets the hover too (`mov word [hover],
    /// FFFFh`).
    pub mode_reset_forgets_hover: bool,
    /// The forest, when the build has one: the room loader takes its
    /// slot for a row of its kind, the unload keeps that slot, and the
    /// kind two places share is asked of it and of the reveal kind.
    pub forest: Option<Forest>,
    /// The slot the entry fills with a fixed point after the objects'
    /// slots, when the build fills one.
    pub screen_slot: Option<ScreenSlot>,
    /// The door the player came in by.
    pub entry_door: EntryDoor,
    /// How many carried items the strip shows at once, as taking one out
    /// scrolls the list back by it (`add ax, n` on the first visible
    /// slot).
    pub inventory_window: u16,
    /// The callback, when the build keeps one.
    pub callback: Option<Callback>,
    /// How the slot word is laid out. `None`: the kind in bits 12–15 and
    /// the box's style word in bits 0–11, copied as it stands; otherwise
    /// the kind in bits 0–3 and the style recoded from two fields.
    pub slot_style: Option<SlotStyle>,
    /// The effect slots.
    pub effects: Effects,
    /// The room slots that have a song: those below this.
    pub song_slots: u16,
    /// Which strip and figure files a visit loads.
    pub figure_set: FigureSet,
    /// The id the strip's loader splits the carried list at: below it the
    /// first strip file's items, from it on the second's. `None`: the
    /// whole list is in range.
    pub strip_split: Option<u8>,
    /// The flags a page's controls set.
    pub text_flags: TextFlags,
}

impl Profile {
    /// Reads every capability off the routines `map` names in `img`.
    pub(crate) fn read(img: &Image, map: &LibraryMap) -> Result<Self> {
        let d = Dispatch::read(img, map.dispatch)?;
        Ok(Profile {
            lock_forgets_hover: lock_forgets_hover(img, map.input_lock, 0x0E)?,
            unlock_forgets_hover: lock_forgets_hover(img, map.input_unlock, 0x26)?,
            hover_off_forgets_hover: hover_off_forgets_hover(img, map.hover_off)?,
            task_start_tells_hook: task_start_tells_hook(img, map.task_start)?,
            script_exit_hovers_off: script_exit_hovers_off(img, map)?,
            leave_fade: leave(img, map.visit_leave)?.0,
            leave_spares_kind: leave(img, map.visit_leave)?.1,
            entry_reveals: entry_reveals(img, map.visit_finish_entry)?,
            entry_fade_in: entry_fade_in(img, map.visit_finish_entry)?,
            hover_on_forgets_hover: hover_on(img, map.hover_on)?.0,
            hover_on_polls: hover_on(img, map.hover_on)?.1,
            kind4_live_when_mode_one: kind4_live_when_mode_one(img, map.obj_state)?,
            walk_to_node_remembers: walk_to_node_remembers(img, map.walk_to_node)?,
            anim_checks_count: anim_checks_count(img, map.obj_anim)?,
            anim_wakes_only_kind: anim_wakes_only_kind(img, map.obj_anim)?,
            tri_state_kinds: tri_state_kinds(img, map.objs_save_state)?,
            dispatch_starts_asked_task: dispatch_starts_asked_task(img, map.dispatch)?,
            forgotten_hover_is_nothing: d.forgotten_is_nothing,
            room_click_forgets_hover: d.room_click_forgets,
            hover_off_only_notes: d.hover_off_only_notes,
            verb6_refreshes_status: d.verb6_refreshes,
            verb_bound: d.verb_bound,
            locked_stops_hover_text: d.locked_stops_hover_text,
            mode_reset_forgets_hover: mode_reset_forgets_hover(img, map.mode_reset, d.last.0)?,
            forest: forest(img, map)?,
            screen_slot: screen_slot(img, map.visit_enter)?,
            entry_door: entry_door(img, map.visit_enter)?,
            inventory_window: inventory_window(img, map.inv_take)?,
            callback: callback(img, map)?,
            slot_style: slot_style(img, map.slot_resolve)?,
            effects: effects(img, map.effects_load)?,
            song_slots: song_slots(img, map.slot_song)?,
            figure_set: figure_set(img, map.visit_setup)?,
            strip_split: strip_split(img, map.strip_load)?,
            text_flags: text_flags(img, map.text_scan, map.task_query)?,
        })
    }
}

/// The bytes of the routine at `at`, up to `len` of them, or an error that
/// names the routine.
fn body<'a>(img: &'a Image, at: (u16, u16), len: usize, what: &str) -> Result<&'a [u8]> {
    let start = usize::from(at.0) * 16 + usize::from(at.1);
    img.bytes().get(start..start + len).ok_or_else(|| {
        format!(
            "{what} at {:04X}h:{:04X}h lies outside the program",
            at.0, at.1
        )
        .into()
    })
}

/// `mov word [m16], FFFFh`: `C7 06 lo hi FF FF`.
fn is_store_ffff(b: &[u8]) -> bool {
    b.len() >= 6 && b[0] == 0xC7 && b[1] == 0x06 && b[4] == 0xFF && b[5] == 0xFF
}

/// Hover off opens with `mov word [m16], 0` (`C7 06 lo hi 00 00`) and
/// goes on either with the two stores that forget the hover and a third
/// that puts the refresh age at 1, then the status's far call — or with
/// that far call at once. *Dunkle Schatten*'s `0EB9h:18F4h` (`0x10484`)
/// has the stores.
fn hover_off_forgets_hover(img: &Image, at: (u16, u16)) -> Result<bool> {
    let b = body(img, at, 25, "hover off")?;
    if b[..2] != [0xC7, 0x06] || b[4..6] != [0, 0] {
        return Err(refused("hover off", at));
    }
    let tail = &b[6..];
    let age_one = tail.len() >= 19
        && tail[12..14] == [0xC7, 0x06]
        && tail[16..18] == [0x01, 0x00]
        && tail[18] == 0x9A;
    if is_store_ffff(tail) && is_store_ffff(&tail[6..]) && age_one {
        Ok(true)
    } else if tail[0] == 0x9A {
        Ok(false)
    } else {
        Err(refused("hover off", at))
    }
}

/// Starting a task is a `push bp ; mov bp, sp` routine ending in
/// `pop bp ; retf`; the build that tells the hook has `push 0Ah ; call far
/// [m16]` (`6A 0A FF 1E`) before that end. *Dunkle Schatten*'s
/// `0CFDh:1A3Fh` (`0x0EA0F`) has it at `0x0EA50`.
fn task_start_tells_hook(img: &Image, at: (u16, u16)) -> Result<bool> {
    let b = body(img, at, 0x60, "the task start")?;
    let end = b
        .windows(2)
        .position(|w| w == [0x5D, 0xCB])
        .filter(|_| b[..3] == [0x55, 0x8B, 0xEC])
        .ok_or_else(|| refused("the task start", at))?;
    Ok(b[..end].windows(4).any(|w| w == [0x6A, 0x0A, 0xFF, 0x1E]))
}

/// The script's exit gate opens with `push bp ; mov bp, sp` and then
/// either calls hover off (`9A` and the far address the map names) or
/// goes straight to its stores (`C7 06`). *Dunkle Schatten*'s
/// `1586h:05E1h` (`0x15E41`) calls it.
fn script_exit_hovers_off(img: &Image, map: &LibraryMap) -> Result<bool> {
    let at = map.script_exit;
    let b = body(img, at, 8, "the script's exit gate")?;
    if b[..3] != [0x55, 0x8B, 0xEC] {
        return Err(refused("the script's exit gate", at));
    }
    let (seg, off) = map.hover_off;
    let [o0, o1] = off.to_le_bytes();
    let [s0, s1] = seg.to_le_bytes();
    let call = [0x9A, o0, o1, s0, s1];
    if b[3..8] == call {
        Ok(true)
    } else if b[3..5] == [0xC7, 0x06] {
        Ok(false)
    } else {
        Err(refused("the script's exit gate", at))
    }
}

/// The leaving fades the music with `push imm16 ; call far` (`68 lo hi
/// 9A`), the first such push in the routine. The five bytes before it are
/// either `cmp ax, k ; je rel8` (`3D k 00 74 rel`), the kind that spares
/// the music, or the end of the far call that asked for the pair's kind.
/// *Dunkle Schatten*'s `0EB9h:017Ah` (`0x0ED0A`) has `cmp ax, 3` at
/// `0x0ED34` and `push 5DCh` at `0x0ED39`.
fn leave(img: &Image, at: (u16, u16)) -> Result<(u16, Option<u16>)> {
    let b = body(img, at, 0x60, "the leaving")?;
    let i = b
        .windows(4)
        .position(|w| w[0] == 0x68 && w[3] == 0x9A)
        .filter(|&i| i >= 5)
        .ok_or_else(|| refused("the leaving", at))?;
    let fade = u16::from_le_bytes([b[i + 1], b[i + 2]]);
    let before = &b[i - 5..i];
    if before[0] == 0x3D && before[2] == 0 && before[3] == 0x74 {
        Ok((fade, Some(u16::from(before[1]))))
    } else if before[0] == 0x9A {
        Ok((fade, None))
    } else {
        Err(refused("the leaving", at))
    }
}

/// The entry asks for the pair's kind with a far call and then either
/// tests two kinds — `cmp ax, a ; je ; cmp ax, b ; je` (`3D a 00 74 rel
/// 3D b 00 74`) — or goes on to the next far call without a test.
/// *Dunkle Schatten*'s `0EB9h:00B8h` has `cmp ax, 3` at `0x0EC68` and
/// `cmp ax, 5` at `0x0EC6D`.
fn entry_reveals(img: &Image, at: (u16, u16)) -> Result<Option<(u16, u16)>> {
    let b = body(img, at, 0x40, "the entry's end")?;
    let tested = b.windows(10).find(|w| {
        w[0] == 0x3D && w[2] == 0 && w[3] == 0x74 && w[5] == 0x3D && w[7] == 0 && w[8] == 0x74
    });
    Ok(tested.map(|w| (u16::from(w[1]), u16::from(w[6]))))
}

/// The room's song starts as `push -1 ; push word [song+2] ; push word
/// [song]` (`6A FF FF 36`), and a build that fades it in pushes the fade's
/// milliseconds first (`68 lo hi`). *Dunkle Schatten*'s entry pushes
/// nothing before its `push -1` at `0x0ECAD`.
fn entry_fade_in(img: &Image, at: (u16, u16)) -> Result<Option<u16>> {
    let b = body(img, at, 0xC0, "the entry's end")?;
    let i = b
        .windows(4)
        .position(|w| w == [0x6A, 0xFF, 0xFF, 0x36])
        .ok_or_else(|| refused("the entry's end", at))?;
    if i >= 3 && b[i - 3] == 0x68 {
        Ok(Some(u16::from_le_bytes([b[i - 2], b[i - 1]])))
    } else {
        Ok(None)
    }
}

/// Hover on opens with `mov word [m16], 1` (`C7 06 lo hi 01 00`). One
/// build goes on with the two stores that forget the hover and a third
/// that puts the age at 1, then five far calls — the pointer unfrozen, the
/// keys flushed, the input polled twice, the pointer shown; the other goes
/// straight to three of them, without the polls. *Dunkle Schatten*'s
/// `0EB9h:1917h` (`0x104A7`) has the stores and the polls.
fn hover_on(img: &Image, at: (u16, u16)) -> Result<(bool, bool)> {
    let b = body(img, at, 0x40, "hover on")?;
    if b[..2] != [0xC7, 0x06] || b[4..6] != [0x01, 0x00] {
        return Err(refused("hover on", at));
    }
    let forgets = is_store_ffff(&b[6..]) && is_store_ffff(&b[12..]) && b[18..20] == [0xC7, 0x06];
    let calls_at = if forgets { 24 } else { 6 };
    let calls = b[calls_at..]
        .chunks(5)
        .take_while(|c| c.first() == Some(&0x9A))
        .count();
    if b.get(calls_at + 5 * calls) != Some(&0xCB) {
        return Err(refused("hover on", at));
    }
    match calls {
        5 => Ok((forgets, true)),
        3 => Ok((forgets, false)),
        _ => Err(refused("hover on", at)),
    }
}

/// An object's state answers for kind 4 with `and ax, 3 ; cmp ax, 1`
/// (`25 03 00 3D 01 00`) and then `jne` (`75`, live when the bits are 1) or
/// `je` (`74`, live when they are not). *Dunkle Schatten*'s `14DDh:07E7h`
/// has `jne` at `0x1560A`.
fn kind4_live_when_mode_one(img: &Image, at: (u16, u16)) -> Result<bool> {
    let b = body(img, at, 0x64, "the object state")?;
    let i = b
        .windows(6)
        .position(|w| w == [0x25, 0x03, 0x00, 0x3D, 0x01, 0x00])
        .ok_or_else(|| refused("the object state", at))?;
    match b.get(i + 6) {
        Some(0x75) => Ok(true),
        Some(0x74) => Ok(false),
        _ => Err(refused("the object state", at)),
    }
}

/// A walk to a node ends `add sp, 8` (`83 C4 08`) after the walk's far
/// call, and then either `or ax, ax ; je ; mov [m16], si ; pop si ; pop
/// bp ; retf` or `pop bp ; retf` at once. *Dunkle Schatten*'s
/// `14DDh:0A56h` stores at `0x15840`.
fn walk_to_node_remembers(img: &Image, at: (u16, u16)) -> Result<bool> {
    let b = body(img, at, 0x28, "the walk to a node")?;
    let i = b
        .windows(3)
        .position(|w| w == [0x83, 0xC4, 0x08])
        .ok_or_else(|| refused("the walk to a node", at))?;
    let tail = &b[i + 3..];
    if tail.len() >= 10 && tail[..4] == [0x0B, 0xC0, 0x74, 0x04] && tail[4..6] == [0x89, 0x36] {
        Ok(true)
    } else if tail[..2] == [0x5D, 0xCB] {
        Ok(false)
    } else {
        Err(refused("the walk to a node", at))
    }
}

/// The arm for a starting animation loads the object's animation block
/// into `[bp-4]:[bp-2]` (`89 56 FC`) and then either tests it for null
/// (`8B 46 FC 0B 46 FE 74`), before the count, or goes on to the object's
/// live bit (`26 F6 47 02 01`). *Dunkle Schatten*'s `0883h:1F82h` tests at
/// `0x0A7EF`.
fn anim_checks_count(img: &Image, at: (u16, u16)) -> Result<bool> {
    let b = body(img, at, 0x60, "the animation start")?;
    let i = b
        .windows(3)
        .position(|w| w == [0x89, 0x56, 0xFC])
        .ok_or_else(|| refused("the animation start", at))?;
    let tail = &b[i + 3..];
    if tail[..7] == [0x8B, 0x46, 0xFC, 0x0B, 0x46, 0xFE, 0x74] {
        Ok(true)
    } else if tail[..5] == [0x26, 0xF6, 0x47, 0x02, 0x01] {
        Ok(false)
    } else {
        Err(refused("the animation start", at))
    }
}

/// After the object's live bit is tested (`26 F6 47 02 01 75`), a dormant
/// one is either tested for its kind (`26 80 3F k 75`) or woken at once
/// (`26 80 4F 02 03`, its flags `or 3`). *Dunkle Schatten*'s test is at
/// `0x0A843`, for kind 4.
fn anim_wakes_only_kind(img: &Image, at: (u16, u16)) -> Result<Option<u8>> {
    let b = body(img, at, 0xA0, "the animation start")?;
    let i = b
        .windows(6)
        .position(|w| w == [0x26, 0xF6, 0x47, 0x02, 0x01, 0x75])
        .ok_or_else(|| refused("the animation start", at))?;
    let tail = &b[i + 7..];
    if tail[..3] == [0x26, 0x80, 0x3F] && tail[4] == 0x75 {
        Ok(Some(tail[3]))
    } else if tail[..5] == [0x26, 0x80, 0x4F, 0x02, 0x03] {
        Ok(None)
    } else {
        Err(refused("the animation start", at))
    }
}

/// The state row's loop reads an object's kind (`26 8A 07 B4 00`) and
/// chooses its encoding either by a chain — `or ax, ax ; je` for kind 0,
/// then `cmp ax, k ; je` to one arm for each further kind — or by a jump
/// table (`8B D8 83 FB n 77 rel D1 E3 2E FF A7 lo hi`, in the code
/// segment), where the kinds whose entry is kind 1's share its arm.
/// *Dunkle Schatten*'s chain at `0x154A5` names 1 and 5.
fn tri_state_kinds(img: &Image, at: (u16, u16)) -> Result<u8> {
    let b = body(img, at, 0x70, "the state row")?;
    let i = b
        .windows(5)
        .position(|w| w == [0x26, 0x8A, 0x07, 0xB4, 0x00])
        .ok_or_else(|| refused("the state row", at))?;
    let t = &b[i + 5..];
    if t[..3] == [0x0B, 0xC0, 0x74] {
        let mut kinds = 0u8;
        let mut c = &t[4..];
        while c.len() >= 5 && c[0] == 0x3D && c[2] == 0 && c[3] == 0x74 {
            kinds |= 1u8.checked_shl(u32::from(c[1])).unwrap_or(0);
            c = &c[5..];
        }
        if kinds == 0 {
            return Err(refused("the state row", at));
        }
        Ok(kinds)
    } else if t[..4] == [0x8B, 0xD8, 0x83, 0xFB]
        && t[5] == 0x77
        && t[7..11] == [0xD1, 0xE3, 0x2E, 0xFF]
        && t[11] == 0xA7
    {
        let n = usize::from(t[4]);
        let table = usize::from(at.0) * 16 + usize::from(u16::from_le_bytes([t[12], t[13]]));
        let entry = |k: usize| img.word(table + 2 * k);
        let one = entry(1)?;
        let mut kinds = 0u8;
        for k in 1..=n.min(7) {
            if entry(k)? == one {
                kinds |= 1 << k;
            }
        }
        Ok(kinds)
    } else {
        Err(refused("the state row", at))
    }
}

/// After `enter ; push si ; push di` (6 bytes) the dispatcher makes a far
/// call (`9A`, 5 bytes), and then either tests the word a script set to
/// ask for task 0 (`83 3E lo hi 00 74`) or makes its next far call.
/// *Dunkle Schatten*'s test is at `0x0FB69`.
fn dispatch_starts_asked_task(img: &Image, at: (u16, u16)) -> Result<bool> {
    let b = body(img, at, 0x20, "the dispatcher")?;
    if b[..6] != [0xC8, 0x0A, 0x00, 0x00, 0x56, 0x57] || b[6] != 0x9A {
        return Err(refused("the dispatcher", at));
    }
    let t = &b[11..];
    if t[..2] == [0x83, 0x3E] && t[4] == 0 && t[5] == 0x74 {
        Ok(true)
    } else if t[0] == 0x9A {
        Ok(false)
    } else {
        Err(refused("the dispatcher", at))
    }
}

/// What the dispatcher's body says about the hover.
struct Dispatch {
    /// The last hover's two words, `(high, low)`.
    last: (u16, u16),
    forgotten_is_nothing: bool,
    room_click_forgets: bool,
    hover_off_only_notes: bool,
    verb6_refreshes: bool,
    verb_bound: u16,
    locked_stops_hover_text: bool,
}

impl Dispatch {
    /// The dispatcher compares the last hover with the hotspot under the
    /// pointer as `mov ax, [last+2] ; mov dx, [last] ; cmp ax, [cur+2] ;
    /// jne ; cmp dx, [cur] ; jne ; jmp` — in *Dunkle Schatten* at `0x0FDE1`
    /// with `[28A5h]`/`[28A3h]` and `[1862h]`/`[1860h]` — once in the body.
    /// Everything else is found from those four words.
    fn read(img: &Image, at: (u16, u16)) -> Result<Self> {
        let b = body(img, at, 0x4A0, "the dispatcher")?;
        let w = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        let hits: Vec<usize> = (0..b.len() - 20)
            .filter(|&i| {
                b[i] == 0xA1
                    && b[i + 3..i + 5] == [0x8B, 0x16]
                    && b[i + 7..i + 9] == [0x3B, 0x06]
                    && b[i + 11..i + 13] == [0x75, 0x09]
                    && b[i + 13..i + 15] == [0x3B, 0x16]
                    && b[i + 17..i + 20] == [0x75, 0x03, 0xE9]
            })
            .collect();
        let [c] = hits[..] else {
            return Err(refused("the dispatcher", at));
        };
        let last = (w(c + 1), w(c + 5));
        let cur = (w(c + 9), w(c + 15));
        let pair = |v: u16| -> Vec<usize> {
            let [vl, vh] = v.to_le_bytes();
            let [h0, h1] = last.0.to_le_bytes();
            let [l0, l1] = last.1.to_le_bytes();
            let p = [0xC7, 0x06, h0, h1, vl, vh, 0xC7, 0x06, l0, l1, vl, vh];
            b.windows(12)
                .enumerate()
                .filter(|(_, x)| *x == p)
                .map(|(i, _)| i)
                .collect()
        };
        let (ffff, zero) = (pair(0xFFFF), pair(0));
        let forgotten_is_nothing = match (ffff.is_empty(), zero.is_empty()) {
            (false, true) => false,
            (true, false) => true,
            _ => return Err(refused("the dispatcher", at)),
        };
        // The room's click takes the hotspot under the pointer into
        // `[bp-4]:[bp-2]`.
        let [c2, c3] = cur.0.to_le_bytes();
        let [c0, c1] = cur.1.to_le_bytes();
        let take = [0xA1, c2, c3, 0x8B, 0x16, c0, c1, 0x89, 0x46, 0xFE];
        let room = b
            .windows(10)
            .position(|x| x == take)
            .ok_or_else(|| refused("the dispatcher", at))?;
        let room_click_forgets = room >= 12 && ffff.contains(&(room - 12));
        let after = &b[c + 22..];
        let hover_off_only_notes = if after[..2] == [0x83, 0x3E] && after[4] == 0 {
            true
        } else if after[0] == 0x9A {
            false
        } else {
            return Err(refused("the dispatcher", at));
        };
        let verb6_refreshes = b
            .windows(6)
            .any(|x| x[..2] == [0xC7, 0x06] && x[4..] == [0x0A, 0x00]);
        let verb_bound = b
            .windows(6)
            .find(|x| x[..4] == [0xF7, 0xD6, 0x83, 0xFE] && x[5] == 0x7D)
            .map(|x| u16::from(x[4]))
            .ok_or_else(|| refused("the dispatcher", at))?;
        // Under the lock (`test word [bits], 2 ; je`, `F7 06 lo hi 02 00
        // 74`) the build either goes on to the developer's bit and from it
        // to `or di, di ; jge ; push 0 ; call far` (`0B FF 7D xx 6A 00 9A`),
        // stopping task 0 when the query answered −1 — or ends the frame.
        let lock = b
            .windows(7)
            .position(|x| x[..2] == [0xF7, 0x06] && x[4..] == [0x02, 0x00, 0x74])
            .ok_or_else(|| refused("the dispatcher", at))?;
        let locked_stops_hover_text = b[lock..]
            .windows(7)
            .take(0x40)
            .any(|x| x[..3] == [0x0B, 0xFF, 0x7D] && x[4..] == [0x6A, 0x00, 0x9A]);
        Ok(Dispatch {
            locked_stops_hover_text,
            last,
            forgotten_is_nothing,
            room_click_forgets,
            hover_off_only_notes,
            verb6_refreshes,
            verb_bound,
        })
    }
}

/// The mode reset forgets the hover with `mov word [last+2], FFFFh`, or
/// does not write it at all. *Dunkle Schatten*'s `0EB9h:003Fh` writes it
/// at `0x0EC35`.
fn mode_reset_forgets_hover(img: &Image, at: (u16, u16), high: u16) -> Result<bool> {
    let b = body(img, at, 0x80, "the mode reset")?;
    let [h0, h1] = high.to_le_bytes();
    Ok(b.windows(6).any(|x| x == [0xC7, 0x06, h0, h1, 0xFF, 0xFF]))
}

/// The forest is read in three routines, and they must agree.
///
/// - The room loader compares the row's kind and takes a fixed slot:
///   `cmp ax, k ; jne +5 ; mov si, n` (`3D k 00 75 05 BE n`). In *Dunkle
///   Schatten* this is `0x077F2`.
/// - The unload keeps that slot: `cmp ax, k ; jne +3 ; mov si, n`
///   (`3D k 00 75 03 BE n`), at `0x083F6`.
/// - The pair's kind is told by `cmp ax, k ; je +5 ; cmp ax, r ; jne`
///   (`3D k 00 74 05 3D r 00 75`), at `0x1571B`.
///
/// A build with none of the three has no forest; one with some of them is
/// refused.
fn forest(img: &Image, map: &LibraryMap) -> Result<Option<Forest>> {
    let find = |at: (u16, u16), len: usize, what: &str, shape: &dyn Fn(&[u8]) -> bool| {
        body(img, at, len, what).map(|b| b.windows(10).find(|w| shape(w)).map(<[u8]>::to_vec))
    };
    let loader = find(map.room_loader, 0x200, "the room loader", &|w| {
        w[0] == 0x3D && w[2] == 0 && w[3..5] == [0x75, 0x05] && w[5] == 0xBE
    })?;
    let unload = find(map.slot_unload, 0xA0, "the unload", &|w| {
        w[0] == 0x3D && w[2] == 0 && w[3..5] == [0x75, 0x03] && w[5] == 0xBE
    })?;
    let pair = find(map.place_pair, 0xB0, "the pair's kind", &|w| {
        w[0] == 0x3D
            && w[2] == 0
            && w[3..5] == [0x74, 0x05]
            && w[5] == 0x3D
            && w[7] == 0
            && w[8] == 0x75
    })?;
    match (loader, unload, pair) {
        (None, None, None) => Ok(None),
        (Some(l), Some(u), Some(p)) => {
            let word = |w: &[u8], i: usize| u16::from_le_bytes([w[i], w[i + 1]]);
            let f = Forest {
                kind: word(&l, 1),
                slot: word(&l, 6),
                reveal: word(&p, 6),
            };
            if word(&u, 1) != f.kind || word(&u, 6) != f.slot || word(&p, 1) != f.kind {
                return Err(refused("the room loader", map.room_loader));
            }
            Ok(Some(f))
        }
        _ => Err(refused("the room loader", map.room_loader)),
    }
}

/// The entry's object loop ends `cmp di|si, n ; jl` (`83 FF|FE n 7C`);
/// then either a slot is written as a point — `push y ; push x ; push
/// code ; push slot ; call far` (`6A y 68 x 6A code 6A slot 9A`) — or the
/// entry goes on to its records (`C4 5E`). *Dunkle Schatten* writes slot
/// 10 at `0x0EE58`.
fn screen_slot(img: &Image, at: (u16, u16)) -> Result<Option<ScreenSlot>> {
    let b = body(img, at, 0x180, "the entry")?;
    let i = b
        .windows(4)
        .position(|w| w[0] == 0x83 && (w[1] == 0xFF || w[1] == 0xFE) && w[3] == 0x7C)
        .ok_or_else(|| refused("the entry", at))?;
    let t = &b[i + 5..];
    if t[0] == 0x6A && t[2] == 0x68 && t[5] == 0x6A && t[7] == 0x6A && t[9] == 0x9A {
        Ok(Some(ScreenSlot {
            slot: u16::from(t[8]),
            code: u16::from(t[6]),
            x: u16::from_le_bytes([t[3], t[4]]),
            y: u16::from(t[1]),
        }))
    } else if t[..2] == [0xC4, 0x5E] {
        Ok(None)
    } else {
        Err(refused("the entry", at))
    }
}

/// The entry tests the door's object for its kind, `cmp byte es:[bx], k ;
/// jne` (`26 80 3F k 75`), and then either remembers it — `mov [m16], si`
/// (`89 36`) — and shows it on `push f`, or shows it at once (`6A f`).
/// *Dunkle Schatten* remembers at `0x0EF22` and shows frame 1.
fn entry_door(img: &Image, at: (u16, u16)) -> Result<EntryDoor> {
    let b = body(img, at, 0x220, "the entry")?;
    let i = b
        .windows(5)
        .position(|w| w[..3] == [0x26, 0x80, 0x3F] && w[4] == 0x75)
        .ok_or_else(|| refused("the entry", at))?;
    let kind = b[i + 3];
    let t = &b[i + 6..];
    let (remembered, f) = if t[..2] == [0x89, 0x36] && t[4] == 0x6A {
        (true, t[5])
    } else if t[0] == 0x6A {
        (false, t[1])
    } else {
        return Err(refused("the entry", at));
    };
    Ok(EntryDoor {
        kind,
        remembered,
        frame: i16::from(i8::from_le_bytes([f])),
    })
}

/// Taking an item out compares the first visible slot plus the window
/// with the count, `add ax, n` (`05 n 00`), the first such add in the
/// routine. *Dunkle Schatten*'s `0C3Fh:09E1h` adds 8 at `0x0CE21`.
fn inventory_window(img: &Image, at: (u16, u16)) -> Result<u16> {
    let b = body(img, at, 0xB5, "the item taken")?;
    b.windows(3)
        .find(|w| w[0] == 0x05 && w[2] == 0 && w[1] != 0)
        .map(|w| u16::from(w[1]))
        .ok_or_else(|| refused("the item taken", at))
}

/// The lock and its lifting open with `or word [m16], 2` (`83 0E lo hi
/// 02`) and `and word [m16], FFFDh` (`83 26 lo hi FD`) — `op` is the
/// second byte — and go on either with the two stores that forget the
/// hover and then the band's `push 4 ; call far`, or with that `push 4 ;
/// call far` at once. *Dunkle Schatten*'s `0EB9h:188Ah` (`0x1041A`) and
/// `0EB9h:18A4h` (`0x10434`) have the stores.
fn lock_forgets_hover(img: &Image, at: (u16, u16), op: u8) -> Result<bool> {
    let what = if op == 0x0E {
        "the input lock"
    } else {
        "the lock's lifting"
    };
    let b = body(img, at, 25, what)?;
    if b[..2] != [0x83, op] || !matches!(b[4], 0x02 | 0xFD) {
        return Err(refused(what, at));
    }
    let tail = &b[5..];
    if is_store_ffff(tail) && is_store_ffff(&tail[6..]) && tail[12..15] == [0x6A, 0x04, 0x9A] {
        Ok(true)
    } else if tail[..3] == [0x6A, 0x04, 0x9A] {
        Ok(false)
    } else {
        Err(refused(what, at))
    }
}

/// The callback is read in two routines, and they must agree.
///
/// - The visit's setup either opens with `mov word [w+2], seg ; mov word
///   [w], off` (`C7 06 w+2 seg C7 06 w off`), putting the pointer back, or
///   with its first `push` (`6A`) and keeps none. *Dunkle Schatten*'s
///   `0EB9h:0001h` keeps none.
/// - The entry asks through it in a loop: `cmp word [bp+6], loc ; je ;
///   mov si, first ; jmp ; push si ; call far [w]` (`83 7E 06 loc 74 rel
///   BE first 00 EB rel 56 FF 1E w`), closed by `inc si ; cmp si, last ;
///   jle` (`46 83 FE last 7E`).
fn callback(img: &Image, map: &LibraryMap) -> Result<Option<Callback>> {
    let at = map.visit_setup;
    let b = body(img, at, 0x10, "the visit's setup")?;
    let reset = if b[..2] == [0xC7, 0x06] && b[6..8] == [0xC7, 0x06] {
        let w = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        let (hi, lo) = (w(2), w(8));
        if hi != lo.wrapping_add(2) {
            return Err(refused("the visit's setup", at));
        }
        Some((lo, (w(4), w(10))))
    } else if b[0] == 0x6A {
        None
    } else {
        return Err(refused("the visit's setup", at));
    };
    let e = body(img, map.visit_enter, 0x2C0, "the entry")?;
    let asked = e.windows(16).position(|x| {
        x[..3] == [0x83, 0x7E, 0x06]
            && x[4] == 0x74
            && x[6] == 0xBE
            && x[8] == 0
            && x[9] == 0xEB
            && x[11..14] == [0x56, 0xFF, 0x1E]
    });
    match (reset, asked) {
        (None, None) => Ok(None),
        (Some((word, default)), Some(i)) => {
            let x = &e[i..];
            if u16::from_le_bytes([x[14], x[15]]) != word {
                return Err(refused("the entry", map.visit_enter));
            }
            let last = e[i + 16..]
                .windows(5)
                .find(|y| y[..3] == [0x46, 0x83, 0xFE] && y[4] == 0x7E)
                .map(|y| u16::from(y[3]))
                .ok_or_else(|| refused("the entry", map.visit_enter))?;
            Ok(Some(Callback {
                word,
                default,
                ids: (u16::from(x[7]), last),
                skips_location: u16::from(x[3]),
            }))
        }
        _ => Err(refused("the visit's setup", at)),
    }
}

/// The resolver reads the slot word's kind either with `and ax, 0F000h`
/// (`25 00 F0`, *Dunkle Schatten*'s `0x0D2A0`) — and copies the rest as
/// the style word — or with `and bx, 0Fh` (`83 E3 0F`), and then recodes
/// two fields:
///
/// - the vertical one through a sparse switch, `mov cx, n ; mov bx, tab`
///   (`B9 n 00 BB tab`) over `n` codes in the code segment and their
///   targets after them (`jmp cs:[bx + 2n]`), each target storing the
///   style with `mov word [bx+m16], s` (`C7 87 m16 s`);
/// - the horizontal one with `and ax, 0C0h ; cmp ax, a ; je ; cmp ax, b ;
///   je` (`25 C0 00 3D a 00 74 r 3D b 00 74 r`), each target oring in the
///   style with `or word [bx+m16], s` (`83 8F m16 s`).
fn slot_style(img: &Image, at: (u16, u16)) -> Result<Option<SlotStyle>> {
    let what = "the slot resolver";
    let b = body(img, at, 0x200, what)?;
    let head = &b[..0x20];
    if head.windows(3).any(|w| w == [0x25, 0x00, 0xF0]) {
        return Ok(None);
    }
    if !head.windows(3).any(|w| w == [0x83, 0xE3, 0x0F]) {
        return Err(refused(what, at));
    }
    let base = usize::from(at.0) * 16;
    let word = |i: usize| img.word(i);
    let first = |from: usize, shape: &dyn Fn(&[u8]) -> bool, len: usize| -> Result<Vec<u8>> {
        img.bytes()
            .get(from..from + 0x30)
            .and_then(|t| t.windows(len).find(|w| shape(w)))
            .map(<[u8]>::to_vec)
            .ok_or_else(|| refused(what, at))
    };
    let sw = b
        .windows(6)
        .position(|w| w[0] == 0xB9 && w[2] == 0 && w[3] == 0xBB)
        .ok_or_else(|| refused(what, at))?;
    let n = usize::from(b[sw + 1]);
    let tab = base + usize::from(u16::from_le_bytes([b[sw + 4], b[sw + 5]]));
    if n != 4 {
        return Err(refused(what, at));
    }
    let mut vertical = [(0u16, 0u16); 4];
    for (k, v) in vertical.iter_mut().enumerate() {
        let code = word(tab + 2 * k)?;
        let target = base + usize::from(word(tab + 2 * n + 2 * k)?);
        let store = first(target, &|w| w[..2] == [0xC7, 0x87], 6)?;
        *v = (code, u16::from_le_bytes([store[4], store[5]]));
    }
    let h = b
        .windows(13)
        .position(|w| {
            w[..3] == [0x25, 0xC0, 0x00]
                && w[3] == 0x3D
                && w[5] == 0
                && w[6] == 0x74
                && w[8] == 0x3D
                && w[10] == 0
                && w[11] == 0x74
        })
        .ok_or_else(|| refused(what, at))?;
    let start = usize::from(at.0) * 16 + usize::from(at.1) + h;
    let mut horizontal = [(0u16, 0u16); 2];
    for (k, (code_at, rel_at)) in [(4, 7), (9, 12)].into_iter().enumerate() {
        let code = u16::from(b[h + code_at]);
        let target = start + rel_at + 1 + usize::from(b[h + rel_at]);
        let or = first(target, &|w| w[..2] == [0x83, 0x8F], 5)?;
        horizontal[k] = (code, u16::from(or[4]));
    }
    Ok(Some(SlotStyle {
        vertical,
        horizontal,
    }))
}

/// A room's effects: the routine bounds the room slot with `cmp ax, b ;
/// jge` (`3D b 00 7D`), indexes the masks with `shl bx, 1` (`D1 E3`, a
/// word each) or `shl bx, 2` (`C1 E3 02`, a double word), starts its loop
/// at `mov word [bp-2], n` (`C7 46 FE n 00`, the slots) and ends it at
/// `cmp ax, k ; jg` (`3D k 00 7F`, the first room slot). *Dunkle
/// Schatten*'s `1458h:00F7h` has 1Ah (`0x14689`), a word, 15h
/// (`0x146A3`) and 0Ch (`0x146F5`).
fn effects(img: &Image, at: (u16, u16)) -> Result<Effects> {
    let what = "the room's effects";
    let b = body(img, at, 0x90, what)?;
    let find = |shape: &dyn Fn(&[u8]) -> bool, len: usize| {
        b.windows(len)
            .position(shape)
            .ok_or_else(|| refused(what, at))
    };
    let r = find(&|w| w[0] == 0x3D && w[2] == 0 && w[3] == 0x7D, 4)?;
    let rooms = u16::from(b[r + 1]);
    let wide_masks = match b.get(r + 5..r + 8) {
        Some([0x8B, 0xD8, 0xD1]) => false,
        Some([0x8B, 0xD8, 0xC1]) => true,
        _ => return Err(refused(what, at)),
    };
    let n = find(&|w| w[..3] == [0xC7, 0x46, 0xFE] && w[4] == 0, 5)?;
    let k = find(&|w| w[0] == 0x3D && w[2] == 0 && w[3] == 0x7F, 4)?;
    Ok(Effects {
        slots: usize::from(b[n + 3]),
        room_first: usize::from(b[k + 1]),
        rooms,
        wide_masks,
    })
}

/// A room slot's song: `mov ax, [bp+6] ; cmp ax, n ; jge` (`8B 46 06 3D n
/// 00 7D`). *Dunkle Schatten*'s `1197h:0035h` has 1Eh (`0x119AC`).
fn song_slots(img: &Image, at: (u16, u16)) -> Result<u16> {
    let b = body(img, at, 0x20, "the slot's song")?;
    b.windows(7)
        .find(|w| w[..4] == [0x8B, 0x46, 0x06, 0x3D] && w[5] == 0 && w[6] == 0x7D)
        .map(|w| u16::from(w[4]))
        .ok_or_else(|| refused("the slot's song", at))
}

/// The visit's setup hands the figure's loader its digit: either `push n
/// ; push 1 ; call far` (`6A n 6A 01 9A`) — *Dunkle Schatten*'s
/// `0EB9h:0001h`, with 0 — or the location word compared, `cmp word
/// [loc], k ; jle +5 ; mov ax, 1 ; jmp +2 ; xor ax, ax ; push ax ; push 1
/// ; call far` (`83 3E loc k 7E 05 B8 01 00 EB 02 33 C0 50 6A 01 9A`).
fn figure_set(img: &Image, at: (u16, u16)) -> Result<FigureSet> {
    let b = body(img, at, 0x30, "the visit's setup")?;
    let cmp = [
        0x7E, 0x05, 0xB8, 0x01, 0x00, 0xEB, 0x02, 0x33, 0xC0, 0x50, 0x6A, 0x01, 0x9A,
    ];
    if let Some(w) = b
        .windows(18)
        .find(|w| w[..2] == [0x83, 0x3E] && w[5..] == cmp)
    {
        return Ok(FigureSet::AfterLocation {
            word: u16::from_le_bytes([w[2], w[3]]),
            above: u16::from(w[4]),
        });
    }
    b.windows(5)
        .find(|w| w[0] == 0x6A && w[2..] == [0x6A, 0x01, 0x9A])
        .map(|w| FigureSet::Fixed(u16::from(w[1])))
        .ok_or_else(|| refused("the visit's setup", at))
}

/// The strip's loader either walks the carried list down from its end
/// while an item is at or above a bound, `cmp byte [bx+list], s ; jb`
/// (`80 BF list s 72`), and splits the range there — or sets the range to
/// the whole list at once, `mov byte [lo], 0` (`C6 06 lo 00`). *Dunkle
/// Schatten*'s `0C1Ch:000Ch` does the second (`0x0C1D5`).
fn strip_split(img: &Image, at: (u16, u16)) -> Result<Option<u8>> {
    let b = body(img, at, 0x30, "the strip's loader")?;
    if let Some(w) = b
        .windows(6)
        .find(|w| w[..2] == [0x80, 0xBF] && w[5] == 0x72)
    {
        return Ok(Some(w[4]));
    }
    if b.windows(5).any(|w| w[..2] == [0xC6, 0x06] && w[4] == 0) {
        return Ok(None);
    }
    Err(refused("the strip's loader", at))
}

/// The scan is a `push bp ; mov bp, sp` routine ending in `pop di ; pop
/// si ; pop bp ; ret`. It clears task 0's flags with
/// `mov word [flags], 0` (`C7 06 lo hi 00 00`) and sends `03 n` through a
/// table: `dec bx ; cmp bx, last ; ja` (`4B 83 FB last 77`), `last` 4 when
/// the build has `03 05` and 3 when not. Its arms set the bits with
/// `or word [flags], bit` (`81 0E lo hi bit`, or `83 0E lo hi bit` for a
/// bit below 80h): `03 01`'s, `03 02`'s and,
/// with five controls, `03 05`'s, in that order. *Dunkle Schatten*'s
/// `0CFDh:01BEh` (`0x0D18E`) sets `100h`, `200h` and `400h`.
///
/// The query's text arm tests them as it takes a click: `test word
/// [flags], mask` (`F7 06 lo hi mask`) with `mask` armed, `03 01`'s and
/// `03 02`'s bit, and with `03 05` first `and ax, 8000h | bit` (`25 lo hi`);
/// a query that tests anything else is refused (`0x0E010`, `0x0DFFA`).
fn text_flags(img: &Image, scan: (u16, u16), query: (u16, u16)) -> Result<TextFlags> {
    let b = body(img, scan, 0x100, "the page's scan")?;
    let end = b
        .windows(4)
        .position(|w| w == [0x5F, 0x5E, 0x5D, 0xC3])
        .filter(|_| b[..3] == [0x55, 0x8B, 0xEC])
        .ok_or_else(|| refused("the page's scan", scan))?;
    let b = &b[..end];
    let word = b
        .windows(6)
        .find(|w| w[..2] == [0xC7, 0x06] && w[4..] == [0, 0])
        .map(|w| [w[2], w[3]])
        .ok_or_else(|| refused("the page's scan", scan))?;
    let last = b
        .windows(5)
        .find(|w| w[..3] == [0x4B, 0x83, 0xFB] && w[4] == 0x77)
        .map(|w| w[3])
        .ok_or_else(|| refused("the page's scan", scan))?;
    let bits: Vec<u16> = (0..b.len())
        .filter_map(|k| match b.get(k..k + 6)? {
            [0x81, 0x0E, lo, hi, a, c] if [*lo, *hi] == word => Some(u16::from_le_bytes([*a, *c])),
            [0x83, 0x0E, lo, hi, a, _] if [*lo, *hi] == word => Some(u16::from(*a)),
            _ => None,
        })
        .collect();
    let flags = match (last, bits.as_slice()) {
        (3, &[hold, firm]) => TextFlags {
            hold,
            firm,
            quick: None,
        },
        (4, &[hold, firm, quick]) => TextFlags {
            hold,
            firm,
            quick: Some(quick),
        },
        _ => return Err(refused("the page's scan", scan)),
    };
    let q = body(img, query, 0xA0, "the task query")?;
    let mask = (0x8000 | flags.hold | flags.firm).to_le_bytes();
    let tests = q
        .windows(6)
        .any(|w| w[..2] == [0xF7, 0x06] && w[2..4] == word && w[4..] == mask);
    let quick_ok = flags.quick.is_none_or(|bit| {
        let m = (0x8000 | bit).to_le_bytes();
        q.windows(3).any(|w| w[0] == 0x25 && w[1..] == m)
    });
    if tests && quick_ok {
        Ok(flags)
    } else {
        Err(refused("the task query", query))
    }
}

fn refused(what: &str, at: (u16, u16)) -> crate::error::Error {
    format!(
        "{what} at {:04X}h:{:04X}h is not a build of the library this runtime has read",
        at.0, at.1
    )
    .into()
}
