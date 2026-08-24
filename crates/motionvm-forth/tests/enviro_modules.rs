//! The 16-bit machine against the compiled modules of Die Enviro-Kids
//! greifen ein.
//!
//! These need the original files: `DATA.-1-` for the modules and `ENVIRO.EXE`
//! for the kernel table. They skip themselves without them. What they pin is
//! that the machine runs the game's own bytecode as the modules' structure
//! says it must — the library's helper words compute what their names say,
//! the resident set binds every id it calls, and `RUN` executes up to the
//! first word that only the engine can answer.

use motionvm_formats::m16::{Container, Segment, mz, scr::ScrModule};
use motionvm_forth::m16::{Vm, word_address};
use motionvm_forth::{Error, Host, NullHost, Result};
use motionvm_testutil::{game_file, gamedata_enviro};

/// The container and a machine bound to the game's kernel, or `None` to skip.
fn machine() -> Option<(Container, Vm)> {
    let dir = gamedata_enviro()?;
    let c = Container::open_dir(&dir).expect("DATA.-1-");
    let img = mz::Image::open(game_file(&dir, "ENVIRO.EXE")).expect("ENVIRO.EXE");
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

/// `RUN`'s resident library plus the boot module.
const LIBRARY: [usize; 10] = [100, 600, 601, 602, 603, 604, 605, 606, 607, 609];

#[test]
fn the_library_helpers_compute_what_their_names_say() {
    let Some((c, mut vm)) = machine() else {
        eprintln!("skipping: no ENVIRO gamedata directory");
        return;
    };
    for n in LIBRARY {
        load(&c, &mut vm, n);
    }
    // `STARTLOC` is a variable in module 601 whose compiled value is 13; `++`
    // in module 600 is `DUP @ 1 + SWAP !`. Run the one, then the other.
    let startloc = word_address(&vm, 601, "STARTLOC").unwrap();
    vm.call(startloc, &mut NullHost).unwrap();
    let addr = vm.data[0] as u16;
    assert_eq!(vm.mem.fetch(addr) as i16, 13, "STARTLOC's compiled value");
    vm.call(word_address(&vm, 600, "++").unwrap(), &mut NullHost)
        .unwrap();
    assert!(vm.data.is_empty());
    assert_eq!(vm.mem.fetch(addr) as i16, 14, "++ incremented it in place");
    // `2*` is `DUP +`; `0!` stores zero; `DUP0` is `DUP 0`.
    vm.data.push(21);
    vm.call(word_address(&vm, 600, "2*").unwrap(), &mut NullHost)
        .unwrap();
    assert_eq!(vm.data, [42]);
    vm.data.clear();
    vm.data.push(addr as i32);
    vm.call(word_address(&vm, 600, "0!").unwrap(), &mut NullHost)
        .unwrap();
    assert_eq!(vm.mem.fetch(addr), 0);
    vm.data.push(5);
    vm.call(word_address(&vm, 600, "DUP0").unwrap(), &mut NullHost)
        .unwrap();
    assert_eq!(vm.data, [5, 5, 0]);
    vm.data.clear();
    // `LOCINIT` is `CONST 549`; `A_LDITEM S_LDITEM *` is the item table's
    // byte size, 35 × 32.
    vm.call(word_address(&vm, 601, "LOCINIT").unwrap(), &mut NullHost)
        .unwrap();
    assert_eq!(vm.data, [549]);
    vm.data.clear();
    for w in ["A_LDITEM", "S_LDITEM", "*"] {
        let at = match w {
            "*" => {
                // A kernel word has no address; assemble the multiply by
                // running the two constants and multiplying on the stack.
                let (b, a) = (vm.data.pop().unwrap(), vm.data.pop().unwrap());
                vm.data.push(a * b);
                continue;
            }
            _ => word_address(&vm, 601, w).unwrap(),
        };
        vm.call(at, &mut NullHost).unwrap();
    }
    assert_eq!(vm.data, [1120]);
}

#[test]
fn the_resident_set_binds_every_id_the_library_calls_or_names_a_transient_module() {
    let Some((c, mut vm)) = machine() else {
        eprintln!("skipping: no ENVIRO gamedata directory");
        return;
    };
    for n in LIBRARY {
        load(&c, &mut vm, n);
    }
    // Ids defined by modules `RUN` loads only briefly, or by a location's
    // three modules: a call to one of these from the library is resolved
    // only while that module is resident.
    let transient = |id: u16| {
        (1050..=1087).contains(&id)      // 610, the intro
            || id == 1092                 // 611
            || (1449..=1450).contains(&id) // 612
            || (420..=431).contains(&id)  // 650, the save menu
            || (1670..=1685).contains(&id) // 651, the walk directions
            || (895..=897).contains(&id)  // 614, dialogue
            || (1110..=1154).contains(&id) // 615, the newspaper
            || id == 549                  // every location macro
            || (560..=750).contains(&id)  // scene modules
            || (750..=800).contains(&id) // click modules
    };
    let binding = vm.binding().clone();
    let dis = motionvm_formats::m16::disasm::Disassembler::new(&binding);
    let mut unresolved = Vec::new();
    for n in LIBRARY {
        let m = ScrModule::parse(c.item(Segment::Scr, n).unwrap().unwrap()).unwrap();
        for e in &m.entries {
            for cell in dis.decode(e) {
                if let motionvm_formats::m16::disasm::Cell::Call { id, .. } = cell
                    && vm.mem.resolve(id).is_none()
                    && !transient(id)
                {
                    unresolved.push((n, e.name.clone(), id));
                }
            }
        }
    }
    assert!(unresolved.is_empty(), "{unresolved:?}");
}

/// A host that answers `=>GET` and `=>ERASE` out of the container and
/// records every other kernel word it is asked for.
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
        eprintln!("skipping: no ENVIRO gamedata directory");
        return;
    };
    load(&c, &mut vm, 100);
    let mut host = Loader {
        container: &c,
        asked: Vec::new(),
    };
    let run = word_address(&vm, 100, "RUN").unwrap();
    let err = vm.call(run, &mut host).unwrap_err();
    // `RUN` loads the library, loads and drops the walk-direction module
    // after storing its table into DO_XDIR, and then calls TOGFX — the first
    // word only the engine can answer.
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
    // `1685 DO_XDIR !` ran: the variable in module 601 holds the id of
    // module 651's direction word.
    let do_xdir = word_address(&vm, 601, "DO_XDIR").unwrap();
    vm.data.clear();
    vm.call(do_xdir, &mut NullHost).unwrap();
    let addr = vm.data[0] as u16;
    assert_eq!(vm.mem.fetch(addr), 1685);
    // The ten constants 1..10 `RUN` pushed before `=>GET 651` are still on
    // the stack below — it pushes them and never pops them before TOGFX.
    assert!(!vm.mem.is_loaded(651), "=>ERASE 651 dropped it");
    assert!(vm.mem.is_loaded(609));
}
