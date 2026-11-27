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
# at your copy of Falsches Spiel mit Eddie M., GAMEDATA_CHECKER at your copy
# of Checker 2000, GAMEDATA_DS1 at your copy of Dunkle Schatten,
# GAMEDATA_CAPZINS at your copy of Captain Zins and GAMEDATA_BIFI2 at your
# copy of Bi-Fi 2 - Action in Hollywood — the
# defaults are directories next to this one, which is where a checkout beside
# installed copies of the games finds them. Tests
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
DEFAULT_GAMEDATA_DS1 := justfile_directory() / ".." / "games" / "DS1"
GAMEDATA_DS1 := DEFAULT_GAMEDATA_DS1
DEFAULT_GAMEDATA_CAPZINS := justfile_directory() / ".." / "games" / "CAPZINS"
GAMEDATA_CAPZINS := DEFAULT_GAMEDATA_CAPZINS
DEFAULT_GAMEDATA_BIFI2 := justfile_directory() / ".." / "games" / "BIFI2"
GAMEDATA_BIFI2 := DEFAULT_GAMEDATA_BIFI2

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
# of Dunkle Schatten 2, the ENGINE.RSC of Checker 2000. The two PARSEC games
# are probed for their room containers, RESOURCE/DSROOM.DAT and
# RESOURCE/CZROOM.DAT, which no other game ships.
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
_DATA_DS1 := if GAMEDATA_DS1 != DEFAULT_GAMEDATA_DS1 { GAMEDATA_DS1 } \
    else if path_exists(GAMEDATA_DS1 / "RESOURCE" / "DSROOM.DAT") == "true" { GAMEDATA_DS1 } \
    else { "" }
_DATA_CAPZINS := if GAMEDATA_CAPZINS != DEFAULT_GAMEDATA_CAPZINS { GAMEDATA_CAPZINS } \
    else if path_exists(GAMEDATA_CAPZINS / "RESOURCE" / "CZROOM.DAT") == "true" { GAMEDATA_CAPZINS } \
    else { "" }
_DATA_BIFI2 := if GAMEDATA_BIFI2 != DEFAULT_GAMEDATA_BIFI2 { GAMEDATA_BIFI2 } \
    else if path_exists(GAMEDATA_BIFI2 / "RESOURCE" / "BFROOM.DAT") == "true" { GAMEDATA_BIFI2 } \
    else { "" }

# Savegames cannot be reconstructed, only played to, and never enter the
# repository. They are kept the way the games are: a directory beside this one,
# `../saves/`, holding each game's own save directory — `ds1/`, `ds2/`, … — as
# the program writes them, because the engine is pointed at the parent and puts
# the game's name on itself. The default is passed only when that directory is
# there; `just SAVES=… test` names another. Without either, the savegame tests
# skip themselves and say so, and so does a test whose game has no save
# directory below it.
DEFAULT_SAVES := justfile_directory() / ".." / "saves"
SAVES := DEFAULT_SAVES
_SAVES := if SAVES != DEFAULT_SAVES { SAVES } \
    else if path_exists(DEFAULT_SAVES) == "true" { DEFAULT_SAVES } \
    else { "" }

# Where every game reaches a cargo command, written once.
#
# Several recipes hand the suite its data. Spelling the variables out in each
# would make a further game a matter of remembering every one of them; this is
# the one block they share, and adding a game touches it once.
_GAMES := 'MOTIONVM_GAMEDATA_DS2="' + _DATA_DS2 + '" ' + \
    'MOTIONVM_GAMEDATA_ENVIRO="' + _DATA_ENVIRO + '" ' + \
    'MOTIONVM_GAMEDATA_JEFFJET="' + _DATA_JEFFJET + '" ' + \
    'MOTIONVM_GAMEDATA_HFA="' + _DATA_HFA + '" ' + \
    'MOTIONVM_GAMEDATA_VLOOMES="' + _DATA_VLOOMES + '" ' + \
    'MOTIONVM_GAMEDATA_EDDIEM="' + _DATA_EDDIEM + '" ' + \
    'MOTIONVM_GAMEDATA_CHECKER="' + _DATA_CHECKER + '" ' + \
    'MOTIONVM_GAMEDATA_DS1="' + _DATA_DS1 + '" ' + \
    'MOTIONVM_GAMEDATA_CAPZINS="' + _DATA_CAPZINS + '" ' + \
    'MOTIONVM_GAMEDATA_BIFI2="' + _DATA_BIFI2 + '" ' + \
    'MOTIONVM_SAVES="' + _SAVES + '"'

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
# starts needing a file, or a skip that stops being a skip, is invisible here
# and only surfaces after a push. This is the floor, not the gate: `just
# check` is still what has to pass before a change is finished.
#
# CI's floor, on your own machine: the suite with no game data at all.
check-nodata:
    MOTIONVM_NO_GAMEDATA=1 RUSTFLAGS="-D warnings" cargo test --workspace

