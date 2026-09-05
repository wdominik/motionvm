//! What a picture is made of, once the resources are open: the descriptor
//! list and the cursor into it, the fonts and text tables the drawer measures
//! with, and the caches a screen is composed from.
//!
//! The **descriptor** is the engine's one unit of "something on a screen" — a
//! sprite, a block, a line of text, a walker — and everything that draws
//! reaches it through the list here. `ACTDESC` selects one and every `SD…`
//! word acts on the selection, which is why the cursor lives beside the list
//! rather than being passed around: the original keeps it in a global too
//! (`0xDB4C0` for the handle, `0xF2AF0` for the resolved record).
//!
//! The caches are lazy on purpose. A sprite is decoded the first time
//! something draws it, not when the container opens, because a game ships
//! sixteen hundred of them and enters a location with a dozen. The first
//! sprite drawn also brings its palette with it, which is the original's rule
//! and belongs to drawing rather than to loading.
//!
//! The **screens** are not here: they are `Display`'s, together with the view
//! each one is seen through and the surface it composes into.

use crate::{Descriptor, TextTemplate};
use motionvm_motion_formats::TextTable;
use motionvm_motion_formats::font::{Font, FontRefTable};
use motionvm_render::Picture;
use std::collections::BTreeMap;

/// The descriptors, the text apparatus and the resource caches.
#[derive(Debug)]
pub(crate) struct Scene {
    pub(crate) descriptors: Vec<Descriptor>,

    pub(crate) selected: Option<usize>,

    /// The number `ACTDESC` last stored, for the re-resolution a later
    /// `ACTSCR` does in the per-screen scheme.
    pub(crate) selected_handle: Option<u32>,

    /// The stamp counter behind [`Descriptor::stamp`].
    pub(crate) level_stamp: u64,

    pub(crate) next_descriptor: u32,

    pub(crate) templates: Vec<TextTemplate>,

    /// Fonts by the handle `+FONT` handed out.
    pub(crate) fonts: BTreeMap<i32, Font>,

    pub(crate) next_font: i32,

    pub(crate) font_refs: Option<FontRefTable>,

    /// `000.FNT`, which a text descriptor uses when nothing chose a font.
    pub(crate) system_font: Option<Font>,

    /// Text tables by resource id, as `SDTB` names them.
    pub(crate) texts: BTreeMap<i32, TextTable>,

    pub(crate) sprites: BTreeMap<u32, Picture>,

    pub(crate) palettes: BTreeMap<i32, motionvm_render::Palette>,
}

impl Default for Scene {
    /// Empty, with both handle counters at **1**.
    ///
    /// Not zero: a descriptor handle and a font handle are both what the
    /// script gets back and passes to `ACTDESC` or `SDFNT`, and zero is what
    /// those words read as "none". A run that handed out handle 0 first would
    /// have its first descriptor unselectable.
    fn default() -> Self {
        Self {
            descriptors: Vec::new(),
            selected: None,
            selected_handle: None,
            level_stamp: 0,
            next_descriptor: 1,
            templates: Vec::new(),
            fonts: BTreeMap::new(),
            next_font: 1,
            font_refs: None,
            system_font: None,
            texts: BTreeMap::new(),
            sprites: BTreeMap::new(),
            palettes: BTreeMap::new(),
        }
    }
}
