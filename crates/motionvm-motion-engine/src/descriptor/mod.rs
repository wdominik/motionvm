//! The scene graph: what the engine draws, and what it remembers about each item.
//!
//! A descriptor is the original's 0x5c-byte record. It carries one kind of
//! content — a sprite, a block, a text, or nothing — plus a place, a level, a
//! wait and a callback. The per-frame walk visits them in level order, and the
//! drawer paints onto a surface that persists: a descriptor is drawn when it
//! is marked, and switching one off marks its neighbours rather than erasing
//! it. What erases is `SDAUTOBUF` — see [`Descriptor::auto_buffer`].
//!
//! The savegame codecs live here too: the numbers they write are these enums
//! and nothing else uses them.
//!
//! The record and its enums are here; the table they sit in — the selection,
//! the handles, the frame order and the marking — is [`table`], and the
//! `SD…`/`GD…` words that change one is [`words`].

use crate::{Field, Fields};

mod table;
mod words;

/// What a coordinate on a descriptor means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// The value is the left or top edge. `NEWSETDESC` starts here.
    #[default]
    Edge,
    /// The value is the center, so the edge is it minus half the drawn size.
    ///
    /// Verified by arithmetic against the frame captured from the original:
    /// sprite 86 is 320x200, `2000 SD%SHR` makes it 640x400, and the title
    /// macro sets the center to (320, 240). Minus half the size that is
    /// exactly (0, 40) — the position the pixel comparison already agreed on.
    Center,
    /// The value is the far edge — the right one across, the bottom one down.
    ///
    /// Read from the pair of routines that work out a descriptor's rectangle,
    /// `0x6beec` across and `0x6c57c` down. Both dispatch on the same mode and
    /// differ only in which field they take:
    ///
    /// ```text
    /// mode 1:  edge   = value
    /// mode 2:  edge   = value - size/2
    /// mode 3:  edge   = value - size
    /// ```
    ///
    /// (The routines subtract a further 4 to 6 pixels, but that is the margin
    /// of the area they mark for repainting, not part of the position.)
    ///
    /// For `SDOY` down this is the figure's feet, which is what a walking
    /// character is placed by.
    FarEdge,
}

