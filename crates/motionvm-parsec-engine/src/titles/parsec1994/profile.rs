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
    /// A scene bound to a task: which bank's place a scene number names.
    pub scene_bind: (u16, u16),
    /// The script's exit gate installed.
    pub script_exit: (u16, u16),
    /// The entry finished: first picture, effect, music, tasks.
    pub visit_finish_entry: (u16, u16),
    /// The entry's two effects, which it chooses between: the room
    /// revealed, and the room slid in.
    pub entry_reveal: (u16, u16),
    pub entry_slide: (u16, u16),
    /// Leaving a location.
    pub visit_leave: (u16, u16),
    /// Hover on again.
    pub hover_on: (u16, u16),
    /// An object's state as the logic asks it.
    pub obj_state: (u16, u16),
    /// The player walked to a node; `None` for a build that has no such
    /// routine and walks its figure from the logic.
    pub walk_to_node: Option<(u16, u16)>,
    /// An animation started on an object.
    pub obj_anim: (u16, u16),
    /// The location's objects kept in its state row.
    pub objs_save_state: (u16, u16),
    /// The objects' state read back from the location's row as the
    /// visit's setup loads the room.
    pub objs_load_state: (u16, u16),
    /// An object's hotspot made to follow its live bit.
    pub hotspot_follow: (u16, u16),
    /// An object shown, and hidden.
    pub obj_show: (u16, u16),
    pub obj_hide: (u16, u16),
    /// A hotspot's own line, played by task 0.
    pub say_text: (u16, u16),
    /// The options' gate.
    pub options_gate: (u16, u16),
    /// One frame of input dispatched.
    pub dispatch: (u16, u16),
    /// The mode machine reset.
    pub mode_reset: (u16, u16),
    /// Whether the chosen object wants a second one, and the machine is
    /// put into mode 3 for it.
    pub second_wanted: (u16, u16),
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
    /// An item given.
    pub inv_give: (u16, u16),
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
    /// The status band painted: the line, the strip's widgets, the band
    /// held dark.
    pub status_paint: (u16, u16),
    /// The status line's gate, the frame pass's first act.
    pub status_gate: (u16, u16),
    /// The mode's step, `0EB9h:0965h(busy)`: what mode 8 does once what
    /// it waits for is over.
    pub mode_step: (u16, u16),
    /// The strip redrawn whole under a fresh choice menu: the panel and
    /// the items, and in some builds the band cleared and held dark.
    pub menu_strip: (u16, u16),
    /// The animation's signal, what an `17` opcode's code does.
    pub anim_signal: (u16, u16),
    /// The task armed, `0CFDh:1A2Eh(k)`.
    pub task_arm: (u16, u16),
    /// The held page let go, `0CFDh:1BA2h(k)`.
    pub page_release: (u16, u16),
    /// A sample, `0EB9h:1949h(k)`.
    pub play_sample: (u16, u16),
    /// The script's own interaction, `0EB9h:177Ah(id, facing, scene)`.
    pub interaction: (u16, u16),
    /// A record bound to the animation entry that names it,
    /// `14DDh:0098h(rec)`.
    pub figure_bind: (u16, u16),
    /// The animator's step of one record whose countdown ran out, the
    /// near routine the animator's pass calls, `0883h:012Ah(rec)`.
    pub animator_step: (u16, u16),
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

/// How a build's object state answers whether an object is live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjectState {
    /// Kinds 1 and 2 answer bit 0 of `+2`; kind 4 whether the two low bits
    /// are 1 (`and ax, 3 ; cmp ax, 1`), live then when `when_mode_one`
    /// (`jne` to the "not live" answer) or else when they are anything
    /// else (`je`); any other kind answers 3. *Dunkle Schatten*'s
    /// `14DDh:07E7h` (`0x155B7`), with `jne`, and *Captain Zins*'s
    /// `1021h:07B0h` (`0x109C0`), with `je`.
    Modes {
        /// `jne` rather than `je` after the compare.
        when_mode_one: bool,
    },
    /// Kind 4 answers bit 1 of `+2` (`and ax, 2`); every other kind
    /// answers 3 and an id with no record 1. *Bi-Fi 2*'s
    /// `0F48h:074Bh` (`0x0FBCB`–`0x0FC03`).
    Bit1,
}

/// The range of the carried list the strip shows, as the strip's loader
/// sets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StripRange {
    /// The whole list, and the first visible slot put back inside it.
    Whole,
    /// Split at this id: below it the first strip file's items, from it
    /// on the second's, and the first visible slot put back inside the
    /// part in range.
    Split(u8),
    /// No range: the loader keeps none, and the first visible slot stays
    /// where it is.
    Unranged,
}

/// The show walks a chain head's parts and sets each one not yet live
/// with `test byte es:[bx+2], 1 ; jne ; or byte es:[bx+2], n` (`26 F6 47
/// 02 01 75 r 26 80 4F 02 n`): 3 live and hidden in *Dunkle Schatten*
/// (`0x15673`) and *Captain Zins* (`1021h:0813h`, `0x10A23`), 1 live alone
/// in *Bi-Fi 2* (`0x0FC5D`).
fn show_hides_parts(img: &Image, at: (u16, u16)) -> Result<bool> {
    let b = body(img, at, 0x90, "the object shown")?;
    let n = b
        .windows(12)
        .find(|w| {
            w[..6] == [0x26, 0xF6, 0x47, 0x02, 0x01, 0x75] && w[7..11] == [0x26, 0x80, 0x4F, 0x02]
        })
        .map(|w| w[11]);
    match n {
        Some(3) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(refused("the object shown", at)),
    }
}

/// The hide's walk over a chain head's parts ends `mov ax, es:[bx] ; mov
/// [bp-6], ax ; or ax, ax ; jne` (`26 8B 07 89 46 FA 0B C0 75 r`), and then
/// either clears the head's live bit, `les bx, [bp-4] ; and byte es:[bx+2],
/// 0FEh` (`C4 5E FC 26 80 67 02 FE`, *Dunkle Schatten*'s `0x157AD`,
/// *Captain Zins*'s `1021h:0912h`, `0x10B22`), or jumps past it (`EB`,
/// *Bi-Fi 2*'s `0x0FDB2`).
fn hide_clears_head(img: &Image, at: (u16, u16)) -> Result<bool> {
    let b = body(img, at, 0xB0, "the object hidden")?;
    let end = b
        .windows(10)
        .position(|w| w[..9] == [0x26, 0x8B, 0x07, 0x89, 0x46, 0xFA, 0x0B, 0xC0, 0x75])
        .ok_or_else(|| refused("the object hidden", at))?
        + 10;
    match b.get(end..end + 8) {
        Some([0xC4, 0x5E, 0xFC, 0x26, 0x80, 0x67, 0x02, 0xFE]) => Ok(true),
        Some([0xEB, ..]) => Ok(false),
        _ => Err(refused("the object hidden", at)),
    }
}

/// A hotspot's own line, after the substitution (`mov si, dx`, `8B F2`),
/// clears task 0's slot (`mov word [m16], 0`, `C7 06 lo hi 00 00`) and
/// then either sets its speakers to 1 (`C7 06 lo hi 01 00`, *Dunkle
/// Schatten*'s `0x0E8BC`, *Captain Zins*'s `0849h:18A3h`, `0x09D33`) or
/// goes on to the scratch's end (`les bx`, `C4 1E`, *Bi-Fi 2*'s
/// `0x098C3`).
fn own_line_sets_speakers(img: &Image, at: (u16, u16)) -> Result<bool> {
    let b = body(img, at, 0x40, "the hotspot's own line")?;
    let i = b
        .windows(8)
        .position(|w| w[..4] == [0x8B, 0xF2, 0xC7, 0x06] && w[6..] == [0x00, 0x00])
        .ok_or_else(|| refused("the hotspot's own line", at))?
        + 8;
    match b.get(i..i + 6) {
        Some([0xC7, 0x06, _, _, 0x01, 0x00]) => Ok(true),
        Some([0xC4, 0x1E, ..]) => Ok(false),
        _ => Err(refused("the hotspot's own line", at)),
    }
}

/// The follow sends the object's kind less one through a table, `dec ax ;
/// mov bx, ax ; cmp bx, n ; ja ; shl bx, 1 ; jmp cs:[bx+t]` (`48 8B D8 83
/// FB n 77 r D1 E3 2E FF A7 t`): a kind whose arm is where an object
/// without an id goes (`cmp si, 8000h ; je`, `81 FE 00 80 74 r`) has no
/// hotspot to follow it; every other arm must be where a kind past `n +
/// 1` goes, the follow. *Dunkle Schatten*'s `14DDh:074Bh` keeps 1 to 4 and 9
/// (`cs:07D5h`); *Captain Zins*'s (`cs:07A8h`) and *Bi-Fi 2*'s
/// (`cs:0743h`) 1 to 4.
fn hotspot_kept(img: &Image, at: (u16, u16)) -> Result<u16> {
    let no = || refused("the hotspot's follow", at);
    let b = body(img, at, 0x40, "the hotspot's follow")?;
    let rel = |from: usize, r: u8| from.checked_add_signed(isize::from(i8::from_le_bytes([r])));
    let e = b
        .windows(6)
        .position(|w| w[..5] == [0x81, 0xFE, 0x00, 0x80, 0x74])
        .ok_or_else(no)?;
    let end = rel(e + 6, b[e + 5]).ok_or_else(no)?;
    let t = b
        .windows(15)
        .position(|w| {
            w[..4] == [0x48, 0x8B, 0xD8, 0x83]
                && w[4] == 0xFB
                && w[6] == 0x77
                && w[8..13] == [0xD1, 0xE3, 0x2E, 0xFF, 0xA7]
        })
        .ok_or_else(no)?;
    let n = b[t + 5];
    let follows = rel(t + 8, b[t + 7]).ok_or_else(no)?;
    let start = usize::from(at.0) * 16 + usize::from(at.1);
    let table = usize::from(at.0) * 16 + usize::from(u16::from_le_bytes([b[t + 13], b[t + 14]]));
    let mut kept = 0u16;
    for k in 0..=u16::from(n) {
        let off = usize::from(img.word(table + 2 * usize::from(k))?);
        let arm = (usize::from(at.0) * 16 + off)
            .checked_sub(start)
            .ok_or_else(no)?;
        if arm == end {
            kept |= 1 << (k + 1);
        } else if arm != follows {
            return Err(no());
        }
    }
    Ok(kept)
}

/// The task start either tests the armed bit before it clears it — `test
/// word [bx+m16], 8000h ; je` (`F7 87 lo hi 00 80 74`) — or clears it at
/// once, `and word [bx+m16], 7FFFh` (`81 A7 lo hi FF 7F`) the first
/// thing after the row is indexed. *Dunkle Schatten*'s `0CFDh:1A3Fh` tests
/// at `0x0EA1B` and *Captain Zins*'s `0849h:1A20h` at `0x09EBB`; *Bi-Fi
/// 2*'s `080Ch:1950h` clears at `0x09A1B`.
fn task_start_needs_armed(img: &Image, at: (u16, u16)) -> Result<bool> {
    let b = body(img, at, 0x20, "the task start")?;
    let test = b
        .windows(7)
        .position(|w| w[..2] == [0xF7, 0x87] && w[4..] == [0x00, 0x80, 0x74]);
    let clear = b
        .windows(6)
        .position(|w| w[..2] == [0x81, 0xA7] && w[4..] == [0xFF, 0x7F]);
    match (test, clear) {
        (Some(t), Some(c)) if t < c => Ok(true),
        (None, Some(_)) => Ok(false),
        _ => Err(refused("the task start", at)),
    }
}

/// Escape's arm in the dispatcher locks the input with a near call into
/// its own segment (`nop ; push cs ; call`, `90 0E E8 rel`) and stores the
/// verb at once, `mov word [verb], n` followed by the gate's segment
/// (`C7 06 lo hi n 00 C7 06`), once in the body. *Dunkle Schatten*'s sets
/// 6 at `0x0FD32`, *Captain Zins*'s `0A01h:1006h` (`0x0B016`) 6 too;
/// *Bi-Fi 2*'s 0Eh at `0x0AC6D`.
fn escape_verb(img: &Image, at: (u16, u16), lock: (u16, u16)) -> Result<u8> {
    let no = || refused("the dispatcher's Escape", at);
    let b = body(img, at, 0x400, "the dispatcher")?;
    let found: Vec<u8> = (0..b.len().saturating_sub(16))
        .filter(|&i| {
            let target = (usize::from(at.1) + i + 5)
                .wrapping_add(usize::from(u16::from_le_bytes([b[i + 3], b[i + 4]])))
                & 0xFFFF;
            b[i..i + 3] == [0x90, 0x0E, 0xE8]
                && at.0 == lock.0
                && target == usize::from(lock.1)
                && b[i + 5..i + 7] == [0xC7, 0x06]
                && b[i + 10] == 0
                && b[i + 11..i + 13] == [0xC7, 0x06]
        })
        .map(|i| b[i + 9])
        .collect();
    match found[..] {
        [verb] => Ok(verb),
        _ => Err(no()),
    }
}

