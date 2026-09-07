//! The text layout: what a text descriptor shows, made from its resource
//! text, its line window and its five insert slots — the routine at `0x6c9f8`
//! (R109) and `0x5a100` (R78), and the engine's own `#`-formatter it hands
//! the result to (`0x11d66`, R78 `0x11bd0`).
//!
//! Both builds lay a text out the same way up to one branch:
//!
//! 1. The resource text is fetched by table and entry (`0x6c846`, R78
//!    `0x5a0a0`).
//! 2. Its **line window** is copied into a static buffer (`0xee700`, R78
//!    `0xc9028`): lines are counted at every `\n`, the copy starts once the
//!    count reaches `SDSTARTLINE` (`+0x18` of the text record) and stops once
//!    it reaches `SDSTARTLINE + SDALINES` (`+0x1c`) — so the newline that
//!    ends the last line goes with it. `SDTXT` and `SDTB` write 0 and 99
//!    there when they make the record, and a text nobody windowed is whole.
//! 3. The five **insert slots** (`+0x20`) become the formatter's arguments,
//!    and the window is formatted into the record's text buffer (`+0xc`,
//!    `0xf0640`, R78 `0xc9fc8`).
//!
//! The branch is R109's: it first looks for a set slot (`0x6cb00`) and, when
//! there is none, points the record at the resource text as it stands
//! (`0x6cd39`) — no window, no formatting. R78 has no such look and formats
//! every text. How a slot becomes an argument is the other difference, and
//! both are [`TextInserts`]'s to say.
//!
//! The formatter is `printf` with `#` for `%`. A directive is `#`, then an
//! optional fill `F<c>`, an optional width `L<n>` (pad after) or `R<n>` (pad
//! before), an optional `u` (unsigned), then one of `i` or `l` (a number),
//! `s` (the string at a pointer) or `c` (a byte). Any other letter is consumed
//! and prints nothing — `##` prints nothing, and so does a `#` before a
//! newline, which is how one of Checker 2000's help texts loses a blank line.
//! What `s` reads through its pointer is the machine's memory, so the layout
//! runs against the [`AddressSpace`], and the game runs it before each frame is
//! drawn — as the original lays out inside its drawer and again in every word
//! that measures a text.

use std::collections::BTreeMap;

use motionvm_motion_forth::AddressSpace;

use crate::Descriptor;
use crate::Engine;
use crate::Field;
use crate::Insert;
use crate::descriptor::SLOTS;
use crate::profile::TextInserts;
use crate::words::Word;

/// What `SDTXT` and `SDTB` write into `+0x1c` when they make the record
/// (`0x71bf5`, R78 `0x5da40`): more lines than any text has.
pub(crate) const ALL_LINES: i32 = 0x63;

/// How far a `#s` reads for its NUL before giving up. The original reads
/// until it finds one; a string this long is a pointer at something else.
const STRING_CAP: i32 = 4096;

/// One formatter argument, as the layout builds it out of a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arg {
    /// An empty slot, or one whose kind names nothing: the original passes 0.
    Null,
    /// A pointer into the machine's memory (`0x67404`, R78 `0x568f0`) — kept
    /// as the address it was made from, which is the only form it has here.
    Address(i32),
    /// A number as it stands.
    Number(i32),
}

impl Arg {
    /// What R109's layout makes of a slot by its kind (`0x6cc25`–`0x6cd0c`):
    /// 0 a pointer at the string, 1 the number as it is, 2 the number in the
    /// cell the address names, anything else nought.
    fn kinded(slot: Insert, mem: &dyn AddressSpace) -> Arg {
        match slot.kind {
            0 if slot.value == 0 => Arg::Null,
            0 => Arg::Address(slot.value),
            1 => Arg::Number(slot.value),
            2 if slot.value == 0 => Arg::Null,
            2 => Arg::Number(mem.fetch_cell(slot.value).unwrap_or(0)),
            _ => Arg::Null,
        }
    }

