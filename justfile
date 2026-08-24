# What "green" means, in one command.
#
# CI runs the same four commands on three platforms, but without the game data,
# so it is the floor; `just check` with your own copy of a game is the gate —
# what has to pass before a change is finished.
#
# The game data is not in the repository and cannot be. Point GAMEDATA_DS2 at
# your copy of Dunkle Schatten 2 and GAMEDATA_ENVIRO at your copy of Die
# Enviro-Kids greifen ein — the defaults are directories next to this one,
# which is where a checkout beside installed copies of the games finds them.
# Tests that need data and cannot find it skip themselves; a *wrong* path
# panics rather than skipping, so a typo cannot read as "no data on this
# machine".
DEFAULT_GAMEDATA_DS2 := justfile_directory() / ".." / "games" / "DS2"
GAMEDATA_DS2 := DEFAULT_GAMEDATA_DS2
DEFAULT_GAMEDATA_ENVIRO := justfile_directory() / ".." / "games" / "ENVIRO"
GAMEDATA_ENVIRO := DEFAULT_GAMEDATA_ENVIRO

# What actually reaches the suite.
#
# A path you named is always passed through, so a typo panics and says so. The
# built-in default is passed only when the data is really there — otherwise a
# clone on a machine that has no copy of a game would panic on the very
# command this file exists to define, instead of skipping the way the README
# describes.
_DATA_DS2 := if GAMEDATA_DS2 != DEFAULT_GAMEDATA_DS2 { GAMEDATA_DS2 } \
    else if path_exists(GAMEDATA_DS2 / "001.RSC") == "true" { GAMEDATA_DS2 } \
    else { "" }
_DATA_ENVIRO := if GAMEDATA_ENVIRO != DEFAULT_GAMEDATA_ENVIRO { GAMEDATA_ENVIRO } \
    else if path_exists(GAMEDATA_ENVIRO / "DATA.-1-") == "true" { GAMEDATA_ENVIRO } \
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
        MOTIONVM_SAVES="{{ SAVES }}" RUSTFLAGS="-D warnings" cargo test --workspace

# One test target, e.g. `just test-one scenes` or `just test-one fm_driver`.
test-one target:
    MOTIONVM_GAMEDATA_DS2="{{ _DATA_DS2 }}" MOTIONVM_GAMEDATA_ENVIRO="{{ _DATA_ENVIRO }}" \
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

# The games themselves. `run` is Dunkle Schatten 2, `run-enviro` the
# 16-bit Die Enviro-Kids greifen ein.
run *ARGS:
    cargo run --release -p motionvm-app -- "{{ GAMEDATA_DS2 }}" {{ ARGS }}

run-enviro *ARGS:
    cargo run --release -p motionvm-app -- "{{ GAMEDATA_ENVIRO }}" {{ ARGS }}
