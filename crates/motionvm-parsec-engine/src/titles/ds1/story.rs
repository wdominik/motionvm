//! Story logic shared by several locations.
//!
//! The segment of the global rule handlers (`1586h`) also holds a routine
//! of plain story writes, `1586:06B1` (image `0x15F11`), that three
//! locations' handlers call (`0x1BA3` and `0x1FA6` in the JZ, `0x2ADC` in
//! the Schule, `0x3643` at the Brand). It calls no engine routine; it sets
//! the state bytes a chapter turn leaves behind. What the turn means in the
//! story is not read here, so the function is named by its address.

use crate::titles::parsec1994::Api;

/// `1586:06B1`: the state bytes after the turn; the four people bytes
/// move on only when already set.
pub(crate) fn routine_06b1(api: &mut dyn Api) {
    api.set_byte(0x270E, 1); // 0x15F11
    api.set_byte(0x26FE, 0x0A);
    api.set_byte(0x26FF, 0x0C);
    for (addr, next) in [(0x26FB, 8), (0x2700, 0x0E), (0x2703, 0x10), (0x2706, 0x12)] {
        // 0x15F20, 0x15F2C, 0x15F38, 0x15F44
        if api.byte(addr) != 0 {
            api.set_byte(addr, next);
        }
    }
    api.set_byte(0x2720, 0); // 0x15F50
    api.set_byte(0x2717, 8);
    api.set_byte(0x272F, 0);
    api.set_byte(0x2729, 9);
}
