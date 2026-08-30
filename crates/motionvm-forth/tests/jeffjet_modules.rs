//! The 16-bit machine against the compiled modules of Jeff Jet - Abenteuer
//! InfoHighway.
//!
//! These need the original files: `DATA.-1-` and `DATA.-2-` for the modules
//! and `HPPLAY.EXE` for the kernel table. They skip themselves without them.
//!
//! Where `enviro_modules.rs` pins the machine against the game the 16-bit
//! readers were written for, this pins it against the one whose kernel table
//! is a *different* one: `HPPLAY.EXE` is an older build with five fewer words,
//! and its ordinals from 124 up sit four below their namesakes there. A table
//! taken from the other game would still bind — and would run this bytecode
//! calling the wrong words. Running `RUN` and watching which words it asks for
//! by name is what would catch that.
//!
//! The game this file drives is Jeff Jet (MOTION 16-bit).

use motionvm_formats::m16::{Container, Segment, mz, scr::ScrModule};
use motionvm_forth::m16::{Vm, word_address};
use motionvm_forth::{Error, Host, NullHost, Result};
use motionvm_testutil::{game_file, gamedata_jeffjet};

/// The container and a machine bound to the game's kernel, or `None` to skip.
fn machine() -> Option<(Container, Vm)> {
    let dir = gamedata_jeffjet()?;
    let c = Container::open_dir(&dir).expect("DATA.-1- and DATA.-2-");
    let img = mz::Image::open(game_file(&dir, "HPPLAY.EXE")).expect("HPPLAY.EXE");
    let binding = mz::binding_of(&img, &mz::kernel_words(&img)).expect("the kernel binds");
    Some((c, Vm::new(&binding)))
}

fn load(c: &Container, vm: &mut Vm, number: usize) {
    let item = c
        .item(Segment::Scr, number)
        .unwrap()
        .unwrap_or_else(|| panic!("module {number} is empty"));
    let parsed = ScrModule::parse(item).unwrap();
    vm.load(item, &parsed)
        .unwrap_or_else(|e| panic!("module {number}: {e}"));
}

struct Loader<'a> {
    container: &'a Container,
    asked: Vec<String>,
}

impl Host<Vm> for Loader<'_> {
    fn word(&mut self, name: &str, vm: &mut Vm) -> Result<bool> {
        match name {
            "=>GET" => {
                let n = vm.data.pop().expect("a module number") as usize;
                let item = self.container.item(Segment::Scr, n).unwrap().unwrap();
                let parsed = ScrModule::parse(item).unwrap();
                vm.load(item, &parsed)?;
                self.asked.push(format!("=>GET {n}"));
                Ok(true)
            }
            "=>ERASE" => {
                let n = vm.data.pop().expect("a module number") as u16;
                vm.unload(n);
                self.asked.push(format!("=>ERASE {n}"));
                Ok(true)
            }
            other => {
                self.asked.push(other.to_string());
                Ok(false)
            }
        }
    }
}

#[test]
fn run_executes_on_this_machine_up_to_the_first_engine_word() {
    let Some((c, mut vm)) = machine() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    load(&c, &mut vm, 100);
    let mut host = Loader {
        container: &c,
        asked: Vec::new(),
    };
    let run = word_address(&vm, 100, "RUN").unwrap();
    let err = vm.call(run, &mut host).unwrap_err();
    // The same boot shape as the other 16-bit game's, module for module: the
    // resident library, then the walk-direction module loaded and dropped
    // again once its table is in `DO_XDIR`, then `TOGFX` — the first word only
    // the engine can answer. Every name here came out of HPPLAY.EXE's own
    // table; a table borrowed from the later build would have named these
    // ordinals differently.
    assert_eq!(
        host.asked,
        [
            "=>GET 600",
            "=>GET 601",
            "=>GET 602",
            "=>GET 603",
            "=>GET 604",
            "=>GET 605",
            "=>GET 606",
            "=>GET 607",
            "=>GET 609",
            "=>GET 651",
            "=>ERASE 651",
            "TOGFX",
        ]
    );
    match err {
        Error::Unimplemented { name, at, .. } => {
            assert_eq!(name, "TOGFX");
            assert_eq!(at.module(), 100);
        }
        other => panic!("{other:?}"),
    }
    let do_xdir = word_address(&vm, 601, "DO_XDIR").unwrap();
    vm.data.clear();
    vm.call(do_xdir, &mut NullHost).unwrap();
    let addr = vm.data[0] as u16;
    // 1680 here, 1685 in Die Enviro-Kids greifen ein: the id is the game's,
    // the mechanism the engine's.
    assert_eq!(vm.mem.fetch(addr), 1680, "module 651's direction word");
    assert!(!vm.mem.is_loaded(651), "=>ERASE 651 dropped it");
    assert!(vm.mem.is_loaded(609));
}

/// What a constant word of a loaded module answers.
fn value(vm: &mut Vm, module: u16, name: &str) -> i32 {
    let addr =
        word_address(vm, module, name).unwrap_or_else(|| panic!("module {module} has no {name}"));
    vm.data.clear();
    vm.call(addr, &mut NullHost).unwrap();
    vm.data.pop().expect("a value")
}

#[test]
fn the_library_computes_the_table_sizes_the_blocks_are_cut_to() {
    let Some((c, mut vm)) = machine() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    for n in [100, 600, 601, 602, 603, 604, 605, 606, 607, 609] {
        load(&c, &mut vm, n);
    }
    // The per-location blocks are cut to what these constants multiply out to,
    // and the container hands them over at exactly those sizes — 1120, 452,
    // 300 and 182 bytes, for all thirteen locations. Reading the constants off
    // the machine and the blocks off the container is the two halves meeting.
    let lditem = value(&mut vm, 601, "A_LDITEM") * value(&mut vm, 601, "S_LDITEM");
    assert_eq!(lditem, 1120);
    for n in 1..=13 {
        assert_eq!(
            c.item(Segment::Blk, 200 + n).unwrap().map(<[u8]>::len),
            Some(lditem as usize),
            "block {}",
            200 + n
        );
    }
}
