//! The engine's side of saving and loading.
//!
//! [`crate::save`] is the file format; this is what the engine puts into it and
//! takes back out — the display state a savegame keeps, and the directory it is
//! allowed to write to.
//!
//! That directory is the one place this rebuild deliberately refuses to follow
//! the original. The original writes its slots beside `ENGINE.EXE`, among the
//! shipped data; [`crate::Engine::set_saves`] rejects any path inside the game
//! directory, because that copy may well be read-only and is not ours to
//! change.

use crate::descriptor::{placement_code, placement_of};
use crate::resources::resolve;
use crate::{Descriptor, Engine, Shows, save};
use crate::{Field, Fields};
use motionvm_motion_forth::cell;

impl Engine {
    /// Points saving and loading at a directory, creating it if need be.
    ///
    /// Refuses any directory that is the game data or lies inside it. The rule
    /// that the shipped files stay untouched is worth more as something the
    /// code cannot break than as something a person has to remember, and this
    /// is the one place where a path to write to enters the engine.
    ///
    /// Both sides are resolved so a relative path, a `..` or a symlinked parent
    /// cannot walk around the check — and the check comes *first*, before the
    /// directory is created. Creating and then refusing leaves exactly the
    /// thing behind that the refusal exists to prevent.
    pub(crate) fn set_saves(
        &mut self,
        dir: &std::path::Path,
        slug: &'static str,
    ) -> Result<(), String> {
        let saves = resolve(dir)?;
        if let Some(data) = self.dir.as_ref().map(|d| resolve(d)).transpose()?
            && saves.starts_with(&data)
        {
            return Err(format!(
                "savegames may not go into the game data: {} is inside {}",
                saves.display(),
                data.display()
            ));
        }
        std::fs::create_dir_all(&saves).map_err(|e| format!("{}: {e}", saves.display()))?;
        self.persistence.dir = Some(saves);
        self.persistence.slug = slug;
        Ok(())
    }

    /// Where saving and loading go, once a directory has been set.
    pub(crate) fn saves(&self) -> Option<&std::path::Path> {
        self.persistence.dir.as_deref()
    }

    /// Reads one of a slot's three files, saying plainly when the slot is not
    /// whole.
    ///
    /// A missing file on the load path is not an ordinary I/O fault. `EXIST`
    /// answers off the `.blk` alone, as the original's does (`0x66b0e` tests
    /// that one name and nothing else), so a slot the game offers can still
    /// be missing the two files that carry the state — and there is one way
    /// that happens: the run that wrote it stopped between two of the three
    /// words. Each file is written whole or not at all
    /// ([`crate::save::write_atomically`]), so the slot is the only thing
    /// that can be half-made.
    ///
    /// "No such file or directory" leaves the player to work that out. This
    /// says which artifact is gone and that the save was interrupted, and
    /// leaves everything on disk where it is.
    pub(crate) fn read_slot(
        &self,
        word: &'static str,
        id: i32,
        suffix: &str,
    ) -> Result<Vec<u8>, motionvm_motion_forth::Error> {
        let Some(path) = self.save_path(id, suffix) else {
            return Err(motionvm_motion_forth::Error::NoSaveDir { word, id });
        };
        match std::fs::read(&path) {
            Ok(bytes) => Ok(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(motionvm_motion_forth::Error::Savegame {
                    what: format!(
                        "{word} {id}: slot {id} is incomplete — {id:03}.{suffix} is missing, so \
                         the save it belongs to was interrupted while it was being written"
                    ),
                })
            }
            Err(source) => Err(motionvm_motion_forth::Error::Io { word, path, source }),
        }
    }