/// The entry snaps the figure onto the entry's own node with `push -1 ;
/// push si|di ; push ds ; push rec ; call far ; add sp, 8` (`6A FF 56|57 1E
/// 68 lo hi 9A … 83 C4 08`; *Captain Zins* keeps the entry in `di`) and
/// then either tests the answer, `or ax, ax ; je` (`0B C0 74`), to go on to
/// node `7Fh` and the first exit — *Dunkle Schatten*'s `0x0EF85`,
/// *Captain Zins*'s `0A01h:023Ch` (`0x0A24C`) — or is done (`EB`, *Bi-Fi
/// 2*'s `0x09F64`).
fn place_falls_back(img: &Image, at: (u16, u16)) -> Result<bool> {
    let b = body(img, at, 0x2C0, "the entry")?;
    let i = b
        .windows(15)
        .position(|w| {
            w[..2] == [0x6A, 0xFF]
                && matches!(w[2], 0x56 | 0x57)
                && w[3..5] == [0x1E, 0x68]
                && w[7] == 0x9A
                && w[12..] == [0x83, 0xC4, 0x08]
        })
        .ok_or_else(|| refused("the entry", at))?;
    match b.get(i + 15..i + 18) {
        Some([0x0B, 0xC0, 0x74]) => Ok(true),
        Some([0xEB, ..]) => Ok(false),
        _ => Err(refused("the entry", at)),
    }
}

/// An item given out of sight either scrolls the strip to it, `mov
/// [scroll], dl` (`88 16 lo hi`, *Dunkle Schatten*'s `0x0CDB0`, *Captain
/// Zins*'s `078Ah:0961h`, `0x08201`), or to the
/// start of the row before it: `cmp dx, n ; jge` (`83 FA n 7D`), then
/// `add al, −n ; and al, −n` (`04 −n 24 −n`, *Bi-Fi 2*'s
/// `0x07E7B`–`0x07E8B`). The take must agree: by rows it steps back a row
/// against the count rounded down, `and dx, −n` (`83 E2 −n`) and `add al,
/// −n` (`04 −n`, `0x07F38`, `0x07F42`).
fn inventory_rows(img: &Image, map: &LibraryMap) -> Result<Option<u8>> {
    let give = body(img, map.inv_give, 0x80, "the item given")?;
    let take = body(img, map.inv_take, 0xB0, "the item taken")?;
    let has = |b: &[u8], p: &[u8]| b.windows(p.len()).any(|w| w == p);
    let row = give
        .windows(5)
        .find(|w| w[..2] == [0x83, 0xFA] && w[3] == 0x7D)
        .map(|w| w[2])
        .filter(|&n| {
            let m = n.wrapping_neg();
            n.is_power_of_two() && has(give, &[0x04, m, 0x24, m])
        });
    let by_item = give.windows(2).any(|w| w == [0x88, 0x16]);
    match (row, by_item) {
        (Some(n), false) => {
            let m = n.wrapping_neg();
            if has(take, &[0x83, 0xE2, m]) && has(take, &[0x04, m]) {
                Ok(Some(n))
            } else {
                Err(refused("the item taken", map.inv_take))
            }
        }
        (None, true) => Ok(None),
        _ => Err(refused("the item given", map.inv_give)),
    }
}

/// How a scene number names a place: a negative one is the complement of
/// a place, of bank 999 below `4000h` and of the inventory's bank from it
/// on, less [`Self::item_base`]; any other a place of the location's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SceneLookup {
    /// What an inventory place is less: `4001h` in *Dunkle Schatten*
    /// (`sub si, 4001h`, `0x0D592`), whose handlers hand `~(id + 4000h)`
    /// for item `id` and so name place `id − 1`; `4000h` in *Captain Zins*
    /// (`sub si, 4000h`, `0x08A70`) and *Bi-Fi 2* (`add ax, 0C000h`,
    /// `0x08657`), place `id`.
    pub item_base: u16,
    /// Whether the place is held against the bank's count before it is
    /// read (`cmp [bx+counts], si ; jle`).
    pub bounded: bool,
    /// Whether a place that names nothing falls back on bank 999's place
    /// `13h`, unless the `/TEST` switch is on (`test word [m16], 100h ;
    /// jne`).
    pub falls_back: bool,
}

impl SceneLookup {
    /// Read off the routine: `sub si, n` (`81 EE n`) or `add ax, −n` (`05
    /// −n`, `mov ax, si ; add ax, −n`, `8B C6 05 −n`) after `cmp si,
    /// 4000h`; a count compared, `cmp [bx+m16], ax|si`
    /// (`39 87`, `39 B7`) or `cmp si, [m16]` (`3B 36`); the fallback's
    /// switch test (`F7 06 lo hi 00 01 75`).
    fn read(img: &Image, at: (u16, u16)) -> Result<Self> {
        let no = || refused("the scene's lookup", at);
        let b = body(img, at, 0xC0, "the scene's lookup")?;
        let split = b
            .windows(5)
            .position(|w| w == [0x81, 0xFE, 0x00, 0x40, 0x7C])
            .ok_or_else(no)?;
        let t = &b[split..];
        let sub = t
            .windows(4)
            .find(|w| w[..2] == [0x81, 0xEE])
            .map(|w| u16::from_le_bytes([w[2], w[3]]));
        let add = t
            .windows(5)
            .find(|w| w[..3] == [0x8B, 0xC6, 0x05])
            .map(|w| u16::from_le_bytes([w[3], w[4]]).wrapping_neg());
        let item_base = match (sub, add) {
            (Some(n), _) | (None, Some(n)) if (0x4000..=0x4001).contains(&n) => n,
            _ => return Err(no()),
        };
        let bounded = b
            .windows(2)
            .any(|w| w == [0x39, 0x87] || w == [0x39, 0xB7] || w == [0x3B, 0x36]);
        let falls_back = b
            .windows(7)
            .any(|w| w[..2] == [0xF7, 0x06] && w[4..] == [0x00, 0x01, 0x75]);
        Ok(SceneLookup {
            item_base,
            bounded,
            falls_back,
        })
    }
}

/// The options' gate beyond *Dunkle Schatten*'s: whether it drops the ticks
/// the menu was up for, and whether a wait bit holds the options back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OptionsGate {
    /// `mov word [ticks], 0` after the menu (*Dunkle Schatten*'s
    /// `0x0F036`, *Captain Zins*'s `0A01h:04DAh`, `0x0A4EA`); *Bi-Fi 2*'s
    /// goes on to the input at once (`0x0A019`).
    pub drops_ticks: bool,
    /// The bit that holds the options back, when the build tests one.
    pub held: Option<OptionsHeld>,
}

/// The options held back: with `bit` among the wait bits the gate opens no
/// menu. In the location `silent_at` it ends as if the menu had closed;
/// anywhere else it plays `scene`, waits for the tasks (`wait`), lets the
/// gate and the input go and answers 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OptionsHeld {
    pub bit: u16,
    /// The word the location running is read from.
    pub here_word: u16,
    pub silent_at: u16,
    pub scene: i16,
    pub wait: u16,
}

impl OptionsGate {
    /// The gate tests the wait bits, `test word [bits], b ; je` (`F7 06 lo
    /// hi b 00 74 r`), then the location running, `cmp word [here], k ; je`
    /// (`83 3E lo hi k 74 r`), then plays `push s ; call far` (`6A s 9A`)
    /// and sets the wait, `or word [bits], w` (`83 0E lo hi w`) — *Captain
    /// Zins*'s from `0x0A4FF`, *Bi-Fi 2*'s from `0x09FDA`; *Dunkle Schatten*'s
    /// tests nothing. The menu is the far call a near one follows (`9A …
    /// E8`), and after them either `mov word [m16], 0` (`C7 06 lo hi 00 00`,
    /// the ticks dropped) or anything else.
    fn read(img: &Image, at: (u16, u16)) -> Result<Self> {
        let no = || refused("the options' gate", at);
        let b = body(img, at, 0x70, "the options' gate")?;
        let held = match b
            .windows(8)
            .position(|w| w[..2] == [0xF7, 0x06] && w[5] == 0 && w[6] == 0x74)
        {
            None => None,
            Some(i) => {
                let c = b.get(i + 8..i + 18).ok_or_else(no)?;
                if c[..2] != [0x83, 0x3E] || c[5] != 0x74 || c[7] != 0x6A || c[9] != 0x9A {
                    return Err(no());
                }
                let wait = b[i..]
                    .windows(5)
                    .find(|w| w[..2] == [0x83, 0x0E])
                    .map(|w| u16::from(w[4]))
                    .ok_or_else(no)?;
                Some(OptionsHeld {
                    bit: u16::from(b[i + 4]),
                    here_word: u16::from_le_bytes([c[2], c[3]]),
                    silent_at: u16::from(c[4]),
                    scene: i16::from(i8::from_le_bytes([c[8]])),
                    wait,
                })
            }
        };
        let menu = b
            .windows(6)
            .position(|w| w[0] == 0x9A && w[5] == 0xE8)
            .ok_or_else(no)?;
        let drops_ticks = b
            .get(menu + 8..menu + 14)
            .is_some_and(|w| w[..2] == [0xC7, 0x06] && w[4..] == [0, 0]);
        Ok(OptionsGate { drops_ticks, held })
    }
}

/// The status line's gate, `cmp word [age], 0 ; je` (`83 3E lo hi 00 74
/// r`), `test word [bits], 5 ; jne` (`F7 06 lo hi 05 00 75 r`), `mov ax,
/// [age] ; inc word [age]` (`A1 lo hi FF 06 lo hi`), `cmp ax, k ; jng`
/// (`3D k 00 7E`): the line is composed again once the age was above `k` —
/// *Dunkle Schatten*'s 3 (`0x101EB`), *Captain Zins*'s and *Bi-Fi 2*'s 2
/// (`0x0B63F`, `0x0B105`).
fn status_after(img: &Image, at: (u16, u16)) -> Result<i16> {
    let b = body(img, at, 0x30, "the status line's gate")?;
    let i = b
        .windows(5)
        .position(|w| w[..2] == [0x83, 0x3E] && w[4] == 0)
        .ok_or_else(|| refused("the status line's gate", at))?;
    let c = b
        .get(i..i + 25)
        .ok_or_else(|| refused("the status line's gate", at))?;
    let age = [c[2], c[3]];
    let shaped = c[5] == 0x74
        && c[7..9] == [0xF7, 0x06]
        && c[11..13] == [5, 0]
        && c[13] == 0x75
        && c[15] == 0xA1
        && c[16..18] == age
        && c[18..20] == [0xFF, 0x06]
        && c[20..22] == age
        && c[22] == 0x3D
        && b.get(i + 25) == Some(&0x7E);
    if !shaped {
        return Err(refused("the status line's gate", at));
    }
    Ok(i16::from(c[23]) | (i16::from(c[24]) << 8))
}

/// Whether a build keeps records of kind 8 apart when it binds a record to
/// its animation entry.
///
/// *Dunkle Schatten*'s `14DDh:0098h` tests the record's kind once the
/// entry is found (`les bx, [bp+6] ; cmp byte es:[bx], 8 ; jnz`,
/// `0x14F0A`): a kind-8 record takes the record's frame into the entry and
/// state `0Bh` (`0x14F13`–`0x14F3D`), any other the base moved by its
/// block and state `0Ah`. The same build's `0883h:20D9h` rebinds a record
/// in state `0Bh` (`0x0A92D`) and `0A9Fh:0C0Ch` turns no kind-8 record
/// (`0x0B613`). *Captain Zins*'s bind `1021h:00A0h` (`0x102B0`) tests
/// the kind too. *Bi-Fi 2*'s bind goes straight to the base (`les bx,
/// [bp+6] ; mov ax, es:[bx+0Ch] ; mov es:[bx+2Eh], ax`, `0x0F5C1`), and
/// its start and its turn have neither arm (`0x05D3D`, `0x06916`).
fn kind8_records(img: &Image, at: (u16, u16)) -> Result<bool> {
    const KIND8: [u8; 8] = [0xC4, 0x5E, 0x06, 0x26, 0x80, 0x3F, 0x08, 0x75];
    const BASE: [u8; 11] = [
        0xC4, 0x5E, 0x06, 0x26, 0x8B, 0x47, 0x0C, 0x26, 0x89, 0x47, 0x2E,
    ];
    let b = body(img, at, 0x140, "the record's animation bind")?;
    let first = |pat: &[u8]| b.windows(pat.len()).position(|w| w == pat);
    match (first(&KIND8), first(&BASE)) {
        (Some(k), Some(base)) if k < base => Ok(true),
        (None, Some(_)) => Ok(false),
        _ => Err(refused("the record's animation bind", at)),
    }
}

