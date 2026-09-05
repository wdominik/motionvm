//! The contract between the window and a game, whatever engine runs it.
//!
//! Everything about a *game* crosses through this crate and nothing else
//! does: an engine implements what is declared here, the window calls it, and
//! neither ever names the other. What an engine measured from its original —
//! its formats, its words, its timing — stays on the engine's side of the
//! line; what a platform owns — the window, the devices, the event loop —
//! stays on the window's.

pub use motionvm_render::{Frame, Framebuffer, Palette};

/// How big a game's picture is, in its own pixels.
///
/// A struct and not a pair for the reason [`PixelAspect`] is one: two numbers
/// of the same type in a tuple invite each other's place, and a picture
/// composed at the transposed size is plausible enough to ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    /// Pixels per row.
    pub width: u16,
    /// Rows.
    pub height: u16,
}

/// The shape of one of a game's pixels on its original monitor, as the
/// familiar pixel aspect ratio: width to height. Square is 1:1; the era's
/// 320×200 mode, filling a 4:3 monitor, is 5:6 — each pixel six units tall
/// for five wide.
///
/// A struct and not a pair, because a pair invites the two numbers in the
/// wrong order, and a picture squashed by a swapped aspect looks plausible
/// enough to ship. Buildable literally, like [`KeyPress`], so a test can
/// name one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelAspect {
    /// Horizontal units of the ratio.
    pub width: u32,
    /// Vertical units of the ratio.
    pub height: u32,
}

/// Square pixels, the default shape.
impl Default for PixelAspect {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
        }
    }
}

/// The error a game answers with when opening or running fails.
///
/// A box rather than an enum, because no caller on the window's side ever
/// branches on a case: the window prints the message, shows it in a dialog,
/// or degrades the one feature it belongs to. Each engine keeps its own rich
/// error type behind this, and its `Display` output *is* the message a player
/// reads — so an engine's messages are part of what its tests pin down.
/// `Send + Sync`, so that opening a game — the slowest thing a window does —
/// can move to a worker thread the day a window wants a splash screen; a
/// family's error type has to clear the same bar.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// Result over the contract's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A physical key, by position — what a scan code names, not what the layout
/// types. [`Key::Other`] stands for every key without a variant here: such a
/// press still carries its [`KeyPress::text`], which is all an engine can read
/// off it.
///
/// `non_exhaustive`, so a key the contract learns to name later — a wheel
/// click, a keypad — is an addition and not a breakage: every match on this
/// enum carries a catch-all arm from day one.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A letter key, named by its US-layout capital: `Key::Letter(b'A')` is
    /// the key in A's position whatever the layout in force makes of it.
    /// Values outside `b'A'..=b'Z'` name no key.
    Letter(u8),
    /// A function key: `Key::Function(1)` is F1.
    Function(u8),
    /// A digit key in the top row, by position: `Key::Digit(1)` is the key
    /// that types `1` on a US layout, whatever the layout in force types —
    /// which is the half a layout like AZERTY takes away from
    /// [`KeyPress::text`]. Values outside `0..=9` name no key.
    Digit(u8),
    /// The up arrow.
    Up,
    /// The down arrow.
    Down,
    /// The left arrow.
    Left,
    /// The right arrow.
    Right,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Insert.
    Insert,
    /// Delete.
    Delete,
    /// Enter.
    Enter,
    /// Escape.
    Escape,
    /// Backspace.
    Backspace,
    /// Tab.
    Tab,
    /// The space bar.
    Space,
    /// Any key without a variant of its own.
    Other,
}

/// A mouse button, as the platform names it.
///
/// `non_exhaustive` like [`Key`]: a wheel or a middle button the contract
/// learns to name later is an addition, not a breakage.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    /// The left button.
    Left,
    /// The right button.
    Right,
}

