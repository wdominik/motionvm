//! The record the game keeps its walkers in.
//!
//! A walker is not a thing beside the objects — it **is** an object
//! record, the same 32 bytes as everything else a room draws, and the
//! program keeps two of them ready in its data segment. They read as an
//! object the way the object drawer (`0BB6h:000Ah`) reads one: kind
//! `0103h`, live, align 2 (center the cel on `x` and put its foot on `y`),
//! standing at `(100, 100)`, drawing cel 0 out of **bundle 1** — which is
//! the file the game loads its figure from — at scale `0100h`, with id
//! `0FFh`. The second record is the same with bundle 2 and id `0FEh`.
//!
//! The last two bytes are the gait. `+14h` is the interval: the leg
//! setup multiplies an edge's length by it (`0x0AF0B`–`0x0AF10`), and the
//! animator puts it back on the countdown at each phase step
//! (`0x08AF7`). `+15h` is how many phases a direction block has
//! (`0x0ABE2`), the block holding one more cel, the standing one. The
//! mover pass `0A9Fh:0892h` and the animator `0883h:0305h`, run on these
//! bytes under an 8086 emulator, are measured in `walker.rs`.
//!
//! | | player | interval | phases | second mover |
//! |---|---|---:|---:|---|
//! | Dunkle Schatten | `DS:1693` | 13 | 6 | `DS:16C3` |

use crate::exe::Image;
use crate::num;

use crate::error::Result;

/// The kind word every walker record opens with.
const KIND: u16 = 0x0103;
/// Where a fresh walker stands before a room puts it anywhere.
const HOME: u16 = 100;
/// Its scale, in the `0100h`-is-one the object drawer multiplies by.
const SCALE: u16 = 0x0100;
/// The id of the record the player uses.
const PLAYER_ID: u8 = 0xFF;
/// The id of the second one.
const SECOND_ID: u8 = 0xFE;

/// One walker record, as the program ships it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mover {
    /// Where the record sits, as an offset into the program's data
    /// segment — the address the program's code uses for it.
    pub at: u16,
    /// The bundle its cels come from. 1 for the player.
    pub bundle: u8,
    /// How the cel sits on the position: 2 is centered with its foot on
    /// `y`, which is what both records carry.
    pub align: u8,
    /// Its id, `0FFh` for the player and `0FEh` for the second.
    pub id: u8,
    /// Byte `+14h`, the interval: what the leg cost is built from, and
    /// what the animator puts back on the countdown per phase.
    pub interval: u8,
    /// Byte `+15h`, walk phases per direction; a
    /// direction block holds one more cel than that, the standing one.
    pub phases: u8,
}

/// `mov ax, es:[bx+0Ah]` / `pop dx` / `imul dx` / `cwd` — the leg setup
/// taking an edge's length and multiplying it by the walker's interval.
const LEG_PRODUCT: [u8; 8] = [0x26, 0x8B, 0x47, 0x0A, 0x5A, 0xF7, 0xEA, 0x99];
/// `push dx` / `push ax`, the product going out as a long.
const PUSH_PRODUCT: [u8; 2] = [0x52, 0x50];

/// The dividend the program builds its t-step from.
///
/// The leg setup multiplies an edge's length by the walker's interval,
/// truncates the product to sixteen bits, and hands both it and a constant
/// to the runtime's long divide at `0000h:01E2h`. That helper takes its
/// **first** long from the last two words pushed and divides it by the
/// second, so the constant is the dividend and the product the divisor —
/// which is why a long edge costs more ticks than a short one.
///
/// Dunkle Schatten's is `2FFFAh` (`0x0AF15`). It is pushed as two words,
/// high then low, so the low word is the one pushed last.
pub(crate) fn leg_dividend(img: &Image) -> Result<i32> {
    let b = img.bytes();
    let mut seen: Option<i32> = None;
    for i in 0..b.len().saturating_sub(0x14) {
        if b[i..i + 8] != LEG_PRODUCT || b[i + 8..i + 10] != PUSH_PRODUCT {
            continue;
        }
        // Two pushes of an immediate, high word first.
        let (hi, n) = push_imm(b, i + 10).ok_or("no dividend after the product")?;
        let (lo, m) = push_imm(b, i + 10 + n).ok_or("only half a dividend")?;
        if b[i + 10 + n + m] != 0x9A {
            continue;
        }
        let v = (i32::from(hi) << 16) | i32::from(num::unsigned(lo));
        match seen {
            None => seen = Some(v),
            Some(w) if w == v => {}
            Some(w) => {
                return Err(format!(
                    "the leg is built from both {w:#x} and {v:#x}; \
                     which is the walk's is not decidable"
                )
                .into());
            }
        }
    }
    seen.ok_or_else(|| "the program never builds a leg from an edge's length".into())
}

