//! Just enough JSON to emit string arrays, so the tools stay dependency-light.

/// Renders `s` as a JSON string literal, escaping what the spec requires.
pub(crate) fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::quote;

    #[test]
    fn escapes_quotes_backslashes_and_newlines() {
        assert_eq!(quote("a\"b\\c\nd"), r#""a\"b\\c\nd""#);
    }

    #[test]
    fn escapes_other_control_chars_as_unicode() {
        // Expect the eight characters: quote, backslash, u, 0, 0, 0, 1, quote.
        assert_eq!(quote("\u{1}"), "\"\\u0001\"");
    }

    #[test]
    fn passes_umlauts_through() {
        assert_eq!(quote("Tür"), "\"Tür\"");
    }
}