/// The highest state the animator's step has an arm of its own for.
///
/// The step switches on `+12h` through a table, bounded first: `mov al,
/// es:[bx+12h] ; mov ah, 0 ; mov bx, ax ; cmp bx, n ; jna ; jmp far_arm ;
/// shl bx, 1 ; jmp cs:[bx+t]` (`26 8A 47 12 B4 00 8B D8 83 FB n 76 03 E9 r
/// r D1 E3 2E FF A7`). The arm past the bound calls the record hook the
/// row installed, with the record and code 0, or clears the state
/// without one: `mov ax, [w] ; or ax, [w+2] ; jz ; push 0 ; push
/// [bp+6] ; push [bp+4] ; call far [w] ; add sp, 6 ; jmp ; les bx,
/// [bp+4] ; mov byte es:[bx+12h], 0` (`A1 w w 0B 06 w+2 w+2 74 11 6A 00
/// FF 76 06 FF 76 04 FF 1E w w 83 C4 06 EB 08 C4 5E 04 26 C6 47 12 00`).
/// *Dunkle Schatten*'s `0883h:012Ah` has `0Bh` (`0x0897D`, the arm at
/// `0x08AD2` on `[2871h]`), *Captain Zins*'s `03E8h:0127h` `0Bh` (`0x03FCA`,
/// `0x0411F` on `[1E3Ch]`), *Bi-Fi 2*'s `03F7h:011Ch` `0Ah` (`0x040AE`,
/// `0x041E0` on `[2D1Ch]`).
fn anim_arms(img: &Image, at: (u16, u16)) -> Result<u8> {
    const SWITCH: [u8; 10] = [0x26, 0x8A, 0x47, 0x12, 0xB4, 0x00, 0x8B, 0xD8, 0x83, 0xFB];
    const TABLE: [u8; 5] = [0xD1, 0xE3, 0x2E, 0xFF, 0xA7];
    let no = || refused("the animator's step", at);
    let b = body(img, at, 0x200, "the animator's step")?;
    let i = b
        .get(..0x40)
        .and_then(|h| h.windows(SWITCH.len()).position(|w| w == SWITCH))
        .ok_or_else(no)?;
    let n = *b.get(i + 10).ok_or_else(no)?;
    if b.get(i + 11..i + 14) != Some(&[0x76, 0x03, 0xE9][..])
        || b.get(i + 16..i + 21) != Some(&TABLE[..])
    {
        return Err(no());
    }
    let rel = u16::from_le_bytes([b[i + 14], b[i + 15]]);
    let arm = (i + 16).wrapping_add(usize::from(rel)) & 0xFFFF;
    let a = b.get(arm..arm + 34).ok_or_else(no)?;
    let w = [a[1], a[2]];
    let w2 = u16::from_le_bytes(w).wrapping_add(2).to_le_bytes();
    let want: [u8; 34] = [
        0xA1, w[0], w[1], 0x0B, 0x06, w2[0], w2[1], 0x74, 0x11, 0x6A, 0x00, 0xFF, 0x76, 0x06, 0xFF,
        0x76, 0x04, 0xFF, 0x1E, w[0], w[1], 0x83, 0xC4, 0x06, 0xEB, 0x08, 0xC4, 0x5E, 0x04, 0x26,
        0xC6, 0x47, 0x12, 0x00,
    ];
    if a != want {
        return Err(no());
    }
    Ok(n)
}

/// How a build writes an object's byte in the state row and reads it
/// back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordCodes {
    /// 1 not live, 2 hidden, else the frame plus three (*Dunkle
    /// Schatten*'s `0x154CE`–`0x154EA`, read back at `0x15241`–`0x15260`;
    /// *Captain Zins*'s `1021h:0653h`, `0x10863`, read back by
    /// `1021h:029Bh`, `0x104AB`).
    Hidden,
    /// 1 when the flags are clear, else the frame plus two, and no hidden
    /// object: read back as live on that frame (*Bi-Fi 2*'s
    /// `0x0FB04`–`0x0FB18`, `0x0F8B0`–`0x0F8CA`).
    Plain,
}

/// The state row as the visit's setup reads it back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Records {
    /// The codes.
    pub codes: RecordCodes,
    /// The kinds besides 0 whose objects take a byte back, as many as the
    /// build reads.
    pub read: [Option<u16>; 3],
    /// A kind whose object, consuming no byte, is put live on frame 1 when
    /// its frame is 0 (`cmp byte es:[bx+9], 0 ; jne`).
    pub fresh: Option<u16>,
}

impl Records {
    /// The read-back sends the object's kind (`mov al, es:[bx] ; mov ah,
    /// 0`, `26 8A 07 B4 00`) through either a chain — `or ax, ax ; je`, then
    /// `cmp ax, k ; je` for each kind (`0B C0 74 r`, `3D k 00 74 r`) — or a
    /// table, `mov bx, ax ; cmp bx, n ; ja ; shl bx, 1 ; jmp cs:[bx+t]`
    /// (`8B D8 83 FB n 77 r D1 E3 2E FF A7 t`). An arm that takes a byte
    /// opens `les bx, [bp+6] ; inc word [bp+6]` (`C4 5E 06 FF 46 06`); the
    /// fresh kind's `les bx, [bp-0Ch] ; cmp byte es:[bx+9], 0 ; jne` (`C4 5E
    /// F4 26 80 7F 09 00 75`); a table's other arms must be its default.
    /// The codes are told by both routines: `test byte es:[bx+2], 2` in the
    /// store (`26 F6 47 02 02`) and `mov byte es:[bx+2], 3` with `add al,
    /// 0FDh` in the read-back (`26 C6 47 02 03`, `04 FD`) — or `cmp byte
    /// es:[bx+2], 0 ; je` (`26 80 7F 02 00 74`) and `add al, 0FEh` (`04
    /// FE`). *Dunkle Schatten*'s chain (`0x151ED`) reads kinds 1 and 5;
    /// *Captain Zins*'s table (`0x10622`, `cs:0647h`) kinds 1, 4 and 5;
    /// *Bi-Fi 2*'s chain (`0x0F855`) kind 1, and kind 4 fresh.
    fn read(img: &Image, map: &LibraryMap) -> Result<Self> {
        let at = map.objs_load_state;
        let no = || refused("the state row's read-back", at);
        let b = body(img, at, 0x2C0, "the state row's read-back")?;
        let st = body(img, map.objs_save_state, 0xC0, "the state row")?;
        let has = |b: &[u8], p: &[u8]| b.windows(p.len()).any(|w| w == p);
        let codes = if has(st, &[0x26, 0xF6, 0x47, 0x02, 0x02])
            && has(b, &[0x26, 0xC6, 0x47, 0x02, 0x03])
            && has(b, &[0x04, 0xFD])
        {
            RecordCodes::Hidden
        } else if has(st, &[0x26, 0x80, 0x7F, 0x02, 0x00, 0x74]) && has(b, &[0x04, 0xFE]) {
            RecordCodes::Plain
        } else {
            return Err(no());
        };
        let p = b
            .windows(5)
            .position(|w| w == [0x26, 0x8A, 0x07, 0xB4, 0x00])
            .ok_or_else(no)?
            + 5;
        let rel = |from: usize, r: u8| from.checked_add_signed(isize::from(i8::from_le_bytes([r])));
        // (kind, arm) for every kind but 0, and where a table's default is.
        let mut arms = Vec::new();
        let mut default = None;
        if b.get(p..p + 3) == Some(&[0x0B, 0xC0, 0x74][..]) {
            let mut q = p + 4;
            while let Some(w) = b
                .get(q..q + 5)
                .filter(|w| w[0] == 0x3D && w[2] == 0 && w[3] == 0x74)
            {
                arms.push((u16::from(w[1]), rel(q + 5, w[4]).ok_or_else(no)?));
                q += 5;
            }
        } else if let Some(w) = b.get(p..p + 13).filter(|w| {
            w[..3] == [0x8B, 0xD8, 0x83]
                && w[3] == 0xFB
                && w[5] == 0x77
                && w[7..11] == [0xD1, 0xE3, 0x2E, 0xFF]
        }) {
            if w[11] != 0xA7 {
                return Err(no());
            }
            let n = u16::from(w[4]);
            default = rel(p + 7, w[6]);
            let table =
                usize::from(at.0) * 16 + usize::from(u16::from_le_bytes([w[12], b[p + 13]]));
            let start = usize::from(at.0) * 16 + usize::from(at.1);
            for k in 1..=n {
                let off = usize::from(img.word(table + 2 * usize::from(k))?);
                let arm = (usize::from(at.0) * 16 + off)
                    .checked_sub(start)
                    .ok_or_else(no)?;
                arms.push((k, arm));
            }
        } else {
            return Err(no());
        }
        let mut records = Records {
            codes,
            read: [None; 3],
            fresh: None,
        };
        let mut n = 0;
        for (k, arm) in arms {
            let a = b.get(arm..arm + 9).ok_or_else(no)?;
            if a[..6] == [0xC4, 0x5E, 0x06, 0xFF, 0x46, 0x06] {
                *records.read.get_mut(n).ok_or_else(no)? = Some(k);
                n += 1;
            } else if a == [0xC4, 0x5E, 0xF4, 0x26, 0x80, 0x7F, 0x09, 0x00, 0x75] {
                if records.fresh.replace(k).is_some() {
                    return Err(no());
                }
            } else if Some(arm) != default {
                return Err(no());
            }
        }
        Ok(records)
    }

    /// Whether kind `k` takes a byte back.
    pub(crate) fn reads(&self, k: u16) -> bool {
        self.read.contains(&Some(k))
    }
}

/// Which of its strip and figure files a build loads for a visit: the
/// digit `n` the files' names take for their `%`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FigureSet {
    /// Always the same one (*Dunkle Schatten*'s, 0).
    Fixed(u16),
    /// One file with no digit in its name, kept once it is loaded: the
    /// visit asks the loader with `−2` (`push 0FEh`), which returns at once
    /// for a negative argument when the file is in (`cmp word [bp+8], 0 ;
    /// jl`, *Bi-Fi 2*'s `0x0B74A`). The player's record keeps no
    /// digit.
    Kept,
    /// 1 once the location word is above `above`, else 0 (*Captain
    /// Zins*'s, `[1A5Ch]` above 2).
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
            FigureSet::Kept => vec![0],
            FigureSet::AfterLocation { .. } => vec![0, 1],
        }
    }

    /// The digit for a visit to `loc`.
    pub(crate) fn digit_for(self, loc: i16) -> u16 {
        match self {
            FigureSet::Fixed(n) => n,
            FigureSet::Kept => 0,
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
    /// The horizontal field (`and ax, 0C0h`), value by value; `None` for
    /// a build that does not read it.
    pub horizontal: Option<[(u16, u16); 2]>,
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
        let horizontal = self.horizontal.as_ref().map_or(&[][..], |h| &h[..]);
        pick(&self.vertical, code & 0x30) | pick(horizontal, code & 0xC0)
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
    /// The entries, first and last, whose object is a door when it is of
    /// [`Self::kind`].
    pub ids: (u16, u16),
    /// The object kind a door is.
    pub kind: u8,
    /// Whether the door is remembered until it has swung shut.
    pub remembered: bool,
    /// The frame it is shown on; below zero, hidden.
    pub frame: i16,
    /// Entries past [`Self::ids`] whose object is shown so whatever its
    /// kind, each with whether it is remembered too.
    pub more: [Option<(u16, bool)>; 3],
}

/// How a build's room loader, its slot unload and the pair routine treat
/// rows of the kinds they name. *Dunkle Schatten*'s forest is one such
/// kind: its rows share one picture, kept loaded between two of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SharedRooms {
    /// The kinds the pair routine answers when the place left and the
    /// place ahead are both of it; it answers 0 for any other pair.
    pub pairs: [Option<u16>; 3],
    /// The kinds whose rows the loader takes one fixed slot for, with the
    /// slot. Between two rows of such a kind the loader loads nothing and
    /// the unload keeps the room.
    pub shared: [Option<(u16, u16)>; 2],
    /// A kind whose rows load their own slot, but between two of them the
    /// loader skips the slot's song and effects (`mov dx, FFFFh`) and the
    /// unload works by [`Self::sound_flags`] (`mov si, FFFFh`).
    pub sound_kept: Option<u16>,
    /// The flags the unload works by for a pair of [`Self::sound_kept`]
    /// rows (`or si, si ; jnl ; mov di, n`); a build may carry it with no
    /// such kind.
    pub sound_flags: u16,
    /// The kind whose rows have their exits written into the room's
    /// object map as the loader binds it, and how.
    pub exits: Option<MapExits>,
}

impl SharedRooms {
    /// The slot rows of `kind` share, if they share one.
    pub(crate) fn slot(&self, kind: u16) -> Option<u16> {
        self.shared
            .iter()
            .flatten()
            .find(|s| s.0 == kind)
            .map(|s| s.1)
    }

    /// Whether the pair routine answers `kind`.
    pub(crate) fn pairs(&self, kind: u16) -> bool {
        self.pairs.contains(&Some(kind))
    }
}

/// A row's exits written into its object map: the record's first exit
/// byte and the three entries `i`, `i + 4`, `i + 8` per exit `i` of four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MapExits {
    /// The rows' kind (`cmp word es:[bx+7], k`).
    pub kind: u16,
    /// Where in the location record the four exit bytes start.
    pub first: u16,
    /// How the entries are filled.
    pub form: ExitForm,
}