/// One key press, as the platform reports it: the physical key, what the
/// layout in force makes of it, and the modifiers that were down.
///
/// Deliberately this crate's own type rather than a windowing library's,
/// so that a test can build one — a translator with no test is how cursor
/// keys end up on the floor. The platform fills it from its own events;
/// what an engine's scripts read out of a press is that engine's translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyPress {
    /// The physical key, by position.
    pub key: Key,
    /// The character this press types under the layout in force, if any.
    pub text: Option<char>,
    /// Shift was down.
    pub shift: bool,
    /// Ctrl was down.
    pub ctrl: bool,
    /// Alt was down.
    pub alt: bool,
}

/// Samples for the platform's audio device.
///
/// `fill` renders interleaved stereo, two `i16` per frame, at whatever rate
/// the source was built for — the platform owns the device, learns the rate
/// from it, and hands that rate to whoever builds the source. `Send`, because
/// the platform moves the source onto its audio thread and calls `fill` from
/// the device's callback; anything slow — file reading, parsing — belongs on
/// the far side of whatever channel feeds the source.
pub trait AudioSource: Send {
    /// Renders the next `out.len() / 2` frames of interleaved stereo.
    fn fill(&mut self, out: &mut [i16]);
}

/// A game a window can drive, whichever engine runs it.
///
/// What a window needs: start it, step it, feed it input, take the picture
/// and the palette, know how long a frame lasts, give it music and a place
/// to save, ask whether it has ended. The methods with default bodies are
/// the optional affordances — music, saving, a location request — which a
/// game without the concept simply leaves alone. The engine behind
/// it stays out of reach — a window that could reach in and move the game's
/// state would be able to produce a picture the original never could.
///
/// `Send`, so a window may open a game on a worker thread — opening is the
/// slowest thing a window does — and move it back. `Sync` is deliberately
/// not promised: a game is driven from one thread at a time.
pub trait Playable: Send {
    /// The game's full name, as a window shows it.
    ///
    /// Words and not a roster type, because the roster is the family's own:
    /// what a window does with the answer — a title bar — needs a string.
    fn name(&self) -> &str;
    /// The size of the picture [`Playable::frame`] answers with.
    ///
    /// Constant for the game's lifetime, like [`Playable::pixel_aspect`]: a
    /// window may cache both, and a family whose original switches modes
    /// mid-run renders into one size and says so here.
    fn display_size(&self) -> Size;
    /// The shape of one of that picture's pixels on the game's own monitor.
    /// Square unless the game says otherwise: the display modes of the time
    /// were all shown on 4:3 screens, and a mode whose grid is not 4:3 had
    /// pixels stretched to make up the difference. A window that wants to
    /// show the picture as its first players saw it scales its two axes in
    /// this ratio. Constant for the game's lifetime, like
    /// [`Playable::display_size`].
    fn pixel_aspect(&self) -> PixelAspect {
        PixelAspect::default()
    }
    /// Reseeds whatever the game draws random numbers from, called before
    /// [`Playable::start`] and not again.
    ///
    /// The default body ignores it, which is right for a game with nothing
    /// random in it. A game that has something is expected to be *repeatable*
    /// without this call — the same seed, or none, gives the same run — because
    /// that is what lets a scene be rendered twice and compared, which is how
    /// an engine of this kind is checked at all. So the platform hands over a
    /// seed rather than the engine reaching for a clock: a window wants a
    /// different game every launch, and a test wants the same one every run,
    /// and only the caller knows which it is.
    fn seed(&mut self, seed: u64) {
        let _ = seed;
    }
    /// Begins the game the way it begins itself, and returns once the game
    /// is parked in its own frame loop — everything after this is
    /// [`Playable::step`]'s. Music, if any, is opened first — see
    /// [`Playable::open_music`].
    fn start(&mut self) -> Result<()>;
    /// One step of the game — what the original engine's native loop does
    /// once per frame.
    fn step(&mut self) -> Result<()>;
    /// Hands the game the pointer's position, in the game's own coordinates.
    ///
    /// Delivered as it moves, not once a frame, so the pointer a game draws
    /// itself can follow the hand without waiting for the next step.
    fn pointer(&mut self, x: i32, y: i32);
    /// Hands the game one button transition — a press or a release, as the
    /// platform saw it.
    ///
    /// Every transition crosses, releases included, so nothing about a
    /// button dies before the seam: what a press *means* — a one-frame
    /// click, a held drag — is the engine's reading, made behind this call
    /// at the game's own pace. A press between two frames still lands, the
    /// way a keystroke in the buffer does.
    fn button(&mut self, which: Button, down: bool);
    /// Takes one key press.
    ///
    /// Nothing is answered on purpose: a press the game has no code for, or
    /// one arriving on a full buffer, is dropped as silently as the hardware
    /// of its day dropped it. What a press *means* is the engine's
    /// translation, made behind this call.
    fn key_down(&mut self, press: &KeyPress);
    /// Takes one key release.
    ///
    /// Its own method because a release is not a press: it was the same call
    /// with a `down: bool`, and every implementation opened by testing that
    /// flag and dropping half the calls. A game that reads only presses
    /// leaves this alone, which is the default body, and one that reads held
    /// keys has the releases it needs.
    fn key_up(&mut self, press: &KeyPress) {
        let _ = press;
    }
    /// The scroll wheel, in lines: positive `dy` rolls away from the hand,
    /// positive `dx` to the right. A game without anything to scroll leaves
    /// it alone, which is the default body.
    fn wheel(&mut self, dx: f32, dy: f32) {
        let _ = (dx, dy);
    }
    /// The typed stream, as the layout and the input method produce it — a
    /// whole string per commit, where [`KeyPress::text`] carries only the
    /// single character a key translator wants. A game with no text field
    /// leaves it alone, which is the default body.
    fn text(&mut self, text: &str) {
        let _ = text;
    }
    /// The modifier keys' level state, delivered as it changes — for a game
    /// that reads "is Shift down *now*" on a click rather than on a press.
    /// The default body leaves it alone.
    fn modifiers(&mut self, shift: bool, ctrl: bool, alt: bool) {
        let _ = (shift, ctrl, alt);
    }
    /// The frame to show, pointer and all — the picture and the palette its
    /// indices mean, together.
    ///
    /// Borrowed, and the two halves in one call, because that is the only
    /// shape that can be: a picture handed over on its own would have to be
    /// owned, since the palette would be fetched by a second borrow — and an
    /// owned frame is 307 200 bytes allocated for every present.
    ///
    /// `&mut self` because a game composes the frame when it is asked for one
    /// — the pointer goes on last, over whatever the drawer left.
    fn frame(&mut self) -> Frame<'_>;
    /// How long the frame [`Playable::step`] is about to run should last on
    /// screen, or `None` for a frame the game wants no wait after at all —
    /// the window then paces the loop as it likes.
    fn frame_duration(&self) -> Option<std::time::Duration>;
    /// Opens the game's music at the platform's device rate, answering the
    /// source its audio thread renders — or `None` for a game with nothing
    /// to play, which is the default body: no stream opens and nothing is
    /// said, because silence by design is not a defect. An error is music
    /// that *should* have come up and did not, and its message is what
    /// "sound is off:" reports.
    ///
    /// Called **before** [`Playable::start`], because a game's first song
    /// can fire during its own startup. Everything past the samples is the
    /// family's own: how a song reaches the source, and in what shape, never
    /// crosses this contract.
    fn open_music(&mut self, rate: u32) -> Result<Option<Box<dyn AudioSource>>> {
        let _ = rate;
        Ok(None)
    }
    /// Points saving and loading at a directory — the game's own, under the
    /// one given. Two games pointed at the same directory keep their slots
    /// apart, which they could not do if the caller chose the name. A game
    /// with no notion of saving accepts and ignores the offer, which is the
    /// default body.
    fn set_saves(&mut self, _dir: &std::path::Path) -> Result<()> {
        Ok(())
    }
    /// Where saving and loading go, or `None` while there is nowhere.
    fn saves(&self) -> Option<&std::path::Path> {
        None
    }
    /// What the game noticed about its own run: see [`Diagnostic`]. The
    /// default body has nothing to say.
    ///
    /// A pull rather than a sink handed in at open, so that a game reports
    /// only when asked and a window decides when that is — at exit, or behind
    /// a key. Calling it twice answers twice: it is the report as it stands,
    /// not a queue that empties.
    fn diagnostics(&self) -> Vec<Diagnostic> {
        Vec::new()
    }
    /// Whether the game has run to its end.
    fn finished(&self) -> bool;
    /// Asks the game to begin at location `n` — the game's own numbering of
    /// its places — instead of where it would. A debug affordance behind the
    /// window's `--loc`; a game with no such numbering ignores the request,
    /// which is the default body.
    fn request_location(&mut self, _n: i32) -> Result<()> {
        Ok(())
    }
    /// The location the game itself wants to begin at, if it says.
    fn start_location(&self) -> Option<i32> {
        None
    }
}

