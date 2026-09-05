//! The 16-bit machine against the compiled modules of Falsches Spiel mit
//! Eddie M.
//!
//! These need the original files: the three `DATA.-n-` volumes for the
//! modules and `STERN.EXE` for the kernel table. They skip themselves without
//! them.
//!
//! This is the build that binds like `LL.EXE` and reads like `HPPLAY.EXE`: an
//! eighty-word core table puts its domain table at 102, three below the three
//! later builds', while the domain words are Jeff Jet's 146, name for name.
//! A table baked from either neighbor would bind this bytecode and run it
//! against the wrong handlers from the first domain word on. Running `RUN`
//! and watching which words it asks for by name is what holds the binding to
//! the game's own binary — and what shows the ten constants `RUN` leaves on
//! the stack, which its `CTRL` checks on every frame.
//!
//! The game this file drives is Falsches Spiel mit Eddie M. (MOTION 16-bit).

use motionvm_motion_formats::m16::{Container, Segment, mz, scr::ScrModule};
use motionvm_motion_forth::cell;
use motionvm_motion_forth::m16::{Vm, word_address};
use motionvm_motion_forth::{Error, Host, NullHost, Result};
use motionvm_motion_testutil::{game_file, gamedata_eddiem};

/// The container and a machine bound to the game's kernel, or `None` to skip.
fn machine() -> Option<(Container, Vm)> {
    let dir = gamedata_eddiem()?;
    let c = Container::open_dir(&dir).expect("the three volumes");
    let img = mz::Image::open(game_file(&dir, "STERN.EXE")).expect("STERN.EXE");
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
    fn word(&mut self, ordinal: u32, vm: &mut Vm) -> Result<bool> {
        let name = vm.ordinal_name(ordinal).unwrap_or_default().to_owned();
        match name.as_str() {
            "=>GET" => {
                let n = cell::at(vm.data.pop().expect("a module number")).unwrap();
                let item = self.container.item(Segment::Scr, n).unwrap().unwrap();
                let parsed = ScrModule::parse(item).unwrap();
                vm.load(item, &parsed)?;
                self.asked.push(format!("=>GET {n}"));
                Ok(true)
            }
            "=>ERASE" => {
                let n = cell::low16(vm.data.pop().expect("a module number"));
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
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    load(&c, &mut vm, 100);
    let mut host = Loader {
        container: &c,
        asked: Vec::new(),
    };
    let run = word_address(&vm, 100, "RUN").unwrap();
    let err = vm.call(run, &mut host).unwrap_err();
    // The sibling games' opening less the walk-direction module they fetch
    // and drop: the resident library, then straight into the graphics mode.
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
    // The ten constants `RUN` pushes before `TOGFX` and never pops. They are
    // a sentinel: `CTRL` opens every frame on `DUP 10 !=` and prints the top
    // of the stack when the check fails, so a kernel word that leaves a cell
    // behind is caught by the game itself.
    assert_eq!(vm.data, (1..=10).collect::<Vec<i32>>());
    assert!(vm.mem.is_loaded(609));
    assert!(!vm.mem.is_loaded(610), "the intro is fetched later, by RUN");
}

#[test]
fn the_library_computes_the_table_sizes_the_blocks_are_cut_to() {
    let Some((c, mut vm)) = machine() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
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
    let size = cell::at(a * s).unwrap();
    assert_eq!(size, 1120);
    // `INCLLOC` reads block 200+N into `_LDITEM` at exactly that size, for
    // every one of the fifteen locations.
    for n in 1..=15usize {
        let item = c.item(Segment::Blk, 200 + n).unwrap();
        assert_eq!(item.map(<[u8]>::len), Some(size), "block {}", 200 + n);
    }
    // And the variable the game opens in: `STARTLOC` is declared as 3, the
    // flat, and `RUN` enters it without storing anything first.
    let startloc = word_address(&vm, 601, "STARTLOC").unwrap();
    vm.data.clear();
    vm.call(startloc, &mut NullHost).unwrap();
    let addr = cell::low16(vm.data[0]);
    assert_eq!(vm.mem.fetch(addr), 3);
}