/// The two ways a build fills a row's map from its exits. Entry `i`
/// always takes the exit's id; entry `i + 8` takes 2000 more than it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitForm {
    /// *Dunkle Schatten*: no exit hides entry `i` and `i + 8`; an exit
    /// clears entry `i + 4` and hides it.
    Cleared,
    /// *Bi-Fi 2*: no exit clears entry `i + 4` and hides all three;
    /// an exit gives entry `i + 4` its id plus this offset.
    Offset(u16),
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
    /// Whether the task start starts only an armed task (`test word
    /// [bx+flags], 8000h ; je` before the bit is cleared).
    pub task_start_needs_armed: bool,
    /// The verb Escape sets as it puts the options' gate up.
    pub escape_verb: u8,
    /// Whether the entry, when the entry's own node is not in the room,
    /// tries node `7Fh` and then the row's first exit.
    pub place_falls_back: bool,
    /// For a build that scrolls the strip by rows when an item is given or
    /// taken, the row; `None` for one that scrolls by the item.
    pub inventory_rows: Option<u8>,
    /// How a scene number names a place.
    pub scenes: SceneLookup,
    /// Whether the script's exit gate turns hover off before it installs
    /// itself (`call far hover_off` as its first instruction).
    pub script_exit_hovers_off: bool,
    /// The fade the music gets when a location is left, in milliseconds
    /// (`push imm16` before the fade's far call).
    pub leave_fade: u16,
    /// The kinds two places may share for the leaving to spare the music
    /// and the transition (`cmp ax, k ; je` after the pair's kind is
    /// asked), as many as the build tests.
    pub leave_spares: [Option<u16>; 3],
    /// Whether the leaving ends by setting the status line to the empty
    /// string and painting the band.
    pub leave_clears_status: bool,
    /// Whether mode 8, once it has asked the gate, sets the status line's
    /// age to `0Ah` ([`mode_step_ages_status`]).
    pub mode_step_ages_status: bool,
    /// How the entry brings the room up, by the location and the pair of
    /// places.
    pub entry_end: EntryEnd,
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
    /// How the object state answers.
    pub object_state: ObjectState,
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
    /// How the state row is read back.
    pub records: Records,
    /// The kinds whose objects have no hotspot to follow them, one bit per
    /// kind.
    pub hotspot_kept: u16,
    /// Whether showing the head of a chain shows its parts hidden, until
    /// the animator places them (`or byte es:[bx+2], 3`), rather than
    /// only live (`… 1`).
    pub show_hides_parts: bool,
    /// Whether hiding the head of a chain hides the head too, not its
    /// parts alone.
    pub hide_clears_head: bool,
    /// Whether a hotspot's own line sets task 0's speakers to 1.
    pub own_line_sets_speakers: bool,
    /// What the options' gate does besides opening the menu.
    pub options_gate: OptionsGate,
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
    /// Whether choosing verb 6 takes its own road, to mode 5 with no
    /// object (`cmp si, 6 ; je`).
    pub verb6_arm: bool,
    /// Whether choosing verb 6 asks for the status line again with an age
    /// of 0Ah (`mov word [m16], 0Ah`).
    pub verb6_refreshes_status: bool,
    /// The verbs below this are the strip's (`not si ; cmp si, n ; jge`).
    pub verb_bound: u16,
    /// What makes the chosen object want a second one ([`SecondWanted`]).
    pub second_wanted: SecondWanted,
    /// Whether a frame under the lock stops task 0 when it shows a hover
    /// text, kind 2, which the query answers −1 for.
    pub locked_stops_hover_text: bool,
    /// Whether the mode reset forgets the hover too (`mov word [hover],
    /// FFFFh`).
    pub mode_reset_forgets_hover: bool,
    /// The rows the room loader, the unload and the pair routine treat
    /// apart by their kind.
    pub rooms: SharedRooms,
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
    /// The figure file's first resource that is a cel: 1 when the loader
    /// steps its slot past the palette pointer the file opens with, 0 when
    /// it keeps the file's start.
    pub figure_first: usize,
    /// What range the strip's loader gives the carried list.
    pub strip: StripRange,
    /// The flags a page's controls set.
    pub text_flags: TextFlags,
    /// The row the status line is set on and the band held dark from.
    pub status_row: i32,
    /// How many rows the band held dark is high.
    pub status_band: i32,
    /// The inks the status line is set in.
    pub status_inks: StatusInks,
    /// The status line's age past which the gate composes it again.
    pub status_after: i16,
    /// What the choice menu's strip redraw does to the band besides the
    /// panel and the items ([`menu_band`]).
    pub menu_band: Option<MenuBand>,
    /// What each code of an animation's signal does.
    pub signal: super::signal::SignalArms,
    /// What the script's own interaction chooses with.
    pub interaction: Interaction,
    /// Whether records of kind 8 bind to their animation in state `0Bh`,
    /// and the start and the turn treat them apart ([`kind8_records`]).
    pub kind8_records: bool,
    /// The highest state the animator's step has an arm for; a record in
    /// a state above it goes to the row's record hook, or has its state
    /// cleared without one ([`anim_arms`]).
    pub anim_arms: u8,
}

/// The script's own interaction: the id below which an object chosen is
/// an item, and the verb the machine is set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Interaction {
    pub items_end: u16,
    pub verb: u16,
}

impl Interaction {
    /// `cmp si, 100h ; jl` (`81 FE 00 01 7C`), then `cmp si, k ; jl` (`83
    /// FE k 7C`), the items' end; and `mov word [mode], 7 ; mov word
    /// [verb], v` (`C7 06 lo hi 07 00 C7 06 lo hi v 00`), the verb — *Dunkle
    /// Schatten*'s 17h and 2 (`0x1031E`, `0x103DE`), *Captain Zins*'s 1Ah and 2
    /// (`0x0B772`, `0x0B832`), *Bi-Fi 2*'s 30h and 3 (`0x0B238`,
    /// `0x0B2F8`).
    fn read(img: &Image, at: (u16, u16)) -> Result<Self> {
        let no = || refused("the script's interaction", at);
        let b = body(img, at, 0x110, "the script's interaction")?;
        let i = b
            .windows(5)
            .position(|w| w == [0x81, 0xFE, 0x00, 0x01, 0x7C])
            .ok_or_else(no)?;
        let j = find_from(b, i, &[0x83, 0xFE]).ok_or_else(no)?;
        if b.get(j + 3) != Some(&0x7C) {
            return Err(no());
        }
        let v = b
            .windows(12)
            .find(|w| w[..2] == [0xC7, 0x06] && w[4..8] == [7, 0, 0xC7, 0x06] && w[11] == 0)
            .ok_or_else(no)?;
        Ok(Interaction {
            items_end: u16::from(b[j + 2]),
            verb: u16::from(v[10]),
        })
    }
}

impl Profile {
    /// Reads every capability off the routines `map` names in `img`.
    pub(crate) fn read(img: &Image, map: &LibraryMap) -> Result<Self> {
        let d = Dispatch::read(img, map.dispatch)?;
        let (status_row, status_band) = status_band(img, map.status_paint)?;
        let status_after = status_after(img, map.status_gate)?;
        let signal = super::signal::SignalArms::read(img, map)?;
        let interaction = Interaction::read(img, map.interaction)?;
        let leave = Leave::read(img, map.visit_leave)?;
        Ok(Profile {
            lock_forgets_hover: lock_forgets_hover(img, map.input_lock, 0x0E)?,
            unlock_forgets_hover: lock_forgets_hover(img, map.input_unlock, 0x26)?,
            hover_off_forgets_hover: hover_off_forgets_hover(img, map.hover_off)?,
            task_start_tells_hook: task_start_tells_hook(img, map.task_start)?,
            task_start_needs_armed: task_start_needs_armed(img, map.task_start)?,
            escape_verb: escape_verb(img, map.dispatch, map.input_lock)?,
            place_falls_back: place_falls_back(img, map.visit_enter)?,
            inventory_rows: inventory_rows(img, map)?,
            scenes: SceneLookup::read(img, map.scene_bind)?,
            script_exit_hovers_off: script_exit_hovers_off(img, map)?,
            leave_fade: leave.fade,
            leave_spares: leave.spares,
            leave_clears_status: leave.clears_status,
            mode_step_ages_status: mode_step_ages_status(img, map.mode_step)?,
            entry_end: EntryEnd::read(img, map)?,
            entry_fade_in: entry_fade_in(img, map.visit_finish_entry)?,
            hover_on_forgets_hover: hover_on(img, map.hover_on)?.0,
            hover_on_polls: hover_on(img, map.hover_on)?.1,
            object_state: object_state(img, map.obj_state)?,
            walk_to_node_remembers: match map.walk_to_node {
                Some(at) => walk_to_node_remembers(img, at)?,
                // A build without the routine: its logic walks the figure
                // itself and nothing asks what such a walk remembers.
                None => false,
            },
            anim_checks_count: anim_checks_count(img, map.obj_anim)?,
            anim_wakes_only_kind: anim_wakes_only_kind(img, map.obj_anim)?,
            records: Records::read(img, map)?,
            hotspot_kept: hotspot_kept(img, map.hotspot_follow)?,
            show_hides_parts: show_hides_parts(img, map.obj_show)?,
            hide_clears_head: hide_clears_head(img, map.obj_hide)?,
            own_line_sets_speakers: own_line_sets_speakers(img, map.say_text)?,
            options_gate: OptionsGate::read(img, map.options_gate)?,
            tri_state_kinds: tri_state_kinds(img, map.objs_save_state)?,
            dispatch_starts_asked_task: dispatch_starts_asked_task(img, map.dispatch)?,
            forgotten_hover_is_nothing: d.forgotten_is_nothing,
            room_click_forgets_hover: d.room_click_forgets,
            hover_off_only_notes: d.hover_off_only_notes,
            verb6_arm: d.verb6_arm,
            verb6_refreshes_status: d.verb6_refreshes,
            verb_bound: d.verb_bound,
            second_wanted: SecondWanted::read(img, map.second_wanted)?,
            locked_stops_hover_text: d.locked_stops_hover_text,
            mode_reset_forgets_hover: mode_reset_forgets_hover(
                img,
                map.mode_reset,
                d.last,
                d.forgotten_is_nothing,
            )?,
            rooms: shared_rooms(img, map)?,
            screen_slot: screen_slot(img, map.visit_enter)?,
            entry_door: entry_door(img, map.visit_enter)?,
            inventory_window: inventory_window(img, map.inv_take)?,
            callback: callback(img, map)?,
            slot_style: slot_style(img, map.slot_resolve)?,
            effects: effects(img, map.effects_load)?,
            song_slots: song_slots(img, map.slot_song)?,
            figure_set: figure_set(img, map.visit_setup)?,
            figure_first: figure_first(img, map.visit_setup)?,
            strip: strip_range(img, map.strip_load)?,
            text_flags: text_flags(img, map.text_scan, map.task_query)?,
            status_row,
            status_band,
            status_inks: status_inks(img, map.status_paint)?,
            status_after,
            menu_band: menu_band(img, map.menu_strip)?,
            signal,
            interaction,
            kind8_records: kind8_records(img, map.figure_bind)?,
            anim_arms: anim_arms(img, map.animator_step)?,
        })
    }
}

/// The bytes of the routine at `at`, up to `len` of them, or an error that
/// names the routine.
/// What makes the object a click has chosen want a second one, so that
/// the machine goes to mode 3 for it instead of looking for its rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SecondWanted {
    /// The verb alone: 5 or 9 (`mov ax, [verb] ; cmp ax, 5 ; je ; cmp ax,
    /// 9 ; jne`). *Dunkle Schatten*'s `0EB9h:0776h` (`0x0F306`), *Captain
    /// Zins*'s `0A01h:07F1h` (`0x0A801`).
    Verbs,
    /// The rules: a search with phase 8 (`push 8 ; call far`), and for an
    /// answer of −1 or 2 with verb 4 a probe for a verb-5 rule with phase
    /// `2030h` (`mov word [verb], 5 ; push 2030h ; call far`), the verb put
    /// back to 4 after it. A probe that finds one, or an answer of 0, takes
    /// the verb one on (`inc word [verb]`); any other answer keeps it. *Bi-Fi
    /// 2*'s `09B6h:0768h` (`0x0A2C8`–`0x0A30C`).
    Rules,
}

impl SecondWanted {
    fn read(img: &Image, at: (u16, u16)) -> Result<Self> {
        let b = body(img, at, 0x40, "the second object's want")?;
        if b[0] == 0xA1
            && b[3..9] == [0x3D, 0x05, 0x00, 0x74, 0x05, 0x3D]
            && b[9..12] == [0x09, 0x00, 0x75]
        {
            return Ok(SecondWanted::Verbs);
        }
        // `push si ; push 8 ; call far ; pop cx ; cmp ax, FFFFh ; je ; or
        // ax, ax ; je ; cmp ax, 2 ; jne ; xor si, si ; cmp word [verb], 4 ;
        // jne ; mov word [verb], 5 ; push 2030h ; call far`.
        let [v0, v1] = [b[0x1B], b[0x1C]];
        if b[..3] == [0x56, 0x6A, 0x08]
            && b[3] == 0x9A
            && b[8..12] == [0x59, 0x3D, 0xFF, 0xFF]
            && b[12] == 0x74
            && b[14..17] == [0x0B, 0xC0, 0x74]
            && b[18..22] == [0x3D, 0x02, 0x00, 0x75]
            && b[23..27] == [0x33, 0xF6, 0x83, 0x3E]
            && b[29..31] == [0x04, 0x75]
            && b[32..34] == [0xC7, 0x06]
            && b[34..38] == [v0, v1, 0x05, 0x00]
            && b[38..41] == [0x68, 0x30, 0x20]
        {
            return Ok(SecondWanted::Rules);
        }
        Err(refused("the second object's want", at))
    }
}

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
/// has the stores; *Captain Zins*'s `0A01h:18B0h` (`0x0B8C0`) and *Bi-Fi
/// 2*'s `09B6h:1826h` (`0x0B386`) make the far call at once.
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
/// `0CFDh:1A3Fh` (`0x0EA0F`) has it at `0x0EA50`; *Captain Zins*'s
/// `0849h:1A20h` (`0x09EB0`) and *Bi-Fi 2*'s `080Ch:1950h` (`0x09A10`) do
/// not.
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
/// `1586h:05E1h` (`0x15E41`) calls it; *Captain Zins*'s `10C3h:05A6h`
/// (`0x111D6`) and *Bi-Fi 2*'s `0FE3h:088Ah` (`0x106BA`) go straight to
/// their stores.
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

