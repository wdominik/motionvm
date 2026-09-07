# What "green" means, in one command.
#
# CI runs the same four commands on three platforms, but without the game data,
# so it is the floor; `just check` with your own copy of a game is the gate —
# what has to pass before a change is finished.
#
# The game data is not in the repository and cannot be. Point GAMEDATA_DS2 at
# your copy of Dunkle Schatten 2, GAMEDATA_ENVIRO at your copy of Die
# Enviro-Kids greifen ein, GAMEDATA_JEFFJET at your copy of Jeff Jet -
# Abenteuer InfoHighway, GAMEDATA_HFA at your copy of Hilfe für Amajambere,
# GAMEDATA_VLOOMES at your copy of Victor Loomes – Das Spiel, GAMEDATA_EDDIEM
# at your copy of Falsches Spiel mit Eddie M. and GAMEDATA_CHECKER at your copy
# of Checker 2000 — the defaults are directories next to this one, which
# is where a checkout beside installed copies of the games finds them. Tests
# that need data and cannot find it skip themselves; a *wrong* path panics
# rather than skipping, so a typo cannot read as "no data on this machine".
DEFAULT_GAMEDATA_DS2 := justfile_directory() / ".." / "games" / "DS2"
GAMEDATA_DS2 := DEFAULT_GAMEDATA_DS2
DEFAULT_GAMEDATA_ENVIRO := justfile_directory() / ".." / "games" / "ENVIRO"
GAMEDATA_ENVIRO := DEFAULT_GAMEDATA_ENVIRO
DEFAULT_GAMEDATA_JEFFJET := justfile_directory() / ".." / "games" / "JEFFJET"
GAMEDATA_JEFFJET := DEFAULT_GAMEDATA_JEFFJET
DEFAULT_GAMEDATA_HFA := justfile_directory() / ".." / "games" / "HFA"
GAMEDATA_HFA := DEFAULT_GAMEDATA_HFA
DEFAULT_GAMEDATA_VLOOMES := justfile_directory() / ".." / "games" / "VLOOMES"
GAMEDATA_VLOOMES := DEFAULT_GAMEDATA_VLOOMES
DEFAULT_GAMEDATA_EDDIEM := justfile_directory() / ".." / "games" / "EDDIEM"
GAMEDATA_EDDIEM := DEFAULT_GAMEDATA_EDDIEM
DEFAULT_GAMEDATA_CHECKER := justfile_directory() / ".." / "games" / "CHECKER"
GAMEDATA_CHECKER := DEFAULT_GAMEDATA_CHECKER

# What actually reaches the suite.
#
# A path you named is always passed through, so a typo panics and says so. The
# built-in default is passed only when the data is really there — otherwise a
# clone on a machine that has no copy of a game would panic on the very
# command this file exists to define, instead of skipping the way the README
# describes. The five 16-bit games are told apart by their engine binary: they
# all ship a DATA.-1-, so probing for that would let any of those defaults match
# another game's directory. The two 32-bit games both ship 001.RSC and
# ENGINE.EXE, so each is probed for the file the other lacks: the loose 000.PAL
# of Dunkle Schatten 2, the ENGINE.RSC of Checker 2000.
_DATA_DS2 := if GAMEDATA_DS2 != DEFAULT_GAMEDATA_DS2 { GAMEDATA_DS2 } \
    else if path_exists(GAMEDATA_DS2 / "000.PAL") == "true" { GAMEDATA_DS2 } \
    else { "" }
_DATA_ENVIRO := if GAMEDATA_ENVIRO != DEFAULT_GAMEDATA_ENVIRO { GAMEDATA_ENVIRO } \
    else if path_exists(GAMEDATA_ENVIRO / "ENVIRO.EXE") == "true" { GAMEDATA_ENVIRO } \
    else { "" }
_DATA_JEFFJET := if GAMEDATA_JEFFJET != DEFAULT_GAMEDATA_JEFFJET { GAMEDATA_JEFFJET } \
    else if path_exists(GAMEDATA_JEFFJET / "HPPLAY.EXE") == "true" { GAMEDATA_JEFFJET } \
    else { "" }
_DATA_HFA := if GAMEDATA_HFA != DEFAULT_GAMEDATA_HFA { GAMEDATA_HFA } \
    else if path_exists(GAMEDATA_HFA / "BMZ.EXE") == "true" { GAMEDATA_HFA } \
    else { "" }
