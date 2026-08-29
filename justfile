# What "green" means, in one command.
#
# CI runs the same four commands on three platforms, but without the game data,
# so it is the floor; `just check` with your own copy of a game is the gate —
# what has to pass before a change is finished.
#
# The game data is not in the repository and cannot be. Point GAMEDATA_DS2 at
# your copy of Dunkle Schatten 2, GAMEDATA_ENVIRO at your copy of Die
# Enviro-Kids greifen ein, GAMEDATA_JEFFJET at your copy of Jeff Jet -
# Abenteuer InfoHighway and GAMEDATA_HFA at your copy of Hilfe für Amajambere
# — the defaults are directories next to this one, which
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

# What actually reaches the suite.
#
# A path you named is always passed through, so a typo panics and says so. The
# built-in default is passed only when the data is really there — otherwise a
# clone on a machine that has no copy of a game would panic on the very
# command this file exists to define, instead of skipping the way the README
# describes. The three 16-bit games are told apart by their engine binary: they
# all ship a DATA.-1-, so probing for that would let any of those defaults match
# another game's directory.
_DATA_DS2 := if GAMEDATA_DS2 != DEFAULT_GAMEDATA_DS2 { GAMEDATA_DS2 } \
    else if path_exists(GAMEDATA_DS2 / "001.RSC") == "true" { GAMEDATA_DS2 } \
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

# Savegames cannot be reconstructed, only played to, so there is no default that
# could work. Set it to a directory holding one — `just test SAVES=…` — and the
# savegame test runs; without it that one test skips itself and says so.
SAVES := ""

_default:
    @just --list

# Everything. Run this before calling a change done.
check: fmt-check clippy test doc

# The test suite, with the games' files.
test:
    MOTIONVM_GAMEDATA_DS2="{{ _DATA_DS2 }}" MOTIONVM_GAMEDATA_ENVIRO="{{ _DATA_ENVIRO }}" \
        MOTIONVM_GAMEDATA_JEFFJET="{{ _DATA_JEFFJET }}" MOTIONVM_GAMEDATA_HFA="{{ _DATA_HFA }}" \
        MOTIONVM_SAVES="{{ SAVES }}" RUSTFLAGS="-D warnings" cargo test --workspace

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
    MOTIONVM_GAMEDATA_DS2="{{ _DATA_DS2 }}" MOTIONVM_GAMEDATA_ENVIRO="{{ _DATA_ENVIRO }}" \
        MOTIONVM_GAMEDATA_JEFFJET="{{ _DATA_JEFFJET }}" MOTIONVM_GAMEDATA_HFA="{{ _DATA_HFA }}" \
        MOTIONVM_SAVES="{{ SAVES }}" cargo test --workspace --test {{ target }} -- --nocapture

fmt:
    cargo fmt --all

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

# The games themselves, one recipe each and none of them the default.
run-ds2 *ARGS:
    cargo run --release -p motionvm-app -- "{{ GAMEDATA_DS2 }}" {{ ARGS }}

run-enviro *ARGS:
    cargo run --release -p motionvm-app -- "{{ GAMEDATA_ENVIRO }}" {{ ARGS }}

run-jeffjet *ARGS:
    cargo run --release -p motionvm-app -- "{{ GAMEDATA_JEFFJET }}" {{ ARGS }}

run-hfa *ARGS:
    cargo run --release -p motionvm-app -- "{{ GAMEDATA_HFA }}" {{ ARGS }}
