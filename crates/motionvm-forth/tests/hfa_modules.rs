//! The 16-bit machine against the compiled modules of Hilfe für Amajambere.
//!
//! These need the original files: `DATA.-1-` and `DATA.-2-` for the modules
//! and `BMZ.EXE` for the kernel table. They skip themselves without them.
//!
//! Where `jeffjet_modules.rs` pins the machine against a build whose ordinals
//! are shifted against ENVIRO's, this pins it against the third build — one
//! whose ordinals are *not* shifted, being ENVIRO's table less its last word.
//! That is the easier case to get wrong quietly: a table baked from ENVIRO
//! would bind this bytecode and name every word of it correctly, right up to
//! an ordinal this build does not have. Running `RUN` and watching which words
//! it asks for by name is what holds the binding to the game's own binary.
//!
//! The game this file drives is Hilfe für Amajambere (MOTION 16-bit).

use motionvm_formats::m16::{Container, Segment, mz, scr::ScrModule};
use motionvm_forth::m16::{Vm, word_address};
use motionvm_forth::{Error, Host, NullHost, Result};
use motionvm_testutil::{game_file, gamedata_hfa};

/// The container and a machine bound to the game's kernel, or `None` to skip.
fn machine() -> Option<(Container, Vm)> {
    let dir = gamedata_hfa()?;
    let c = Container::open_dir(&dir).expect("DATA.-1- and DATA.-2-");
    let img = mz::Image::open(game_file(&dir, "BMZ.EXE")).expect("BMZ.EXE");
    let binding = mz::binding_of(&mz::kernel_words(&img)).expect("the kernel binds");
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
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    load(&c, &mut vm, 100);
    let mut host = Loader {
        container: &c,
        asked: Vec::new(),
    };
    let run = word_address(&vm, 100, "RUN").unwrap();
    let err = vm.call(run, &mut host).unwrap_err();
    // The sibling games' opening, word for word: the library modules resident,
    // 651 fetched for one variable and dropped again, then the graphics mode.
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
    // 1680, as in Jeff Jet; 1685 in Die Enviro-Kids greifen ein. The id is the
    // game's, the mechanism the engine's.
    assert_eq!(vm.mem.fetch(addr), 1680, "module 651's direction word");
    assert!(!vm.mem.is_loaded(651), "=>ERASE 651 dropped it");
    assert!(vm.mem.is_loaded(609));
}

#[test]
fn the_library_computes_the_table_sizes_the_blocks_are_cut_to() {
    let Some((c, mut vm)) = machine() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    load(&c, &mut vm, 601);
    let constant = |vm: &mut Vm, name: &str| {
        let at = word_address(vm, 601, name).unwrap_or_else(|| panic!("no {name}"));
        vm.data.clear();
        vm.call(at, &mut NullHost).unwrap();
        vm.data[0]
    };
    let (a, s) = (constant(&mut vm, "A_LDITEM"), constant(&mut vm, "S_LDITEM"));
    assert_eq!((a, s), (35, 32));
    let size = (a * s) as usize;
    assert_eq!(size, 1120);
    // `INCLLOC` reads block 300+N into `_LDITEM` at exactly that size — a
    // different family number from the sibling games' 200+N, the same length.
    // Location 7 is the one this game ships no table for.
    for n in 1..=20usize {
        let item = c.item(Segment::Blk, 300 + n).unwrap();
        let expected = if n == 7 { None } else { Some(size) };
        assert_eq!(item.map(<[u8]>::len), expected, "block {}", 300 + n);
    }
}