/// One entry of the engine's scene graph.
///
/// The original packs these into a 0x3E-byte struct; the field offsets that are
/// known (`+4` x, `+6` y, `+8` level) came out of the `NEWSETDESC` and `SDX`
/// handlers. Only the behavior is reproduced here, not the layout.
#[derive(Debug, Clone, Default)]
pub struct Descriptor {
    /// What `NEWSETDESC` answered with, and what `ACTDESC` selects by.
    pub handle: u32,
    /// The screen that was current when the descriptor was made.
    pub screen: u32,
    /// The horizontal coordinate; what it means is [`Descriptor::x_mode`].
    pub x: i32,
    /// The vertical coordinate; what it means is [`Descriptor::y_mode`].
    pub y: i32,
    /// `SDLEV`, `SDLV`, `SDZ`: the order the per-frame walk draws in, low
    /// first. Ties keep the order they were made in.
    pub level: i32,
    /// `SDSPR`, `SDBL`, `SDTB`: what this descriptor shows.
    ///
    /// One field, because the original has one: `+0x10`, written by all three
    /// of those words and read back by `GDSPR`, `GDBL` and `GDTB`. Giving a
    /// descriptor a sprite therefore takes its block away, and there is no
    /// state in which it has both.
    pub shows: Shows,
    /// `SDTXT`: index into a text table.
    pub text: Option<i32>,
    /// `SDFNT`: which registered font the text draws in. Unset falls back to
    /// the system font, not to whatever was registered first.
    pub font: Option<i32>,
    /// `SDCOL`: field +0x0C of the text record, composite and unmasked.
    ///
    /// A plain value rather than an option, because the original has no
    /// "unset" to model. The text record does not exist until `SDTXT`
    /// (0x71b4f) or `SDTB` (0x71d45) allocates it, and both allocate through
    /// 0x203F3 → 0x823E0, which is a *zeroing* allocator (`xor %al,%al;
    /// rep stos`). `SDTB` then writes +0x00, +0x04, +0x08, +0x10 and +0x18
    /// explicitly and leaves +0x0C to the allocator — so a text nobody gave a
    /// color is deterministically **0**, black in 54 of the 60 shipped
    /// palettes, and `GDCOL` (0x73177) answers 0 for it.
    ///
    /// Modeling it as an option invites two readers to pick different
    /// defaults — 255 for the drawer, 0 for `GDCOL` — and nothing compares
    /// them. 255 is magenta in 42 of the 60 palettes, and it is the help pages
    /// that show it: every one of their texts runs on the default, because
    /// `SHOW_DOC` contains no `SDCOL` at all and `XYLTITEM.` (module 2,
    /// 0x01364) never gives them one.
    pub color: i32,
    /// `SDTDT`: the text template, which names the outline font and the gaps.
    pub template: Option<i32>,
    /// `SDWAIT`: field +0x18, a plain count of frames.
    ///
    /// The original's per-frame walk over the descriptors (0x68c64, driven by
    /// `ANIMPLAY`) counts this down and runs [`Descriptor::callback`] when it
    /// reaches zero. Neither the walk nor the callbacks happen here yet, which
    /// is why a text shown with a wait never finishes.
    pub wait: i32,
    /// `SDWORD`: field +0x14, the address of a bytecode word.
    ///
    /// Not text word-wrapping, which is what this field was called until the
    /// handler was read: `SDWORD` stores a word address that the frame walk
    /// executes once the wait above expires, and clears the field for 0 or -1.
    /// The game uses it to end a displayed text —
    /// `_IINFO @ ACTDESC … SDACTIVE  0x53370 SDWORD  0 SDWAIT` in module 5 —
    /// and `?TEXTREADY` waits for exactly that to have happened.
    pub callback: i32,
    /// Where the level chain last put it among its equals.
    ///
    /// The 16-bit engine keeps its descriptors on a doubly linked chain
    /// sorted by level, and `SDLEV` (`05f1:1018`) always unlinks and
    /// re-inserts — `0362:2007` walks to the first node whose level is
    /// *greater* and splices in before it, so the freshest set lands
    /// **after** every equal and draws on top of them (the drawer walks the
    /// chain head first, `016a:0a4f`). This stamp is that position: paint
    /// order is `(level, stamp)`. On the 32-bit machine nothing bumps it
    /// after creation, so the order stays what it always was — creation
    /// order among equals; its chain handler is unread. Not saved: a loaded
    /// game restamps in list order, and the next `SDLEV` puts a moving
    /// figure back where the chain would have it.
    pub stamp: u64,
    /// How `x` and `y` are to be read, one mode per axis.
    ///
    /// The original keeps both in the low nibble of the word at +0x12 — bits
    /// 0-1 across, bits 2-3 down — and each setter word writes its own:
    /// `SDX`/`SDY` mean the edge, `SDCX`/`SDCY`/`SDCEN`/`SDVCEN` the center,
    /// `SDOY` an offset. `NEWSETDESC` sets both to the edge, which is why
    /// ignoring the whole thing worked until a figure came along that is placed
    /// by its center.
    pub x_mode: Placement,
    /// What [`Descriptor::y`] means: an edge, a center, or the far edge.
    pub y_mode: Placement,
    /// Everything else a `SD*` word can set, kept by name.
    ///
    /// The descriptor struct in the original is 0x3E bytes of fields whose
    /// meanings are not all known yet. Recording the rest by name keeps the
    /// values — they are needed to reproduce a frame — without inventing a
    /// meaning for each one before it has been measured.
    pub fields: Fields,
    /// `SDINSERT`: the text record's five insert slots (`+0x20`), which the
    /// text layout turns into the `#`-formatter's arguments — the engine's
    /// `layout` module. Cleared when `SDTXT` or `SDTB` first makes the
    /// descriptor a text (`0x71c03`, R78 `0x5da4e`), as the record is.
    pub inserts: [Insert; SLOTS],
    /// Whether the drawer visits it at all — bit 0x80 of the flag byte at
    /// +0x13, set and cleared only by `SDACTIVE`/`SDINACTIVE`.
    ///
    /// **Clearing it erases nothing.** `SDINACTIVE` (0x72033) marks the
    /// descriptor's rectangle on its *own* level (0x6ab6e → 0x6e701), so
    /// everything above it is repainted and nothing below is — and the
    /// descriptor itself is no longer drawn. Its pixels therefore stay on the
    /// surface until something paints over them, unless it carries a
    /// [`Descriptor::auto_buffer`].
    pub active: bool,
    /// Bit 0x40: the descriptor has to be drawn on the next pass.
    ///
    /// A new descriptor starts with it (`NEWSETDESC` writes the flag word
    /// 0xD000 at 0x70d07); every change sets it (0x6ab6e); the damage map sets
    /// it on the neighbours (0x6e8c8); the drawer clears it once it has drawn
    /// (0x69659). Without it a descriptor is skipped even while active
    /// (0x694ed, 0x69680).
    pub dirty: bool,
    /// The drawer's pass that last drew this descriptor, nought for never.
    ///
    /// Not a flag of the original's: its surface remembers what was drawn
    /// over what by holding the pixels, and this is what stands in for that
    /// where a rectangle is built again from the list — anything painted
    /// straight onto the surface goes over what was drawn before it and
    /// under what is drawn after (the `paint` module). Public because the struct is built by
    /// the tests with `..Default::default()`, which needs every field
    /// visible; nothing outside the drawer reads or writes it.
    pub drawn_at: u64,
    /// Bit 0x10: this descriptor is what changed, as opposed to a neighbour.
    ///
    /// Set beside [`Descriptor::dirty`] by 0x6ab6e and cleared by the drawer
    /// with it. In the original it picks between two blitters — 0x27765 /
    /// 0x29ae9 against 0x273e8 / 0x299e1, whose destination is the display's
    /// software surface at 0xE7D7C (0x69331, 0x697cd). motionvm has one
    /// drawing path, onto the screen's own surface, and that path is the one
    /// checked pixel for pixel against the original; the bit is carried so the
    /// distinction is not lost, and what it is for is an open question.
    pub changed: bool,
    /// `SDBUF`, on the 16-bit machine: the off-screen buffer this descriptor
    /// draws through, by the number `SETBUF` allocated it under. What the
    /// 16-bit kernel does with it is unread — see [`crate::Buffers`]; the
    /// 32-bit game keeps `SDBUF` as a by-name field and never reads it.
    pub buffer: Option<i32>,
    /// `SDAUTOBUF`: whether taking this descriptor away puts the picture back.
    ///
    /// `SDAUTOBUF` (0x72104) hands the descriptor a buffer number at +0x1C and
    /// sets flag 0x08, and the original keeps a copy of the surface under it:
    /// the drawer saves the area before every blit (0x696ed → 0x6b936 →
    /// 0x2825d) and 0x6ab6e pastes the copy back when the descriptor is hidden
    /// or moves (0x6ac33). That copy is what makes something disappear — and
    /// without one, nothing does.
    ///
    /// **motionvm rebuilds the area instead of remembering it.** When a
    /// descriptor with this flag goes away, its rectangle is cleared and every
    /// active descriptor across it is drawn again (`Engine::draw_screens`), so
    /// what comes back is what the scene graph says belongs there. The two
    /// agree wherever the picture under the descriptor came from descriptors,
    /// which is everywhere the game puts one of these: `INITANI` gives every
    /// animation the flag (module 6, 0x007dc), as do the captions, the menu
    /// sprites and the mailbox's bars. They differ only for pixels no
    /// descriptor owns — the mailbox's own hidden terminal rows, say, which a
    /// remembered copy would put back and a rebuild leaves out. A remembered
    /// copy also goes stale: it holds the surface as it was when it was taken,
    /// and every redraw underneath it since is news the copy does not have.
    pub auto_buffer: bool,
}