_DATA_VLOOMES := if GAMEDATA_VLOOMES != DEFAULT_GAMEDATA_VLOOMES { GAMEDATA_VLOOMES } \
    else if path_exists(GAMEDATA_VLOOMES / "LL.EXE") == "true" { GAMEDATA_VLOOMES } \
    else { "" }
_DATA_EDDIEM := if GAMEDATA_EDDIEM != DEFAULT_GAMEDATA_EDDIEM { GAMEDATA_EDDIEM } \
    else if path_exists(GAMEDATA_EDDIEM / "STERN.EXE") == "true" { GAMEDATA_EDDIEM } \
    else { "" }
_DATA_CHECKER := if GAMEDATA_CHECKER != DEFAULT_GAMEDATA_CHECKER { GAMEDATA_CHECKER } \
    else if path_exists(GAMEDATA_CHECKER / "ENGINE.RSC") == "true" { GAMEDATA_CHECKER } \
    else { "" }

# Savegames cannot be reconstructed, only played to, so there is no default that
# could work. Set it to the directory the games' own save directories are under
# — the `saves/` the program writes, not `saves/ds2/` — because that is what
# the engine is pointed at and it puts the game's name on itself. With one,
# `just test SAVES=…` runs the savegame test; without it that one test skips
# itself and says so.
SAVES := ""

# Where every game reaches a cargo command, written once.
#
# Three recipes hand the suite its data. Spelling the seven variables out in
# each would make an eighth game a matter of remembering all three; this is
# the one block they share, and adding a game touches it once.
_GAMES := 'MOTIONVM_GAMEDATA_DS2="' + _DATA_DS2 + '" ' + \
    'MOTIONVM_GAMEDATA_ENVIRO="' + _DATA_ENVIRO + '" ' + \
    'MOTIONVM_GAMEDATA_JEFFJET="' + _DATA_JEFFJET + '" ' + \
    'MOTIONVM_GAMEDATA_HFA="' + _DATA_HFA + '" ' + \
    'MOTIONVM_GAMEDATA_VLOOMES="' + _DATA_VLOOMES + '" ' + \
    'MOTIONVM_GAMEDATA_EDDIEM="' + _DATA_EDDIEM + '" ' + \
    'MOTIONVM_GAMEDATA_CHECKER="' + _DATA_CHECKER + '" ' + \
    'MOTIONVM_SAVES="' + SAVES + '"'

_default:
    @just --list

# Everything. Run this before calling a change done.
check: fmt-check clippy test doc

# The test suite, with the games' files.
test:
    {{ _GAMES }} RUSTFLAGS="-D warnings" cargo test --workspace

# `just test` above cannot do this. The per-game variables fall back to
# `../games/<GAME>` beside the checkout — which is the arrangement the README
# recommends — and the fallback is anchored at compile time, so neither an
# empty variable nor a different working directory escapes it. Short of moving
# `../games` aside, the data-free tests could not be exercised in isolation,
# and the data-free path is the only thing CI proves: a test that quietly
# starts needing a file, or a skip that stops being a skip, was invisible here
# and only surfaced after a push. This is the floor, not the gate: `just
# check` is still what has to pass before a change is finished.
#
# CI's floor, on your own machine: the suite with no game data at all.
check-nodata:
    MOTIONVM_NO_GAMEDATA=1 RUSTFLAGS="-D warnings" cargo test --workspace

# One test target, e.g. `just test-one ds2_scenes` or `just test-one enviro_psm`.
test-one target:
    {{ _GAMES }} cargo test --workspace --test {{ target }} -- --nocapture

# Rewrite the reference digest tables from a run of the suite.
#
# Every scene the suites compose and every tune they play has a line in
# `crates/*/tests/digests/<slug>.txt`, and a run holds what it made against it.
# This recipe records instead of checking, so the difference lands in the
# working tree: a deliberate change is then a reviewed line in a diff, and an
# accidental one was a red test before anyone got here.
#
# **Run it with every game on the machine.** A table is only rewritten when the
# run produced lines for it, so a missing game leaves its table alone — but a
# game that is *present* and whose suite skipped part of itself writes a
# shorter table, and that shows up as deletions in the diff. Read them.
digests:
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf target/digests
    MOTIONVM_DIGESTS=record just test
    # One record file per table, named `<crate>.<slug>.lines`, so the table it
    # belongs to is derivable and no game's table is rewritten from another's
    # run. `sort -u` because the suite's threads and binaries appended to it in
    # whatever order they finished; a name that ends up on two lines with two
    # digests is nondeterminism, and the table reader says so rather than
    # taking one of them.
    for f in target/digests/*.lines; do
        base="$(basename "$f" .lines)"
        crate="${base%.*}"
        slug="${base##*.}"
        out="crates/$crate/tests/digests/$slug.txt"
        mkdir -p "$(dirname "$out")"
        {
            echo "# Reference digests for $slug, written by \`just digests\`."
            echo "#"
            echo "# One line per scene or tune: its name, and an FNV-1a 64 of what it"
            echo "# composed or wrote. A digest reconstructs nothing and is not a"
            echo "# fixture derived from the game's data, which is why it may live here"
            echo "# where a rendered frame may not. See motionvm-motion-testutil's"
            echo "# \`digest\` module for what each one covers."
            sort -u "$f"
        } > "$out"
    done
    git diff --stat -- 'crates/*/tests/digests'