    /// What R78's layout makes of a slot (`0x5a2e4`–`0x5a32e`): a pointer at
    /// whatever the address names, or nought.
    fn address(slot: Insert) -> Arg {
        if slot.value == 0 {
            Arg::Null
        } else {
            Arg::Address(slot.value)
        }
    }
}

/// The lines `start` to `start + lines` of `text`, as the layout copies them
/// (`0x6cb78`–`0x6cc08`, R78 `0x5a23a`–`0x5a2ca`): the first pass counts
/// newlines until `start` of them have gone by, the second copies until the
/// count reaches the end — after copying the newline that got it there. Both
/// bounds are the record's cells and compare signed, so a negative start
/// copies from the top and a negative count copies nothing.
pub(crate) fn window(text: &str, start: i32, lines: i32) -> String {
    let end = start.wrapping_add(lines);
    let mut count = 0i32;
    let mut chars = text.chars();
    let mut cur = chars.next();
    while let Some(c) = cur {
        if count >= start {
            break;
        }
        if c == '\n' {
            count += 1;
        }
        cur = chars.next();
    }
    let mut out = String::new();
    while let Some(c) = cur {
        if end <= count {
            break;
        }
        out.push(c);
        if c == '\n' {
            count += 1;
        }
        cur = chars.next();
    }
    out
}

/// What the formatter made of a text, and what it could not.
pub(crate) struct Formatted {
    pub(crate) text: String,
    /// Directives whose result the original owes to its own address space —
    /// a `#i` handed a pointer prints where DOS/4GW put a module — and so has
    /// no counterpart here. Each is a departure to note, not a guess to make.
    pub(crate) departures: Vec<String>,
}

/// Runs `text` through the `#`-formatter with `args` as its five arguments,
/// reading strings out of `mem`.
pub(crate) fn format(text: &str, args: &[Arg; SLOTS], mem: &dyn AddressSpace) -> Formatted {
    let mut f = Formatter {
        chars: text.chars().collect(),
        at: 0,
        args,
        next: 0,
        mem,
        out: String::new(),
        departures: Vec::new(),
    };
    f.run();
    Formatted {
        text: f.out,
        departures: f.departures,
    }
}

/// One run of the formatter (`0x11bd0`; R109 `0x11d66` is the same routine
/// with a null check in front): the format, where it has got to, the
/// arguments and which is next, and the output so far.
struct Formatter<'a> {
    chars: Vec<char>,
    at: usize,
    args: &'a [Arg; SLOTS],
    next: usize,
    mem: &'a dyn AddressSpace,
    out: String,
    departures: Vec<String>,
}