/// A `push imm8`/`push imm16` at `i`, as `(value, length)`.
fn push_imm(b: &[u8], i: usize) -> Option<(i16, usize)> {
    match b.get(i)? {
        0x6A => Some((i16::from(num::signed8(*b.get(i + 1)?)), 2)),
        0x68 => Some((i16::from_le_bytes([*b.get(i + 1)?, *b.get(i + 2)?]), 3)),
        _ => None,
    }
}

/// The two walkers `img` ships, player first.
///
/// Found by the shape rather than by an address, and required to be
/// unambiguous: exactly one record with each id, or the program is
/// refused. Every field checked here is one the object drawer reads, so
/// a record that matched by accident would have to be an object that
/// stands at `(100, 100)` unscaled with an id no room uses.
pub(crate) fn movers(img: &Image) -> Result<(Mover, Mover)> {
    let ds = img.at(0);
    let one = |id: u8| -> Result<Mover> {
        let mut found: Option<Mover> = None;
        let b = img.bytes();
        for a in ds..b.len().saturating_sub(0x16) {
            let shape = u16::from_le_bytes([b[a], b[a + 1]]) == KIND
                && b[a + 2] == 1
                && b[a + 3] == 2
                && u16::from_le_bytes([b[a + 4], b[a + 5]]) == HOME
                && u16::from_le_bytes([b[a + 6], b[a + 7]]) == HOME
                && u16::from_le_bytes([b[a + 0x0A], b[a + 0x0B]]) == 0
                && u16::from_le_bytes([b[a + 0x0E], b[a + 0x0F]]) == SCALE
                && u16::from_le_bytes([b[a + 0x10], b[a + 0x11]]) == SCALE
                && b[a + 0x13] == id;
            if !shape {
                continue;
            }
            let Ok(off) = u16::try_from(a - ds) else {
                continue;
            };
            let m = Mover {
                at: off,
                bundle: b[a + 8],
                align: b[a + 3],
                id,
                interval: b[a + 0x14],
                phases: b[a + 0x15],
            };
            match found {
                None => found = Some(m),
                Some(prev) => {
                    return Err(format!(
                        "two walker records carry id {id:#04X}, at DS:{:04X} and DS:{off:04X}; \
                         which is the program's is not decidable",
                        prev.at
                    )
                    .into());
                }
            }
        }
        found.ok_or_else(|| format!("no walker record with id {id:#04X}").into())
    };
    let player = one(PLAYER_ID)?;
    if player.bundle != 1 {
        return Err(format!(
            "the player's walker draws from bundle {}, and bundle 1 is where a title \
             loads its figure",
            player.bundle
        )
        .into());
    }
    Ok((player, one(SECOND_ID)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dunkle Schatten ships two walker records.
    ///
    /// The shape is what makes them findable: an object standing at
    /// `(100, 100)` at scale one, drawing cel 0, with an id no room uses.
    /// The bundle each draws from is 1 and 2, the two figure files the
    /// game loads, and the two animation bytes are read rather than
    /// written down.
    #[test]
    fn the_program_ships_a_player_and_a_second_mover() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        let img = Image::open_any(&dir, "XP01.EXE").expect("Dunkle Schatten");
        let (player, second) = movers(&img).expect("Dunkle Schatten");
        assert_eq!(player.at, 0x1693, "player");
        assert_eq!(player.bundle, 1, "player bundle");
        assert_eq!(player.align, 2, "player align");
        assert_eq!(player.id, 0xFF, "player id");
        assert_eq!((player.interval, player.phases), (13, 6), "gait");
        assert_eq!(second.at, 0x16C3, "second mover");
        assert_eq!(second.bundle, 2, "second bundle");
        assert_eq!(second.id, 0xFE, "second id");
    }

    /// The t-step is built from the program's own constant.
    ///
    /// The leg setup multiplies an edge's length by the walker's interval
    /// and divides a constant **by** that product — so a long edge costs
    /// more ticks than a short one.
    #[test]
    fn a_leg_is_reached_with_the_programs_constant() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        let img = Image::open_any(&dir, "XP01.EXE").expect("Dunkle Schatten");
        assert_eq!(leg_dividend(&img).expect("Dunkle Schatten"), 0x2_FFFA);
    }
}