# Captain Zins and Bi-Fi 2 played from their first scene to their end
# through the host's input. They run for minutes of game time, so `just
# check` skips them.
#
# Captain Zins and Bi-Fi 2 played to their end, in release.
playthrough:
    {{ _GAMES }} cargo test --release -p motionvm-parsec-engine captain_zins_can_be_played_to_its_end -- --ignored --nocapture
    {{ _GAMES }} cargo test --release -p motionvm-parsec-engine bi_fi_2_can_be_played_to_its_end -- --ignored --nocapture

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
        # The family whose test-support crate defines the table's lines.
        family="${crate#motionvm-}"
        family="${family%%-*}"
        out="crates/$crate/tests/digests/$slug.txt"
        mkdir -p "$(dirname "$out")"
        {
            echo "# Reference digests for $slug, written by \`just digests\`."
            echo "#"
            echo "# One line per scene or tune: its name, and an FNV-1a 64 of what it"
            echo "# composed or wrote. A digest reconstructs nothing and is not a"
            echo "# fixture derived from the game's data, which is why it may live here"
            echo "# where a rendered frame may not. See motionvm-$family-testutil's"
            echo "# \`digest\` module for what each one covers."
            sort -u "$f"
        } > "$out.new"
        # An anchored line keeps its anchor, and keeps its value unless
        # REANCHOR=1 says to take the new one (see the `digest` module).
        if [ -f "$out" ]; then
            awk -v reanchor="${REANCHOR:-}" -v table="$out" '
                FNR == NR {
                    if ($0 !~ /^#/ && $3 == "anchored:") {
                        held[$1] = $2
                        anchor[$1] = substr($0, index($0, "anchored:"))
                    }
                    next
                }
                /^#/ || NF < 2 { print; next }
                ($1 in anchor) {
                    if ($2 != held[$1] && reanchor != "1") {
                        printf "%s: %s is anchored (%s) and digested as %s; the anchored value stays. REANCHOR=1 takes the new one, and the comparison with the original must then pass again.\n", table, $1, anchor[$1], $2 > "/dev/stderr"
                        print $1 "  " held[$1] "  " anchor[$1]
                        moved = 1
                        next
                    }
                    print $1 "  " $2 "  " anchor[$1]
                    next
                }
                { print }
                END { exit moved }
            ' "$out" "$out.new" > "$out.merged" || status=1
            mv "$out.merged" "$out"
            rm "$out.new"
        else
            mv "$out.new" "$out"
        fi
    done
    git diff --stat -- 'crates/*/tests/digests'
    exit "${status:-0}"

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
# `just run jeffjet`, `just run hfa`, `just run vloomes`, `just run eddiem`,
# `just run ds1`, `just run capzins`.
#
# The slug is required and there is no default — a `just run` that picked a
# game would pick it for everyone. A recipe per game would be a copy of one
# line per game; a row in the case below is what a game costs instead.
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
        ds1)     dir='{{ GAMEDATA_DS1 }}' ;;
        capzins) dir='{{ GAMEDATA_CAPZINS }}' ;;
        bifi2)   dir='{{ GAMEDATA_BIFI2 }}' ;;
        *) echo "just run <ds2|checker|enviro|jeffjet|hfa|vloomes|eddiem|ds1|capzins|bifi2> [args]" >&2; exit 2 ;;
    esac
    cargo run --release -p motionvm-app -- "$dir" {{ ARGS }}

# The MOTION family's inspection CLI, from a checkout: `just tools info GAMEDIR`.
tools *ARGS:
    cargo run --release -p motionvm-motion-tools -- {{ARGS}}

# The PARSEC family's, the same way: `just tools-parsec info GAMEDIR`.
tools-parsec *ARGS:
    cargo run --release -p motionvm-parsec-tools -- {{ARGS}}
