//! The two comparisons against the original's own output: a frame against a
//! capture of the original's screen, and a register stream against a DRO
//! recording of the original driving an OPL chip.
//!
//! Both compare two files and nothing else. The frame on our side is the one
//! F12 writes — indices and a palette, the bytes the engine composed — and
//! the register stream is what `registers` renders offline out of the game's
//! own files. The original's side is DOSBox-X's: its screenshot of a
//! 256-color mode, or its `DX-CAPTURE /O` recording. Neither comparison knows
//! an engine, and neither computes anything of the game: what they do is line
//! two files up and name the first place they part. How the captures are
//! made is written down in `docs/motion/verification-method.md`.
//!
//! **A frame is compared on colors as the DAC held them**, six bits a channel.
//! The picture F12 writes is indexed and so is DOSBox-X's own capture, but the
//! two palettes need not agree entry for entry — a scene can hold one color
//! under two indices, and a capture may have been through a tool that
//! renumbered — while what the player saw is the color. So every pixel of both
//! is taken to the six bits the VGA DAC stores, through its own palette, and
//! those are compared; a pixel's index is reported beside its color where the
//! file had one. A screenshot in RGB goes the same way, with the bottom two
//! bits of each channel dropped, which is exactly what a capture that widened
//! six bits to eight put there.
//!
//! **A register stream is compared write for write**, after two things the
//! recorder does that a driver does not. DOSBox-X opens the file at the first
//! note and starts it with a dump of every register the chip holds, in
//! register order — a state, not a sequence — and records afterwards only the
//! writes that *change* a register. So both sides are reduced to changes from
//! a chip that starts at zero, the recording's snapshot is set apart at the
//! first register it names twice, and the two streams are lined up on the
//! write the recording resumes with — the occurrence in ours that agrees
//! longest, so that a write which matches by accident does not decide the
//! comparison. The snapshot is then held against our state at that point,
//! register for register, before the streams are.

use motionvm_render::{Decoded, Pixels, read_png};
use std::fmt::Write as _;
use std::path::Path;

// ------------------------------------------------------------------ frames

/// A window into the original's capture: the picture area of a screenshot
/// that shows more than the emulator's frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Crop {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// A picture reduced to what the comparison looks at: its size, every pixel's
/// color as the DAC held it, and the index the file had for it, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Reduced {
    width: u32,
    height: u32,
    /// Six bits a channel.
    colors: Vec<[u8; 3]>,
    /// The file's own index for each pixel, for an indexed picture.
    indices: Option<Vec<u8>>,
    /// The palette's colors, six bits a channel, for an indexed picture.
    palette: Vec<[u8; 3]>,
}

fn six(rgb8: [u8; 3]) -> [u8; 3] {
    rgb8.map(|c| c >> 2)
}

impl Reduced {
    /// The whole picture, as the file had it.
    fn of(picture: &Decoded) -> Self {
        let (width, height) = (picture.width, picture.height);
        match &picture.pixels {
            Pixels::Indexed { indices, palette } => {
                let palette: Vec<[u8; 3]> = palette.iter().map(|&c| six(c)).collect();
                Self {
                    width,
                    height,
                    colors: indices
                        .iter()
                        .map(|&i| palette.get(usize::from(i)).copied().unwrap_or([0; 3]))
                        .collect(),
                    indices: Some(indices.clone()),
                    palette,
                }
            }
            Pixels::Rgb(pixels) => Self {
                width,
                height,
                colors: pixels.iter().map(|&c| six(c)).collect(),
                indices: None,
                palette: Vec::new(),
            },
        }
    }

