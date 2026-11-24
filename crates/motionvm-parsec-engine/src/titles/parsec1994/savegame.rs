//! The saved game and the settings file of segment `147Ah`: the in-memory
//! buffer, `SAVEGAME.SAV`'s header and checksum, and `SAVEGAME.CFG`.
//!
//! There is one saved game and no slot. The save (`147Ah:044Eh`) copies
//! five blocks of the data segment into a far buffer of its own,
//! `8B1h` bytes, and writes the buffer to the file whole; loading in game
//! (`147Ah:0518h`) reads no file but sends the main loop to location 60,
//! whose handler (`147Ah:052Ch`) copies the blocks back. The file is read
//! once, at the program's start (`147Ah:0072h`), into the same buffer.
//!
//! Both files carry the BIOS tick of the moment they were written, and the
//! tick seeds the game's own generator (`1EB5h:000Ah`) before the check
//! values are drawn or computed. So the save, the settings' write and the
//! start's load each reseed the generator the game rolls with.
//!
//! What is read here is the format and the arithmetic; where a block lives
//! in the data segment is the build's, [`SaveMap`].

// A savegame is the one file the engine reads that a player can hand it, so
// its reader holds the readers' rule: an index has been bounds-checked or is a
// constant, and a sum says what it does at the edge. The module's tests are
// outside the rule, because a test indexes what it built.
#![cfg_attr(
    not(test),
    deny(clippy::indexing_slicing, clippy::arithmetic_side_effects)
)]

use crate::num;
use crate::titles::rng::LcRng;

use super::{Adventure, Api, Far};

/// `mov bx, 3E5h` (`0x147EE`): the settings' check value is multiplied by
/// 997 after each word.
const CFG_FACTOR: u32 = 0x3E5;

/// One block of the data segment the save copies: where in the buffer,
/// from where, how many bytes out and how many back. The way back is the
/// restore row's handler, which copies the same blocks — in *Dunkle
/// Schatten* the last of them shorter.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Block {
    pub buf: u16,
    pub ds: u16,
    pub out: u16,
    pub back: u16,
}

/// Where one build keeps what its saved game is made of, and the shape of
/// its two files.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SaveMap {
    /// The file and the buffer, in bytes.
    pub len: usize,
    /// The checksum's walk: the byte it starts at and how many words it
    /// takes, which may run past the file into the segment's padding.
    pub sum: (usize, usize),
    /// The four words `SAVEGAME.SAV` opens with.
    pub sav_magic: [u16; 4],
    /// The four words `SAVEGAME.CFG` opens with.
    pub cfg_magic: [u16; 4],
    /// The settings file's length.
    pub cfg_len: usize,
    /// How many words from `+12h` its check value takes.
    pub cfg_words: usize,
    /// Whether it carries the walk speed, as a word at `+1Ch`, and the
    /// options mark the settings changed when the speed moves. Its last
    /// byte is 0 either way.
    pub cfg_speed: bool,
    /// The row the main loop is sent to when a game is loaded, whose
    /// handler copies the blocks back.
    pub restore_row: u16,
    /// The five blocks, in the save's order.
    pub blocks: [Block; 5],
    /// The location stack: a count word, then the locations.
    pub stack: u16,
    /// `[1742h]`: the save wants writing.
    pub saved: u16,
    /// `[1744h]`: a game is in the buffer.
    pub loaded: u16,
    /// `[1718h]`: the word both headers carry at `+8`.
    pub version: u16,
    /// `[1ED2h]`: the settings changed since the start.
    pub cfg_dirty: u16,
    /// `[16F3h]`: the mouse setting.
    pub mouse: u16,
    /// `[3DCh]`, `[3DAh]`: the two divisors the mouse setting chooses.
    pub divisors: (u16, u16),
    /// `[167Dh]`: the first of the four text-speed words.
    pub text: u16,
    /// `[16A7h]`: the player's factor per edge.
    pub speed: u16,
    /// The names, `*SAVEGAME.SAV` and `*SAVEGAME.CFG`.
    pub sav_name: u16,
    pub cfg_name: u16,
    /// The three messages the file routines end the program with.
    pub msg_create: u16,
    pub msg_write: u16,
    pub msg_invalid: u16,
}

/// The word at `at`, a byte past the end reading as zero — which is what
/// the segment's padding past the file holds.
fn word(b: &[u8], at: usize) -> u16 {
    let byte = |i: Option<usize>| i.and_then(|i| b.get(i)).copied().unwrap_or(0);
    u16::from_le_bytes([byte(Some(at)), byte(at.checked_add(1))])
}

/// The word `v` written at `at` of a buffer this module sized itself.
fn put(b: &mut [u8], at: usize, v: u16) {
    if let Some(dst) = at.checked_add(2).and_then(|end| b.get_mut(at..end)) {
        dst.copy_from_slice(&v.to_le_bytes());
    }
}