impl Formatter<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    /// The next argument. Past the fifth the original reads on up the stack
    /// into its caller's locals; here it is nought, and noted.
    fn arg(&mut self) -> Arg {
        let arg = self.args.get(self.next).copied();
        self.next += 1;
        arg.unwrap_or_else(|| {
            self.departures
                .push(format!("directive {} reads past the five slots", self.next));
            Arg::Null
        })
    }

    fn pad(&mut self, fill: char, n: i32) {
        for _ in 0..n.max(0) {
            self.out.push(fill);
        }
    }

    /// The main loop: copy until `#`, then one directive (`0x11bea`–`0x1216a`).
    fn run(&mut self) {
        while let Some(c) = self.peek() {
            if c != '#' {
                self.out.push(c);
                self.at += 1;
                continue;
            }
            self.at += 1;
            // The flags, each optional and in this order: `F<fill>`, `L<n>`
            // or `R<n>`, `u`. A fill of NUL — `#F` at the very end — is made
            // a space again where a width would use it.
            let mut fill = ' ';
            let mut left = 0;
            let mut right = 0;
            if self.peek() == Some('F') {
                self.at += 1;
                fill = self.peek().unwrap_or('\0');
                self.at += 1;
            }
            if self.peek() == Some('L') {
                self.at += 1;
                left = self.width();
                if fill == '\0' {
                    fill = ' ';
                }
            } else if self.peek() == Some('R') {
                self.at += 1;
                right = self.width();
                if fill == '\0' {
                    fill = ' ';
                }
            }
            let mut signed = true;
            if self.peek() == Some('u') {
                self.at += 1;
                signed = false;
            }
            // The directive letter — consumed whatever it is, and a text that
            // ends in `#` ends here (the original steps past its terminator).
            let mut ended = false;
            match self.peek() {
                Some('i' | 'l') => self.number(signed, fill, left, right),
                Some('s') => self.string(fill, left, right),
                Some('c') => ended = !self.byte(),
                _ => {}
            }
            if ended {
                return;
            }
            self.at += 1;
        }
    }

    /// A width after `L` or `R` (`0x12280`): spaces skipped, one `-`
    /// accepted and ignored — the routine sets its sign to 1 either way —
    /// then decimal digits.
    fn width(&mut self) -> i32 {
        while self.peek() == Some(' ') {
            self.at += 1;
        }
        if self.peek() == Some('-') {
            self.at += 1;
        }
        let mut n = 0i32;
        while let Some(d) = self.peek().and_then(|c| c.to_digit(10)) {
            n = n
                .wrapping_mul(10)
                .wrapping_add(i32::try_from(d).unwrap_or(0));
            self.at += 1;
        }
        n
    }

    /// `#i` and `#l` (`0x11ce0`, `0x11e09`: the same code twice). The sign
    /// goes first and costs a column of either width; the digits are right-
    /// padded to `R` before and left-padded to `L` after, both against the
    /// digit count of what is left once the sign is off.
    fn number(&mut self, signed: bool, fill: char, mut left: i32, mut right: i32) {
        let mut v = match self.arg() {
            Arg::Null => 0,
            Arg::Number(n) => n,
            Arg::Address(a) => {
                self.departures.push(format!(
                    "#i of an address ({a:#x}) prints where the module lies in the original's \
                     memory; the address stands in"
                ));
                a
            }
        };
        if signed && v < 0 {
            self.out.push('-');
            v = v.wrapping_neg();
            if left > 0 {
                left -= 1;
            }
            if right > 0 {
                right -= 1;
            }
        }
        if right > 0 {
            self.pad(fill, right - digits(v));
        }
        put_number(&mut self.out, v);
        if left > 0 {
            self.pad(fill, left - digits(v));
        }
    }

    /// `#s` (`0x11f7f`): the string at the pointer, padded to `R` before and
    /// `L` after by its length. A null pointer is an empty string — both
    /// builds' `strlen` answers 0 for it and the copy copies nothing.
    fn string(&mut self, fill: char, left: i32, right: i32) {
        let s = match self.arg() {
            Arg::Null | Arg::Number(0) => String::new(),
            Arg::Number(n) => {
                self.departures.push(format!(
                    "#s of a number ({n}) reads the original's memory at that address; nothing \
                     stands in"
                ));
                String::new()
            }
            Arg::Address(a) => string_at(self.mem, a),
        };
        let len = i32::try_from(s.chars().count()).unwrap_or(i32::MAX);
        if right > 0 {
            self.pad(fill, right - len);
        }
        self.out.push_str(&s);
        if left > 0 {
            self.pad(fill, left - len);
        }
    }

    /// `#c` (`0x11f5c`): the argument's low byte, as a CP437 character. A
    /// nought is a NUL written into the buffer, which is where everything
    /// that reads the buffer stops — so the text ends, and `false` says so.
    fn byte(&mut self) -> bool {
        let v = match self.arg() {
            Arg::Null => 0,
            Arg::Number(n) => n,
            Arg::Address(a) => {
                self.departures.push(format!(
                    "#c of an address ({a:#x}) prints a byte of where the module lies in the \
                     original's memory; the address's stands in"
                ));
                a
            }
        };
        let byte = v.to_le_bytes()[0];
        if byte == 0 {
            return false;
        }
        self.out.push(motionvm_motion_formats::cp437_char(byte));
        true
    }
}