/// How many insert slots a text record has: five, `+0x20` to `+0x30`.
pub(crate) const SLOTS: usize = 5;

/// One insert slot of a text record — what `SDINSERT` put there.
///
/// In every shipped script the value is an address: a name buffer the player
/// types into, a date or a score a script word has spelled out into cells
/// (`_NAME1 0 SDINSERT`, `_SSCORE1 0 SDINSERT` in Checker 2000's module 318).
/// Dunkle Schatten 2's debug overlay is the one caller that passes numbers,
/// and it says so with the kind. What a slot means to the layout is the
/// build's — [`crate::Profile`]'s text-insert capability — not the slot's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Insert {
    /// The value `SDINSERT` stored.
    pub value: i32,
    /// R109's kind (`+0x34`): 0 an address whose bytes are the string, 1 a
    /// number, 2 an address whose cell is the number. A build whose
    /// `SDINSERT` takes no kind leaves it 0.
    pub kind: i32,
}

/// What a descriptor shows: the original's `+0x10`.
///
/// One word over two id spaces. Bit 15 marks a sprite and the rest is a
/// graphics id; without it the value is a graphics id too — a block, drawn
/// opaque where a sprite is drawn through its key color — unless `SDTXT` has
/// made the descriptor a text, in which case the same number is the id of the
/// text table to read from (`016a:1eea` decides in that order).
///
/// There is no "nothing" in the 16-bit original: `NEWSETDESC` writes the
/// graphics argument straight into the field, so a descriptor made with 0 is a
/// block on graphic 0 and not an empty one. [`Shows::Nothing`] is what a
/// descriptor holds before any of that, and what a negative argument leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shows {
    /// Nothing chosen yet.
    #[default]
    Nothing,
    /// `SDSPR`: a sprite, drawn through its key color.
    Sprite(u32),
    /// `SDBL` or `SDTB`: a block when the descriptor is not a text, and the
    /// text's table when it is.
    Picture(i32),
}

