//! *Captain Zins*' location logic held to the original's traces: the
//! generation's runner ([`parsec1994::trace`]) with this program's
//! routines, over the cases in `tests/capzins_logic_cases.txt`: the
//! locations' logic, the global rules and the shared hook.
//!
//! [`parsec1994::trace`]: crate::titles::parsec1994::trace

use super::{global_rules, hook, logic};
use crate::titles::parsec1994::trace::{Case, Routines, TraceTitle, parse, run_case};

/// Where `XP01.EXE` has the engine routines its logic calls: the far
/// targets of its own calls, each the counterpart of Dunkle Schatten's
/// routine of the same role by its body.
static ROUTINES: Routines = Routines {
    visit_setup: "0a01:003b",
    visit_enter: "0a01:023c",
    visit_finish_entry: "0a01:00ff",
    dispatch: "0a01:1006",
    frame_pass: "0a01:0213",
    visit_leave: "0a01:01b4",
    slot_unload: "02db:0c3e",
    hover_on: "0a01:18c1",
    input_poll: "163c:01b0",
    pointer_hide: "163c:0b02",
    pointer_unfreeze: "163c:0b9a",
    key_flush: "163c:099d",
    scene_play: "0849:1710",
    task_arm: "0849:1a0f",
    task_start: "0849:1a20",
    scene_sequence: "0a01:1665",
    slot_set_object: "0849:1a4f",
    slot_set_figure: "0849:1a91",
    obj_find: "0701:0518",
    // The library has no counterpart of Dunkle Schatten's `0883h:20B3h`:
    // `03E8h:1F9Fh` is followed by `03E8h:1FB9h` at once (`0x05E39`).
    obj_wake: None,
    obj_show: "1021:0813",
    obj_anim: "03e8:1ede",
    obj_anim_id: "03e8:1f9f",
    obj_hide: "1021:0912",
    obj_state: "1021:07b0",
    objs_save_state: "1021:0653",
    walk_to_node: "1021:09fd",
    anim_pass: "03e8:0302",
    room_redraw: "0701:027b",
    walker_snap: "05f2:0c05",
    walker_detach: "05f2:0ecb",
    input_lock: "0a01:185e",
    slot_set_raw: "0849:1ad5",
    obj_show_frame: "1021:09ad",
    movmem: "1da9:0020",
    setmem: "1da9:0002",
    movmem_from_local: "1da9:0020",
    heap_0720: "1db7:0720",
    mem_free: "1db7:0840",
    entry_door_close: "0a01:1471",
    input_unlock: "0a01:186c",
    face: "1021:0a13",
    pointer_shape_resync: "163c:1297",
    mover_find: "0701:0589",
    walker_bind_graph: "05f2:0e3a",
    walker_unlink: "05f2:0e1b",
    scene_enqueue: "0849:1762",
    ambient_play: "0849:1985",
    pal_set: "0105:09e0",
    song_load: "0ccf:0008",
    music_play: "0105:06a7",
    music_play_fade: "0105:07a3",
    music_fade: "0105:08a7",
    music_stop: "0105:08f8",
    hover_off: "0a01:18b0",
    play_sample: "0a01:18d7",
    walker_walk_to: "05f2:05db",
    inv_take: "078a:09e6",
    farptr_count: "1d7e:000e",
    inv_has: "078a:0b0b",
    random: "1d77:0026",
    script_exit: "10c3:05a6",
    inv_give: "078a:0961",
    panel_refresh: "078a:0069",
    fx_reveal: "0bf4:03a1",
    obj_invalidate: "03e8:000c",
    figure_anim: "03e8:1fb9",
    say_text: "0849:18a3",
    hotspot_follow: "1021:071e",
    figure_face: "05f2:0bc0",
    figure_turn: "05f2:1078",
    task_stop: "0849:19ef",
    page_release: "0849:1b63",
    band_hold: "078a:058e",
    node_block: "05f2:0f83",
    node_open: "05f2:0fdd",
    interaction: "0a01:174e",
    // `[1E7Eh]`: the hotspot record the choice went through, which the
    // global rules read its own lines from (`0x0B7C8`).
    chosen_hotspot: 0x1E7E,
    record_returns: &[
        "0701:0518",
        "0701:0589",
        "1021:0813",
        "1021:0912",
        "1021:09ad",
    ],
    // `[1A5Ah]`.
    exit_word: 0x1A5A,
    // `0x0BCBD`: `call far [18CEh]`.
    location_hook: 0x18CE,
    // `0x0BC8E`: `call far [1980h]`.
    callback: Some(0x1980),
};

/// This title, as the runner takes it.
static TITLE: TraceTitle = TraceTitle {
    routines: &ROUTINES,
    logic: logic::logic,
    global_rule: global_rules::api_rule,
    hook: hook::answer,
};

#[cfg(test)]
mod tests {
    use super::*;
    use motionvm_parsec_testutil::digest::Digest;

    /// The cases, in the order the file lists them.
    fn cases() -> Vec<Case> {
        parse(include_str!("../../../tests/capzins_logic_cases.txt"))
    }

    /// Every case has a name of its own: two heads with one name would
    /// share a line of the table.
    #[test]
    fn every_case_has_a_name_of_its_own() {
        let cases = cases();
        let names: std::collections::BTreeSet<String> = cases.iter().map(Case::name).collect();
        assert_eq!(names.len(), cases.len());
        assert_eq!(cases.len(), 1159);
    }

    /// The location logic does what the original routine does in every
    /// case: the same calls with the same arguments in the same order, the
    /// same bytes changed and the same end, held as one digest per case
    /// against the table, whose lines are anchored to the original.
    ///
    /// Needs the program's `DGROUP`, for the bytes the logic reads, and so
    /// the game's files — `MOTIONVM_GAMEDATA_CAPZINS` — and skips without
    /// them.
    #[test]
    fn the_location_logic_does_what_the_original_does_in_every_case() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_capzins() else {
            eprintln!("skipping: no Captain Zins gamedata directory");
            return;
        };
        let exe = motionvm_parsec_testutil::game_file(&dir, "XP01.EXE");
        let unpacked =
            motionvm_parsec_formats::Unpacked::parse(&std::fs::read(exe).expect("XP01.EXE reads"))
                .expect("XP01.EXE unpacks");
        let dgroup = u16::try_from(unpacked.dgroup().expect("a DGROUP") / 16).unwrap();
        let table =
            motionvm_parsec_testutil::Digests::of(env!("CARGO_MANIFEST_DIR"), "capzins-logic");
        for case in &cases() {
            let outcome = run_case(&TITLE, &unpacked.image, dgroup, case)
                .unwrap_or_else(|| panic!("{}: the logic has no routine for it", case.entry));
            let digest = Digest::new().bytes(outcome.text().as_bytes()).value();
            table.check(&case.name(), digest);
        }
    }
}
