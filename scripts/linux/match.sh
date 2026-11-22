#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
#
# Runs a cutechess-cli match for Vyvræl: an SPRT between two builds, a
# gauntlet against opponents from the ladder, or a head-to-head between two
# builds. scripts/windows/match.ps1 does the same with the same options.
# See docs/TESTING.md.

set -euo pipefail

usage() {
    cat <<'EOF'
Usage:
  match.sh sprt     --test PATH --test-name NAME --base PATH --base-name NAME
                    --elo0 N --elo1 N --alpha N --beta N --rounds N [options]
  match.sh h2h      --test PATH --test-name NAME --base PATH --base-name NAME
                    --rounds N [options]
  match.sh gauntlet --test PATH --test-name NAME --opponent NAME
                    [--opponent NAME ...] --rounds N [options]

Modes:
  sprt      a sequential probability ratio test of the test build against
            the base build. The bounds, alpha and beta have no defaults.
  h2h       a fixed number of rounds between two builds.
  gauntlet  the test build against each opponent, named as in the engine
            list (--engines-json).

A round is two games: each opening is played twice with colours swapped.

Options:
  --rounds N           rounds to play; in sprt the cap after which the test
                       stops undecided (required)
  --tc TC              time control (default 10+0.1)
  --hash MB            hash size for every engine (default 64)
  --concurrency N      games at once (default: physical cores minus two)
  --openings FILE      opening file in EPD format (required, or OPENINGS)
  --engines-json FILE  engine list for gauntlet opponents (or ENGINES_JSON)
  --cutechess PATH     cutechess-cli to use (default: from PATH)
  --out-dir DIR        where the PGN, log and notes go (default: matches)
  --label TEXT         added to the output file names
  --seed N             seed for the opening order (default: random)
  --deterministic      set Seed=822286742 and Variety=0 on the Vyvræl builds
  --low-priority       run at low priority (nice)
  --no-low-priority    do not, whatever the local settings say
  --dry-run            print the cutechess-cli command and stop
  --help               show this text

Build names are ASCII only, for example "Vyvrael 0.1.0-a1" or
"Vyvrael dev-1a2b3c4". Settings can also come from the environment
(VYVRAEL_CONCURRENCY, ...) or scripts/local.env; see
scripts/local.env.example.

Beside the PGN a run leaves a .log and .notes.txt (the command, the git
commit and whether the repository had changes, each engine's file and
SHA-256, the times, the outcome, the number of games and the PGN's
SHA-256), and for a gauntlet a copy of the engine list (.engines.json).
EOF
}

die() {
    printf 'match.sh: %s\n' "$*" >&2
    exit 2
}

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
local_file=${VYVRAEL_LOCAL_ENV:-$script_dir/../local.env}

# The value of KEY: environment VYVRAEL_KEY first, then the local file.
# Prints nothing if neither has it.
setting() {
    local key=$1 var="VYVRAEL_$1" line
    if [[ -n ${!var-} ]]; then
        printf '%s' "${!var}"
        return
    fi
    [[ -f $local_file ]] || return 0
    while IFS= read -r line || [[ -n $line ]]; do
        line=${line%$'\r'}
        line=${line#$'\xef\xbb\xbf'} # a byte-order mark some editors put first
        if [[ $line == "$key="* ]]; then
            printf '%s' "${line#*=}"
            return
        fi
    done <"$local_file"
}

is_whole() { [[ $1 =~ ^[0-9]+$ ]]; }
is_number() { [[ $1 =~ ^-?[0-9]+([.][0-9]+)?$ ]]; }

# Physical cores, not hardware threads; 0 if it cannot be told.
physical_cores() {
    local count=0
    if command -v lscpu >/dev/null 2>&1; then
        # An lscpu that fails or prints nothing usable counts as "cannot tell".
        count=$(lscpu -p=core,socket 2>/dev/null | grep -v '^#' | sort -u | wc -l) || count=0
    fi
    echo "${count:-0}"
}

mode=${1-}
case $mode in
    sprt | h2h | gauntlet) shift ;;
    --help | -h | help)
        usage
        exit 0
        ;;
    '') usage >&2; exit 2 ;;
    *) die "unknown mode '$mode' (expected sprt, h2h or gauntlet); see --help" ;;