/// What the leaving does with the music, the transition and the line.
struct Leave {
    fade: u16,
    spares: [Option<u16>; 3],
    clears_status: bool,
}

impl Leave {
    /// The leaving fades the music with `push imm16 ; call far` (`68 lo
    /// hi 9A`), the first such push in the routine. Before it stand the far
    /// call that asked for the pair's kind and, after that, either nothing
    /// or one `cmp ax, k ; je rel8` (`3D k 00 74 rel`) for each kind that
    /// spares the music, all to the same place: `21h` bytes past the push,
    /// where the fade's path — the fade, the transition, the stop — ends
    /// too. There the routine either sets the line to the empty string,
    /// `push 0 ; push ds ; push s` (`6A 00 1E 68`), or returns (`CB`).
    /// *Dunkle Schatten*'s `0EB9h:017Ah` (`0x0ED0A`) has `cmp ax, 3` at
    /// `0x0ED34`, `push 5DCh` at `0x0ED39` and clears the line at
    /// `0x0ED5A`; *Captain Zins*'s `0A01h:01B4h` (`0x0A1C4`) spares no
    /// kind, pushes `7D0h` and clears the line; *Bi-Fi 2*'s `09B6h:018Eh`
    /// tests 2, 4 and 7 from `0x09D18`, pushes `7D0h` and returns at
    /// `0x09D48`.
    fn read(img: &Image, at: (u16, u16)) -> Result<Self> {
        let no = || refused("the leaving", at);
        let b = body(img, at, 0x80, "the leaving")?;
        let push = b
            .windows(4)
            .position(|w| w[0] == 0x68 && w[3] == 0x9A)
            .filter(|&i| i >= 5)
            .ok_or_else(no)?;
        let fade = u16::from_le_bytes([b[push + 1], b[push + 2]]);
        let end = push + 0x21;
        let mut kinds = Vec::new();
        let mut i = push;
        while i >= 5 {
            let w = &b[i - 5..i];
            if !(w[0] == 0x3D && w[2] == 0 && w[3] == 0x74) {
                break;
            }
            if i.checked_add_signed(isize::from(i8::from_le_bytes([w[4]]))) != Some(end) {
                return Err(no());
            }
            kinds.insert(0, u16::from(w[1]));
            i -= 5;
        }
        if i < 5 || b[i - 5] != 0x9A {
            return Err(no());
        }
        let mut spares = [None; 3];
        if kinds.len() > spares.len() {
            return Err(no());
        }
        for (s, k) in spares.iter_mut().zip(kinds) {
            *s = Some(k);
        }
        let clears_status = match b.get(end..end + 4) {
            Some([0x6A, 0x00, 0x1E, 0x68]) => true,
            Some([0xCB, ..]) => false,
            _ => return Err(no()),
        };
        Ok(Leave {
            fade,
            spares,
            clears_status,
        })
    }
}

/// How the entry brings a room up: its effect, if any, and whether the
/// room's song starts after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntryLook {
    /// The effect; `None` for none at all.
    pub effect: Option<EntryEffect>,
    /// Whether the room's song is started.
    pub song: bool,
}

/// The entry's two effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryEffect {
    /// The room revealed.
    Reveal,
    /// The room slid in.
    Slide,
}

/// The entry's choice of [`EntryLook`]: first by the location, then by the
/// kind the pair of places shares, else one for every other entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntryEnd {
    /// The word the entry reads the location from, when it tests one: the
    /// main loop's.
    pub location_word: Option<u16>,
    /// Locations the entry tests, each with its look.
    pub locations: [Option<(u16, EntryLook)>; 2],
    /// Pair kinds the entry tests, each with its look.
    pub pairs: [Option<(u16, EntryLook)>; 3],
    /// The look of every other entry.
    pub otherwise: EntryLook,
}

impl EntryEnd {
    /// The look of the entry into `location` with the pair's kind `pair`.
    pub(crate) fn look(&self, location: u16, pair: u16) -> EntryLook {
        let find = |rows: &[Option<(u16, EntryLook)>], k: u16| {
            rows.iter().flatten().find(|r| r.0 == k).map(|r| r.1)
        };
        find(&self.locations, location)
            .or_else(|| find(&self.pairs, pair))
            .unwrap_or(self.otherwise)
    }

    /// The entry's end marks the song with `mov si, 1` (`BE 01 00`) or
    /// `mov word [bp-2], 1` (`C7 46 FE 01 00`) and then chooses: `mov ax,
    /// [location]` (`A1 lo hi`) or the far call asking the pair's kind
    /// (`9A`), each followed by `cmp ax, k ; je|jne` (`3D k 00 74|75`), into
    /// arms that clear the mark (`33 F6`, `C7 46 FE 00 00`), call the reveal
    /// or the slide (`9A`), or jump (`EB`). The choice is over at the first
    /// far call that is none of those three. The routine is run here, for
    /// each location and kind it compares with and for neither; the
    /// locations must be tested before the pair is asked. *Dunkle
    /// Schatten*'s `0EB9h:00B8h` tests kinds 3 and 5 from `0x0EC68`;
    /// *Bi-Fi 2*'s `09B6h:00BBh` locations 1 and 24h from `0x09C39`,
    /// then kinds 2, 4 and 7 from `0x09C4E`; *Captain Zins*'s asks the pair
    /// and slides (`0x0A12F`, `0x0A134`).
    fn read(img: &Image, map: &LibraryMap) -> Result<Self> {
        let at = map.visit_finish_entry;
        let no = || refused("the entry's end", at);
        let b = body(img, at, 0x80, "the entry's end")?;
        let start = b
            .windows(5)
            .position(|w| w[..3] == [0xBE, 0x01, 0x00] || w == [0xC7, 0x46, 0xFE, 0x01, 0x00])
            .ok_or_else(no)?;
        let run = EntryRun { b, map, start, at };
        let mut first = Seen::default();
        let otherwise = run.look(None, 0, &mut first)?;
        // The run with neither passes every test once, past its `jne`: it
        // sees each constant the routine compares with.
        let mut end = EntryEnd {
            location_word: first.location_word,
            locations: [None; 2],
            pairs: [None; 3],
            otherwise,
        };
        if first.locations.len() > 2 || first.pairs.len() > 3 {
            return Err(no());
        }
        let mut again = Seen {
            location_word: first.location_word,
            ..Seen::default()
        };
        for (slot, &k) in end.locations.iter_mut().zip(&first.locations) {
            *slot = Some((k, run.look(Some(k), 0, &mut again)?));
        }
        for (slot, &k) in end.pairs.iter_mut().zip(&first.pairs) {
            *slot = Some((k, run.look(None, k, &mut again)?));
        }
        if again.location_word != first.location_word {
            return Err(no());
        }
        Ok(end)
    }
}

/// What a run of the entry's end has seen: the location word it reads and
/// the constants it compares with.
#[derive(Default)]
struct Seen {
    location_word: Option<u16>,
    locations: Vec<u16>,
    pairs: Vec<u16>,
}

/// The entry's end, to be run from the song's mark.
struct EntryRun<'a> {
    b: &'a [u8],
    map: &'a LibraryMap,
    start: usize,
    at: (u16, u16),
}

impl EntryRun<'_> {
    /// The look for a location (`None`: one it tests for no constant) and
    /// a pair's kind.
    fn look(&self, location: Option<u16>, pair: u16, seen: &mut Seen) -> Result<EntryLook> {
        enum Ax {
            Unknown,
            Location,
            Pair,
        }
        let (b, map) = (self.b, self.map);
        let no = || refused("the entry's end", self.at);
        let same = |a: (u16, u16), c: (u16, u16)| {
            usize::from(a.0) * 16 + usize::from(a.1) == usize::from(c.0) * 16 + usize::from(c.1)
        };
        let jump =
            |from: usize, rel: u8| from.checked_add_signed(isize::from(i8::from_le_bytes([rel])));
        let mut p = self.start + if b[self.start] == 0xBE { 3 } else { 5 };
        let (mut song, mut effect, mut ax, mut asked) = (true, None, Ax::Unknown, false);
        for _ in 0..64 {
            let w = b.get(p..p + 5).ok_or_else(no)?;
            match w[0] {
                0xA1 => {
                    let word = u16::from_le_bytes([w[1], w[2]]);
                    if seen.location_word.is_some_and(|l| l != word) || asked {
                        return Err(no());
                    }
                    seen.location_word = Some(word);
                    ax = Ax::Location;
                    p += 3;
                }
                0x9A => {
                    let target = (
                        u16::from_le_bytes([w[3], w[4]]),
                        u16::from_le_bytes([w[1], w[2]]),
                    );
                    if same(target, map.place_pair) {
                        ax = Ax::Pair;
                        asked = true;
                    } else if same(target, map.entry_reveal) {
                        effect = Some(EntryEffect::Reveal);
                    } else if same(target, map.entry_slide) {
                        effect = Some(EntryEffect::Slide);
                    } else {
                        return Ok(EntryLook { effect, song });
                    }
                    p += 5;
                }
                0x3D if w[2] == 0 && matches!(w[3], 0x74 | 0x75) => {
                    let k = u16::from(w[1]);
                    let equal = match ax {
                        Ax::Location => {
                            seen.locations.push(k);
                            location == Some(k)
                        }
                        Ax::Pair => {
                            seen.pairs.push(k);
                            pair == k
                        }
                        Ax::Unknown => return Err(no()),
                    };
                    p = if (w[3] == 0x74) == equal {
                        jump(p + 5, w[4]).ok_or_else(no)?
                    } else {
                        p + 5
                    };
                }
                0x33 if w[1] == 0xF6 => {
                    song = false;
                    p += 2;
                }
                0xC7 if w == [0xC7, 0x46, 0xFE, 0x00, 0x00] => {
                    song = false;
                    p += 5;
                }
                0xEB => p = jump(p + 2, w[1]).ok_or_else(no)?,
                _ => return Err(no()),
            }
        }
        Err(no())
    }
}

/// The room's song starts as `push -1 ; push word [song+2] ; push word
/// [song]` (`6A FF FF 36`), and a build that fades it in pushes the fade's
/// milliseconds first (`68 lo hi`). *Dunkle Schatten*'s entry pushes
/// nothing before its `push -1` at `0x0ECAD`, nor does *Bi-Fi 2*'s
/// `09B6h:00BBh` (`0x09C1B`); *Captain Zins*'s `0A01h:00FFh` (`0x0A10F`)
/// pushes `1770h`, 6000 ms.
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
/// `0EB9h:1917h` (`0x104A7`) has the stores and the polls; *Captain
/// Zins*'s `0A01h:18C1h` (`0x0B8D1`) and *Bi-Fi 2*'s `09B6h:1837h`
/// (`0x0B397`) have neither.
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

/// An object's state answers for kind 4 either with `and ax, 3 ; cmp ax,
/// 1` (`25 03 00 3D 01 00`) and then `jne` (`75`, live when the bits are 1)
/// or `je` (`74`, live when they are not) — *Dunkle Schatten*'s
/// `14DDh:07E7h` has `jne` at `0x1560A`, *Captain Zins*'s `1021h:07B0h`
/// (`0x109C0`) `je` — or, after a compare of the kind
/// with 4 alone (`26 80 3F 04 74`), with `and ax, 2 ; leave ; retf`
/// (`25 02 00 C9 CB`), *Bi-Fi 2*'s at `0x0FBFF`.
fn object_state(img: &Image, at: (u16, u16)) -> Result<ObjectState> {
    let b = body(img, at, 0x64, "the object state")?;
    if let Some(i) = b
        .windows(6)
        .position(|w| w == [0x25, 0x03, 0x00, 0x3D, 0x01, 0x00])
    {
        return match b.get(i + 6) {
            Some(0x75) => Ok(ObjectState::Modes {
                when_mode_one: true,
            }),
            Some(0x74) => Ok(ObjectState::Modes {
                when_mode_one: false,
            }),
            _ => Err(refused("the object state", at)),
        };
    }
    let kind = b
        .windows(5)
        .position(|w| w == [0x26, 0x80, 0x3F, 0x04, 0x74]);
    let bit = b
        .windows(5)
        .position(|w| w == [0x25, 0x02, 0x00, 0xC9, 0xCB]);
    match (kind, bit) {
        (Some(k), Some(t)) if k < t => Ok(ObjectState::Bit1),
        _ => Err(refused("the object state", at)),
    }
}

/// A walk to a node ends `add sp, 8` (`83 C4 08`) after the walk's far
/// call, and then either `or ax, ax ; je ; mov [m16], si ; pop si ; pop
/// bp ; retf` or `pop bp ; retf` at once. *Dunkle Schatten*'s
/// `14DDh:0A56h` stores at `0x15840`; *Captain Zins*'s `1021h:09FDh`
/// (`0x10C0D`) returns at once. *Bi-Fi 2* has no such routine.
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
/// `0x0A7EF`; *Captain Zins*'s `03E8h:1EDEh` (`0x05D5E`) and *Bi-Fi 2*'s
/// `03F7h:1CF9h` (`0x05C69`) go on to the live bit.
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
/// `0x0A843`, for kind 4; *Captain Zins*'s (`0x05D5E`) and *Bi-Fi 2*'s
/// (`0x05C69`) wake it at once.
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
/// *Dunkle Schatten*'s chain at `0x154A5` names 1 and 5; *Captain Zins*'s
/// `1021h:0653h` (`0x10863`) 1, 4 and 5; *Bi-Fi 2*'s `0F48h:0611h`
/// (`0x0FA91`) 1 and 5.
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
/// *Dunkle Schatten*'s test is at `0x0FB69`; *Captain Zins*'s
/// `0A01h:1006h` (`0x0B016`) and *Bi-Fi 2*'s `09B6h:0FB3h` (`0x0AB13`)
/// make the next far call.
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
    verb6_arm: bool,
    verb6_refreshes: bool,
    verb_bound: u16,
    locked_stops_hover_text: bool,
}