/// How many characters `v` prints as (`0x123d0`): the sign counts, a value
/// past a thousand million is ten digits, and nought is one.
fn digits(v: i32) -> i32 {
    let mut n = 0i32;
    let mut v = i64::from(v);
    if v < 0 {
        n += 1;
        v = -v;
    }
    if v > 1_000_000_000 {
        n += 10;
    } else {
        let mut p = 1i64;
        while v / p != 0 {
            p *= 10;
            n += 1;
        }
    }
    if n == 0 { 1 } else { n }
}

/// Writes `v` in decimal (`0x12320`): a `-` and the magnitude for a negative
/// value, one digit per power of ten from the highest down.
fn put_number(out: &mut String, v: i32) {
    let mut n = digits(v);
    let mut v = i64::from(v);
    if v < 0 {
        out.push('-');
        v = -v;
        n -= 1;
    }
    let mut p = 10i64.pow(u32::try_from(n - 1).unwrap_or(0));
    for _ in 0..n {
        let digit = u8::try_from((v / p) % 10).unwrap_or(0);
        out.push(char::from(b'0' + digit));
        v -= (v / p) * p;
        p /= 10;
    }
}

/// The NUL-terminated CP437 string at `addr`, read a byte at a time out of
/// the machine's memory; a read that fails — an address in no loaded module
/// — ends it where it fails.
fn string_at(mem: &dyn AddressSpace, addr: i32) -> String {
    let mut bytes = Vec::new();
    for i in 0..STRING_CAP {
        match mem.fetch_byte(mem.offset(addr, i)) {
            Ok(0) | Err(_) => break,
            Ok(b) => bytes.push(b),
        }
    }
    motionvm_motion_formats::cp437_to_string(&bytes)
}

impl Engine {
    /// The text descriptor `d` shows, laid out against `mem` as the original
    /// lays it out inside its drawer (`0x6c9f8`, R78 `0x5a100`); `None` where
    /// it shows no text or its table has no such entry.
    ///
    /// Which slots become which arguments, and whether a text with none set
    /// is formatted at all, is the build's — [`TextInserts`]. A build without
    /// the word shows the resource text as it stands.
    pub(crate) fn laid_out_text(
        &mut self,
        d: &Descriptor,
        mem: &dyn AddressSpace,
    ) -> Option<String> {
        let raw = self.descriptor_text(d)?;
        let args = match self.profile.inserts {
            TextInserts::Absent => return Some(raw),
            // `0x6cb00`: the first set slot, or none — and none means the
            // resource text itself, window and all.
            TextInserts::Kinded if d.inserts.iter().all(|s| s.kind == 0 && s.value == 0) => {
                return Some(raw);
            }
            TextInserts::Kinded => d.inserts.map(|s| Arg::kinded(s, mem)),
            TextInserts::Addresses => d.inserts.map(Arg::address),
        };
        let start = d.fields.get(Field::SDSTARTLINE).unwrap_or(0);
        let lines = d.fields.get(Field::SDALINES).unwrap_or(ALL_LINES);
        let formatted = format(&window(&raw, start, lines), &args, mem);
        for note in formatted.departures {
            self.note_unhandled(Word::SDINSERT, Some(note));
        }
        Some(formatted.text)
    }

    /// Lays out every text descriptor against `mem`, which is what the
    /// original does inside its drawer for each one it draws. The game runs
    /// this before each frame is drawn, so what a frame shows is read out of
    /// the machine's memory as it stands at the frame — a name buffer the
    /// script has typed into since the last `SDINSERT` shows its new letters,
    /// as it does there.
    pub(crate) fn lay_out_texts(&mut self, mem: &dyn AddressSpace) {
        let texts: Vec<Descriptor> = self
            .scene
            .descriptors
            .iter()
            .filter(|d| d.is_text())
            .cloned()
            .collect();
        let mut laid_out = BTreeMap::new();
        for d in &texts {
            if let Some(text) = self.laid_out_text(d, mem) {
                laid_out.insert(d.handle, text);
            }
        }
        self.laid_out = laid_out;
    }