esac

test_path='' test_name='' base_path='' base_name=''
elo0='' elo1='' alpha='' beta='' rounds='' label='' seed=''
tc='10+0.1' hash=64 dry_run=false deterministic=false
concurrency=$(setting CONCURRENCY)
openings=$(setting OPENINGS)
engines_json=$(setting ENGINES_JSON)
cutechess=$(setting CUTECHESS_CLI)
out_dir=$(setting MATCH_DIR)
low_priority=$(setting LOW_PRIORITY)
opponents=()

need() { [[ $# -ge 2 ]] || die "$1 needs a value"; }
while [[ $# -gt 0 ]]; do
    case $1 in
        --test) need "$@"; test_path=$2; shift 2 ;;
        --test-name) need "$@"; test_name=$2; shift 2 ;;
        --base) need "$@"; base_path=$2; shift 2 ;;
        --base-name) need "$@"; base_name=$2; shift 2 ;;
        --opponent) need "$@"; opponents+=("$2"); shift 2 ;;
        --elo0) need "$@"; elo0=$2; shift 2 ;;
        --elo1) need "$@"; elo1=$2; shift 2 ;;
        --alpha) need "$@"; alpha=$2; shift 2 ;;
        --beta) need "$@"; beta=$2; shift 2 ;;
        --rounds) need "$@"; rounds=$2; shift 2 ;;
        --tc) need "$@"; tc=$2; shift 2 ;;
        --hash) need "$@"; hash=$2; shift 2 ;;
        --concurrency) need "$@"; concurrency=$2; shift 2 ;;
        --openings) need "$@"; openings=$2; shift 2 ;;
        --engines-json) need "$@"; engines_json=$2; shift 2 ;;
        --cutechess) need "$@"; cutechess=$2; shift 2 ;;
        --out-dir) need "$@"; out_dir=$2; shift 2 ;;
        --label) need "$@"; label=$2; shift 2 ;;
        --seed) need "$@"; seed=$2; shift 2 ;;
        --deterministic) deterministic=true; shift ;;
        --low-priority) low_priority=true; shift ;;
        --no-low-priority) low_priority=false; shift ;;
        --dry-run) dry_run=true; shift ;;
        --help | -h) usage; exit 0 ;;
        *) die "unknown option '$1'; see --help" ;;
    esac
done

# --- what every mode needs -------------------------------------------------

