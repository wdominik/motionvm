//! The engine's side of saving and loading.
//!
//! [`crate::save`] is the file format; this is what the engine puts into it and
//! takes back out — the display state a savegame keeps, and the directory it is
//! allowed to write to.
//!
//! That directory is the one place this rebuild deliberately refuses to follow
//! the original. The original writes its slots beside `ENGINE.EXE`, among the
//! shipped data; [`Engine::set_saves`] rejects any path inside the game
//! directory, because that copy may well be read-only and is not ours to
//! change.

use crate::descriptor::{DESCRIPTOR_FIELDS, kind_code, kind_of, placement_code, placement_of};
use crate::resources::resolve;
use crate::{Descriptor, Engine, save};
use std::collections::BTreeMap;

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
    pub(crate) fn set_saves(&mut self, dir: &std::path::Path) -> std::result::Result<(), String> {
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
        self.saves = Some(saves);
        Ok(())
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
        self.saves
            .as_ref()
            .map(|d| d.join(format!("{id:03}.{suffix}")))
    }

    /// The display state a savegame keeps, as `PUTANIM` writes it.
    pub(crate) fn snapshot(&self) -> save::Anim {
        save::Anim {
            next_descriptor: self.next_descriptor,
            // The handle, not the index: `GETANIM` replaces the whole list, so
            // an index into the old one would point at a different descriptor.
            current: self
                .selected
                .and_then(|i| self.descriptors.get(i))
                .map(|d| d.handle),
            screen: self.display.current,
            pointer_visible: self.pointer_visible,
            dialog_offset: self.dialog_offset,
            dialog_return: self.dialog_return,
            palette: self.display.palette.raw,
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
            flips: self.flips.clone(),
            descriptors: self
                .descriptors
                .iter()
                .map(|d| save::DescriptorState {
                    handle: d.handle,
                    screen: d.screen,
                    x: d.x,
                    y: d.y,
                    level: d.level,
                    sprite: d.sprite.map(|v| v as i32),
                    block: d.block.map(|v| v as i32),
                    text: d.text,
                    table: d.table,
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
                    kind: kind_code(d.kind),
                    active: d.active,
                    auto_buffer: d.auto_buffer,
                    fields: d
                        .fields
                        .iter()
                        .map(|(k, v)| ((*k).to_string(), *v))
                        .collect(),
                })
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
    pub(crate) fn restore(&mut self, anim: save::Anim) -> std::result::Result<(), String> {
        let mut descriptors = Vec::with_capacity(anim.descriptors.len());
        for d in &anim.descriptors {
            let mut fields = BTreeMap::new();
            for (name, value) in &d.fields {
                let key = DESCRIPTOR_FIELDS
                    .iter()
                    .find(|k| **k == name.as_str())
                    .ok_or_else(|| format!("savegame names an unknown descriptor field {name}"))?;
                fields.insert(*key, *value);
            }
            descriptors.push(Descriptor {
                handle: d.handle,
                screen: d.screen,
                x: d.x,
                y: d.y,
                level: d.level,
                sprite: d.sprite.map(|v| v as u32),
                block: d.block.map(|v| v as u32),
                text: d.text,
                table: d.table,
                font: d.font,
                color: d.color.unwrap_or(0),
                template: d.template,
                wait: d.wait,
                callback: d.callback,
                x_mode: placement_of(d.x_mode)?,
                y_mode: placement_of(d.y_mode)?,
                fields,
                kind: kind_of(d.kind)?,
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
        self.display.palette = motionvm_formats::Palette::from_6bit(&anim.palette);
        self.next_descriptor = anim.next_descriptor;
        self.pointer_visible = anim.pointer_visible;
        self.dialog_offset = anim.dialog_offset;
        self.dialog_return = anim.dialog_return;
        self.selected = anim
            .current
            .and_then(|h| descriptors.iter().position(|d| d.handle == h));
        self.descriptors = descriptors;

        // The mirrored sprites are made again rather than carried: their ids
        // are not in any resource file, so nothing else could bring them back.
        for &(from, to) in &anim.flips {
            let Some(sprite) = self.sprite(from) else {
                continue;
            };
            let mut flipped = sprite.clone();
            let w = sprite.width as usize;
            for (row, out) in sprite
                .pixels
                .chunks_exact(w)
                .zip(flipped.pixels.chunks_exact_mut(w))
            {
                for (x, p) in row.iter().enumerate() {
                    out[w - 1 - x] = *p;
                }
            }
            self.sprites.insert(to, flipped);
        }
        self.flips = anim.flips;
        Ok(())
    }
}
