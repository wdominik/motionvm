//! The 16-bit machine against the compiled modules of Victor Loomes.
//!
//! These need the original files: `DATA.-1-` for the modules and `LL.EXE` for
//! the kernel table. They skip themselves without them.
//!
//! The other three suites pin the machine against builds whose domain table
//! binds at 105. This one is the build that does not: its core table is two
//! words shorter and it registers one placeholder fewer, so its domain table
//! binds at 102. That is the failure this file exists to make loud. A table
//! baked from any later build binds this bytecode without complaint and names
//! every word of it three places along — `SETSHADE` would come out as
//! `NEWANIM`, `SDTXT` would not be reachable at all — and nothing would say
//! so. Walking every module and running `RUN` until it asks the engine for a
//! word by name is what holds the base to the game's own binary.
//!
//! The game this file drives is Victor Loomes (MOTION 16-bit).

use motionvm_formats::m16::{Container, Segment, disasm, mz, scr::ScrModule};
use motionvm_forth::m16::{Vm, word_address};
use motionvm_forth::{Error, Host, Result};
use motionvm_testutil::{game_file, gamedata_vloomes};

/// The container and a machine bound to the game's kernel, or `None` to skip.
fn machine() -> Option<(Container, Vm)> {
    let dir = gamedata_vloomes()?;
    let c = Container::open_dir(&dir).expect("DATA.-1-");
    let img = mz::Image::open(game_file(&dir, "LL.EXE")).expect("LL.EXE");
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
fn run_loads_the_library_this_game_has_and_asks_for_its_own_first_word() {
    let Some((c, mut vm)) = machine() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    load(&c, &mut vm, 100);
    let mut host = Loader {
        container: &c,
        asked: Vec::new(),
    };
    let run = word_address(&vm, 100, "RUN").unwrap();
    let err = vm.call(run, &mut host).unwrap_err();
    // The later games fetch 600 to 607 and 609; this one has no 601 — its
    // location variables live in 605 — and fetches 608 instead, which is
    // where its inventory words are.
    assert_eq!(
        host.asked,
        [
            "=>GET 600",
            "=>GET 602",
            "=>GET 603",
            "=>GET 604",
            "=>GET 605",
            "=>GET 606",
            "=>GET 607",
            "=>GET 608",
            "=>GET 609",
            "BUFON",
        ]
    );
    // The proof of the base: `BUFON` is the domain table's index 99, so its
    // ordinal is 201. Bound at the later builds' 105 the same cell would name
    // the word three places along, and nothing would say so.
    match err {
        Error::Unimplemented { name, ordinal, at } => {
            assert_eq!(name, "BUFON");
            assert_eq!(ordinal, 201);
            assert_eq!(at.module(), 100);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_module_loads_and_no_cell_names_a_word_the_kernel_does_not_have() {
    let Some((c, mut vm)) = machine() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    // Through the disassembler rather than cell by cell, because a cell that
    // follows `_PutLit` is a literal and a negative one carries bit 15: only
    // something that knows which words take an operand can tell an ordinal
    // from a number.
    let modules: Vec<ScrModule> = c
        .present(Segment::Scr)
        .into_iter()
        .map(|n| ScrModule::parse(c.item(Segment::Scr, n).unwrap().unwrap()).unwrap())
        .collect();
    assert_eq!(modules.len(), 36);
    let mut d = disasm::Disassembler::new(vm.binding());
    for m in &modules {
        d.learn(m);
    }
    let mut bodies = 0;
    for m in &modules {
        for e in &m.entries {
            if e.is_variable() || e.is_constant() {
                continue;
            }
            bodies += 1;
            let cells = d.decode(e);
            assert!(
                !cells
                    .iter()
                    .any(|c| matches!(c, disasm::Cell::UnknownKernel { .. })),
                "module {} word {}: an ordinal the table does not name",
                m.module,
                e.name
            );
            assert!(
                matches!(cells.last(), Some(disasm::Cell::Kernel { name, .. }) if name == "##"),
                "module {} word {} does not end on ##",
                m.module,
                e.name
            );
        }
    }
    assert_eq!(bodies, 695, "every colon definition in the game");

    // Loading is per module because the ids are reused: every one of the
    // twelve location macros defines id 549.
    for id in c.present(Segment::Scr) {
        let item = c.item(Segment::Scr, id).unwrap().unwrap();
        let parsed = ScrModule::parse(item).unwrap();
        vm.load(item, &parsed)
            .unwrap_or_else(|e| panic!("module {id}: {e}"));
        vm.unload(id as u16);
    }
}