    /// The picture inside `crop`, then every `scale`-th pixel of it — the
    /// way a screenshot of a pixel-doubled window comes back to the frame.
    fn window(&self, crop: Option<Crop>, scale: u32) -> Result<Self, String> {
        let crop = crop.unwrap_or(Crop {
            x: 0,
            y: 0,
            w: self.width,
            h: self.height,
        });
        if crop.x + crop.w > self.width || crop.y + crop.h > self.height {
            return Err(format!(
                "--crop {},{},{},{} reaches outside a picture {} by {}",
                crop.x, crop.y, crop.w, crop.h, self.width, self.height
            ));
        }
        if scale == 0 {
            return Err("--scale 0 keeps no pixel".into());
        }
        let (width, height) = (crop.w / scale, crop.h / scale);
        let at = |x: u32, y: u32| {
            usize::try_from((crop.y + y * scale) * self.width + crop.x + x * scale)
                .unwrap_or(usize::MAX)
        };
        let mut colors = Vec::with_capacity(usize::try_from(width * height).unwrap_or(0));
        let mut indices = self.indices.as_ref().map(|_| Vec::new());
        for y in 0..height {
            for x in 0..width {
                colors.push(self.colors[at(x, y)]);
                if let (Some(out), Some(all)) = (indices.as_mut(), self.indices.as_ref()) {
                    out.push(all[at(x, y)]);
                }
            }
        }
        Ok(Self {
            width,
            height,
            colors,
            indices,
            palette: self.palette.clone(),
        })
    }
}

/// One pixel, as a report names it: its position, and on each side the
/// color and the file's index where the file had one.
fn pixel(picture: &Reduced, at: usize) -> String {
    let [r, g, b] = picture.colors[at].map(|c| (c << 2) | (c >> 4));
    match &picture.indices {
        Some(indices) => format!("index {} = #{r:02x}{g:02x}{b:02x}", indices[at]),
        None => format!("#{r:02x}{g:02x}{b:02x}"),
    }
}

/// What `compare-frame` found, ready to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrameReport {
    /// Whether the two show the same picture.
    pub same: bool,
    /// The lines to print.
    pub text: String,
}

/// F12's picture against the original's, reduced as described at the top.
fn frames(ours: &Reduced, theirs: &Reduced) -> FrameReport {
    let mut text = String::new();
    if (ours.width, ours.height) != (theirs.width, theirs.height) {
        let _ = writeln!(
            text,
            "the pictures differ in size: ours {} by {}, theirs {} by {} — a capture of a \
             larger window wants --crop and --scale",
            ours.width, ours.height, theirs.width, theirs.height
        );
        return FrameReport { same: false, text };
    }
    let pixels = ours.colors.len();
    let mut differing = 0usize;
    let mut first = None;
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    let mut foreign = 0usize;
    for (at, (a, b)) in ours.colors.iter().zip(&theirs.colors).enumerate() {
        if a == b {
            continue;
        }
        differing += 1;
        let (x, y) = (
            u32::try_from(at).unwrap_or(0) % ours.width,
            u32::try_from(at).unwrap_or(0) / ours.width,
        );
        first.get_or_insert((x, y, at));
        bounds = Some(match bounds {
            None => (x, y, x, y),
            Some((x1, y1, x2, y2)) => (x1.min(x), y1.min(y), x2.max(x), y2.max(y)),
        });
        if !ours.palette.contains(b) {
            foreign += 1;
        }
    }
    let Some((x, y, at)) = first else {
        let _ = writeln!(
            text,
            "the same picture: {pixels} pixels, {} by {}, agree on every color",
            ours.width, ours.height
        );
        return FrameReport { same: true, text };
    };
    let (x1, y1, x2, y2) = bounds.unwrap_or((x, y, x, y));
    let _ = writeln!(
        text,
        "{differing} of {pixels} pixels differ, inside ({x1},{y1})-({x2},{y2})"
    );
    let _ = writeln!(
        text,
        "the first at ({x},{y}): ours {}, theirs {}",
        pixel(ours, at),
        pixel(theirs, at)
    );
    if foreign != 0 {
        let _ = writeln!(
            text,
            "{foreign} of the differing pixels show a color our palette does not hold at all"
        );
    }
    FrameReport { same: false, text }
}

/// `compare-frame`: the two files, reduced and compared; the report on
/// stdout, and whether they agree.
pub(crate) fn frame(
    ours: &Path,
    theirs: &Path,
    crop: Option<Crop>,
    scale: u32,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mine = read_png(ours).map_err(|e| format!("{}: {e}", ours.display()))?;
    if !matches!(mine.pixels, Pixels::Indexed { .. }) {
        return Err(format!(
            "{}: not an indexed picture; the one to hold against the original is the \
             frame F12 writes",
            ours.display()
        )
        .into());
    }
    let capture = read_png(theirs).map_err(|e| format!("{}: {e}", theirs.display()))?;
    let theirs = Reduced::of(&capture).window(crop, scale)?;
    let report = frames(&Reduced::of(&mine), &theirs);
    print!("{}", report.text);
    Ok(report.same)
}