/// Something a game noticed about its own run and could not act on.
///
/// Not an error: nothing here stopped anything. These are the things a
/// reimplementation knows about itself that nobody else can — a resource it
/// walked past, a read that went nowhere, a song it could not decode — and
/// that would otherwise be lost the moment the run ends. An engine of this
/// kind is a claim about another program, and a claim needs a way to say
/// where it fell short of one.
///
/// The family collects them; the window decides what to do with them, which
/// is why nothing here prints. A library that prints has chosen for its caller
/// both where the report goes and when, and on a windowed build the answer to
/// the first is *nowhere*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// What this is about, in a word or two: a heading a reader can group by
    /// and a window can use as a prefix. A literal, because a category is
    /// always one.
    pub subject: &'static str,
    /// The sentence itself, in the family's own words.
    pub detail: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.subject, self.detail)
    }
}

/// One line of a family's roster, for a window to show: what a game is
/// called and what a copy of it has to hold.
///
/// `non_exhaustive` with a constructor, so a field the windows come to want
/// later is an addition and not a breakage for every family's roster. The
/// `&'static str` fields do constrain a roster to static data — a family
/// whose names come out of its game files would need the contract widened
/// first, and that trade keeps a card `Copy`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameCard {
    /// The game's full name, the one on the box.
    pub name: &'static str,
    /// The name without its subtitle — what prose calls the game.
    pub short: &'static str,
    /// The files that identify a copy, the way the family's own messages
    /// name them.
    pub needs: &'static str,
}

