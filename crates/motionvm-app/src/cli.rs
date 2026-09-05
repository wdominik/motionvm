//! What the command line says, and what the program does when it says
//! nothing.
//!
//! The parser, the usage text it is checked against, and the folder dialog a
//! double-clicked binary falls back on. Kept apart from the window loop
//! because none of it needs one: this is the part that runs before there is
//! anything on the screen, and the part the tests at the bottom can drive
//! without opening one.

use super::roster;
use motionvm_playable::Playable;
use std::path::PathBuf;

/// The usage text, with the game list built from the roster so the two
/// cannot drift apart: what the program plays is what its help names.
pub(crate) fn usage() -> String {
    let games = roster::FAMILIES
        .iter()
        .flat_map(|f| f.games())
        .map(|g| format!("{} ({})", g.needs, g.short))
        .collect::<Vec<_>>()
        .join(", or ");
    let gamedir = wrapped(
        &format!(
            "the directory a game is installed in: {games}. \
             Without one, a folder dialog asks for it."
        ),
        60,
        "\n                  ",
    );
    format!(
        "\
motionvm — rebuilt game engines, for the games built with them

usage: motionvm [GAMEDIR] [options]

  GAMEDIR         {gamedir}

options:
  --loc N         start in location N — the game's own numbering of its
                  places — instead of where the game would begin.
  --no-sound      do not open an audio device.
  -h, --help      this text.
"
    )
}

/// Greedy word wrap for the help's second column: lines of at most `width`
/// characters, joined by `newline` — which carries the column's indent.
fn wrapped(text: &str, width: usize, newline: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines.join(newline)
}

/// Everything the command line can say.
#[derive(Debug)]
pub(crate) struct Options {
    /// The game directory, when the command line names one. `None` means
    /// nobody typed a path — a double-clicked binary, most often — and the
    /// folder dialog asks instead. There is deliberately no silent default: a
    /// relative `../gamedata` that happens to exist is a checkout's accident,
    /// not a player's choice, and one that does not exist fails with a message
    /// about a directory the player never named.
    pub(crate) dir: Option<PathBuf>,
    pub(crate) wanted: Option<i32>,
    pub(crate) quiet: bool,
    pub(crate) help: bool,
}

/// Reads the command line, or says what is wrong with it.
///
/// A hand-written parser rather than a crate, to keep the dependency list at
/// the six it needs to run at all.
///
/// The one thing worth being careful about is what a positional argument is.
/// "The first argument that does not start with `--`" is the obvious rule and
/// the wrong one: it reads `motionvm --loc 5` as a game directory called `5`,
/// because the value of a flag is indistinguishable from a positional unless
/// the flag is consumed together with it. So flags are walked in order and
/// value-taking ones swallow their operand, and only what is left over can be
/// the directory.
pub(crate) fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut opt = Options {
        dir: None,
        wanted: None,
        quiet: false,
        help: false,
    };
    let mut positional = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |flag: &str| -> Result<String, String> {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match a.as_str() {
            "-h" | "--help" => opt.help = true,
            "--no-sound" => opt.quiet = true,
            "--loc" => {
                let v = value("--loc")?;
                opt.wanted = Some(
                    v.parse()
                        .map_err(|_| format!("--loc wants a number, not {v:?}"))?,
                );
            }
            // Rejected rather than ignored: a mistyped flag that is silently
            // dropped looks exactly like one that did nothing, and the two are
            // worth telling apart.
            _ if a.starts_with('-') && a.len() > 1 => {
                return Err(format!("unknown option {a}\n\n{}", usage()));
            }
            _ if positional.is_some() => return Err(format!("more than one game directory: {a}")),
            _ => positional = Some(a.clone()),
        }
    }
    opt.dir = positional.map(PathBuf::from);
    Ok(opt)
}

