# What "green" means, in one command.
#
# There is no CI. `just check` is the substitute: it is what has to pass before
# a change is finished.
#
# The game data is not in the repository and cannot be. Point GAMEDATA at your
# own copy — the default is a directory next to this one, which is where a
# checkout beside an installed copy of the game finds it. Tests that need data
# and cannot find it skip themselves; a *wrong* GAMEDATA panics rather than
# skipping, so a mistyped path cannot read as "no data on this machine".
DEFAULT_GAMEDATA := justfile_directory() / ".." / "gamedata"
GAMEDATA := DEFAULT_GAMEDATA

# What actually reaches the suite.
#
# A path you named is always passed through, so a typo panics and says so. The
# built-in default is passed only when the data is really there — otherwise a
# clone on a machine that has no copy of the game would panic on the very
# command this file exists to define, instead of skipping the way the README
# describes.
_DATA := if GAMEDATA != DEFAULT_GAMEDATA { GAMEDATA } \
    else if path_exists(GAMEDATA / "001.RSC") == "true" { GAMEDATA } \
    else { "" }

# Savegames cannot be reconstructed, only played to, so there is no default that
# could work. Set it to a directory holding one — `just test SAVES=…` — and the
# savegame test runs; without it that one test skips itself and says so.
SAVES := ""

_default:
    @just --list

# Everything. Run this before calling a change done.
check: fmt-check clippy test doc

# The test suite, with the game's files.
test:
    MOTIONVM_GAMEDATA="{{ _DATA }}" MOTIONVM_SAVES="{{ SAVES }}" \
        cargo test --workspace

# One test target, e.g. `just test-one scenes` or `just test-one fm_driver`.
test-one target:
    MOTIONVM_GAMEDATA="{{ _DATA }}" MOTIONVM_SAVES="{{ SAVES }}" \
        cargo test --workspace --test {{ target }} -- --nocapture

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

# Lint. correctness and suspicious are denied in [workspace.lints], so this
# fails on them rather than warning.
#
# Lint everything, denying the correctness and suspicious groups.
clippy:
    cargo clippy --workspace --all-targets

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
# `--document-private-items` below. That is a warning, not an error, and it
# went unnoticed until CI denied it.
doc:
    RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links -D warnings" \
        cargo doc --workspace --no-deps --document-private-items

# The game itself.
run *ARGS:
    cargo run --release -p motionvm-app -- "{{ GAMEDATA }}" {{ ARGS }}
