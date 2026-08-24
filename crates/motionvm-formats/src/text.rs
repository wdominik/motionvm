//! String tables, as both engine generations decode them.
//!
//! A text table is a numbered list of strings — the entries `SDTB` selects a
//! table of and `SDTXT n` picks entry `n` from. The two generations store it
//! differently ([`crate::m32::text`], [`crate::m16::text`]) and read it into
//! this one structure.

#[derive(Debug, Clone, Default)]
/// One text resource: the strings a `SDTB` table holds, in order.
pub struct TextTable {
    /// The strings, CP437-decoded, index 0 being entry 1 to the game.
    pub strings: Vec<String>,
}

impl TextTable {
    /// Entry `index`, counting from zero. `SDTXT n` is entry `n - 1`.
    pub fn get(&self, index: usize) -> Option<&str> {
        self.strings.get(index).map(String::as_str)
    }
}