    /// The game a savegame written now belongs to, for its header.
    pub(crate) fn save_slug(&self) -> &'static str {
        self.persistence.slug
    }

    /// The path a resource id takes on disk in the save directory.
    ///
    /// The suffix is the literal one from the original's format string, so the
    /// three artifacts of a save keep the names the game gave them: `701.blk`
    /// for the location `PUT` writes, `701.FRZ` for the module image and
    /// `701.anm` for the display state. Case included — the engine's own
    /// templates disagree with each other (`"#F0R3i.blk"` writes lowercase
    /// where the catalog reader builds `"%03d.BLK"`), which costs nothing under
    /// DOS and would cost a lookup here.
    pub(crate) fn save_path(&self, id: i32, suffix: &str) -> Option<std::path::PathBuf> {
        self.persistence
            .dir
            .as_ref()
            .map(|d| d.join(format!("{id:03}.{suffix}")))
    }

    /// The display state a savegame keeps, as `PUTANIM` writes it.
    pub(crate) fn snapshot(&self) -> save::Anim {
        save::Anim {
            next_descriptor: self.scene.next_descriptor,
            // The handle, not the index: `GETANIM` replaces the whole list, so
            // an index into the old one would point at a different descriptor.
            current: self
                .scene
                .selected
                .and_then(|i| self.scene.descriptors.get(i))
                .map(|d| d.handle),
            screen: self.display.current,
            pointer_visible: self.cursor_state.visible,
            dialog_offset: self.dialogue.offset,
            dialog_return: self.dialogue.return_node,
            palette: self.script_palette().raw,
            screens: self
                .display
                .screens
                .iter()
                .map(|s| save::ScreenState {
                    handle: s.handle,
                    size: s.size,
                    full_view: s.full_view,
                    view: s.view,
                    view_pos: s.view_pos,
                    pos: s.pos,
                    origin: s.origin,
                })
                .collect(),
            flips: self.persistence.flips.clone(),
            descriptors: self
                .scene
                .descriptors
                .iter()
                .map(|d| save::DescriptorState {
                    handle: d.handle,
                    screen: d.screen,
                    x: d.x,
                    y: d.y,
                    level: d.level,
                    shows: match d.shows {
                        Shows::Nothing => (0, 0),
                        Shows::Sprite(id) => (1, cell::signed(id)),
                        Shows::Picture(id) => (2, id),
                    },
                    text: d.text,
                    font: d.font,
                    // The file keeps the option it always had, so savegames
                    // written before the field became a plain value still
                    // load. The original has no "unset" — see [`Descriptor`].
                    color: Some(d.color),
                    template: d.template,
                    wait: d.wait,
                    callback: d.callback,
                    x_mode: placement_code(d.x_mode),
                    y_mode: placement_code(d.y_mode),
                    active: d.active,
                    auto_buffer: d.auto_buffer,
                    fields: d
                        .fields
                        .iter()
                        .map(|(f, v)| (f.name().to_string(), v))
                        .collect(),
                    buffer: d.buffer,
                })
                .collect(),
            buffers_on: self.buffers.on,
            buffers: self
                .buffers
                .iter()
                .map(|(id, b)| (id, b.width, b.height))
                .collect(),
        }
    }

    /// Puts a saved display state back, as `GETANIM` does.
    ///
    /// Everything is turned into engine values first and only then installed,
    /// for the same reason `=>GETAS` checks before it writes: a name or a mode
    /// this build does not know has to stop the load, not leave half a scene
    /// standing. The named descriptor fields are the sharp edge — the map keys
    /// are `&'static str`, so an unknown name cannot be reconstructed at all.
    pub(crate) fn restore(&mut self, anim: save::Anim) -> Result<(), String> {
        let mut descriptors = Vec::with_capacity(anim.descriptors.len());
        for d in &anim.descriptors {
            let mut fields = Fields::default();
            for (name, value) in &d.fields {
                let field = Field::of(name)
                    .ok_or_else(|| format!("savegame names an unknown descriptor field {name}"))?;
                fields.set(field, *value);
            }
            descriptors.push(Descriptor {
                handle: d.handle,
                stamp: 0,
                screen: d.screen,
                x: d.x,
                y: d.y,
                level: d.level,
                shows: match d.shows {
                    (1, id) => Shows::Sprite(cell::unsigned(id)),
                    (2, id) => Shows::Picture(id),
                    _ => Shows::Nothing,
                },
                text: d.text,
                font: d.font,
                color: d.color.unwrap_or(0),
                template: d.template,
                wait: d.wait,
                callback: d.callback,
                // Kept by a 16-bit savegame; the 32-bit game never sets it.
                buffer: d.buffer,
                x_mode: placement_of(d.x_mode)?,
                y_mode: placement_of(d.y_mode)?,
                fields,
                active: d.active,
                // A savegame carries the game's state, not the surface: the
                // original reloads the location and paints it again. So
                // everything comes back dirty — the load is followed by a
                // `FADEIN`, which would mark it all anyway (0x6a8f9).
                auto_buffer: d.auto_buffer,
                dirty: true,
                changed: true,
            });
        }

        for s in &anim.screens {
            let Some(screen) = self.display.screen_mut(s.handle) else {
                return Err(format!(
                    "savegame names screen {} which does not exist",
                    s.handle
                ));
            };
            screen.size = s.size;
            screen.full_view = s.full_view;
            screen.set_view(s.view.0, s.view.1);
            screen.view_pos = s.view_pos;
            screen.pos = s.pos;
            screen.origin = s.origin;
        }
        self.display.current = anim.screen;
        self.display.palette = motionvm_render::Palette::from_6bit(&anim.palette);
        self.scene.next_descriptor = anim.next_descriptor;
        self.cursor_state.visible = anim.pointer_visible;
        self.dialogue.offset = anim.dialog_offset;
        self.dialogue.return_node = anim.dialog_return;
        // In the per-screen scheme the number names a descriptor of the
        // active screen, as `ACTDESC` would resolve it.
        self.scene.selected_handle = anim.current;
        self.scene.selected = anim.current.and_then(|h| {
            descriptors.iter().position(|d| {
                d.handle == h
                    && (!self.profile.per_screen_descriptors || Some(d.screen) == anim.screen)
            })
        });
        // The stamps are not in the file: list order stands in for the
        // chain until the next `SDLEV` moves things — see
        // [`Descriptor::stamp`].
        self.scene.descriptors = descriptors;
        let mut stamp = self.scene.level_stamp;
        for d in &mut self.scene.descriptors {
            stamp += 1;
            d.stamp = stamp;
        }
        self.scene.level_stamp = stamp;
        self.buffers.on = anim.buffers_on;
        self.buffers.reset();
        for &(id, width, height) in &anim.buffers {
            self.buffers.set(id, i32::from(width), i32::from(height));
        }

        // The mirrored sprites are made again rather than carried: their ids
        // are not in any resource file, so nothing else could bring them back.
        for &(from, to) in &anim.flips {
            let Some(sprite) = self.sprite(from) else {
                continue;
            };
            let mut flipped = sprite.clone();
            let w = usize::from(sprite.width);
            for (row, out) in sprite
                .pixels
                .chunks_exact(w)
                .zip(flipped.pixels.chunks_exact_mut(w))
            {
                for (x, p) in row.iter().enumerate() {
                    out[w - 1 - x] = *p;
                }
            }
            self.scene.sprites.insert(to, flipped);
        }
        self.persistence.flips = anim.flips;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Descriptor, Engine, Field, Profile, Screen};

    /// A descriptor field this build writes is one it can read back.
    ///
    /// `SDBLK` could not be, and nothing said so: its word wrote it into the
    /// descriptor's field map and the loader checked names against a slice
    /// beside that map which did not list it, so a savegame taken in the one
    /// scene that sets it — Die Enviro-Kids greifen ein's newspaper, module
    /// 615 — was written and then refused by name. The field list is one enum
    /// now and cannot drift from itself; this drives the round trip anyway,
    /// because the bug was in the *path* and not only in the list.
    #[test]
    fn every_field_survives_the_round_trip() {
        for field in Field::ALL {
            let mut e = Engine::new(Profile::motion16());
            let mut s = Screen::new(1);
            s.active = true;
            e.add_screen(s);
            let mut d = Descriptor {
                handle: 1,
                screen: 1,
                active: true,
                ..Default::default()
            };
            d.fields.set(field, 4711);
            e.add_descriptor(d);

            let anim = e.snapshot();
            e.restore(anim)
                .unwrap_or_else(|err| panic!("{}: {err}", field.name()));
            assert_eq!(
                e.descriptors()[0].fields.get(field),
                Some(4711),
                "{} came back changed",
                field.name()
            );
        }
    }
}
