//! Victor Loomes: the car key on the car, in all three of its outcomes.
//!
//! These need the game's files — `MOTIONVM_GAMEDATA_VLOOMES` — and skip
//! without them.
//!
//! What they guard is the return-stack layout of `DO … LOOP`. `CALC_XUSE`
//! (module 104) asks `SCHLÜSSEL ?INVINCL` on the way to every answer but the
//! key-consuming one, and `?INVINCL` (module 603) leaves its inventory scan
//! through `STOPLOOP` — `R> R> DROP R> DUP >R >R >R`, which drops the index
//! and doubles the limit *on the return stack*. That only works on a machine
//! that keeps both cells there, the way `_LoopStart` does; a limit kept
//! anywhere else turns the early exit into a loop that never ends, which is
//! how using the key on the car stopped the game on its step limit.
//!
//! The order path is the game's own: a click stores the order and `WALKER`
//! runs `_DO_ORDER @ EXECUTE` when the walk lands, which dispatches on
//! `?OBJORDER` to `DO_USE` (module 608). The tests here enter at `DO_ORDER`
//! with the order variables set as `GIVE_ORDER` would leave them —
//! `_OBJORDER` = `O:USE` (2), `_OBJACT` = `SCHLÜSSEL` (21), `_OBJPAS` =
//! `AUTO` (22) — which is the landed order without the walk.

use motionvm_engine::{Game, Playable, titles};
use motionvm_forth::m16::Vm;
use motionvm_forth::{Address, Machine};
use motionvm_testutil::gamedata_vloomes;

/// Runs `RUN` until it has left the intro and entered its first location.
fn into_the_game() -> Option<Game<Vm>> {
    let dir = gamedata_vloomes()?;
    let mut game = titles::vloomes::open(&dir).expect("the game opens");
    game.start().expect("RUN starts");
    for i in 0..4000 {
        let click = i % 150 >= 100 && i % 150 < 102;
        game.set_input(160, 100, click, false, 0).expect("input");
        game.pump().expect("pump");
        game.step().expect("step");
        if game.get_var(605, "AO").unwrap_or(0) > 0 {
            for _ in 0..600 {
                game.set_input(160, 100, false, false, 0).expect("input");
                game.pump().expect("pump");
                game.step().expect("step");
            }
            return Some(game);
        }
    }
    panic!("the intro never reached a location");
}

/// Into the street — location 4, where the car is.
fn at_the_car() -> Option<Game<Vm>> {
    let mut game = into_the_game()?;
    game.request_location(4).expect("ask for location 4");
    for _ in 0..900 {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.pump().expect("pump");
        game.step().expect("step");
    }
    assert_eq!(game.get_var(605, "AO"), Some(4), "arrived at the street");
    Some(game)
}

/// Frames with no input, panicking with the frame number and the VM's own
/// position when a step stops — `StepLimit` carries no address, so `here()`
/// is the one thing that names the looping cell.
fn frames(game: &mut Game<Vm>, n: i32, what: &str) {
    for f in 0..n {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.pump()
            .unwrap_or_else(|e| panic!("{what}, frame {f}, at {}: {e}", game.vm.here()));
        game.step()
            .unwrap_or_else(|e| panic!("{what}, frame {f}, at {}: {e}", game.vm.here()));
    }
}

/// `NEWSREAD` and `ZMTEST` are not variables but offsets into `KOND`,
/// module 604's state array: `30 K+` and `34 K+`. `set_variable` writes one
/// cell past the address it is given — a variable's `_PutAdr` — so aiming it
/// `offset` bytes into `KOND`'s body hits exactly the cell `n K+ @` reads.
fn set_kond(game: &mut Game<Vm>, offset: u32, value: i32) {
    let kond = game.address(604, "KOND").expect("KOND");
    game.vm
        .set_variable(Address::new(kond.module(), kond.offset() + offset), value)
        .expect("a KOND cell");
}

/// The landed order: use the key on the car.
fn key_on_car(game: &mut Game<Vm>) -> Result<(), motionvm_engine::Error> {
    game.set_var(605, "_OBJORDER", 2).expect("O:USE");
    game.set_var(605, "_OBJACT", 21).expect("SCHLÜSSEL");
    game.set_var(605, "_OBJPAS", 22).expect("AUTO");
    game.call(608, "DO_ORDER", &[])
}

#[test]
fn key_on_car_before_the_newspaper() {
    let Some(mut game) = at_the_car() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    game.vm.step_limit = 300_000;
    key_on_car(&mut game).unwrap_or_else(|e| panic!("DO_ORDER, at {}: {e}", game.vm.here()));
    frames(&mut game, 1200, "after the order");
    assert_eq!(game.get_var(605, "AO"), Some(4), "still in the street");
}

#[test]
fn key_on_car_after_the_newspaper_drives_off() {
    let Some(mut game) = at_the_car() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    game.vm.step_limit = 300_000;
    set_kond(&mut game, 30, 1); // NEWSREAD
    key_on_car(&mut game).unwrap_or_else(|e| panic!("DO_ORDER, at {}: {e}", game.vm.here()));
    // `CALC_XUSE` has set `SETBUSY` and `1 SET_LOCALTA`: the drive-away runs
    // in `CALCMOVE`'s task 1 over the following frames and ends on
    // `0 _CM_STAT ! … 2 NAO !` — the car takes Victor to location 2.
    for f in 0..6000 {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.pump()
            .unwrap_or_else(|e| panic!("drive-away, frame {f}, at {}: {e}", game.vm.here()));
        game.step()
            .unwrap_or_else(|e| panic!("drive-away, frame {f}, at {}: {e}", game.vm.here()));
        if game.get_var(605, "AO") == Some(2) {
            return;
        }
    }
    panic!(
        "the drive-away never arrived: AO {:?}, _CM_STAT {:?}",
        game.get_var(605, "AO"),
        game.get_var(606, "_CM_STAT")
    );
}

#[test]
fn key_on_car_with_the_savings_contract_takes_the_key() {
    let Some(mut game) = at_the_car() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    game.vm.step_limit = 300_000;
    set_kond(&mut game, 34, 1); // ZMTEST
    key_on_car(&mut game).unwrap_or_else(|e| panic!("DO_ORDER, at {}: {e}", game.vm.here()));
    frames(&mut game, 1200, "after the key is taken");
    assert_eq!(game.get_var(605, "AO"), Some(4), "still in the street");
}