/// The byte `v` written at `at`, as [`put`] writes a word.
fn set(b: &mut [u8], at: usize, v: u8) {
    if let Some(dst) = b.get_mut(at) {
        *dst = v;
    }
}

/// The offsets of `n` consecutive words from `from`.
fn words_from(from: usize, n: usize) -> impl Iterator<Item = usize> {
    (from..).step_by(2).take(n)
}

/// `1EB5h:000Ah(lo, hi)`: the generator's seed set (`0x1EB5A`).
fn srand(rng: &mut LcRng, lo: u16, hi: u16) {
    rng.lo = lo;
    rng.hi = hi;
}

/// The saved game's sum and xor over a buffer, from the tick at `+0Ah`.
///
/// The tick seeds the generator (`1EB5h:000Ah` at `0x14A2D`); its first
/// draw `rnd(1, FFFEh)` starts the sum, sign-extended (`0x14A39`), and its
/// second starts the xor (`0x14A51`). Each word of the walk is then added
/// into the sum at `+0Eh` sign-extended, with the carry into `+10h`, and
/// xored into `+12h` (`0x14A9E`–`0x14AAC`).
///
/// `buf` is the file's bytes; the walk's last nine bytes are the
/// segment's padding, zero.
pub(crate) fn sav_check(buf: &[u8], map: &SaveMap, rng: &mut LcRng) -> (u32, u16) {
    srand(rng, word(buf, 0x0A), word(buf, 0x0C));
    let r1 = rng.inclusive(1, 0xFFFE);
    let r2 = rng.inclusive(1, 0xFFFE);
    let mut sum = num::unsigned32(i32::from(num::signed(r1)));
    let mut xor = r2;
    let file = buf.get(..map.len).unwrap_or(buf);
    for at in words_from(map.sum.0, map.sum.1) {
        let w = word(file, at);
        sum = sum.wrapping_add(num::unsigned32(i32::from(num::signed(w))));
        xor ^= w;
    }
    (sum, xor)
}

/// The settings' check value (`147Ah:0007h`, `0x147A7`): the tick as the
/// start, then for each of the six words from `+12h` the word added —
/// unsigned, `adc 0` at `0x147E4` — and the sum multiplied by 997
/// (`0x147F7`, the compiler's long multiply). The tick also seeds the
/// generator first (`0x147BF`).
pub(crate) fn cfg_check(cfg: &[u8], words: usize, rng: &mut LcRng) -> u32 {
    let (lo, hi) = (word(cfg, 0x0A), word(cfg, 0x0C));
    srand(rng, lo, hi);
    let mut v = (u32::from(hi) << 16) | u32::from(lo);
    for at in words_from(0x12, words) {
        v = v.wrapping_add(u32::from(word(cfg, at)));
        v = v.wrapping_mul(CFG_FACTOR);
    }
    v
}

/// Whether a file read at the start is a saved game: the size the reader
/// asks for, then the sum, the xor and the magic (`0x149A3`–`0x149C5`).
/// The generator is reseeded either way, as the original reseeds it before
/// it compares.
pub(crate) fn sav_valid(buf: &[u8], map: &SaveMap, rng: &mut LcRng) -> bool {
    if buf.len() != map.len {
        return false;
    }
    let (sum, xor) = sav_check(buf, map, rng);
    let stored = (u32::from(word(buf, 0x10)) << 16) | u32::from(word(buf, 0x0E));
    sum == stored && xor == word(buf, 0x12) && magic_is(buf, &map.sav_magic)
}

/// Whether the four words at the start of `b` are `magic`.
fn magic_is(b: &[u8], magic: &[u16; 4]) -> bool {
    words_from(0, 4).zip(magic).all(|(at, &m)| word(b, at) == m)
}

/// What the start does with a settings file it found (`0x1485B`–`0x148D7`).
pub(crate) enum Cfg {
    /// Not the size, not the magic, or not the check value: passed over.
    Ignored,
    /// Taken: the mouse setting, the four text words, the walk speed when
    /// the file carries it.
    Taken {
        mouse: u16,
        text: [u8; 8],
        speed: Option<u8>,
    },
}

/// Reads a settings file. The magic is compared first and the check value,
/// which reseeds the generator, only after it (`0x148A6`).
pub(crate) fn cfg_read(cfg: &[u8], map: &SaveMap, rng: &mut LcRng) -> Cfg {
    if cfg.len() != map.cfg_len || !magic_is(cfg, &map.cfg_magic) {
        return Cfg::Ignored;
    }
    let v = cfg_check(cfg, map.cfg_words, rng);
    if (u32::from(word(cfg, 0x10)) << 16) | u32::from(word(cfg, 0x0E)) != v {
        return Cfg::Ignored;
    }
    let Some(&text) = cfg.get(0x14..0x1C).and_then(|t| t.first_chunk::<8>()) else {
        return Cfg::Ignored;
    };
    let speed = if map.cfg_speed {
        let Some(&s) = cfg.get(0x1C) else {
            return Cfg::Ignored;
        };
        Some(s)
    } else {
        None
    };
    Cfg::Taken {
        mouse: word(cfg, 0x12),
        text,
        speed,
    }
}