impl GameCard {
    /// A card from its three names.
    pub fn new(name: &'static str, short: &'static str, needs: &'static str) -> Self {
        Self { name, short, needs }
    }
}

/// One engine family: everything a window can do with an engine it cannot
/// name.
///
/// A family is a set of crates behind one implementation of this trait, and
/// the window holds a roster of families and nothing else — so adding one to
/// the program is adding a roster line, never editing call sites. `Sync`,
/// because a roster is a `static` list of these.
pub trait Family: Sync {
    /// The family's name, for prose.
    fn name(&self) -> &'static str;
    /// The games the family plays, in the order its documentation lists them.
    ///
    /// Borrowed and static: a roster is a fixed list, and it was rebuilt into
    /// a fresh `Vec` every time a usage text was printed or a directory was
    /// refused. That the cards are `&'static str` already constrains a roster
    /// to static data, so answering a slice of them costs a family nothing it
    /// was not already paying.
    fn games(&self) -> &'static [GameCard];
    /// The game `dir` holds, if it holds one of this family's — answered
    /// from file names alone, so a window may ask every family cheaply
    /// before any of them opens anything, and told by its card, so what was
    /// recognized can be said before the slow open runs.
    fn detect(&self, dir: &std::path::Path) -> Option<GameCard>;
    /// Opens the game in `dir`.
    ///
    /// The error's `Display` output is the message a player reads; for a
    /// directory holding none of the family's games it lists what every one
    /// of them would need.
    fn open(&self, dir: &std::path::Path) -> Result<Box<dyn Playable>>;
}