# The measurement rigs: what a frame costs and how fast the machines run.
#
# Release only — a debug build measures the optimizer, not the code — and
# `#[ignore]`d, so `just check` never pays for them. They assert nothing: a
# wall-clock number is a property of the machine it ran on. What they are for
# is that a figure quoted about the cost of anything here can be reproduced and
# argued with, and that a change to the interpreter is measured before and
# after rather than reasoned about.
bench:
    {{ _GAMES }} cargo test --release -p motionvm-motion-engine \
        --test throughput --test ds2_timing -- --ignored --nocapture

fmt:
    cargo fmt --all

# What the dependency list is allowed to be: `deny.toml` says it, and CI runs
# exactly this.
#
# Not part of `check`, because it wants a tool `cargo` does not bring
# (`cargo install cargo-deny`) and it reaches the network for the advisory
# database. It belongs in the run before a dependency change goes out, and CI
# runs it on every push regardless.
#
# The two extra denials keep the file honest rather than merely passing: a
# license nobody uses and a duplicate that has gone away would otherwise sit
# in it forever.
deny:
    cargo deny check -D license-not-encountered -D unmatched-skip

fmt-check:
    cargo fmt --all --check

# Lint. correctness and suspicious are denied in [workspace.lints], and
# `-D warnings` matches CI, which denies them globally — without it an
# ordinary warning passes here and reds CI.
clippy:
    RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets

# Broken intra-doc links are errors here.
#
# `--document-private-items` because rustdoc only resolves links on items it
# documents, and by default that is the public surface alone. In this project
# the private comments carry as much of the evidence as the public ones — the
# ENGINE.EXE citations sit wherever the code does — so a gate that checked half
# of them would only read as if it checked them.
#
# Build the documentation, failing on a broken link.
#
# `-D warnings` on top of the broken-link denial, because the interesting case
# is not a link that resolves nowhere but one that resolves *here and not on
# docs.rs*: a public doc pointing at a private item only works because of
# `--document-private-items` below. That is a warning, not an error, so
# without the flag it passes locally and only CI's deny catches it.
doc:
    RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links -D warnings" \
        cargo doc --workspace --no-deps --document-private-items

# One of the games: `just run ds2`, `just run checker`, `just run enviro`,
# `just run jeffjet`, `just run hfa`, `just run vloomes`, `just run eddiem`.
#
# The slug is required and there is no default, which is the rule this file has
# always kept — a `just run` that picked a game would pick it for everyone.
# Seven recipes would be seven copies of one line, and an eighth game an
# eighth copy; a row in the case below is what a game costs instead.
run GAME *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ GAME }}" in
        ds2)     dir='{{ GAMEDATA_DS2 }}' ;;
        enviro)  dir='{{ GAMEDATA_ENVIRO }}' ;;
        jeffjet) dir='{{ GAMEDATA_JEFFJET }}' ;;
        hfa)     dir='{{ GAMEDATA_HFA }}' ;;
        vloomes) dir='{{ GAMEDATA_VLOOMES }}' ;;
        eddiem)  dir='{{ GAMEDATA_EDDIEM }}' ;;
        checker) dir='{{ GAMEDATA_CHECKER }}' ;;
        *) echo "just run <ds2|checker|enviro|jeffjet|hfa|vloomes|eddiem> [args]" >&2; exit 2 ;;
    esac
    cargo run --release -p motionvm-app -- "$dir" {{ ARGS }}

# The family's inspection CLI, from a checkout: `just tools info GAMEDIR`.
tools *ARGS:
    cargo run --release -p motionvm-motion-tools -- {{ARGS}}
