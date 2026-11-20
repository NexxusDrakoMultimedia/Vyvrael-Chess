#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
#
# Computes a rating list with Ordo from one or more PGN files, relative to
# an anchor engine. scripts/windows/rate.ps1 does the same with the same
# options. See docs/TESTING.md.

set -euo pipefail

usage() {
    cat <<'EOF'
Usage:
  rate.sh [options] FILE.pgn [FILE.pgn ...]

Runs Ordo over the games and writes a text table and a CSV file next to the
first PGN file. The defaults are those of Ladder Baseline 1.

Options:
  --anchor NAME        the engine whose rating is fixed (default "Stockfish 19")
  --anchor-rating N    the rating it is fixed at (default 0)
  --simulations N      simulations for the error bars (default 500; 0 for none)
  --out STEM           output files STEM.txt and STEM.csv
                       (default: the first PGN's name with -ratings added)
  --ordo PATH          ordo to use (default: from PATH, or ORDO in
                       scripts/local.env)
  --dry-run            print the ordo command and stop
  --help               show this text

The anchor must have played in the games. Ratings are relative to it: they
are not CCRL or FIDE ratings.
EOF
}

die() {
    printf 'rate.sh: %s\n' "$*" >&2
    exit 2
}

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
local_file=${VYVRAEL_LOCAL_ENV:-$script_dir/../local.env}

# The value of KEY: environment VYVRAEL_KEY first, then the local file.
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

anchor='Stockfish 19' anchor_rating=0 simulations=500 out='' dry_run=false
ordo=$(setting ORDO)
pgns=()

need() { [[ $# -ge 2 ]] || die "$1 needs a value"; }
while [[ $# -gt 0 ]]; do
    case $1 in
        --anchor) need "$@"; anchor=$2; shift 2 ;;
        --anchor-rating) need "$@"; anchor_rating=$2; shift 2 ;;
        --simulations) need "$@"; simulations=$2; shift 2 ;;
        --out) need "$@"; out=$2; shift 2 ;;
        --ordo) need "$@"; ordo=$2; shift 2 ;;
        --dry-run) dry_run=true; shift ;;
        --help | -h) usage; exit 0 ;;
        --) shift; pgns+=("$@"); break ;;
        -*) die "unknown option '$1'; see --help" ;;
        *) pgns+=("$1"); shift ;;
    esac
done

((${#pgns[@]} > 0)) || { usage >&2; exit 2; }
[[ -n $anchor ]] || die "--anchor may not be empty"
[[ $anchor != *'"'* ]] || die "--anchor may not contain a double quote"
[[ $anchor_rating =~ ^-?[0-9]+([.][0-9]+)?$ ]] || die "--anchor-rating must be a number, got '$anchor_rating'"
[[ $simulations =~ ^[0-9]+$ ]] || die "--simulations must be a whole number, got '$simulations'"

anchor_seen=false
for pgn in "${pgns[@]}"; do
    [[ -f $pgn ]] || die "PGN file not found: $pgn"
    [[ $pgn != *$'\n'* ]] || die "a PGN file name may not contain a line break"
    if grep -q -F -e "[White \"$anchor\"]" -e "[Black \"$anchor\"]" "$pgn"; then
        anchor_seen=true
    fi
done
if [[ $anchor_seen == false ]]; then
    players=$(cat "${pgns[@]}" | sed -n 's/^\[\(White\|Black\) "\(.*\)"\]\r\{0,1\}$/\2/p' | sort -u | paste -sd ',' - | sed 's/,/, /g')
    die "the anchor '$anchor' played in none of the games; give --anchor one of: ${players:-no players found}"
fi

out=${out:-${pgns[0]%.pgn}-ratings}
text="$out.txt"
csv="$out.csv"

ordo=${ordo:-ordo}
needed_flags=(-a -A -W -p -o -c)
((simulations > 0)) && needed_flags+=(-s)
many_by_list=true
if command -v "$ordo" >/dev/null 2>&1; then
    help_text=$("$ordo" -h 2>&1) || true
    for flag in "${needed_flags[@]}"; do
        grep -q -- "^ *$flag\b" <<<"$help_text" || die "this ordo ($ordo) does not list the option $flag in its help; the script needs it"
    done
    grep -q -- '^ *-P\b' <<<"$help_text" || many_by_list=false
elif [[ $dry_run == true ]]; then
    printf 'rate.sh: warning: ordo not found (%s); its options were not checked\n' "$ordo" >&2
else
    die "ordo not found ($ordo). Install it (docs/TESTING.md), or give --ordo or ORDO in scripts/local.env"
fi

# Several files: Ordo's own list file where it has one, else one joined file.
input=(-p "${pgns[0]}")
helper=''
if ((${#pgns[@]} > 1)); then
    if [[ $many_by_list == true ]]; then
        helper="$out.pgn-list.txt"
        input=(-P "$helper")
    else
        helper="$out.joined.pgn"
        input=(-p "$helper")
    fi
fi

cmd=("$ordo" -a "$anchor_rating" -A "$anchor" -W)
((simulations > 0)) && cmd+=(-s "$simulations")
cmd+=("${input[@]}" -o "$text" -c "$csv")

if [[ $dry_run == true ]]; then
    printf '%q ' "${cmd[@]}"
    printf '\n'
    exit 0
fi

mkdir -p "$(dirname "$out")" || die "cannot create the folder for $out"
if [[ -n $helper ]]; then
    if [[ $many_by_list == true ]]; then
        printf '%s\n' "${pgns[@]}" >"$helper"
    else
        : >"$helper"
        for pgn in "${pgns[@]}"; do
            cat "$pgn" >>"$helper"
            printf '\n' >>"$helper"
        done
    fi
fi

# Tables left by an earlier run must not pass for this run's.
rm -f "$text" "$csv"
status=0
"${cmd[@]}" || status=$?
[[ -z $helper ]] || rm -f "$helper"
((status == 0)) || die "ordo failed with status $status"
[[ -s $text ]] || die "ordo wrote no table to $text"
[[ -f $csv ]] || die "ordo wrote no CSV file to $csv"

cat "$text"
echo
echo "================ ORDER (strongest first) ================"
# The table's rows start with a rank; the name runs up to the colon.
sed -n 's/^ *\([0-9][0-9]*\) \(.*[^ ]\) *: .*/\1. \2/p' "$text"
echo "=========================================================="
echo "Read the order, not the distances. Lopsided results stretch the Elo"
echo "scale, and with few games the error bars are wide, so close neighbours"
echo "are not really separated. Ratings are relative to $anchor = $anchor_rating."
echo "Written: $text and $csv"
