//! Strings built into a word body: `_PutStringAdr`.
//!
//! The opcode (504, handler 0x665da) pushes the address of the bytes that
//! follow it and then steps over them — it is an instruction, not a whole word
//! behavior, and 147 sites across thirteen modules depend on that. Every
//! `DO_GIVE` and every `DO_INFO` in eleven location modules ends in one, as do
//! the fallbacks `CALCGIVE`/`CALCINFO` in the game library.
//!
//! Two things can go wrong and only one of them is visible in the answer: push
//! the wrong address, or step the wrong distance. A short step leaves the
//! interpreter standing on the string itself, which it would then run as
//! threaded code. So the test asserts both — the address, and that the word
//! afterwards returns instead of wandering off.
//!
//! The game data this file drives is Dunkle Schatten 2's (MOTION 32-bit).

use motionvm_engine::Game;
use motionvm_forth::Address;
use motionvm_testutil::gamedata_ds2;

/// Reads a NUL-terminated name out of module memory, the way every consumer of
/// one of these addresses does.
fn name_at(game: &Game, packed: u32) -> String {
    (0..16u32)
        .map(|i| {
            let at = Address::new(packed >> 16, (packed & 0xffff) + i);
            game.vm.mem.fetch_byte(at).unwrap_or(0)
        })
        .take_while(|b| *b != 0)
        .map(|b| b as char)
        .collect()
}

/// The two library fallbacks answer with the address of their own string.
///
/// `CALCINFO` is `… DROP _PutStringAdr "DINFO"` and `CALCGIVE` the same with
/// `"DGIVE"`; with no location hook and no global one installed, both take that
/// arm. The word returning at all is the second half of the assertion.
#[test]
fn a_word_can_answer_with_a_string_built_into_it() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    for (word, want) in [("CALCINFO", "DINFO"), ("CALCGIVE", "DGIVE")] {
        let mut game = Game::open(&dir).expect("game opens");
        // One value for the `DROP` that stands in front of the string.
        game.call(5, word, &[0])
            .unwrap_or_else(|e| panic!("5:{word}: {e}"));

        let answered = game.vm.data.pop().expect("one address") as u32;
        assert_eq!(
            name_at(&game, answered),
            want,
            "5:{word} should answer with its own name"
        );
        assert!(
            game.vm.data.is_empty(),
            "and with nothing else: {:?}",
            game.vm.data
        );

        // The address is the string itself, not the opcode cell before it.
        let before = Address::new(answered >> 16, (answered & 0xffff) - 4);
        assert_eq!(
            game.vm.fetch(before).expect("the cell before"),
            0x4000_01f8,
            "the address should start at the text, one cell past the opcode"
        );
    }
}

/// The one string in the game whose length divides by four.
///
/// `"GANRUFBA"` is eight characters, and there the compiler and the engine
/// disagree: the compiler laid down three cells (text, text, terminator), the
/// engine steps over `(8 + 3) >> 2 = 2` and lands on that terminator — which is
/// a zero cell, so the word returns from inside its own string table.
///
/// Measured in module 215: the text sits at 0x23c8, the padding cell at 0x23d0
/// is zero, and `_CheckElse` follows at 0x23d4. Taking the compiler's count
/// instead would land on that `_CheckElse`, whose jump reaches the same return
/// with the same value on the stack — the two are indistinguishable from
/// outside, which is why the original never noticed. This pins the address and
/// the clean return; the step count itself is pinned by the reading, not by
/// anything observable.
#[test]
fn a_string_whose_length_divides_by_four_still_lands_on_a_return() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    // The arm that offers the answering machine: the location's dialogue phase
    // is 1, and the thing being given is that machine.
    game.set_var(215, "_?ACTDIALPA", 1)
        .expect("the dialogue phase");
    let machine = {
        game.call(11, "ANRUFBEANTW", &[])
            .expect("the item constant");
        game.vm.data.pop().expect("its value")
    };
    game.call(215, "DO_GIVE", &[machine]).expect("215:DO_GIVE");

    let answered = game.vm.data.pop().expect("one address") as u32;
    assert_eq!(name_at(&game, answered), "GANRUFBA");
    assert!(
        game.vm.data.is_empty(),
        "and nothing else: {:?}",
        game.vm.data
    );
    // Two cells on from the text is the padding cell, and it is a return.
    let after = Address::new(answered >> 16, (answered & 0xffff) + 8);
    assert_eq!(
        game.vm.fetch(after).expect("the cell after"),
        0,
        "the engine steps onto a return"
    );
}