// ---------------------------------------------------- register streams

/// One register write, on either side: where it went and what it was.
pub(crate) type Write = (u16, u8);

/// A DRO v2 recording: the writes, with the time each was made at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Recording {
    /// The chip the recorder had: 0 OPL2, 1 OPL3, 2 dual OPL2.
    pub hardware: u8,
    /// How long it ran, in milliseconds, by its own header.
    pub length_ms: u32,
    /// The writes in order, each with the millisecond it fell in.
    pub writes: Vec<(u32, u16, u8)>,
}

/// Reads a DRO version 2 file, DOSBox-X's format for `DX-CAPTURE /O`.
///
/// The header: the signature, the version as two `u16`s, the count of pairs
/// and the length in milliseconds, the hardware type, the format, the
/// compression, and then the two delay codes and the length of the code map
/// — the delay codes are **not** 0 and 1, whatever third-party descriptions
/// say; they sit at `0x17` and `0x18`, after which the code map maps every
/// other code to a register. A code's top bit selects the second register
/// bank.
pub(crate) fn read_dro(bytes: &[u8]) -> Result<Recording, String> {
    if bytes.get(..8) != Some(b"DBRAWOPL".as_slice()) {
        return Err("not a DRO recording (no DBRAWOPL signature)".into());
    }
    let u16_at = |at: usize| {
        bytes
            .get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .ok_or_else(|| String::from("the header ends early"))
    };
    let u32_at = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| String::from("the header ends early"))
    };
    let (major, minor) = (u16_at(8)?, u16_at(10)?);
    if major != 2 {
        return Err(format!(
            "DRO version {major}.{minor}; only version 2 is read"
        ));
    }
    let pairs = usize::try_from(u32_at(12)?).unwrap_or(usize::MAX);
    let length_ms = u32_at(16)?;
    let byte = |at: usize| {
        bytes
            .get(at)
            .copied()
            .ok_or_else(|| String::from("the header ends early"))
    };
    let (hardware, format, compression) = (byte(20)?, byte(21)?, byte(22)?);
    if format != 0 || compression != 0 {
        return Err(format!(
            "DRO format {format}, compression {compression}; only the interleaved, \
             uncompressed kind is read"
        ));
    }
    let (short, long, table_len) = (byte(23)?, byte(24)?, usize::from(byte(25)?));
    let table = bytes
        .get(26..26 + table_len)
        .ok_or_else(|| String::from("the code map ends early"))?;
    let mut writes = Vec::new();
    let mut at = 26 + table_len;
    let mut now = 0u32;
    for _ in 0..pairs {
        let Some(&[code, value]) = bytes.get(at..at + 2).and_then(|p| p.first_chunk::<2>()) else {
            return Err(format!(
                "the file ends after {} of the {pairs} pairs its header counts",
                writes.len()
            ));
        };
        at += 2;
        if code == short {
            now = now.saturating_add(u32::from(value) + 1);
        } else if code == long {
            now = now.saturating_add((u32::from(value) + 1) * 256);
        } else {
            let &register = table
                .get(usize::from(code & 0x7f))
                .ok_or_else(|| format!("code {code:#04x} is outside the code map"))?;
            let register = u16::from(register) | (u16::from(code & 0x80) << 1);
            writes.push((now, register, value));
        }
    }
    Ok(Recording {
        hardware,
        length_ms,
        writes,
    })
}

/// One line of a registers file: the tick a write fell on, the register and
/// the value, the register and value in hex.
pub(crate) fn write_line(out: &mut String, tick: u32, register: u16, value: u8) {
    let _ = writeln!(out, "{tick} {register:#05x} {value:#04x}");
}

