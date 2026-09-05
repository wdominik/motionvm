//! A window for the engine.
//!
//! The frontend is deliberately thin. The engine renders into an indexed
//! framebuffer and this does two things with it: expand it through the
//! current palette, and put it on screen at an integer scale.
//!
//! That framebuffer has been checked pixel for pixel against the original
//! engine's own output: the title screen, location 23, all 307 200 pixels of
//! it over 206 distinct indices, every index mapping to one color and every
//! color back to one index. **One frame** — the claim is worth exactly that:
//! one scene, drawn through scaling, layering, the palette and the composition
//! rule, agreeing completely.
//!
//! Integer scaling is not a preference. The games are hand-drawn pixel art;
//! any other factor resamples it and invents colors that were never in the
//! palette. So the picture is scaled by whole numbers and centered in
//! whatever space is left, with black around it. The two axes carry their own
//! whole number, because the pixels themselves were not square everywhere:
//! a game whose mode filled a 4:3 monitor with a grid that is not 4:3 had
//! pixels taller than wide ([`motionvm_playable::Playable::pixel_aspect`]), so its picture is
//! drawn in sx×sy blocks with sy/sx as close to that ratio as whole numbers
//! allow — exact once the window is big enough. A square-pixel game keeps
//! sx = sy.
//!
//! This file is the front door and nothing else: how the program stops, where
//! it says so, and where it keeps what it writes. What it plays is decided in
//! [`cli`], the window and its loop are [`app`]'s, the scaling arithmetic is
//! [`scale`]'s, and the families it can open are named in [`roster`].

// On Windows the release build is a windowed program, not a console one, so a
// double-clicked `motionvm.exe` opens the game and not a black console behind
// it. The price is that everything written to stderr — `savegames in …`,
// `sound is off`, `the game stopped`, and `--help` — goes nowhere there; what
// a double-clicking player has to see reaches them through the folder dialog
// and its message box instead. Debug builds keep the console, so a Windows
// developer still sees the messages. The attribute means nothing anywhere else.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

mod app;
mod cli;
mod keys;
mod roster;
mod scale;
mod sound;

fn main() {
    // Not `main() -> Result`, which would `Debug`-print whatever came back:
    // `Error: Io(Os { code: 2, kind: NotFound, … })` is the shape of the type,
    // not of the problem, and the one error a first-time player is most likely
    // to hit — a game directory that is not one — has a message written for
    // them that only survives if it goes out through `Display`.
    if let Err(e) = app::run() {
        fatal(&e.to_string());
        std::process::exit(1);
    }
}

/// Says why the program is stopping, where a player will see it.
///
/// The release build is a windowed program (see the attribute at the top of
/// this file), so it has no console: `eprintln!` on Windows goes nowhere at
/// all, and a game that stops is then a window that closes. Every fatal line
/// goes to a dialog as well — on every platform, because a player who started
/// the program from a launcher has no terminal wherever they are.
///
/// stderr keeps its line, and first: it is what a script redirecting output
/// gets, and it is there whether or not a desktop can put a box on the screen.
fn fatal(what: &str) {
    eprintln!("motionvm: {what}");
    log(what);
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("motionvm")
        .set_description(what)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// Appends a line to the log file, if one was asked for.
///
/// `MOTIONVM_LOG=1` puts it at `motionvm.log` in the data directory beside the
/// savegames; `MOTIONVM_LOG=<path>` puts it where the path says. Off by
/// default, because a program that writes a file nobody asked for is a program
/// that leaves litter.
///
/// It exists for the lines a windowed build otherwise loses — the diagnostics
/// a run reports on the way out, the reason a save directory was refused, the
/// word a game stopped on. A dialog says one of them; a log says all of them,
/// in order, after the fact.
fn log(what: &str) {
    use std::io::Write as _;
    let Some(path) = std::env::var_os("MOTIONVM_LOG") else {
        return;
    };
    let path = if path == "1" || path.is_empty() {
        data_path("motionvm.log")
    } else {
        PathBuf::from(path)
    };
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "{what}");
    }
}

/// A line worth keeping but not worth stopping for: to stderr, and to the log
/// if one was asked for.
fn note(what: &str) {
    eprintln!("{what}");
    log(what);
}

/// Where the program keeps the files it writes: savegames and screenshots.
///
/// **Not the working directory.** A relative default would land these
/// wherever the program happens to be started — for anyone building from a
/// checkout, in the source tree. Runtime output does
/// not belong beside the sources, and no amount of ignoring it there makes it
/// belong.
///
/// Resolved by hand rather than through a crate, for the same reason the
/// argument parser is: three environment variables and a join is not worth a
/// dependency.
///
/// | platform | directory |
/// |---|---|
/// | macOS | `$HOME/Library/Application Support/motionvm` |
/// | Windows | `%APPDATA%\\motionvm` |
/// | anything else | `$XDG_DATA_HOME/motionvm`, else `$HOME/.local/share/motionvm` |
///
/// `None` when the environment names no home at all — a bare `cron` job, a
/// daemon, a container without `HOME`. The caller then falls back to a relative
/// path, because refusing to run would be worse than writing where the old
/// default wrote.
fn data_dir() -> Option<PathBuf> {
    let var = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    if cfg!(target_os = "macos") {
        Some(var("HOME")?.join("Library/Application Support/motionvm"))
    } else if cfg!(target_os = "windows") {
        Some(var("APPDATA")?.join("motionvm"))
    } else {
        var("XDG_DATA_HOME")
            .or_else(|| var("HOME").map(|h| h.join(".local/share")))
            .map(|d| d.join("motionvm"))
    }
}

/// Where a file the program writes goes: under [`data_dir`] when there is one,
/// otherwise the plain relative name.
fn data_path(name: &str) -> PathBuf {
    data_dir().map_or_else(|| PathBuf::from(name), |d| d.join(name))
}
