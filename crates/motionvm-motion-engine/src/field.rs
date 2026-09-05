//! The descriptor fields the engine keeps by name rather than by meaning.
//!
//! The original's descriptor is 0x3E bytes, and not every field in it has been
//! read. The ones that have a meaning here are modelled — position, level, what
//! it shows — and the rest are kept as values under the name the word that
//! wrote them has, so a frame can be reproduced without inventing a meaning
//! for a field before it has been measured.
//!
//! They were kept in a `BTreeMap<&'static str, i32>`, and a savegame wrote the
//! names. Two things followed. Every read on the drawing path — `SD%SHR` and
//! its per-axis pair are read for every scaled descriptor, on every frame —
//! compared strings; and the set of names a savegame could name was a `&[&str]`
//! beside the map, which nothing held to the set of names the words actually
//! write. It had drifted: `SDBLK` was written by its word and missing from the
//! list, so a savegame taken in Die Enviro-Kids greifen ein's newspaper — the
//! one scene that sets it — was written and then refused on load, by name.
//!
//! An enum cannot drift from itself. [`Field::of`] and [`Field::name`] are the
//! savegame's two directions and a test holds them against each other, so a
//! field that exists is a field a savegame can carry.

/// One descriptor field kept by name.
///
/// Spelled the kernel's way, as the kernel word enum is and for the same
/// reason: this is how the disassembly and every page under `docs/` write
/// them, and [`Field::name`] answers exactly what a savegame stores.
#[expect(
    non_camel_case_types,
    reason = "the kernel's own spelling, which is what a savegame stores and \
              what the documentation uses"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    /// `SD%SHR` — the scale both axes take when neither has its own.
    SD_PCT_SHR,
    /// `SDV%SHR` — the vertical scale.
    SDV_PCT_SHR,
    /// `SDH%SHR` — the horizontal scale.
    SDH_PCT_SHR,
    /// `SDBUF` — the buffer a 16-bit descriptor draws through.
    SDBUF,
    /// `SDSTARTLINE` — the first line of a text block that is drawn.
    SDSTARTLINE,
    /// `SDALINES` — how many lines of it.
    SDALINES,
    /// `SDTRANS` — the transparency setting.
    SDTRANS,
    /// `SDSHADE` — the shade setting.
    SDSHADE,
    /// `INSERT` — what `SDINSERT` puts into a text.
    INSERT,
    /// `SDBLK` — block justification: every line starts at the block's left
    /// edge and its inner spaces stretch to the widest line. Set by its own
    /// word and cleared by `SDNORM`, so it is present or absent rather than
    /// valued.
    SDBLK,
}

impl Field {
    /// Every field, in the order they are stored.
    pub const ALL: [Field; 10] = [
        Field::SD_PCT_SHR,
        Field::SDV_PCT_SHR,
        Field::SDH_PCT_SHR,
        Field::SDBUF,
        Field::SDSTARTLINE,
        Field::SDALINES,
        Field::SDTRANS,
        Field::SDSHADE,
        Field::INSERT,
        Field::SDBLK,
    ];

    /// The name a savegame stores, and the word that writes it.
    pub fn name(self) -> &'static str {
        match self {
            Field::SD_PCT_SHR => "SD%SHR",
            Field::SDV_PCT_SHR => "SDV%SHR",
            Field::SDH_PCT_SHR => "SDH%SHR",
            Field::SDBUF => "SDBUF",
            Field::SDSTARTLINE => "SDSTARTLINE",
            Field::SDALINES => "SDALINES",
            Field::SDTRANS => "SDTRANS",
            Field::SDSHADE => "SDSHADE",
            Field::INSERT => "INSERT",
            Field::SDBLK => "SDBLK",
        }
    }

    /// The field a savegame's name means, or `None` for one this build has
    /// never written.
    pub fn of(name: &str) -> Option<Field> {
        Field::ALL.into_iter().find(|f| f.name() == name)
    }

    /// Where this field sits in a [`Fields`]: the variant's position, which
    /// is what a fieldless enum's discriminant is.
    #[expect(
        clippy::as_conversions,
        reason = "a fieldless enum's discriminant is its position in the array"
    )]
    const fn index(self) -> usize {
        self as usize
    }
}

/// A descriptor's named fields: one slot each, present or absent.
///
/// An array rather than a map, and the difference is not only the string
/// compares. A descriptor is cloned on every frame that measures one, and an
/// array clones without touching the allocator where a `BTreeMap` does not.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Fields([Option<i32>; Field::ALL.len()]);

impl Fields {
    /// What `field` holds, or `None` when nothing has set it.
    pub fn get(&self, field: Field) -> Option<i32> {
        self.0[field.index()]
    }

    /// Sets it.
    pub fn set(&mut self, field: Field, value: i32) {
        self.0[field.index()] = Some(value);
    }

    /// Takes it away again — `SDNORM` does this to `SDBLK`.
    pub fn clear(&mut self, field: Field) {
        self.0[field.index()] = None;
    }

    /// The fields that are set, in [`Field::ALL`]'s order.
    pub fn iter(&self) -> impl Iterator<Item = (Field, i32)> + '_ {
        Field::ALL
            .into_iter()
            .filter_map(|f| self.get(f).map(|v| (f, v)))
    }

    /// How many are set.
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Whether none is.
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::Field;
    use motionvm_motion_forth::cell;

    /// The two directions a savegame uses agree, so a field this build writes
    /// is one it can read back. `SDBLK` was written by its word and missing
    /// from the list the loader checked, which made a savegame taken in the
    /// one scene that sets it unloadable; nothing could hold the two together
    /// while they were a match and a slice.
    #[test]
    fn every_field_survives_a_savegame() {
        for f in Field::ALL {
            assert_eq!(Field::of(f.name()), Some(f), "{}", f.name());
        }
        assert_eq!(Field::of("NOSUCHFIELD"), None);
    }

    /// Each field has a slot of its own.
    #[test]
    fn the_slots_are_distinct() {
        let mut fields = super::Fields::default();
        for (i, f) in Field::ALL.into_iter().enumerate() {
            fields.set(f, cell::count(i));
        }
        for (i, f) in Field::ALL.into_iter().enumerate() {
            assert_eq!(fields.get(f), Some(cell::count(i)), "{}", f.name());
        }
    }
}