impl Dispatch {
    /// The dispatcher compares the last hover with the hotspot under the
    /// pointer as `mov ax, [last+2] ; mov dx, [last] ; cmp ax, [cur+2] ;
    /// jne ; cmp dx, [cur] ; jne ; jmp` — in *Dunkle Schatten* at `0x0FDE1`
    /// with `[28A5h]`/`[28A3h]` and `[1862h]`/`[1860h]` — once in the body,
    /// as in *Captain Zins*'s `0A01h:1006h` (`0x0B016`) and *Bi-Fi 2*'s
    /// `09B6h:0FB3h` (`0x0AB13`).
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
        // The sixth verb's arm: `cmp si, 6 ; je` (`83 FE 06 74 r`), into
        // `mov word [two], 3 ; mov word [mode], 5` and, in a build that
        // refreshes the line, `mov word [age], 0Ah` (`C7 06 lo hi 0A 00`)
        // — *Dunkle Schatten*'s at `0x0FC32`, `0x0FC4C`. *Bi-Fi 2*'s
        // dispatcher has none.
        let arms: Vec<usize> = b
            .windows(5)
            .enumerate()
            .filter(|(_, x)| x[..4] == [0x83, 0xFE, 0x06, 0x74])
            .filter_map(|(i, x)| (i + 5).checked_add_signed(isize::from(i8::from_le_bytes([x[4]]))))
            .collect();
        let verb6_arm = !arms.is_empty();
        let verb6_refreshes = match arms.first() {
            None => false,
            Some(&t) => {
                let arm = b
                    .get(t..t + 0x20)
                    .ok_or_else(|| refused("the dispatcher", at))?;
                if arm.get(..2) != Some(&[0xC7, 0x06][..]) {
                    return Err(refused("the dispatcher", at));
                }
                arm.windows(6)
                    .take(0x18)
                    .any(|x| x[..2] == [0xC7, 0x06] && x[4..] == [0x0A, 0x00])
            }
        };
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
            verb6_arm,
            verb6_refreshes,
            verb_bound,
        })
    }
}

/// The mode reset forgets the hover with `mov word [last+2], FFFFh`, or
/// with `mov word [last+2], 0 ; mov word [last], 0` in a build whose
/// dispatcher forgets it as 0:0 too ([`Dispatch`]'s `forgotten_is_nothing`),
/// or does not write it at all. *Dunkle Schatten*'s `0EB9h:003Fh` writes
/// `FFFFh` at `0x0EC35`; *Bi-Fi 2*'s `09B6h:0042h` 0:0 at `0x09C08`;
/// *Captain Zins*'s `0A01h:0092h` (`0x0A0A2`) does not write it.
fn mode_reset_forgets_hover(
    img: &Image,
    at: (u16, u16),
    last: (u16, u16),
    forgotten_is_nothing: bool,
) -> Result<bool> {
    let b = body(img, at, 0x80, "the mode reset")?;
    let [h0, h1] = last.0.to_le_bytes();
    let [l0, l1] = last.1.to_le_bytes();
    if b.windows(6).any(|x| x == [0xC7, 0x06, h0, h1, 0xFF, 0xFF]) {
        return Ok(true);
    }
    let zero = b
        .windows(12)
        .any(|x| x == [0xC7, 0x06, h0, h1, 0, 0, 0xC7, 0x06, l0, l1, 0, 0]);
    match (zero, forgotten_is_nothing) {
        (false, _) => Ok(false),
        (true, true) => Ok(true),
        (true, false) => Err(refused("the mode reset", at)),
    }
}

/// The rows apart by kind are read in three routines, and they must
/// agree.
///
/// Each routine points `es:bx` at the row (`mov bx, [m16] ; imul bx, bx,
/// n ; mov ax, seg ; mov es, ax`, `8B 1E lo hi 6B DB n B8 lo hi 8E C0`)
/// and then either goes on without its kind — the build has no such rows
/// — or reads the kind (`mov ax, es:[bx+7]`, `26 8B 87 07 00`) and tests
/// it with a chain of `cmp ax, k ; je|jne rel8` (`3D k 00 74|75 rel`):
///
/// - The room loader, after the far call to the pair routine (`9A off seg
///   8B D0`, `mov dx, ax`): a kind whose arm is `mov si, n` (`BE n`) shares
///   slot `n`; one whose arm is `or dx, dx ; je ; mov dx, FFFFh` (`0B D2
///   74 rel BA FF FF`) keeps the sound. The chain ends at `mov si,
///   [location]` (`8B 36`), followed in some builds by `xor dx, dx` (`33
///   D2`), the pair's answer dropped for every other kind. *Dunkle
///   Schatten*'s has `cmp ax, 3` at `0x077F2`, *Bi-Fi 2*'s `2`, `4`
///   and `7` from `0x02EE2`.
/// - The unload, after it has compared the two places' kinds (`26 3B 87
///   07 00`): an arm `mov si, n` (`BE n`) keeps the room for `n` above
///   zero and the sound for `n` below. Only the sign is read after it
///   (`or si, si`). The chain ends at `or si, si ; jnl ; mov di, n` (`0B F6
///   7D 05 BF n`), the flags for the kept sound. *Dunkle Schatten*'s at
///   `0x083F6`, *Bi-Fi 2*'s from `0x03B14`.
/// - The pair routine, after its own compare (`26 3B 87 07 00`): `cmp ax,
///   k ; je` for each kind but the last, which is `cmp ax, k ; jne`.
///   *Dunkle Schatten*'s `0x1571B`, *Bi-Fi 2*'s `0x0FD05`.
///
/// The loader's kinds and the unload's must be the same, each in the
/// same role, and each one the pair routine answers; a kind the pair
/// routine answers that the loader does not name must be dropped by it.
/// *Captain Zins*'s three read no kind.
fn shared_rooms(img: &Image, map: &LibraryMap) -> Result<SharedRooms> {
    let (loader, zeroes) = loader_kinds(img, map)?;
    let (unload, sound_flags) = unload_kinds(img, map.slot_unload)?;
    let pairs = pair_kinds(img, map.place_pair)?;
    let refused_loader = || refused("the room loader", map.room_loader);
    let same_roles = loader.len() == unload.len()
        && loader
            .iter()
            .zip(&unload)
            .all(|(&(k, slot), &(u, room))| k == u && slot.is_some() == room);
    let answered = loader.iter().all(|(k, _)| pairs.contains(k));
    let dropped = zeroes || pairs.iter().all(|p| loader.iter().any(|(k, _)| k == p));
    if !same_roles || !answered || !dropped || pairs.len() > 3 {
        return Err(refused_loader());
    }
    let mut rooms = SharedRooms {
        sound_flags,
        exits: map_exits(img, map)?,
        ..SharedRooms::default()
    };
    for (slot, &p) in rooms.pairs.iter_mut().zip(&pairs) {
        *slot = Some(p);
    }
    let mut shared = loader.iter().filter_map(|&(k, s)| s.map(|s| (k, s)));
    for slot in &mut rooms.shared {
        *slot = shared.next();
    }
    if shared.next().is_some() {
        return Err(refused_loader());
    }
    let mut sound = loader.iter().filter(|(_, s)| s.is_none()).map(|&(k, _)| k);
    rooms.sound_kept = sound.next();
    if sound.next().is_some() {
        return Err(refused_loader());
    }
    Ok(rooms)
}

/// `mov ax, es:[bx+7]` (`26 8B 87 07 00`): the row's kind read.
const KIND_READ: [u8; 5] = [0x26, 0x8B, 0x87, 0x07, 0x00];
/// `cmp ax, es:[bx+7]` (`26 3B 87 07 00`): the two places' kinds compared.
const KIND_CMP: [u8; 5] = [0x26, 0x3B, 0x87, 0x07, 0x00];

fn word_at(b: &[u8], i: usize) -> Option<u16> {
    b.get(i..i + 2).map(|w| u16::from_le_bytes([w[0], w[1]]))
}

fn find_from(b: &[u8], from: usize, pat: &[u8]) -> Option<usize> {
    b.get(from..)
        .and_then(|t| t.windows(pat.len()).position(|w| w == pat))
        .map(|i| from + i)
}

/// `es:bx` at the row from `p`, and the kind read after it: where the
/// chain starts, or `None` when the routine reads no kind.
fn row_kind(b: &[u8], p: usize) -> Option<Option<usize>> {
    let w = b.get(p..p + 12)?;
    let setup = w[..2] == [0x8B, 0x1E] && w[4..6] == [0x6B, 0xDB] && w[7] == 0xB8;
    if !setup || w[10..] != [0x8E, 0xC0] {
        return None;
    }
    Some((b.get(p + 12..p + 17) == Some(&KIND_READ[..])).then_some(p + 17))
}

/// One link of a chain: the kind, where its arm starts and where the next
/// test does.
fn kind_link(b: &[u8], p: usize) -> Option<(u16, usize, usize)> {
    let w = b.get(p..p + 5)?;
    if w[0] != 0x3D || w[2] != 0 || !matches!(w[3], 0x74 | 0x75) {
        return None;
    }
    let next = p + 5;
    let target = next.checked_add_signed(isize::from(i8::from_le_bytes([w[4]])))?;
    Some(if w[3] == 0x74 {
        (u16::from(w[1]), target, next)
    } else {
        (u16::from(w[1]), next, target)
    })
}

/// The loader's kinds, each with its shared slot or `None` for the kind
/// that keeps the sound, and whether it drops the pair's answer for every
/// other kind.
/// A loader's kinds: each with its shared slot, or `None` for the kind
/// that keeps the sound.
type LoaderKinds = Vec<(u16, Option<u16>)>;

fn loader_kinds(img: &Image, map: &LibraryMap) -> Result<(LoaderKinds, bool)> {
    let refused_loader = || refused("the room loader", map.room_loader);
    let b = body(img, map.room_loader, 0x200, "the room loader")?;
    let [o0, o1] = map.place_pair.1.to_le_bytes();
    let [s0, s1] = map.place_pair.0.to_le_bytes();
    let mut loader = Vec::new();
    let call = find_from(b, 0, &[0x9A, o0, o1, s0, s1, 0x8B, 0xD0]).ok_or_else(refused_loader)?;
    let chain = row_kind(b, call + 7).ok_or_else(refused_loader)?;
    let mut p = chain.unwrap_or(call + 19);
    if chain.is_some() {
        while let Some((k, arm, next)) = kind_link(b, p) {
            let a = b.get(arm..arm + 7).ok_or_else(refused_loader)?;
            if a[0] == 0xBE {
                loader.push((k, Some(u16::from_le_bytes([a[1], a[2]]))));
            } else if a[..3] == [0x0B, 0xD2, 0x74] && a[4..] == [0xBA, 0xFF, 0xFF] {
                loader.push((k, None));
            } else {
                return Err(refused_loader());
            }
            p = next;
        }
    }
    if b.get(p..p + 2) != Some(&[0x8B, 0x36][..]) {
        return Err(refused_loader());
    }
    Ok((loader, b.get(p + 4..p + 6) == Some(&[0x33, 0xD2][..])))
}

/// The unload's kinds, each with whether it keeps the room (else the
/// sound), and the flags it works by for the kept sound.
fn unload_kinds(img: &Image, at: (u16, u16)) -> Result<(Vec<(u16, bool)>, u16)> {
    let refused_unload = || refused("the unload", at);
    let b = body(img, at, 0xC0, "the unload")?;
    let mut unload = Vec::new();
    let cmp = find_from(b, 0, &KIND_CMP).ok_or_else(refused_unload)?;
    if b.get(cmp + 5) != Some(&0x75) {
        return Err(refused_unload());
    }
    let chain = row_kind(b, cmp + 7).ok_or_else(refused_unload)?;
    let mut p = chain.unwrap_or(cmp + 19);
    while let Some((k, arm, next)) = chain.and_then(|_| kind_link(b, p)) {
        let n = match b.get(arm) {
            Some(0xBE) => word_at(b, arm + 1).ok_or_else(refused_unload)?,
            _ => return Err(refused_unload()),
        };
        unload.push((k, crate::num::signed(n) > 0));
        p = next;
    }
    if b.get(p..p + 2) != Some(&[0x0B, 0xF6][..]) {
        return Err(refused_unload());
    }
    let sound_flags =
        find_from(b, p, &[0x0B, 0xF6, 0x7D, 0x05, 0xBF]).and_then(|i| word_at(b, i + 5));
    if sound_flags.is_none() && unload.iter().any(|&(_, room)| !room) {
        return Err(refused_unload());
    }
    Ok((unload, sound_flags.unwrap_or(0)))
}