impl Adventure {
    /// A string of the program's data segment, as the program has it.
    fn ds_text(&self, off: u16) -> String {
        self.image
            .c_string(self.image.at(usize::from(off)))
            .unwrap_or_default()
    }

    /// The file name `0590h:0DD5h` builds: a leading `*` means the name as
    /// it stands, in the current directory (`0x66D5`).
    pub(crate) fn save_file_name(&self, off: u16) -> String {
        let name = self.ds_text(off);
        name.strip_prefix('*')
            .map_or_else(|| name.clone(), str::to_string)
    }

    /// One of the file routines' fatal messages, with the file's name put
    /// where the message has its `%s`, and the line end taken off.
    pub(crate) fn save_message(&self, off: u16, name: &str) -> String {
        self.ds_text(off)
            .replacen("%s", name, 1)
            .trim_end()
            .to_string()
    }

    /// `147Ah:0072h`'s first act: the buffer cleared, the file's length
    /// (`0x14821`, `1EE7h:0004h`).
    pub(crate) fn save_buffer_clear(&mut self, map: &SaveMap) {
        for i in 0..map.len {
            self.save_buffer_set(i, 0);
        }
    }

    /// A file read at the start, put into the buffer (`0590h:0BD2h` at
    /// `0x14922`).
    pub(crate) fn save_buffer_fill(&mut self, map: &SaveMap, bytes: &[u8]) {
        for (i, &b) in bytes.iter().take(map.len).enumerate() {
            self.save_buffer_set(i, b);
        }
    }

    /// `147Ah:044Eh` (`0x14BEE`), the save. Answers the file's bytes when
    /// there is something to write: nothing before the first location has
    /// been entered (`0x14BF2`, `[1F4Ch] < 0`).
    pub(crate) fn save_044e(&mut self, map: &SaveMap, tick: u32) -> Option<Vec<u8>> {
        let here = self.word(self.map().here);
        if num::signed(here) < 0 {
            return None;
        }
        // 0x14C07: the location's objects into its own row.
        let row = self.ds(self.map().object_states.wrapping_add(here << 5));
        self.objs_save_state(row);
        // 0x14C0F: the location pushed.
        let depth = self.word(map.stack).wrapping_add(1);
        self.set_word(map.stack, depth);
        self.set_word(map.stack.wrapping_add(depth << 1), here);
        let seg = self.map().save_buffer;
        for b in map.blocks {
            let src = self.ds(b.ds);
            self.movmem(src, Far { off: b.buf, seg }, b.out);
        }
        // 0x14CA3: popped again; a save is wanted and a game is held.
        let depth = self.word(map.stack).wrapping_sub(1);
        self.set_word(map.stack, depth);
        self.set_word(map.saved, 1);
        self.set_word(map.loaded, 1);
        self.save_0241(map, tick)
    }

    /// `147Ah:0241h` (`0x149E1`): the header, the checksum, the bytes. The
    /// tick at `+0Ah` is the BIOS tick, `0:046Ch` low and `0:046Eh` high
    /// (`0x14A11`, `0x14A15`); the file's last byte, `+8B0h`, is cleared
    /// (`0x14A60`); and the
    /// word at `+8` is `[1718h]` (`0x14A7D`).
    fn save_0241(&mut self, map: &SaveMap, tick: u32) -> Option<Vec<u8>> {
        if self.word(map.saved) == 0 {
            return None;
        }
        let mut buf = self.save_buffer().get(..map.len)?.to_vec();
        put(&mut buf, 0x0A, num::low_word(tick));
        put(&mut buf, 0x0C, num::high_word(tick));
        set(&mut buf, map.len.checked_sub(1)?, 0);
        for (at, &m) in words_from(0, 4).zip(&map.sav_magic) {
            put(&mut buf, at, m);
        }
        put(&mut buf, 0x08, self.word(map.version));
        let (sum, xor) = sav_check(&buf, map, &mut self.rng);
        put(&mut buf, 0x0E, num::low_word(sum));
        put(&mut buf, 0x10, num::high_word(sum));
        put(&mut buf, 0x12, xor);
        self.save_buffer_fill(map, &buf);
        Some(buf)
    }