/// Reads a registers file back: every line that is not blank or a `#`
/// comment is a tick, a register and a value.
pub(crate) fn read_registers(text: &str) -> Result<Vec<(u32, u16, u8)>, String> {
    let number = |s: &str| -> Option<u32> {
        s.strip_prefix("0x")
            .map_or_else(|| s.parse().ok(), |h| u32::from_str_radix(h, 16).ok())
    };
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        let parsed = match fields[..] {
            [t, r, v] => number(t).zip(number(r)).zip(number(v)),
            _ => None,
        };
        let Some(((tick, register), value)) = parsed else {
            return Err(format!("line {}: not `tick register value`: {line}", n + 1));
        };
        let (Ok(register), Ok(value)) = (u16::try_from(register), u8::try_from(value)) else {
            return Err(format!(
                "line {}: a register or value out of range: {line}",
                n + 1
            ));
        };
        out.push((tick, register, value));
    }
    Ok(out)
}

/// Drops every write that leaves a register holding what it already held,
/// from a chip that starts at zero — the recorder's own rule.
pub(crate) fn changes_only(writes: impl IntoIterator<Item = Write>) -> Vec<Write> {
    let mut state = [0u8; 0x200];
    let mut out = Vec::new();
    for (register, value) in writes {
        let slot = &mut state[usize::from(register & 0x1ff)];
        if *slot != value {
            *slot = value;
            out.push((register, value));
        }
    }
    out
}

/// How the two streams line up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Alignment {
    /// Where the recording's snapshot ends and its live writes begin.
    pub resume: usize,
    /// Our write the recording resumes with.
    pub ours_at: usize,
    /// Registers the snapshot has one way and our state at `ours_at` another:
    /// the register, the recording's value, and ours if we ever wrote it.
    pub state_differs: Vec<(u16, u8, Option<u8>)>,
}

/// Lines a rendered stream up with a recording, both already reduced to
/// changes.
pub(crate) fn align(ours: &[Write], theirs: &[Write]) -> Result<Alignment, String> {
    let mut seen = std::collections::HashSet::new();
    let resume = theirs
        .iter()
        .position(|&(r, _)| !seen.insert(r))
        .ok_or_else(|| {
            String::from(
                "the recording never writes a register twice: it is all snapshot and \
                 holds no stream to compare",
            )
        })?;
    let anchor = theirs[resume];
    let agreement = |at: usize| {
        ours[at..]
            .iter()
            .zip(&theirs[resume..])
            .take_while(|(a, b)| a == b)
            .count()
    };
    let ours_at = (0..ours.len())
        .filter(|&i| ours[i] == anchor)
        .max_by_key(|&i| agreement(i))
        .ok_or_else(|| {
            format!(
                "the recording resumes with register {:#05x} = {:#04x}, which we never write",
                anchor.0, anchor.1
            )
        })?;
    let state = |writes: &[Write]| {
        let mut s = std::collections::BTreeMap::new();
        for &(r, v) in writes {
            s.insert(r, v);
        }
        s
    };
    let mine = state(&ours[..ours_at]);
    let state_differs = state(&theirs[..resume])
        .into_iter()
        .filter(|(r, v)| mine.get(r) != Some(v))
        .map(|(r, v)| (r, v, mine.get(&r).copied()))
        .collect();
    Ok(Alignment {
        resume,
        ours_at,
        state_differs,
    })
}

/// What `compare-dro` found, ready to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StreamReport {
    /// Whether the streams agree over everything the recording holds.
    pub same: bool,
    /// The lines to print.
    pub text: String,
}