/// The kinds the pair routine answers.
fn pair_kinds(img: &Image, at: (u16, u16)) -> Result<Vec<u16>> {
    let refused_pair = || refused("the pair's kind", at);
    let b = body(img, at, 0xB0, "the pair's kind")?;
    let mut pairs = Vec::new();
    let cmp = find_from(b, 0, &KIND_CMP).ok_or_else(refused_pair)?;
    if b.get(cmp + 5) != Some(&0x75) {
        return Err(refused_pair());
    }
    if let Some(mut p) = row_kind(b, cmp + 7).ok_or_else(refused_pair)? {
        while let Some(w) = b.get(p..p + 4) {
            if w[0] != 0x3D || w[2] != 0 {
                break;
            }
            pairs.push(u16::from(w[1]));
            match w[3] {
                0x74 => p += 5,
                0x75 => break,
                _ => return Err(refused_pair()),
            }
        }
    }
    Ok(pairs)
}

/// The loader writes a row's exits into its map for the rows of one kind:
/// `cmp word es:[bx+7], k ; je +3 ; jmp` (`26 83 BF 07 00 k 74 03 E9`),
/// then reads each exit byte (`mov al, es:[bx+n]`, `26 8A 87 n`) and,
/// after `cmp word es:[bx+2], 0 ; jne` (`26 83 7F 02 00 75 rel`), fills
/// the entries one of two ways, told by the `jne`'s target: `mov ax, cx
/// ; add ax, 4 ; imul ax, ax, 1Ch ; les bx, [bp-2Ch] ; add bx, ax ; mov
/// word es:[bx+2], 0` ([`ExitForm::Cleared`]), or `mov ax, cx ; imul ax,
/// ax, 1Ch ; les bx, [bp-2Ch] ; add bx, ax ; mov ax, es:[bx+2] ; add ax,
/// n` ([`ExitForm::Offset`]). *Dunkle Schatten*'s test is at `0x07CA7`,
/// *Bi-Fi 2*'s at `0x033A6`; *Captain Zins*'s loader `02DBh:010Ah`
/// (`0x02EBA`) writes no exits.
fn map_exits(img: &Image, map: &LibraryMap) -> Result<Option<MapExits>> {
    let refused_loader = || refused("the room loader", map.room_loader);
    let b = body(img, map.room_loader, 0x800, "the room loader")?;
    let Some(t) = b
        .windows(9)
        .position(|w| w[..5] == [0x26, 0x83, 0xBF, 0x07, 0x00] && w[6..] == [0x74, 0x03, 0xE9])
    else {
        return Ok(None);
    };
    let kind = u16::from(b[t + 5]);
    let tail = &b[t..];
    let read = tail
        .windows(3)
        .position(|w| w == [0x26, 0x8A, 0x87])
        .ok_or_else(refused_loader)?;
    let first = u16::from_le_bytes([tail[read + 3], tail[read + 4]]);
    let test = tail
        .windows(6)
        .position(|w| w[..5] == [0x26, 0x83, 0x7F, 0x02, 0x00] && w[5] == 0x75)
        .ok_or_else(refused_loader)?;
    let rel = isize::from(i8::from_le_bytes([tail[test + 6]]));
    let at = (test + 7)
        .checked_add_signed(rel)
        .ok_or_else(refused_loader)?;
    let arm = tail.get(at..at + 19).ok_or_else(refused_loader)?;
    const CLEARED: [u8; 19] = [
        0x8B, 0xC1, 0x05, 0x04, 0x00, 0x6B, 0xC0, 0x1C, 0xC4, 0x5E, 0xD4, 0x03, 0xD8, 0x26, 0xC7,
        0x47, 0x02, 0x00, 0x00,
    ];
    const OFFSET: [u8; 15] = [
        0x8B, 0xC1, 0x6B, 0xC0, 0x1C, 0xC4, 0x5E, 0xD4, 0x03, 0xD8, 0x26, 0x8B, 0x47, 0x02, 0x05,
    ];
    let form = if arm == CLEARED {
        ExitForm::Cleared
    } else if arm[..15] == OFFSET {
        ExitForm::Offset(u16::from_le_bytes([arm[15], arm[16]]))
    } else {
        return Err(refused_loader());
    };
    Ok(Some(MapExits { kind, first, form }))
}

/// The entry's object loop ends `cmp di|si, n ; jl` (`83 FF|FE n 7C`);
/// then either a slot is written as a point — `push y ; push x ; push
/// code ; push slot ; call far` (`6A y 68 x 6A code 6A slot 9A`) — or the
/// entry goes on to its records (`C4 5E`). *Dunkle Schatten* writes slot
/// 10 at `0x0EE58`; *Captain Zins*'s `0A01h:023Ch` (`0x0A24C`) and *Bi-Fi
/// 2*'s `09B6h:0212h` (`0x09D72`) go on to their records.
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

/// The entry bounds the entry id, `cmp si|di, a ; jl ; cmp si|di, b ; jg`
/// (`83 FE|FF a 7C rel 83 FE|FF b 7F rel`), tests the door's object for its
/// kind, `cmp byte es:[bx], k ; jne` (`26 80 3F k 75`), and then — at once
/// or after a `jmp` (`EB`) — either remembers it, `mov [m16], si` (`89
/// 36`), and shows it on `push f`, or shows it at once (`6A f`). Past `b`
/// a build may test single ids, `mov ax, si ; cmp ax, n ; je` (`8B C6`,
/// then `3D n 00 74 rel` each), into the same two arms. *Dunkle
/// Schatten*'s bounds are at `0x0EF0F`, it remembers at `0x0EF22` and
/// shows frame 1; *Captain Zins*'s `0A01h:023Ch` (`0x0A24C`) bounds 100 to
/// 109 too and shows the door at once, hidden; *Bi-Fi 2*'s `09B6h:0212h`
/// (`0x09D72`) bounds 110 to 119, and its single ids are at `0x09EEE`.
fn entry_door(img: &Image, at: (u16, u16)) -> Result<EntryDoor> {
    let no = || refused("the entry", at);
    let b = body(img, at, 0x220, "the entry")?;
    let bounds = b
        .windows(10)
        .position(|w| {
            w[0] == 0x83
                && matches!(w[1], 0xFE | 0xFF)
                && w[3] == 0x7C
                && w[5] == 0x83
                && w[6] == w[1]
                && w[8] == 0x7F
        })
        .ok_or_else(no)?;
    let ids = (u16::from(b[bounds + 2]), u16::from(b[bounds + 7]));
    let past = (bounds + 10)
        .checked_add_signed(isize::from(i8::from_le_bytes([b[bounds + 9]])))
        .ok_or_else(no)?;
    let i = bounds
        + b[bounds..]
            .windows(5)
            .position(|w| w[..3] == [0x26, 0x80, 0x3F] && w[4] == 0x75)
            .ok_or_else(no)?;
    let kind = b[i + 3];
    let mut arm = i + 6;
    if b.get(arm) == Some(&0xEB) {
        arm = (arm + 2)
            .checked_add_signed(isize::from(i8::from_le_bytes([b[arm + 1]])))
            .ok_or_else(no)?;
    }
    // One arm: whether it remembers and the frame it shows.
    let arm_at = |p: usize| -> Option<(bool, u8)> {
        let t = b.get(p..p + 6)?;
        if t[..2] == [0x89, 0x36] && t[4] == 0x6A {
            Some((true, t[5]))
        } else if t[0] == 0x6A {
            Some((false, t[1]))
        } else {
            None
        }
    };
    let (remembered, f) = arm_at(arm).ok_or_else(no)?;
    let mut more = [None; 3];
    if b.get(past..past + 2) == Some(&[0x8B, 0xC6][..]) {
        let mut p = past + 2;
        let mut n = 0;
        while let Some(w) = b
            .get(p..p + 5)
            .filter(|w| w[0] == 0x3D && w[2] == 0 && w[3] == 0x74)
        {
            let target = (p + 5)
                .checked_add_signed(isize::from(i8::from_le_bytes([w[4]])))
                .ok_or_else(no)?;
            // Into the arm the doors take, or past its store into the show.
            let remembers = if target == arm && remembered {
                true
            } else if remembered && target == arm + 4 {
                false
            } else {
                return Err(no());
            };
            *more.get_mut(n).ok_or_else(no)? = Some((u16::from(w[1]), remembers));
            n += 1;
            p += 5;
        }
    }
    Ok(EntryDoor {
        ids,
        kind,
        remembered,
        frame: i16::from(i8::from_le_bytes([f])),
        more,
    })
}

/// The painter holds the band dark with `push word [m16] ; push word
/// [m16] ; push h ; push 140h ; push y ; push 0 ; call far`
/// (`FF 36 … FF 36 … 6A h 68 40 01 68 lo hi 6A 00 9A`), `h` rows from
/// `y` the whole width; it sets each glyph of the line on the same row,
/// `push ax ; push y ; push si ; call far` (`50 68 lo hi 56 9A`) — ink,
/// row, column. *Dunkle Schatten*'s `0C3Fh:005Ch` holds 12 rows from 80h
/// (`0x0C89C`, `0x0C7BE`); *Captain Zins*'s `078Ah:0069h` 12 from 82h
/// (`0x07D59`, `0x07C7B`); *Bi-Fi 2*'s `0751h:0067h` 48h from 80h
/// (`0x07987`, `0x078B0`), the strip below the line too. A painter whose
/// two rows differ is refused. Answers the row and the band's height.
fn status_band(img: &Image, at: (u16, u16)) -> Result<(i32, i32)> {
    let what = "the status painter";
    let b = body(img, at, 0x466, what)?;
    let row = b
        .windows(19)
        .find(|w| {
            w[..2] == [0xFF, 0x36]
                && w[4..6] == [0xFF, 0x36]
                && w[8] == 0x6A
                && w[10..13] == [0x68, 0x40, 0x01]
                && w[13] == 0x68
                && w[16..19] == [0x6A, 0x00, 0x9A]
        })
        .map(|w| (w[9], [w[14], w[15]]))
        .ok_or_else(|| refused(what, at))?;
    let (height, row) = row;
    let glyph = [0x50, 0x68, row[0], row[1], 0x56, 0x9A];
    if !b.windows(6).any(|w| w == glyph) {
        return Err(refused(what, at));
    }
    Ok((i32::from(u16::from_le_bytes(row)), i32::from(height)))
}

/// The band under a choice menu, as the menu's strip redraw leaves it: the
/// line's rows filled with color 0 and, from the same row, the band held
/// dark, the menu's lines set over it afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MenuBand {
    /// The row both start at.
    pub row: i32,
    /// How many rows are filled with color 0.
    pub cleared: i32,
    /// How many rows are held dark.
    pub dark: i32,
}

/// The choice menu's strip redraw is a `push si ; push di` routine that
/// ends `mov word [m16], FFFFh ; pop di ; pop si ; retf` (`C7 06 lo hi FF
/// FF 5F 5E CB`). It draws the panel and the items and either ends there
/// — *Dunkle Schatten*'s `0C3Fh:04D2h` (`0x0C8C2`–`0x0C970`), *Captain
/// Zins*'s `078Ah:04DFh` (`0x07D7F`–`0x07E2D`) — or, after the items, fills
/// the line's rows with color 0, `push 0 ; push h ; push 140h ; push y ;
/// push 0 ; call far` (`6A 00 6A h 68 40 01 68 lo hi 6A 00 9A`, the fill
/// `1B72h:00D2h`), and holds the band dark the way the status painter does
/// ([`status_band`]'s `FF 36 … 6A h 68 40 01 68 lo hi 6A 00 9A`): *Bi-Fi
/// 2*'s `0751h:04A2h` fills 0Ch rows from 80h (`0x07A62`) and holds 48h
/// from 80h dark (`0x07A7C`, `0DE4h:0123h`) with no test before either.
/// A fill without the hold, or the two from different rows, is refused.
fn menu_band(img: &Image, at: (u16, u16)) -> Result<Option<MenuBand>> {
    let what = "the menu's strip redraw";
    let b = body(img, at, 0x100, what)?;
    let end = b
        .windows(9)
        .position(|w| is_store_ffff(w) && w[6..9] == [0x5F, 0x5E, 0xCB])
        .ok_or_else(|| refused(what, at))?;
    if b[..2] != [0x56, 0x57] {
        return Err(refused(what, at));
    }
    let b = &b[..end];
    let rows = |w: &[u8]| (i32::from(w[0]), i32::from(u16::from_le_bytes([w[5], w[6]])));
    let cleared = b
        .windows(13)
        .find(|w| {
            w[..3] == [0x6A, 0x00, 0x6A]
                && w[4..8] == [0x68, 0x40, 0x01, 0x68]
                && w[10..13] == [0x6A, 0x00, 0x9A]
        })
        .map(|w| rows(&w[3..]));
    let dark = b
        .windows(19)
        .find(|w| {
            w[..2] == [0xFF, 0x36]
                && w[4..6] == [0xFF, 0x36]
                && w[8] == 0x6A
                && w[10..14] == [0x68, 0x40, 0x01, 0x68]
                && w[16..19] == [0x6A, 0x00, 0x9A]
        })
        .map(|w| rows(&w[9..]));
    match (cleared, dark) {
        (None, None) => Ok(None),
        (Some((cleared, a)), Some((dark, row))) if a == row => {
            Ok(Some(MenuBand { row, cleared, dark }))
        }
        _ => Err(refused(what, at)),
    }
}

