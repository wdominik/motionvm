//! How a title's location logic meets the runtime.
//!
//! The original's main loop (in *Dunkle Schatten* `0x4FC0`) calls the
//! handler of the location in the main loop's word; the handler enters,
//! runs its own frame loop until an exit is asked for, and leaves. Here the
//! loop is turned inside out once, at this one place: [`Visit`] holds the
//! running location's [`Logic`] and calls its `enter`, then its `frame`
//! once per tick, then its `leave`, and the next location's `enter` — the
//! order the original runs them in.

use super::{Adventure, Api, Far};
use crate::num;

/// One location's logic.
#[derive(Clone, Copy)]
pub(crate) struct Logic {
    pub enter: fn(&mut dyn Api),
    pub frame: fn(&mut dyn Api) -> bool,
    pub leave: fn(&mut dyn Api),
    pub hook: fn(&mut dyn Api, u16, u16, u16),
    pub anim_hook: Option<fn(&mut dyn Api, Far, u16)>,
    pub rule: fn(&mut dyn Api, u16, u16) -> Option<i16>,
}

/// Which logic a title has for each row of its location table.
#[derive(Clone, Copy)]
pub(crate) struct LogicTable {
    /// The logic of the row with this number; `None` for one past the
    /// table.
    pub logic: fn(u16) -> Option<Logic>,
    /// The walk on, for a row that is not a place and a word past the
    /// table.
    pub onward: Logic,
}

/// The visit in progress.
pub(crate) struct Visit {
    logic: Logic,
    table: LogicTable,
}

impl Visit {
    /// The main loop's next call: the handler of the location in its word,
    /// entered. A word past the table, or a passage row, is not
    /// dispatched through the table at all: the main loop bounds the word
    /// against the table and asks the row's kind (in *Dunkle Schatten*
    /// `0x04F8E`–`0x04FA7`) and calls the walk on instead
    /// ([`super::onward`]), which is also the code the passage rows name as
    /// their handler.
    pub(crate) fn enter(adv: &mut Adventure, table: LogicTable) -> Visit {
        let loc = adv.location_word;
        let walks = usize::from(loc) >= adv.locations.count
            || adv.locations.kind(&adv.image, usize::from(loc)) == Some(adv.kinds.passage);
        let logic = (table.logic)(loc)
            .filter(|_| !walks)
            .unwrap_or(table.onward);
        // A fresh call of a handler: a fresh stack frame.
        adv.handler_locals = [0; 8];
        (logic.enter)(adv);
        Visit { logic, table }
    }

    /// One turn of the handler's frame loop; when it ends, the handler
    /// leaves and the next location's is entered.
    ///
    /// `None` when the main loop is over: it runs while `[1F4Ah]` is not
    /// below zero (`0x4FE9`), and quitting sets it to −2 (`0x12403`).
    pub(crate) fn frame(self, adv: &mut Adventure) -> Option<Visit> {
        if (self.logic.frame)(adv) {
            return Some(self);
        }
        (self.logic.leave)(adv);
        adv.take_leave();
        if num::signed(adv.location_word) < 0 {
            return None;
        }
        Some(Visit::enter(adv, self.table))
    }

    /// Leaves the running location without its frame loop ending — for a
    /// jump the host asks for.
    pub(crate) fn leave(self, adv: &mut Adventure) {
        (self.logic.leave)(adv);
        adv.take_leave();
    }
}
