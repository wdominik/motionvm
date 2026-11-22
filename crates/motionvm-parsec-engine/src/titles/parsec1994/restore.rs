//! The way back into a loaded game: the handler of the restore row
//! ([`SaveMap::restore_row`]), which is not a place.
//!
//! Loading in game reads no file: it sends the main loop to the restore
//! row once a game is held in the buffer ([`Adventure::load_0518`]). The
//! row's handler has no hook, no rules and no frame loop of its own: it
//! copies the held blocks back — the location stack and the player's
//! record first, then, past the location it pops, the rest — and returns,
//! and the main loop goes on at the popped location. Read from *Dunkle
//! Schatten*'s `147Ah:052Ch` (image `0x14CCC` to `0x14D7B`), against
//! [`Api`] alone; the addresses in the comments are that build's.
//!
//! [`Adventure::load_0518`]: super::Adventure::load_0518

use super::api::{Api, Far};
use super::savegame::SaveMap;

/// The main loop's three words: the entry, the location it dispatches on,
/// and the location the running handler entered.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MainLoop {
    pub entry: u16,
    pub location: u16,
    pub here: u16,
}

/// The save's blocks in the order the way back copies them: the stack,
/// the player's record, then the story, the object rows and the last.
const STACK: usize = 2;
const PLAYER: usize = 3;
const AFTER_THE_POP: [usize; 3] = [0, 1, 4];

/// The handler, whole: there is no frame loop. `buffer` is the segment of
/// the far buffer the save fills.
pub(crate) fn enter(api: &mut dyn Api, save: &SaveMap, buffer: u16, words: MainLoop) {
    // 0x14CD0: nothing loaded, nothing to do.
    if api.word(save.loaded) == 0 {
        return; // 0x14CD7
    }
    let back = |api: &mut dyn Api, k: usize| {
        let b = save.blocks[k];
        api.movmem(
            Far {
                off: b.buf,
                seg: buffer,
            },
            api.ds(b.ds),
            b.back,
        );
    };
    back(api, STACK); // 0x14CF4
    back(api, PLAYER); // 0x14D0C
    api.set_word(words.here, 0xFFFF); // 0x14D14
    // 0x14D1A: the location on top of the stack, popped.
    let top = api.word(save.stack);
    let loc = api.word(save.stack.wrapping_add(top << 1));
    api.set_word(words.location, loc); // 0x14D24
    api.set_word(save.stack, top.wrapping_sub(1)); // 0x14D27
    api.set_word(words.entry, 0xFFFF); // 0x14D2B
    // 0x14D41, 0x14D5A, 0x14D72.
    for k in AFTER_THE_POP {
        back(api, k);
    }
}
