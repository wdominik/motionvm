//! `SMPPATH`, the loose file that names where the 32-bit engine's speech
//! files are.
//!
//! The sound layer's start-up reads it (`ENGINE.EXE` V0.04.15/R78, the
//! `smppath` string at `0xb4a7c`; R109 the same at `0xbf6f6`): plain text,
//! at most eighty bytes, a `#` starting a comment, and what is left is the
//! directory `->STARTSAMPLE` prefixes a file name with, as `<smppath>\<name>`
//! (`0x6a8ce`, the format `%s\%s`). Checker 2000 ships `C:\Checker\wavs#`
//! and a line end — the install directory's `WAVS` — and Dunkle Schatten 2
//! ships no such file, having no speech to find.
//!
//! The path is the 1996 installation's, drive letter and all, which is the
//! one thing in it a copy cannot use as it stands; how the engine relocates
//! it is the opener's business, and this reader only says what the file
//! says.

/// The directory the file names, as written, with the comment and the
/// surrounding whitespace taken off — or `None` for a file that names
/// nothing.
pub fn parse(text: &[u8]) -> Option<String> {
    let text = text.get(..text.len().min(80)).unwrap_or(text);
    let named = text.split(|&b| b == b'#').next().unwrap_or_default();
    let named = crate::cp437_to_string(named);
    let named = named.trim();
    (!named.is_empty()).then(|| named.to_string())
}

/// The components of a DOS path after its drive, as the directory names
/// between the backslashes: `C:\Checker\wavs` is `["Checker", "wavs"]`.
pub fn components(dos_path: &str) -> Vec<String> {
    let after_drive = dos_path.split_once(':').map_or(dos_path, |(_, rest)| rest);
    after_drive
        .split(['\\', '/'])
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{components, parse};

    /// The shipped file, byte for byte.
    #[test]
    fn checker_2000s_file_names_its_wavs_directory() {
        let shipped = b"C:\\Checker\\wavs#\r\n";
        assert_eq!(parse(shipped).as_deref(), Some("C:\\Checker\\wavs"));
        assert_eq!(components("C:\\Checker\\wavs"), ["Checker", "wavs"]);
    }

    #[test]
    fn a_comment_or_an_empty_file_names_nothing() {
        assert_eq!(parse(b"# nothing here\r\n"), None);
        assert_eq!(parse(b"   \r\n"), None);
        assert_eq!(parse(b""), None);
    }
}