    /// Lays the current descriptor out again. The words that change what a
    /// text shows measure it as they go (`0x6c8c1`, which lays out first), so
    /// each of them runs this before its mark.
    pub(crate) fn lay_out_current(&mut self, mem: &dyn AddressSpace) {
        let Some(d) = self.descriptor_mut().cloned() else {
            return;
        };
        if !d.is_text() {
            return;
        }
        match self.laid_out_text(&d, mem) {
            Some(text) => {
                self.laid_out.insert(d.handle, text);
            }
            None => {
                self.laid_out.remove(&d.handle);
            }
        }
    }

    /// What descriptor `d` shows, as last laid out — or the resource text as
    /// it stands where nothing has laid it out yet, which is every text of a
    /// build without insert slots and every text drawn by a test that has no
    /// machine behind it.
    pub(crate) fn shown_text(&mut self, d: &Descriptor) -> Option<String> {
        if let Some(text) = self.laid_out.get(&d.handle) {
            return Some(text.clone());
        }
        self.descriptor_text(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use motionvm_motion_forth::{Error, Result};

    /// A machine whose memory is one string at one address and one cell at
    /// another; everything else is unmapped.
    struct Bytes {
        at: i32,
        bytes: Vec<u8>,
        cell_at: i32,
        cell: i32,
    }

    impl AddressSpace for Bytes {
        fn fetch_cell(&self, raw: i32) -> Result<i32> {
            (raw == self.cell_at)
                .then_some(self.cell)
                .ok_or_else(|| Error::Unsupported("cell".into()))
        }
        fn store_cell(&mut self, _raw: i32, _value: i32) -> Result<()> {
            Err(Error::Unsupported("store".into()))
        }
        fn fetch_byte(&self, raw: i32) -> Result<u8> {
            usize::try_from(raw - self.at)
                .ok()
                .and_then(|i| self.bytes.get(i).copied())
                .ok_or_else(|| Error::Unsupported("byte".into()))
        }
        fn read_bytes(&self, raw: i32, n: usize) -> Result<Vec<u8>> {
            (0..n)
                .map(|i| self.fetch_byte(raw + i32::try_from(i).unwrap_or(0)))
                .collect()
        }
        fn write_bytes(&mut self, _raw: i32, _bytes: &[u8]) -> Result<()> {
            Err(Error::Unsupported("write".into()))
        }
        fn offset(&self, raw: i32, bytes: i32) -> i32 {
            raw + bytes
        }
        fn cell_size(&self) -> i32 {
            4
        }
        fn callable(&self, raw: i32) -> i32 {
            raw
        }
        fn is_live(&self, _raw: i32) -> bool {
            true
        }
        fn module_image(&self, _module: u32) -> Option<Vec<u8>> {
            None
        }
        fn restore_module(&mut self, _module: u32, _image: &[u8]) -> Result<()> {
            Err(Error::Unsupported("restore".into()))
        }
    }

    fn memory() -> Bytes {
        Bytes {
            at: 0x5_0010,
            bytes: b"Bj\x94rn\0after".to_vec(),
            cell_at: 0x5_0020,
            cell: -42,
        }
    }

    fn args(a: [Arg; SLOTS]) -> [Arg; SLOTS] {
        a
    }

    /// The highscore's texts: five `#s` filled from five name buffers, one
    /// per slot in order, and the string read to its NUL and decoded.
    #[test]
    fn s_reads_the_string_at_the_pointer() {
        let mem = memory();
        let out = format(
            "1. #s\n\n2. #s\n",
            &args([
                Arg::Address(0x5_0010),
                Arg::Null,
                Arg::Null,
                Arg::Null,
                Arg::Null,
            ]),
            &mem,
        );
        assert_eq!(out.text, "1. Björn\n\n2. \n");
        assert!(out.departures.is_empty());
    }

    /// `#i` prints the number, signed unless `u` says otherwise; a kind-2
    /// slot is the cell at the address.
    #[test]
    fn i_prints_a_number_and_the_flags_pad_it() {
        let mem = memory();
        let a = args([
            Arg::Number(7),
            Arg::Number(-15),
            Arg::Number(-15),
            Arg::Number(0),
            Arg::Number(1_234_567_890),
        ]);
        // The sign goes down before the padding — `0x11cf9` writes it, then
        // `0x11d36` pads against the digits that are left — so a right-aligned
        // negative number is "- 15", as the routine has it and not as `printf`
        // would.
        let out = format("#F0R3i|#R4i|#L4ui|#i|#i", &a, &mem);
        assert_eq!(out.text, "007|- 15|-15 |0|1234567890");
        assert_eq!(
            Arg::kinded(
                Insert {
                    value: 0x5_0020,
                    kind: 2
                },
                &mem
            ),
            Arg::Number(-42)
        );
        assert_eq!(
            Arg::kinded(Insert { value: 5, kind: 1 }, &mem),
            Arg::Number(5)
        );
        assert_eq!(
            Arg::kinded(Insert { value: 5, kind: 0 }, &mem),
            Arg::Address(5)
        );
        assert_eq!(Arg::kinded(Insert { value: 5, kind: 3 }, &mem), Arg::Null);
        assert_eq!(Arg::address(Insert { value: 5, kind: 0 }), Arg::Address(5));
        assert_eq!(Arg::address(Insert::default()), Arg::Null);
    }

    /// The letters the formatter does not know, `#` itself among them, are
    /// eaten with their `#`; `#c` writes one byte and a NUL ends the text.
    #[test]
    fn unknown_directives_vanish_and_c_writes_a_byte() {
        let mem = memory();
        let none = args([Arg::Null; SLOTS]);
        assert_eq!(format("a##b#\nc#x", &none, &mem).text, "abc");
        let a = args([
            Arg::Number(0x41),
            Arg::Number(0),
            Arg::Null,
            Arg::Null,
            Arg::Null,
        ]);
        assert_eq!(format("#c#c-gone", &a, &mem).text, "A");
    }

    /// What the original owes to its own memory layout is noted, not made up.
    #[test]
    fn a_pointer_where_a_number_is_asked_is_a_departure() {
        let mem = memory();
        let a = args([
            Arg::Address(0x5_0010),
            Arg::Number(9),
            Arg::Null,
            Arg::Null,
            Arg::Null,
        ]);
        let out = format("#i #s #s", &a, &mem);
        assert_eq!(out.text, "327696  ");
        assert_eq!(out.departures.len(), 2, "{:?}", out.departures);
        let past = format("#i#i#i#i#i#i", &[Arg::Null; SLOTS], &mem);
        assert_eq!(past.text, "000000");
        assert_eq!(past.departures.len(), 1, "the sixth reads past the slots");
    }

    /// The line window: `SDSTARTLINE` lines skipped, `SDALINES` copied with
    /// the newline that closes the last, and the defaults copy everything.
    #[test]
    fn the_window_copies_whole_lines() {
        let text = "one\ntwo\nthree\nfour";
        assert_eq!(window(text, 0, ALL_LINES), text);
        assert_eq!(window(text, 1, 2), "two\nthree\n");
        assert_eq!(window(text, 3, 5), "four");
        assert_eq!(window(text, 9, 1), "");
        assert_eq!(window(text, 0, 0), "");
        assert_eq!(window(text, -1, 1), "", "the end bound is start plus count");
        assert_eq!(window(text, -1, 2), "one\n", "a negative start is the top");
    }

    /// The digit count and the writer agree with the routines they are read
    /// from at the edges: nought, the sign, and the ten-digit cliff.
    #[test]
    fn digits_and_the_writer_agree_at_the_edges() {
        for v in [
            0,
            9,
            10,
            -1,
            -10,
            999_999_999,
            1_000_000_000,
            1_000_000_001,
            i32::MAX,
            i32::MIN,
        ] {
            let mut s = String::new();
            put_number(&mut s, v);
            assert_eq!(s, v.to_string(), "{v}");
            assert_eq!(
                digits(v),
                i32::try_from(s.len()).unwrap(),
                "{v}: the count is the length written"
            );
        }
    }
}