[[ -n $test_path ]] || die "--test is required: the build under test"
[[ -n $test_name ]] || die "--test-name is required"
[[ -n $rounds ]] || die "--rounds is required: a match must have an end"
# 10# reads "08" as 8 and not as a bad octal number; it is passed on as 8.
is_whole "$rounds" && ((10#$rounds > 0)) || die "--rounds must be a whole number above zero, got '$rounds'"
is_whole "$hash" && ((10#$hash > 0)) || die "--hash must be a whole number above zero, got '$hash'"
rounds=$((10#$rounds))
hash=$((10#$hash))
[[ $tc =~ ^[0-9./:+]+$ ]] || die "--tc '$tc' does not look like a time control (for example 10+0.1)"
[[ -z $seed ]] || is_whole "$seed" || die "--seed must be a whole number, got '$seed'"
[[ -z $label || $label =~ ^[A-Za-z0-9._-]+$ ]] || die "--label may only hold letters, digits, '.', '_' and '-'"
out_dir=${out_dir:-matches}
low_priority=${low_priority:-false}
[[ $low_priority == true || $low_priority == false ]] || die "LOW_PRIORITY must be true or false, got '$low_priority'"

check_name() {
    local what=$1 name=$2 other
    # What is left after deleting every byte from space to "~". tr in the C
    # locale compares bytes; a range in a bash pattern or regular expression
    # follows the user's locale instead, and on Linux that refused plain
    # names such as "Vyvrael 0.1.0-a1".
    other=$(printf '%s' "$name" | LC_ALL=C tr -d ' -~')
    [[ -n $name && -z $other ]] || die "$what '$name' must be plain ASCII (for example \"Vyvrael 0.1.0-a1\")"
    [[ $name != *'"'* && $name != *"'"* && $name != *'='* ]] || die "$what '$name' may not contain quotes or '='"
    [[ $name == "${name# }" && $name == "${name% }" ]] || die "$what '$name' may not start or end with a space"
}
check_engine_file() {
    local what=$1 path=$2
    [[ -f $path ]] || die "$what not found: $path"
    [[ -x $path ]] || die "$what is not executable: $path"
}
check_name "--test-name" "$test_name"
check_engine_file "the test build (--test)" "$test_path"

# --- what each mode needs --------------------------------------------------

case $mode in
    sprt | h2h)
        [[ -n $base_path ]] || die "--base is required in $mode mode: the build to compare with"
        [[ -n $base_name ]] || die "--base-name is required in $mode mode"
        check_name "--base-name" "$base_name"
        check_engine_file "the base build (--base)" "$base_path"
        [[ $test_name != "$base_name" ]] || die "the two builds have the same name '$test_name'; the results could not be told apart"
        ((${#opponents[@]} == 0)) || die "--opponent is for gauntlet mode"
        ;;
    gauntlet)
        ((${#opponents[@]} > 0)) || die "gauntlet mode needs at least one --opponent"
        [[ -z $base_path && -z $base_name ]] || die "--base and --base-name are for sprt and h2h modes"
        [[ -n $engines_json ]] || die "gauntlet mode needs the engine list: --engines-json, or ENGINES_JSON in scripts/local.env"
        [[ -f $engines_json ]] || die "engine list not found: $engines_json"
        ;;
esac

if [[ $mode == sprt ]]; then
    for pair in "elo0:$elo0" "elo1:$elo1" "alpha:$alpha" "beta:$beta"; do
        [[ -n ${pair#*:} ]] || die "--${pair%%:*} is required in sprt mode; there are no default bounds (docs/TESTING.md has examples)"
        is_number "${pair#*:}" || die "--${pair%%:*} must be a number, got '${pair#*:}'"
    done
else
    [[ -z $elo0$elo1$alpha$beta ]] || die "--elo0, --elo1, --alpha and --beta are for sprt mode"
fi

# --- opponents from the engine list ----------------------------------------

opponent_args=()
opponent_files=()
if [[ $mode == gauntlet ]]; then
    seen=$'\n'
    for name in "${opponents[@]}"; do
        [[ $name != "$test_name" ]] || die "the build under test has the same name as the opponent '$name'"
        [[ $seen != *$'\n'"$name"$'\n'* ]] || die "opponent '$name' is listed twice"
        seen+="$name"$'\n'
    done
    python=${VYVRAEL_PYTHON:-python3}
    command -v "$python" >/dev/null 2>&1 || die "gauntlet mode needs $python to read the engine list"
    # One line per opponent: the -engine arguments, tab-separated.
    lookup='
import json, os, sys
path, names = sys.argv[1], sys.argv[2:]
try:
    with open(path, encoding="utf-8") as handle:
        engines = json.load(handle)
except (OSError, ValueError) as error:
    sys.exit(f"cannot read the engine list {path}: {error}")
by_name = {entry.get("name"): entry for entry in engines if isinstance(entry, dict)}
for name in names:
    entry = by_name.get(name)
    if entry is None:
        known = ", ".join(sorted(n for n in by_name if n)) or "none"
        sys.exit(f"no engine named \"{name}\" in {path} (known: {known})")
    command = entry.get("command", "")
    directory = entry.get("workingDirectory", "")
    if directory and not os.path.isabs(command) and os.sep not in command and "/" not in command:
        command = os.path.join(directory, command)
    parts = ["name=" + name, "cmd=" + command, "proto=" + entry.get("protocol", "uci")]
    if directory:
        parts.append("dir=" + directory)
    for option in entry.get("options", []):
        value = option.get("value")
        if value is None or option.get("name") in (None, "Hash"):
            continue
        if isinstance(value, bool):
            value = "true" if value else "false"
        parts.append("option.%s=%s" % (option["name"], value))
    if any("\t" in part or "\n" in part for part in parts):
        sys.exit(f"engine \"{name}\" in {path} has a tab or line break in a field")
    print("\t".join(parts))
'
    listing=$("$python" -c "$lookup" "$engines_json" "${opponents[@]}") || die "could not look up the opponents (see the message above)"
    while IFS= read -r line; do
        line=${line%$'\r'}
        IFS=$'\t' read -r -a parts <<<"$line"
        command=''
        for part in "${parts[@]}"; do
            [[ $part == cmd=* ]] && command=${part#cmd=}
        done
        [[ -f $command ]] || die "opponent engine not found: $command (from $engines_json)"
        opponent_args+=(-engine "${parts[@]}")
        opponent_files+=("${parts[0]#name=}"$'\t'"$command")
    done <<<"$listing"
fi

# --- the tools and the files -----------------------------------------------

[[ -n $openings ]] || die "no opening file: give --openings, or set OPENINGS in scripts/local.env"
[[ -f $openings ]] || die "opening file not found: $openings"

# PHYSICAL_CORES gives the count where it cannot be detected.
cores=$(setting PHYSICAL_CORES)
cores=${cores:-$(physical_cores)}
is_whole "$cores" || die "PHYSICAL_CORES must be a whole number, got '$cores'"
cores=$((10#$cores))
if [[ -z $concurrency ]]; then
    concurrency=$((cores > 3 ? cores - 2 : 1))
fi
is_whole "$concurrency" && ((10#$concurrency > 0)) || die "--concurrency must be a whole number above zero, got '$concurrency'"
concurrency=$((10#$concurrency))
if ((cores > 0 && concurrency > cores)); then
    die "--concurrency $concurrency is more than the $cores physical cores of this machine; games would steal time from each other"
fi

cutechess=${cutechess:-cutechess-cli}
needed_flags=(-engine -each -openings -games -rounds -repeat -recover -concurrency
    -pgnout -ratinginterval -resign -draw)
[[ $mode == sprt ]] && needed_flags+=(-sprt)
[[ $mode == gauntlet ]] && needed_flags+=(-tournament -seeds)
[[ -n $seed ]] && needed_flags+=(-srand)
if command -v "$cutechess" >/dev/null 2>&1; then
    help_text=$("$cutechess" -help 2>&1) || true
    for flag in "${needed_flags[@]}"; do
        grep -q -- "^ *$flag\b" <<<"$help_text" || die "this cutechess-cli ($cutechess) does not list the option $flag in its -help; the script needs it"
    done
elif [[ $dry_run == true ]]; then
    printf 'match.sh: warning: cutechess-cli not found (%s); its options were not checked\n' "$cutechess" >&2
else
    die "cutechess-cli not found ($cutechess). Install it (docs/TESTING.md), or give --cutechess or CUTECHESS_CLI in scripts/local.env"
fi

# --- the command -----------------------------------------------------------

slug=$(printf '%s' "$test_name" | tr -c 'A-Za-z0-9._-' '-')
stem="$out_dir/${VYVRAEL_MATCH_DATE:-$(date +%Y-%m-%d)}-$mode-$slug${label:+-$label}"
pgn="$stem.pgn"

build_options=(proto=uci option.Threads=1)
# A fixed seed (the project's default, 0x31031996) and no variety.
[[ $deterministic == true ]] && build_options+=(option.Seed=822286742 option.Variety=0)

cmd=("$cutechess")
[[ $mode == gauntlet ]] && cmd+=(-tournament gauntlet -seeds 1)
cmd+=(-engine "name=$test_name" "cmd=$test_path" "${build_options[@]}")
if [[ $mode == gauntlet ]]; then
    cmd+=("${opponent_args[@]}")
else
    cmd+=(-engine "name=$base_name" "cmd=$base_path" "${build_options[@]}")
fi
cmd+=(-each "tc=$tc" "option.Hash=$hash"
    -openings "file=$openings" format=epd order=random
    -games 2 -repeat -rounds "$rounds"
    -resign movecount=3 score=700 twosided=true
    -draw movenumber=34 movecount=10 score=20
    -recover -concurrency "$concurrency" -ratinginterval 10)
[[ $mode == sprt ]] && cmd+=(-sprt "elo0=$elo0" "elo1=$elo1" "alpha=$alpha" "beta=$beta")
[[ -n $seed ]] && cmd+=(-srand "$seed")
cmd+=(-pgnout "$pgn")
[[ $low_priority == true ]] && cmd=(nice -n 10 "${cmd[@]}")

if [[ $dry_run == true ]]; then
    printf '%q ' "${cmd[@]}"
    printf '\n'
    exit 0
fi

mkdir -p "$out_dir" || die "cannot create the output folder $out_dir"
[[ ! -e $pgn ]] || die "$pgn already exists; games would be mixed. Move it away or give another --label"

# --- run notes, the log and the result --------------------------------------

log="$stem.log"
notes="$stem.notes.txt"

digest() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    else
        echo "unknown (no sha256sum)"
    fi
}
now() { date -u +%Y-%m-%dT%H:%M:%SZ; }
engine_note() { printf '  %s\n    %s\n    %s\n' "$1" "$2" "$(digest "$2")"; }

# The commit of the repository this script belongs to, and whether its
# working tree has changes: read-only git, nothing written.
repo_dir=$(cd "$script_dir/../.." && pwd)
git_program=${VYVRAEL_GIT:-git}
repo_state() {
    local commit changes
    if commit=$("$git_program" -C "$repo_dir" rev-parse HEAD 2>/dev/null) &&
        changes=$("$git_program" -C "$repo_dir" status --porcelain 2>/dev/null); then
        if [[ -n $changes ]]; then echo "$commit (dirty)"; else echo "$commit (clean)"; fi
    else
        echo "unknown (no git, or not a git repository)"
    fi
}

# The engine list a gauntlet used, copied beside the notes, so the
# opponents' options stay on record even if the list changes later.
engines_copy=""
if [[ $mode == gauntlet ]]; then
    engines_copy="$stem.engines.json"
    cp -- "$engines_json" "$engines_copy" || die "cannot copy the engine list to $engines_copy"
fi

{
    echo "Vyvrael match run notes"
    echo "mode:       $mode"
    echo "started:    $(now)"
    echo "repository: $(repo_state)"
    if [[ -n $engines_copy ]]; then
        echo "engines:    $engines_json (copied to $engines_copy)"
    else
        echo "engines:    no engine list used"
    fi
    echo "host cores: $cores physical, $concurrency games at once"
    echo "cutechess:  $("$cutechess" -version 2>&1 | head -n 1)"
    echo "openings:   $openings"
    echo "pgn:        $pgn"
    echo
    echo "engines (name, file, SHA-256):"
    engine_note "$test_name" "$test_path"
    if [[ $mode == gauntlet ]]; then
        for entry in "${opponent_files[@]}"; do
            engine_note "${entry%%$'\t'*}" "${entry#*$'\t'}"
        done
    else
        engine_note "$base_name" "$base_path"
    fi
    echo
    echo "command:"
    printf '  '
    printf '%q ' "${cmd[@]}"
    printf '\n'
} >"$notes"

echo "match.sh: log $log, notes $notes"
status=0
"${cmd[@]}" 2>&1 | tee "$log" || status=${PIPESTATUS[0]}

outcome="finished"
if [[ $mode == sprt ]]; then
    last=$(grep '^SPRT:' "$log" | tail -n 1 || true)
    llr=$(sed -n 's/^SPRT: llr \([^ ]*\).*/\1/p' <<<"$last")
    case $last in
        *'H1 was accepted'*) outcome="H1 accepted: the test build gains at least elo0 (LLR ${llr:-?})" ;;
        *'H0 was accepted'*) outcome="H0 accepted: no gain shown; the change is rejected (LLR ${llr:-?})" ;;
        '') outcome="no SPRT line in the log; nothing can be concluded" ;;
        *) outcome="undecided: the round cap was reached first (LLR ${llr:-?})" ;;
    esac
    echo
    echo "SPRT final: ${last:-none}"
    echo "SPRT outcome: $outcome"
fi
((status == 0)) || outcome="cutechess-cli failed with status $status; $outcome"
if [[ -f $pgn ]]; then
    games=$(grep -c '^\[Event ' "$pgn" || true)
    pgn_digest=$(digest "$pgn")
else
    games="0 (no PGN was written)"
    pgn_digest="none"
fi
{
    echo
    echo "ended:      $(now)"
    echo "status:     $status"
    echo "outcome:    $outcome"
    echo "games:      $games"
    echo "pgn sha256: $pgn_digest"
} >>"$notes"
exit "$status"
