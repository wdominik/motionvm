//! The kernel words, one file per group.
//!
//! `Engine::plain_word` is a single match of ninety-odd arms; the groups here
//! are that match cut into files, nothing more.
//!
//! **The order of the groups is the order of the original's match, and it is
//! load-bearing.** Two arms match on table membership rather than on a literal,
//! so an arm that moves across one of them changes which words it catches; see
//! the note on [`Engine::plain_word`](crate::Engine::plain_word).
//!
//! Within that constraint a group is one subject — descriptors, screens, the
//! inventory bar, the dialogue queue — because the original's own section
//! order is not one: it visits screens twice and palette twice.

mod buffers;
mod descriptors;
mod dialogue;
mod dowalk;
mod input;
mod inventory;
mod m16;
// The two inventory operations are the game's own list surgery and the verb
// menu does it directly, without going round through the words.
pub(crate) use inventory::{
    M16_RULES, M32_RULES, Rules as InventoryRules, add_to_inventory, remove_from_inventory,
    slot_address,
};
mod palette;
pub(crate) mod pointer;
mod redraw;
mod resources;
mod saves;
mod screens;
mod sound;
mod state;
mod text;
mod transitions;
