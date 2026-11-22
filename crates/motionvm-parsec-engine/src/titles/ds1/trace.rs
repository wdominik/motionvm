//! *Dunkle Schatten*'s location logic held to the original's traces: the
//! generation's runner ([`parsec1994::trace`]) with this program's
//! routines, over the cases in `tests/ds1_logic_cases.txt`.

use super::{global_rules, logic};
use crate::titles::parsec1994::trace::{Case, Routines, TraceTitle, parse, run_case};

/// Where `XP01.EXE` has the engine routines its logic calls.
static ROUTINES: Routines = Routines {
    visit_setup: "0eb9:0001",
    visit_enter: "0eb9:0207",
    visit_finish_entry: "0eb9:00b8",
    dispatch: "0eb9:0fce",
    frame_pass: "0eb9:01de",
    visit_leave: "0eb9:017a",
    slot_unload: "0766:0d34",
    hover_on: "0eb9:1917",
    input_poll: "179e:01b0",
    pointer_hide: "179e:0882",
    pointer_unfreeze: "179e:091a",
    key_flush: "179e:071d",
    scene_play: "0cfd:172f",
    task_arm: "0cfd:1a2e",
    task_start: "0cfd:1a3f",
    scene_sequence: "0eb9:1691",
    slot_set_object: "0cfd:1a8c",
    slot_set_figure: "0cfd:1ad0",
    obj_find: "0bb6:052b",
    obj_wake: "0883:20b3",
    obj_show: "14dd:084a",
    obj_anim: "0883:1f82",
    obj_anim_id: "0883:2099",
    obj_hide: "14dd:096b",
    obj_state: "14dd:07e7",
    objs_save_state: "14dd:068a",
    walk_to_node: "14dd:0a56",
    anim_pass: "0883:0305",
    room_redraw: "0bb6:027a",
    walker_snap: "0a9f:0c59",
    walker_detach: "0a9f:0f42",
    input_lock: "0eb9:188a",
    slot_set_raw: "0cfd:1b14",
    obj_show_frame: "14dd:0a06",
    movmem: "1ee7:0022",
    setmem: "1ee7:0004",
    movmem_from_local: "1ee7:0022",
    heap_0720: "1ef8:0720",
    mem_free: "1ef8:0840",
    entry_door_close: "0eb9:149d",
    input_unlock: "0eb9:18a4",
    face: "14dd:0a77",
    pointer_shape_resync: "179e:1017",
    mover_find: "0bb6:05b0",
    walker_bind_graph: "0a9f:0ea6",
    walker_unlink: "0a9f:0e7f",
    scene_enqueue: "0cfd:1781",
    ambient_play: "0cfd:19a4",
    pal_set: "0590:09e2",
    song_load: "1197:000d",
    music_play: "0590:06a9",
    music_play_fade: "0590:07a5",
    music_fade: "0590:08a9",
    music_stop: "0590:08fa",
    hover_off: "0eb9:18f4",
    play_sample: "0eb9:1949",
    walker_walk_to: "0a9f:0617",
    inv_take: "0c3f:09e1",
    farptr_count: "1ebd:0000",
    inv_has: "0c3f:0b06",
    random: "1eb5:0028",
    script_exit: "1586:05e1",
    inv_give: "0c3f:095c",
    panel_refresh: "0c3f:005c",
    fx_reveal: "10b8:03ba",
    record_returns: &[
        "0bb6:052b",
        "0bb6:05b0",
        "14dd:084a",
        "14dd:096b",
        "14dd:0a06",
        "0883:20b3",
    ],
    // `[1F48h]`.
    exit_word: 0x1F48,
};

/// This title, as the runner takes it.
static TITLE: TraceTitle = TraceTitle {
    routines: &ROUTINES,
    logic: logic::logic,
    global_rule: global_rules::api_rule,
};

#[cfg(test)]
mod tests {
    use super::*;
    use motionvm_parsec_testutil::digest::Digest;

    /// The cases, in the order the file lists them.
    fn cases() -> Vec<Case> {
        parse(include_str!("../../../tests/ds1_logic_cases.txt"))
    }

    /// Every case has a name of its own: two heads with one name would
    /// share a line of the table.
    #[test]
    fn every_case_has_a_name_of_its_own() {
        let cases = cases();
        let names: std::collections::BTreeSet<String> = cases.iter().map(Case::name).collect();
        assert_eq!(names.len(), cases.len());
        assert_eq!(cases.len(), 1342);
    }

    /// The location logic does what the original routine does in every
    /// case: the same calls with the same arguments in the same order, the
    /// same bytes changed and the same end, held as one digest per case
    /// against the table, whose lines are anchored to the original.
    ///
    /// Needs the program's `DGROUP`, for the bytes the logic reads, and so
    /// the game's files — `MOTIONVM_GAMEDATA_DS1` — and skips without them.
    #[test]
    fn the_location_logic_does_what_the_original_does_in_every_case() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        let exe = motionvm_parsec_testutil::game_file(&dir, "XP01.EXE");
        let unpacked =
            motionvm_parsec_formats::Unpacked::parse(&std::fs::read(exe).expect("XP01.EXE reads"))
                .expect("XP01.EXE unpacks");
        let dgroup = u16::try_from(unpacked.dgroup().expect("a DGROUP") / 16).unwrap();
        let table = motionvm_parsec_testutil::Digests::of(env!("CARGO_MANIFEST_DIR"), "ds1-logic");
        for case in &cases() {
            let outcome = run_case(&TITLE, &unpacked.image, dgroup, case)
                .unwrap_or_else(|| panic!("{}: the logic has no routine for it", case.entry));
            let digest = Digest::new().bytes(outcome.text().as_bytes()).value();
            table.check(&case.name(), digest);
        }
    }
}