    /// `147Ah:0373h` (`0x14B13`): the settings file, when a setting
    /// changed.
    pub(crate) fn cfg_0373(&mut self, map: &SaveMap, tick: u32) -> Option<Vec<u8>> {
        if self.word(map.cfg_dirty) == 0 {
            return None;
        }
        let mut cfg = vec![0u8; map.cfg_len];
        for (at, &m) in words_from(0, 4).zip(&map.cfg_magic) {
            put(&mut cfg, at, m);
        }
        put(&mut cfg, 0x08, self.word(map.version));
        put(&mut cfg, 0x12, self.word(map.mouse));
        for (i, at) in (0x14..0x1C).enumerate() {
            let b = self.byte(map.text.wrapping_add(num::word(i)));
            set(&mut cfg, at, b);
        }
        // 0x14B68: the speed byte as a word, when the file carries it;
        // then the byte 0 (`0x14B70`), the file's last.
        if map.cfg_speed {
            put(&mut cfg, 0x1C, u16::from(self.byte(map.speed)));
        }
        set(&mut cfg, map.cfg_len.saturating_sub(1), 0);
        put(&mut cfg, 0x0A, num::low_word(tick));
        put(&mut cfg, 0x0C, num::high_word(tick));
        let v = cfg_check(&cfg, map.cfg_words, &mut self.rng);
        put(&mut cfg, 0x0E, num::low_word(v));
        put(&mut cfg, 0x10, num::high_word(v));
        Some(cfg)
    }

    /// The settings taken from a file (`0x148B6`–`0x148D7`).
    pub(crate) fn cfg_take(&mut self, map: &SaveMap, mouse: u16, text: [u8; 8], speed: Option<u8>) {
        self.set_word(map.mouse, mouse);
        self.mouse_divisors_05dc(map);
        for (i, &b) in text.iter().enumerate() {
            self.set_byte(map.text.wrapping_add(num::word(i)), b);
        }
        if let Some(speed) = speed {
            self.set_byte(map.speed, speed);
        }
    }

    /// `147Ah:05DCh` (`0x14D7C`): the mickey divisors by the mouse setting,
    /// through the table at `cs:062Dh`. A setting past 4 becomes 1.
    pub(crate) fn mouse_divisors_05dc(&mut self, map: &SaveMap) {
        let (v, h) = match self.word(map.mouse) {
            0 => (10, 10),
            1 => (20, 20),
            2 => (40, 40),
            3 => (40, 20),
            4 => (20, 40),
            _ => {
                self.set_word(map.mouse, 1);
                (20, 20)
            }
        };
        self.set_word(map.divisors.0, v);
        self.set_word(map.divisors.1, h);
    }

    /// `147Ah:0518h` (`0x14CB8`): loading in game. No file is read: with a
    /// game in the buffer, the main loop is sent to the restore row.
    pub(crate) fn load_0518(&mut self, map: &SaveMap) {
        if self.word(map.loaded) != 0 {
            let m = self.map();
            self.set_word(m.location, map.restore_row);
            self.set_word(m.entry, 1);
        }
    }

    /// The generator, for the settings' and the save's checks.
    pub(crate) fn rng_mut(&mut self) -> &mut LcRng {
        &mut self.rng
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_check_is_seeded_by_the_tick_and_multiplies_by_997() {
        let mut cfg = vec![0u8; 0x1F];
        put(&mut cfg, 0x0A, 0x1234);
        put(&mut cfg, 0x0C, 0x0005);
        put(&mut cfg, 0x12, 1);
        let mut rng = LcRng::default();
        let v = cfg_check(&cfg, 6, &mut rng);
        let mut want: u32 = 0x0005_1234;
        for w in [1u32, 0, 0, 0, 0, 0] {
            want = want.wrapping_add(w).wrapping_mul(997);
        }
        assert_eq!(v, want);
        assert_eq!((rng.lo, rng.hi), (0x1234, 0x0005), "reseeded, not rolled");
    }

    #[test]
    fn the_save_sum_takes_each_word_signed_and_the_padding_as_zero() {
        let mut buf = vec![0u8; crate::titles::ds1::build::SAVE.len];
        put(&mut buf, 0x0A, 7);
        put(&mut buf, 0x14, 0xFFFF);
        buf[0x8B0] = 0x80;
        let mut rng = LcRng::default();
        let (sum, xor) = sav_check(&buf, &crate::titles::ds1::build::SAVE, &mut rng);
        let mut seed = LcRng::default();
        srand(&mut seed, 7, 0);
        let r1 = seed.inclusive(1, 0xFFFE);
        let r2 = seed.inclusive(1, 0xFFFE);
        let start = num::unsigned32(i32::from(num::signed(r1)));
        // 0xFFFF is -1; the last word is 0080h, its high byte the padding.
        assert_eq!(sum, start.wrapping_sub(1).wrapping_add(0x80));
        assert_eq!(xor, r2 ^ 0xFFFF ^ 0x0080);
    }
}