/// The status line's inks: the one it starts in, and what each of the
/// painter's controls switches to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatusInks {
    /// The ink the line starts in.
    pub ink: u8,
    /// `(control, ink)`: a control byte of the line, drawn as nothing, and
    /// the ink the glyphs after it are set in.
    pub controls: [(u8, u8); 3],
}

/// The status painter sets the font record's ink before the line, `mov
/// byte es:[bx+4], ink` (`26 C6 47 04 ink`), and looks each byte up in a
/// table in its own code segment, `mov cx, 4 ; mov bx, table ; mov ax,
/// cs:[bx]` (`B9 04 00 BB lo hi 2E 8B 07`): four codes, then the four arms
/// `jmp word cs:[bx+8]` takes. Code 0 ends the line; each of the other
/// three arms is `les bx, [m16] ; mov byte es:[bx+4], ink` (`C4 1E lo hi
/// 26 C6 47 04 ink`). *Dunkle Schatten*'s `0C3Fh:005Ch` starts in `0F4h`
/// (`0x0C750`), its table at `cs:04C2h` (`0x0C8B2`) sets `0FAh` for 09h
/// (`0x0C78C`), `0FCh` for 0Ah (`0x0C797`) and `0F4h` for 0Dh (`0x0C7A2`);
/// *Captain Zins*'s `078Ah:0069h` the same inks (`0x07C0D`, table at
/// `0x07D6F`); *Bi-Fi 2*'s `0751h:0067h` starts in 74h (`0x07843`), and its
/// table at `cs:0492h` (`0x079A2`) sets 7Ah, 7Ch and 74h (`0x0787F`,
/// `0x0788A`, `0x07895`) — the lower half, which the band held dark turns
/// into the upper.
fn status_inks(img: &Image, at: (u16, u16)) -> Result<StatusInks> {
    let what = "the status painter's inks";
    let no = || refused(what, at);
    let b = body(img, at, 0x466, what)?;
    let ink_at = |w: &[u8]| (w[..4] == [0x26, 0xC6, 0x47, 0x04]).then_some(w[4]);
    let ink = b.windows(5).find_map(ink_at).ok_or_else(no)?;
    let table = b
        .windows(9)
        .find(|w| w[..4] == [0xB9, 0x04, 0x00, 0xBB] && w[6..9] == [0x2E, 0x8B, 0x07])
        .map(|w| u16::from_le_bytes([w[4], w[5]]))
        .ok_or_else(no)?;
    let t = body(img, (at.0, table), 16, what)?;
    let word = |k: usize| u16::from_le_bytes([t[2 * k], t[2 * k + 1]]);
    let mut controls = Vec::new();
    for k in 0..4 {
        let code = word(k);
        if code == 0 {
            continue;
        }
        let arm = body(img, (at.0, word(k + 4)), 9, what)?;
        if arm[..2] != [0xC4, 0x1E] {
            return Err(no());
        }
        let to = ink_at(&arm[4..]).ok_or_else(no)?;
        controls.push((u8::try_from(code).map_err(|_| no())?, to));
    }
    Ok(StatusInks {
        ink,
        controls: controls.try_into().map_err(|_| no())?,
    })
}

/// Taking an item out compares the first visible slot plus the window
/// with the count, `add ax, n` (`05 n 00`), the first such add in the
/// routine. *Dunkle Schatten*'s `0C3Fh:09E1h` adds 8 at `0x0CE21`,
/// *Captain Zins*'s `078Ah:09E6h` (`0x08286`) 6, *Bi-Fi 2*'s `0751h:099Dh`
/// (`0x07EAD`) 8.
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
/// `0EB9h:18A4h` (`0x10434`) have the stores; *Captain Zins*'s
/// (`0x0B86E`, `0x0B87C`) and *Bi-Fi 2*'s (`0x0B334`, `0x0B342`) make the
/// call at once.
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
///   `0EB9h:0001h` and *Bi-Fi 2*'s `09B6h:0004h` keep none; *Captain
///   Zins*'s `0A01h:003Bh` (`0x0A04B`) puts `[1980h]` back to
///   `0A01h:0001h`.
/// - The entry asks through it in a loop: `cmp word [bp+6], loc ; je ;
///   mov si, first ; jmp ; push si ; call far [w]` (`83 7E 06 loc 74 rel
///   BE first 00 EB rel 56 FF 1E w`), closed by `inc si ; cmp si, last ;
///   jle` (`46 83 FE last 7E`). *Captain Zins*'s asks about ids 12h to
///   19h everywhere but location 6.
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
///   style with `or word [bx+m16], s` (`83 8F m16 s`) — or not at all:
///   *Bi-Fi 2*'s `080Ch:02A4h` goes from the vertical field's arms to
///   the record's point (`0x084F4`). *Captain Zins*'s `0849h:02A1h`
///   (`0x08731`) recodes both.
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
    let Some(h) = b.windows(13).position(|w| {
        w[..3] == [0x25, 0xC0, 0x00]
            && w[3] == 0x3D
            && w[5] == 0
            && w[6] == 0x74
            && w[8] == 0x3D
            && w[10] == 0
            && w[11] == 0x74
    }) else {
        // The switch's miss, `loop ; jmp` (`E2 F3 EB rel`), goes where
        // every arm goes; with no horizontal field that is the record's
        // point, `mov bx, [bp+4]` (`8B 5E 04`), not the field's `mov bx,
        // [bp+6]`.
        let miss = sw
            + b[sw..]
                .windows(3)
                .position(|w| w == [0xE2, 0xF3, 0xEB])
                .ok_or_else(|| refused(what, at))?;
        let join = (miss + 4)
            .checked_add_signed(isize::from(i8::from_le_bytes([b[miss + 3]])))
            .ok_or_else(|| refused(what, at))?;
        if b.get(join..join + 3) != Some(&[0x8B, 0x5E, 0x04][..]) {
            return Err(refused(what, at));
        }
        return Ok(Some(SlotStyle {
            vertical,
            horizontal: None,
        }));
    };
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
        horizontal: Some(horizontal),
    }))
}

/// A room's effects: the routine bounds the room slot with `cmp ax, b ;
/// jge` (`3D b 00 7D`), indexes the masks with `shl bx, 1` (`D1 E3`, a
/// word each) or `shl bx, 2` (`C1 E3 02`, a double word), starts its loop
/// at `mov word [bp-2], n` (`C7 46 FE n 00`, the slots) and ends it at
/// `cmp ax, k ; jg` (`3D k 00 7F`, the first room slot). *Dunkle
/// Schatten*'s `1458h:00F7h` has 1Ah (`0x14689`), a word, 15h
/// (`0x146A3`) and 0Ch (`0x146F5`); *Captain Zins*'s `0F9Dh:00F7h`
/// (`0x0FAC7`) 13h, a double word, 2Dh and 0Eh; *Bi-Fi 2*'s `0EC8h:00FAh`
/// (`0x0ED7A`) 28h, a double word, 30h and 0Fh.
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
/// 00 7D`). *Dunkle Schatten*'s `1197h:0035h` has 1Eh (`0x119AC`),
/// *Captain Zins*'s `0CCFh:0030h` (`0x0CD20`) 13h, *Bi-Fi 2*'s
/// `0C59h:002Eh` (`0x0C5BE`) 28h.
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
/// ; call far` (`83 3E loc k 7E 05 B8 01 00 EB 02 33 C0 50 6A 01 9A`) —
/// *Captain Zins*'s `0A01h:003Bh` (`0x0A04B`), `[1A5Ch]` against 2 — or
/// `push −2` to a loader that keeps a loaded file for a negative argument
/// ([`FigureSet::Kept`]) — *Bi-Fi 2*'s `09B6h:0004h` (`0x09B64`).
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
    let i = b
        .windows(9)
        .position(|w| w[0] == 0x6A && w[2..5] == [0x6A, 0x01, 0x9A])
        .ok_or_else(|| refused("the visit's setup", at))?;
    let n = b[i + 1];
    if n < 0x80 {
        return Ok(FigureSet::Fixed(u16::from(n)));
    }
    let w = |k: usize| u16::from_le_bytes([b[i + k], b[i + k + 1]]);
    let loader = body(img, (w(7), w(5)), 0x20, "the figure's loader")?;
    if n == 0xFE
        && loader
            .windows(5)
            .any(|x| x == [0x83, 0x7E, 0x08, 0x00, 0x7C])
    {
        Ok(FigureSet::Kept)
    } else {
        Err(refused("the visit's setup", at))
    }
}

/// Mode 8 of the mode's step, once what it waits for is over, asks the
/// gate with 0, `push 0 ; call near ; pop cx` (`6A 00 E8 lo hi 59`).
/// *Dunkle Schatten*'s `0EB9h:0965h` then sets the status line's age, `mov word
/// [28C3h], 0Ah` (`C7 06 lo hi 0A 00`, `0x0F56D`), before it looks at the
/// door that was come through; *Captain Zins*'s `0A01h:09E0h` (`0x0AA62`) and
/// *Bi-Fi 2*'s `09B6h:098Dh` (`0x0A55F`) go straight on to that door, `cmp
/// word [m16], 0` (`83 3E`), and leave the age as it is.
fn mode_step_ages_status(img: &Image, at: (u16, u16)) -> Result<bool> {
    let what = "the mode's step";
    let b = body(img, at, 0x100, what)?;
    let i = b
        .windows(6)
        .position(|w| w[..3] == [0x6A, 0x00, 0xE8] && w[5] == 0x59)
        .ok_or_else(|| refused(what, at))?;
    match b.get(i + 6..i + 12) {
        Some([0xC7, 0x06, _, _, 0x0A, 0x00]) => Ok(true),
        Some([0x83, 0x3E, ..]) => Ok(false),
        _ => Err(refused(what, at)),
    }
}

/// Where the figure's cels start in its file, read off the loader the
/// visit's setup calls (`push 1 ; call far`, `6A 01 9A`), at its tail
/// `pop si ; leave ; retf` (`5E C9 CB`).
///
/// *Dunkle Schatten*'s `109Dh:000Eh` hands the file's first far pointer to
/// the palette's routine and then steps the slot past it, `add word
/// [bx+2879h], 4` (`83 87 lo hi 04`, `0x10AD0`), and its unload steps back
/// by four before it frees (`0x10AF7`); *Captain Zins*'s loader does the
/// same (`0x0BF02`). Both count their cels from resource 1. *Bi-Fi 2*'s
/// `0B73h:0003h` ends on the store of the loaded pointer, `mov
/// [bx+2D24h], dx` (`89 97 lo hi`, `0x0B7A8`), with no palette call and no
/// step, and its unload frees the pointer as it stands (`0x0B7CA`): its
/// cels are counted from resource 0.
fn figure_first(img: &Image, at: (u16, u16)) -> Result<usize> {
    let what = "the figure's loader";
    let b = body(img, at, 0x30, "the visit's setup")?;
    let i = b
        .windows(7)
        .position(|w| w[..3] == [0x6A, 0x01, 0x9A])
        .ok_or_else(|| refused("the visit's setup", at))?;
    let w = |k: usize| u16::from_le_bytes([b[i + k], b[i + k + 1]]);
    let loader = (w(5), w(3));
    let l = body(img, loader, 0x100, what)?;
    let end = l
        .windows(3)
        .position(|x| x == [0x5E, 0xC9, 0xCB])
        .ok_or_else(|| refused(what, loader))?;
    match l.get(end.wrapping_sub(5)..end) {
        Some([0x83, 0x87, _, _, 4]) => Ok(1),
        Some([_, 0x89, 0x97, _, _]) => Ok(0),
        _ => Err(refused(what, loader)),
    }
}

/// The strip's loader either walks the carried list down from its end
/// while an item is at or above a bound, `cmp byte [bx+list], s ; jb`
/// (`80 BF list s 72`), and splits the range there — or sets the range to
/// the whole list at once, `mov byte [lo], 0` (`C6 06 lo 00`) — or takes
/// no argument and keeps no range: `enter ; push si ; push ds ; push
/// name` (`C8 lo hi 00 56 1E 68`), straight into opening the strip's
/// file. *Captain Zins*'s `0764h:0001h` (`0x07641`) does the first, at
/// 6; *Dunkle Schatten*'s `0C1Ch:000Ch` the second (`0x0C1D5`); *Bi-Fi
/// 2*'s `073Ch:0007h` the third (`0x073C7`).
fn strip_range(img: &Image, at: (u16, u16)) -> Result<StripRange> {
    let b = body(img, at, 0x30, "the strip's loader")?;
    if let Some(w) = b
        .windows(6)
        .find(|w| w[..2] == [0x80, 0xBF] && w[5] == 0x72)
    {
        return Ok(StripRange::Split(w[4]));
    }
    if b.windows(5).any(|w| w[..2] == [0xC6, 0x06] && w[4] == 0) {
        return Ok(StripRange::Whole);
    }
    if b[0] == 0xC8 && b[3] == 0 && b[4..7] == [0x56, 0x1E, 0x68] {
        return Ok(StripRange::Unranged);
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
/// `0CFDh:01BEh` (`0x0D18E`) sets `100h`, `200h` and `400h`; *Captain
/// Zins*'s `0849h:01B3h` (`0x08643`) and *Bi-Fi 2*'s `080Ch:01B6h`
/// (`0x08276`) have four controls and set 1 and 2.
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

pub(super) fn refused(what: &str, at: (u16, u16)) -> crate::error::Error {
    format!(
        "{what} at {:04X}h:{:04X}h is not a build of the library this runtime has read",
        at.0, at.1
    )
    .into()
}