impl Descriptor {
    /// Whether this descriptor draws text rather than a picture.
    ///
    /// The original asks its text word, `+0x12`, and takes any non-zero as
    /// yes (`016a:0b09`, and the same test again in the resolver `016a:1ef6`
    /// and the save-under check `0362:10d9`). `SDTXT` is what puts a value
    /// there, so that is what this asks — and `+0x10`, whatever it holds, is
    /// then read as the id of a text table rather than of a picture.
    pub fn is_text(&self) -> bool {
        self.text.is_some()
    }

    /// Makes the descriptor a text showing `entry`, with the text record as
    /// the 32-bit engine allocates it (`0x71b87`, R78 `0x5d9dd`; `SDTB`'s
    /// own allocation at `0x71d45` is the same): the five insert slots
    /// cleared, the line window 0 and 99 — the unset values.
    pub(crate) fn make_text_record(&mut self, entry: i32) {
        self.text = Some(entry);
        self.inserts = Default::default();
        self.fields.clear(Field::SDSTARTLINE);
        self.fields.clear(Field::SDALINES);
    }
}

impl Shows {
    /// The graphics id to draw, for a descriptor that is not a text.
    pub fn graphic(self) -> Option<u32> {
        match self {
            Shows::Nothing => None,
            Shows::Sprite(id) => Some(id),
            Shows::Picture(id) => u32::try_from(id).ok(),
        }
    }

    /// The text table this reads from, for a descriptor that is a text.
    pub fn table(self) -> Option<i32> {
        match self {
            Shows::Picture(id) => Some(id),
            _ => None,
        }
    }
}

/// A text template defined by `DEFTDT`, which takes seventeen arguments.
#[derive(Debug, Clone)]
pub struct TextTemplate {
    /// The template number `DEFTDT` gave it, which `SDTDT` names.
    pub id: i32,
    /// The arguments as given, deepest first. What each one means is still open;
    /// recording them keeps the information until it is needed.
    pub args: Vec<i32>,
}

/// The two small enums a savegame has to carry, as numbers and back.
///
/// Written out by hand rather than derived, so that adding a variant makes the
/// mapping a decision instead of silently renumbering every saved file.
pub(crate) fn placement_code(p: Placement) -> u8 {
    match p {
        Placement::Edge => 0,
        Placement::Center => 1,
        Placement::FarEdge => 2,
    }
}

pub(crate) fn placement_of(code: u8) -> Result<Placement, String> {
    match code {
        0 => Ok(Placement::Edge),
        1 => Ok(Placement::Center),
        2 => Ok(Placement::FarEdge),
        n => Err(format!(
            "savegame has placement mode {n}, which this build does not know"
        )),
    }
}