/// Asks for the game directory with the platform's own folder dialog, and
/// keeps asking while the answer is not one.
///
/// `None` when the dialog is dismissed — that is the player deciding not to
/// play, not an error — or when a wrong directory's complaint is answered with
/// Cancel. A wrong directory is reported where the player is looking: the
/// message the family's opener writes for exactly this case, in a message
/// box, with OK opening the dialog again. An opener checks for its required
/// files before it reads anything, so a wrong answer costs nothing and the
/// loop is cheap to go round.
///
/// Called before the event loop exists, on the main thread, which is where
/// rfd's synchronous dialogs belong in a program that has no window yet. On
/// Linux the dialog is the XDG desktop portal's, so nothing links at build
/// time — and a desktop with neither the portal service nor `zenity` answers
/// `None` here, the same as a dismissal, which is why the caller's message
/// says how to name the directory without the dialog.
pub(crate) fn choose_game() -> Option<Box<dyn Playable>> {
    loop {
        let dir = rfd::FileDialog::new()
            .set_title("Choose the game directory — the folder the game's own files are in")
            .pick_folder()?;
        let complaint = match roster::find(&dir) {
            Some(family) => match family.open(&dir) {
                Ok(game) => return Some(game),
                Err(e) => e.to_string(),
            },
            None => roster::nobodys(&dir),
        };
        let again = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("motionvm")
            .set_description(format!(
                "{complaint}\n\nOK chooses another directory; Cancel quits."
            ))
            .set_buttons(rfd::MessageButtons::OkCancel)
            .show();
        if !matches!(again, rfd::MessageDialogResult::Ok) {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{data_dir, data_path};

    fn parse(args: &[&str]) -> Result<Options, String> {
        parse_args(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    /// A flag's operand must not fall through to the positional.
    ///
    /// Taken as "the first argument not starting with `--`", the `5` of
    /// `--loc 5` becomes the game directory, and a game gets looked for
    /// inside a directory called `5`.
    #[test]
    fn a_flags_value_is_not_the_game_directory() {
        let o = parse(&["--loc", "5"]).unwrap();
        assert_eq!(o.dir, None, "nothing was named, so nothing is taken");
        assert_eq!(o.wanted, Some(5));
    }

    #[test]
    fn the_directory_can_come_before_or_after_the_flags() {
        for args in [
            vec!["/games/ds2", "--loc", "5"],
            vec!["--loc", "5", "/games/ds2"],
            vec!["--no-sound", "/games/ds2", "--loc", "5"],
        ] {
            let o = parse(&args).unwrap();
            assert_eq!(o.dir, Some(PathBuf::from("/games/ds2")), "{args:?}");
            assert_eq!(o.wanted, Some(5), "{args:?}");
        }
    }

    #[test]
    fn defaults_are_what_the_readme_says() {
        let o = parse(&[]).unwrap();
        assert_eq!(o.dir, None, "no path means the dialog, not ../gamedata");
        assert_eq!(o.wanted, None);
        assert!(!o.quiet);
    }

    /// What the program writes must not land in the working directory, because
    /// for anyone building from a checkout that directory is the source tree —
    /// and nothing on the command line can send it there either.
    ///
    /// Asserted as "absolute", not as a literal path, because the answer is the
    /// platform's and this suite runs on more than one. The one environment
    /// that legitimately has no answer — no `HOME` at all — is the documented
    /// fallback, and there the relative name is the right behavior.
    #[test]
    fn what_the_program_writes_does_not_land_in_the_working_directory() {
        if data_dir().is_none() {
            eprintln!("skipping: the environment names no home directory");
            return;
        }
        for p in [data_path("saves"), data_path("shot.png")] {
            assert!(p.is_absolute(), "{} is relative", p.display());
            assert!(
                p.starts_with(data_dir().unwrap()),
                "{} is not under the data directory",
                p.display()
            );
        }
    }

    #[test]
    fn a_flag_without_its_value_is_refused() {
        assert!(parse(&["--loc"]).is_err());
        // And a value that is not a number says so rather than being dropped.
        let e = parse(&["--loc", "seven"]).unwrap_err();
        assert!(e.contains("wants a number"), "{e}");
    }

    #[test]
    fn an_unknown_option_is_refused_rather_than_ignored() {
        let e = parse(&["--sound"]).unwrap_err();
        assert!(e.contains("unknown option --sound"), "{e}");
    }

    #[test]
    fn two_game_directories_are_refused() {
        assert!(parse(&["/one", "/two"]).is_err());
    }

    #[test]
    fn help_is_recognized_both_ways() {
        assert!(parse(&["-h"]).unwrap().help);
        assert!(parse(&["--help"]).unwrap().help);
    }
}