fn hex_writes(writes: &[Write]) -> String {
    writes
        .iter()
        .map(|(r, v)| format!("{r:#05x}={v:#04x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The rendered stream against the recording, as described at the top.
pub(crate) fn streams(ours_raw: &[Write], recording: &Recording) -> StreamReport {
    let mut text = String::new();
    let chip = match recording.hardware {
        0 => "OPL2",
        1 => "OPL3",
        2 => "two OPL2s",
        _ => "an unknown chip",
    };
    let _ = writeln!(
        text,
        "recording: {} writes over {:.1} s on {chip}",
        recording.writes.len(),
        f64::from(recording.length_ms) / 1000.0
    );
    let theirs = changes_only(recording.writes.iter().map(|&(_, r, v)| (r, v)));
    let ours = changes_only(ours_raw.iter().copied());
    let _ = writeln!(
        text,
        "ours:      {} writes, {} of them changes",
        ours_raw.len(),
        ours.len()
    );
    let alignment = match align(&ours, &theirs) {
        Ok(a) => a,
        Err(why) => {
            let _ = writeln!(text, "{why}");
            return StreamReport { same: false, text };
        }
    };
    let _ = writeln!(
        text,
        "the recording's snapshot at its first note holds {} registers; it resumes with \
         our write {}",
        alignment.resume, alignment.ours_at
    );
    if alignment.state_differs.is_empty() {
        let _ = writeln!(
            text,
            "the chip's state at that note agrees, register for register"
        );
    } else {
        let _ = writeln!(
            text,
            "the chip's state at that note differs in {} registers (register, theirs, ours):",
            alignment.state_differs.len()
        );
        for (r, v, mine) in &alignment.state_differs {
            let _ = writeln!(
                text,
                "  {r:#05x}  {v:#04x}  {}",
                mine.map_or_else(|| String::from("never written"), |m| format!("{m:#04x}"))
            );
        }
    }
    let mine = &ours[alignment.ours_at..];
    let recorded = &theirs[alignment.resume..];
    let compare = mine.len().min(recorded.len());
    match mine.iter().zip(recorded).position(|(a, b)| a != b) {
        None if mine.len() >= recorded.len() => {
            let _ = writeln!(
                text,
                "the streams agree for all {compare} writes the recording holds past its snapshot"
            );
            StreamReport {
                same: alignment.state_differs.is_empty(),
                text,
            }
        }
        None => {
            let _ = writeln!(
                text,
                "the streams agree for {compare} writes, and then ours ends; the recording \
                 holds {} more — render more ticks to compare the rest",
                recorded.len() - compare
            );
            StreamReport { same: false, text }
        }
        Some(at) => {
            let from = at.saturating_sub(4);
            let to = (at + 6).min(compare);
            let _ = writeln!(text, "the streams part at write {at} of {compare}:");
            let _ = writeln!(text, "  ours    {}", hex_writes(&mine[from..to]));
            let _ = writeln!(text, "  theirs  {}", hex_writes(&recorded[from..to]));
            StreamReport { same: false, text }
        }
    }
}

/// `compare-dro`: the two files, compared; the report on stdout, and whether
/// they agree.
pub(crate) fn dro(registers: &Path, capture: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let text =
        std::fs::read_to_string(registers).map_err(|e| format!("{}: {e}", registers.display()))?;
    let ours: Vec<Write> = read_registers(&text)
        .map_err(|e| format!("{}: {e}", registers.display()))?
        .into_iter()
        .map(|(_, r, v)| (r, v))
        .collect();
    let bytes = std::fs::read(capture).map_err(|e| format!("{}: {e}", capture.display()))?;
    let recording = read_dro(&bytes).map_err(|e| format!("{}: {e}", capture.display()))?;
    let report = streams(&ours, &recording);
    print!("{}", report.text);
    Ok(report.same)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indexed(width: u32, height: u32, indices: &[u8], palette: &[[u8; 3]]) -> Decoded {
        Decoded {
            width,
            height,
            pixels: Pixels::Indexed {
                indices: indices.to_vec(),
                palette: palette.to_vec(),
            },
        }
    }

    /// The same picture under a renumbered palette is the same picture; one
    /// pixel of another color is named with its position, both indices and
    /// the color; and a color our palette lacks is counted as such.
    #[test]
    fn frames_are_compared_on_dac_colors_not_on_indices() {
        let ours = indexed(
            3,
            2,
            &[0, 1, 2, 2, 1, 0],
            &[[252, 0, 0], [0, 252, 0], [0, 0, 252]],
        );
        // Their palette holds the same colors in another order, and widened
        // the other way — 255 where ours says 252 — which the DAC's six bits
        // do not tell apart.
        let renumbered = indexed(
            3,
            2,
            &[2, 1, 0, 0, 1, 2],
            &[[0, 0, 255], [0, 255, 0], [255, 0, 0]],
        );
        let report = frames(&Reduced::of(&ours), &Reduced::of(&renumbered));
        assert!(report.same, "{}", report.text);

        let one_off = indexed(
            3,
            2,
            &[2, 1, 0, 0, 1, 1],
            &[[0, 0, 255], [0, 255, 0], [255, 0, 0]],
        );
        let report = frames(&Reduced::of(&ours), &Reduced::of(&one_off));
        assert!(!report.same);
        assert!(
            report
                .text
                .contains("1 of 6 pixels differ, inside (2,1)-(2,1)"),
            "{}",
            report.text
        );
        assert!(
            report
                .text
                .contains("the first at (2,1): ours index 0 = #ff0000, theirs index 1 = #00ff00"),
            "{}",
            report.text
        );

        let foreign = Decoded {
            width: 3,
            height: 2,
            pixels: Pixels::Rgb(vec![
                [252, 0, 0],
                [0, 252, 0],
                [0, 0, 252],
                [0, 0, 252],
                [0, 252, 0],
                [9, 9, 9],
            ]),
        };
        let report = frames(&Reduced::of(&ours), &Reduced::of(&foreign));
        assert!(report.text.contains("theirs #080808"), "{}", report.text);
        assert!(
            report
                .text
                .contains("1 of the differing pixels show a color our palette does not hold"),
            "{}",
            report.text
        );
    }

    /// A screenshot of a pixel-doubled window, with a border around it, comes
    /// back to the frame through `--crop` and `--scale`.
    #[test]
    fn a_crop_and_a_scale_bring_a_screenshot_back_to_the_frame() {
        let ours = indexed(2, 1, &[0, 1], &[[0, 0, 0], [252, 252, 252]]);
        // A 6×4 screenshot: a one-pixel border of gray, then the frame doubled.
        let g = [128, 128, 128];
        let (k, w) = ([0, 0, 0], [255, 255, 255]);
        let shot = Decoded {
            width: 6,
            height: 4,
            pixels: Pixels::Rgb(vec![
                g, g, g, g, g, g, //
                g, k, k, w, w, g, //
                g, k, k, w, w, g, //
                g, g, g, g, g, g,
            ]),
        };
        let theirs = Reduced::of(&shot)
            .window(
                Some(Crop {
                    x: 1,
                    y: 1,
                    w: 4,
                    h: 2,
                }),
                2,
            )
            .unwrap();
        assert_eq!((theirs.width, theirs.height), (2, 1));
        assert!(frames(&Reduced::of(&ours), &theirs).same);
        assert!(
            Reduced::of(&shot)
                .window(
                    Some(Crop {
                        x: 3,
                        y: 1,
                        w: 4,
                        h: 2
                    }),
                    1
                )
                .is_err()
        );
        let report = frames(&Reduced::of(&ours), &Reduced::of(&shot));
        assert!(report.text.contains("differ in size"), "{}", report.text);
    }

    /// A DRO v2 file, built the way DOSBox-X lays one out, reads back with
    /// its times; the delay codes come out of the header, not out of a
    /// convention.
    fn dro_file(writes: &[(u32, u16, u8)]) -> Vec<u8> {
        // A code map of four registers, and delay codes chosen to be
        // anything but 0 and 1.
        let table = [0x01u8, 0xa0, 0xb0, 0x20];
        let (short, long) = (0x7e, 0x7f);
        let mut body = Vec::new();
        let mut now = 0u32;
        for &(at, register, value) in writes {
            let mut wait = at - now;
            while wait > 256 {
                let n = ((wait / 256).min(256) - 1).min(255);
                body.extend_from_slice(&[long, u8::try_from(n).unwrap()]);
                wait -= (n + 1) * 256;
            }
            if wait > 0 {
                body.extend_from_slice(&[short, u8::try_from(wait - 1).unwrap()]);
            }
            now = at;
            let code = table
                .iter()
                .position(|&r| u16::from(r) == register & 0xff)
                .unwrap();
            let code = u8::try_from(code).unwrap() | if register & 0x100 != 0 { 0x80 } else { 0 };
            body.extend_from_slice(&[code, value]);
        }
        let mut file = b"DBRAWOPL".to_vec();
        file.extend_from_slice(&2u16.to_le_bytes());
        file.extend_from_slice(&0u16.to_le_bytes());
        file.extend_from_slice(&u32::try_from(body.len() / 2).unwrap().to_le_bytes());
        file.extend_from_slice(&now.to_le_bytes());
        file.extend_from_slice(&[1, 0, 0, short, long, 4]);
        file.extend_from_slice(&table);
        file.extend_from_slice(&body);
        file
    }

    #[test]
    fn a_dro_recording_reads_back_with_its_times() {
        let writes = [
            (0, 0x001, 0x20),
            (0, 0x1a0, 0x41),
            (300, 0x0b0, 0x31),
            (1000, 0x120, 0x01),
        ];
        let recording = read_dro(&dro_file(&writes)).unwrap();
        assert_eq!(recording.hardware, 1);
        assert_eq!(recording.length_ms, 1000);
        assert_eq!(recording.writes, writes);
        assert!(read_dro(b"RIFF").is_err());
        let mut short = dro_file(&writes);
        short.truncate(short.len() - 3);
        assert!(read_dro(&short).unwrap_err().contains("ends after"));
    }

    /// The registers file: written, read back, comments and blanks skipped,
    /// and a line that is not three numbers refused with its number.
    #[test]
    fn a_registers_file_round_trips() {
        let mut text = String::from("# a comment\n\n");
        write_line(&mut text, 0, 0x105, 0x01);
        write_line(&mut text, 12, 0x1b3, 0x25);
        assert_eq!(
            read_registers(&text).unwrap(),
            vec![(0, 0x105, 0x01), (12, 0x1b3, 0x25)]
        );
        assert!(read_registers("1 2\n").unwrap_err().starts_with("line 1"));
        assert!(
            read_registers("1 2 0x100\n")
                .unwrap_err()
                .contains("out of range")
        );
    }

    /// Our stream from the switch-on, the recorder's file from the first
    /// note: the snapshot is set apart, the streams line up on the resuming
    /// write, and a divergence is named at its position.
    #[test]
    fn streams_are_aligned_on_the_write_the_recording_resumes_with() {
        // The switch-on, then a song: a note on, a change, the note off, on
        // again. Two writes leave a register at zero, which a recorder whose
        // chip starts at zero would drop — and so does the reduction here.
        let ours: Vec<Write> = vec![
            (0x001, 0x20),
            (0x105, 0x01),
            (0x0bd, 0x00),
            (0x0bd, 0x00),
            (0x0a0, 0x41),
            (0x0b0, 0x31),
            (0x0a0, 0x42),
            (0x0b0, 0x11),
            (0x0b0, 0x31),
            (0x0a0, 0x41),
            (0x0a0, 0x41),
            (0x0b0, 0x11),
        ];
        // The recording: the state at the first note-on, in register order,
        // then the writes from there, with times.
        let recorded = Recording {
            hardware: 1,
            length_ms: 900,
            writes: vec![
                (0, 0x001, 0x20),
                (0, 0x0a0, 0x41),
                (0, 0x0b0, 0x31),
                (0, 0x105, 0x01),
                (200, 0x0a0, 0x42),
                (200, 0x0b0, 0x11),
                (400, 0x0b0, 0x31),
                (600, 0x0a0, 0x41),
                (800, 0x0b0, 0x11),
            ],
        };
        let report = streams(&ours, &recorded);
        assert!(report.same, "{}", report.text);
        assert!(
            report
                .text
                .contains("holds 4 registers; it resumes with our write 4"),
            "{}",
            report.text
        );
        assert!(
            report.text.contains("agree for all 5 writes"),
            "{}",
            report.text
        );

        let mut parted = ours.clone();
        parted[8] = (0x0b0, 0x35);
        let report = streams(&parted, &recorded);
        assert!(!report.same);
        assert!(
            report.text.contains("part at write 2 of 5"),
            "{}",
            report.text
        );
        assert!(
            report
                .text
                .contains("ours    0x0a0=0x42 0x0b0=0x11 0x0b0=0x35"),
            "{}",
            report.text
        );

        let mut cold = ours.clone();
        cold[1] = (0x105, 0x00);
        let report = streams(&cold, &recorded);
        assert!(!report.same);
        assert!(
            report.text.contains("differs in 1 registers"),
            "{}",
            report.text
        );
        assert!(
            report.text.contains("0x105  0x01  never written"),
            "{}",
            report.text
        );

        let report = streams(&ours[..8], &recorded);
        assert!(
            report
                .text
                .contains("then ours ends; the recording holds 3 more"),
            "{}",
            report.text
        );
    }
}
