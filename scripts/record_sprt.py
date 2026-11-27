# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Reads the record of an SPRT run made by match.sh or match.ps1.

A run leaves `<name>.notes.txt` (what was run: the builds and their
SHA-256, the command with its bounds, the outcome, the PGN's SHA-256) and
`<name>.log` (what cutechess-cli printed: the score, the Elo estimate and
the SPRT line). `read_run` reads both and checks them against each other,
so that a result is always written down from the files and never retyped.

Anything that is not a finished SPRT with a readable result is refused
with a `RecordError`: a gauntlet, an interrupted or failed run, a log
without an SPRT line, notes and a log that disagree. An undecided test is
a finished one and is read as "undecided", which is not an acceptance.

Standard library only. See "Recording an SPRT result" in docs/TESTING.md.
"""
from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

OUTCOMES = ("H1 accepted", "H0 accepted", "undecided")

SCORE = re.compile(r"^Score of .*: (\d+) - (\d+) - (\d+)\s+\[[\d.]+\]\s+(\d+)\s*$")
ELO = re.compile(r"^Elo difference: (\S+) \+/- ([^,\s]+)")
SPRT = re.compile(
    r"^SPRT: llr (-?[\d.]+(?:e[-+]?\d+)?) \([^)]*\), "
    r"lbound (-?[\d.]+(?:e[-+]?\d+)?), ubound (-?[\d.]+(?:e[-+]?\d+)?)"
    r"(?: - (H[01]) was accepted)?"
)
# How each kind of result line starts, to find the last one before reading it.
PREFIXES = {"score": "Score of ", "elo": "Elo difference:", "sprt": "SPRT:"}


class RecordError(Exception):
    """The files do not hold a finished SPRT that can be recorded."""


@dataclass(frozen=True)
class Run:
    """One finished SPRT, as its notes and log give it."""

    date: str  # the day the run started, UTC
    commit: str  # of the checkout the script ran from, 12 digits
    dirty: bool  # whether that checkout had changes
    test: tuple[str, str]  # name, SHA-256
    base: tuple[str, str]
    options: tuple[str, str]  # the options given to each, or "none"
    elo0: str
    elo1: str
    alpha: str
    beta: str
    tc: str
    hash_mb: str
    rounds: str
    concurrency: str
    wins: int
    losses: int
    draws: int
    games: int  # the games the last score line counted
    elo: str
    margin: str
    llr: str
    lbound: str
    ubound: str
    outcome: str  # one of OUTCOMES
    pgn_sha256: str


def read_text(path: Path) -> list[str]:
    """The lines of a notes or log file, whichever script wrote it: with or
    without a byte-order mark, with either kind of line ending."""
    try:
        text = path.read_text(encoding="utf-8-sig", errors="replace")
    except OSError as problem:
        raise RecordError(f"cannot read {path}: {problem}") from None
    return text.splitlines()


def block(lines: list[str], heading: str) -> list[str]:
    """The indented lines under the line that starts with `heading`."""
    for index, line in enumerate(lines):
        if line.startswith(heading):
            rest = lines[index + 1:]
            end = next((n for n, text in enumerate(rest) if not text.startswith(" ")), len(rest))
            return rest[:end]
    raise RecordError(f"the notes have no '{heading}' part")


def field(lines: list[str], key: str) -> str:
    """The value of the last `key: value` line, or an error naming the key."""
    values = [line[len(key) + 1:].strip() for line in lines if line.startswith(key + ":")]
    if not values:
        raise RecordError(f"the notes have no '{key}:' line (was the run interrupted?)")
    return values[-1]


def setting(command: str, pattern: str, what: str) -> str:
    """One value out of the cutechess-cli command line of the notes."""
    found = re.search(pattern, command)
    if not found:
        raise RecordError(f"the command in the notes has no {what}")
    return found.group(1)


def read_log(lines: list[str]) -> dict[str, str]:
    """The last score, Elo and SPRT lines of a log, taken apart."""
    last = {}
    for pattern, name in ((SCORE, "score"), (ELO, "elo"), (SPRT, "sprt")):
        # The last line of its kind, then read: a last line that cannot be
        # read must not be passed over for an earlier one that can.
        candidates = [line for line in lines if line.startswith(PREFIXES[name])]
        if not candidates:
            raise RecordError(f"the log has no {name} line: nothing can be recorded")
        last[name] = pattern.match(candidates[-1])
        if not last[name]:
            raise RecordError(f"the log's last {name} line cannot be read: {candidates[-1]}")
    wins, losses, draws, games = last["score"].groups()
    llr, lbound, ubound, accepted = last["sprt"].groups()
    return {
        "wins": wins, "losses": losses, "draws": draws, "games": games,
        "elo": last["elo"].group(1), "margin": last["elo"].group(2),
        "llr": llr, "lbound": lbound, "ubound": ubound,
        "outcome": f"{accepted} accepted" if accepted else "undecided",
    }


def read_run(notes_path: Path, log_path: Path | None = None) -> Run:
    """Reads a run from its notes and its log (beside the notes unless
    given) and checks that it is a finished SPRT on which both agree."""
    notes = read_text(notes_path)
    if not notes or notes[0].strip() != "Vyvrael match run notes":
        raise RecordError(f"{notes_path} is not the notes of a match run")
    if field(notes, "mode") != "sprt":
        raise RecordError(f"the run was a {field(notes, 'mode')}, not an SPRT")
    status, said = field(notes, "status"), field(notes, "outcome")
    if status != "0":
        raise RecordError(f"the run failed (status {status}): {said}")
    if log_path is None:
        log_path = notes_path.with_name(notes_path.name.removesuffix(".notes.txt") + ".log")
    log = read_log(read_text(log_path))
    if not said.startswith(log["outcome"]):
        raise RecordError(f"the notes say '{said}' but the log's last SPRT line means {log['outcome']}")

    engines = [line.strip() for line in block(notes, "engines (name")]
    given = [line.strip() for line in block(notes, "options given")]
    if len(engines) != 6 or len(given) != 2:
        raise RecordError("the notes do not list exactly a test and a base build")
    (test, _, test_sha, base, _, base_sha) = engines
    command = " ".join(block(notes, "command:"))
    commit, _, state = field(notes, "repository").partition(" ")
    return Run(
        date=field(notes, "started")[:10],
        commit=commit[:12],
        dirty="dirty" in state,
        test=(test, test_sha),
        base=(base, base_sha),
        options=tuple(options_of(text) for text in given),
        elo0=setting(command, r"\belo0=(-?[\d.]+)", "elo0"),
        elo1=setting(command, r"\belo1=(-?[\d.]+)", "elo1"),
        alpha=setting(command, r"\balpha=([\d.]+)", "alpha"),
        beta=setting(command, r"\bbeta=([\d.]+)", "beta"),
        tc=setting(command, r"\btc=([0-9./:+]+)", "time control"),
        hash_mb=setting(command, r"\boption\.Hash=(\d+)", "hash size"),
        rounds=setting(command, r"-rounds\W+(\d+)", "round cap"),
        concurrency=setting(command, r"-concurrency\W+(\d+)", "number of games at once"),
        wins=int(log["wins"]), losses=int(log["losses"]), draws=int(log["draws"]),
        games=int(log["games"]),
        elo=log["elo"], margin=log["margin"],
        llr=log["llr"], lbound=log["lbound"], ubound=log["ubound"],
        outcome=log["outcome"],
        pgn_sha256=pgn_digest(notes),
    )


def options_of(line: str) -> str:
    """The options of one `name: options` line of the notes."""
    _, colon, options = line.partition(": ")
    if not colon:
        raise RecordError(f"the notes' options line cannot be read: {line}")
    return options


def pgn_digest(notes: list[str]) -> str:
    """The SHA-256 of the run's PGN, which is what identifies the run. The
    scripts write `none` when no PGN was written; such a run has no
    identity and is refused."""
    digest = field(notes, "pgn sha256").lower()
    if not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise RecordError("the notes have no SHA-256 of a PGN (was one written?): the run cannot be identified")
    return digest


# --- writing the result down -------------------------------------------------

HISTORY = Path(__file__).resolve().parents[1] / "docs" / "testing" / "sprt-history.md"
HEADER = (
    "| Date | Change | Test build | Base build | Bounds | alpha, beta | Conditions "
    "| Games | W - L - D | Elo | LLR | Outcome | Run |"
)


def build_cell(build: tuple[str, str], options: str) -> str:
    """A build as the table names it: its name, the start of its SHA-256,
    and the options it was given, if any."""
    name, digest = build
    cell = f"{name} (`{digest[:12]}`)"
    return cell if options == "none" else f"{cell}, {options}"


def row(run: Run, change: str) -> str:
    """The table row for `run`. Nothing of the notes' paths goes into it."""
    change = " ".join(change.split()).replace("|", "\\|")
    if not change:
        raise RecordError("--change needs a few words on what was tested")
    outcome = f"**{run.outcome}**" if run.outcome == "H1 accepted" else run.outcome
    cells = [
        run.date,
        change,
        build_cell(run.test, run.options[0]),
        build_cell(run.base, run.options[1]),
        f"[{run.elo0}, {run.elo1}]",
        f"{run.alpha}, {run.beta}",
        f"{run.tc}, Hash {run.hash_mb}, {run.concurrency} at once, cap {run.rounds} rounds",
        str(run.games),
        f"{run.wins} - {run.losses} - {run.draws}",
        f"{run.elo} +/- {run.margin}",
        f"{run.llr} ({run.lbound}, {run.ubound})",
        outcome,
        f"`{run.pgn_sha256[:12]}`",
    ]
    return "| " + " | ".join(cells) + " |"


def summary(run: Run) -> str:
    """The result in one line, for the commit message of the change."""
    return (
        f"SPRT [{run.elo0}, {run.elo1}], alpha {run.alpha}, beta {run.beta}, {run.tc}: "
        f"{run.wins} - {run.losses} - {run.draws} in {run.games} games, "
        f"Elo {run.elo} +/- {run.margin}, LLR {run.llr} ({run.lbound}, {run.ubound}): "
        f"{run.outcome}. Run {run.pgn_sha256[:12]} in docs/testing/sprt-history.md."
    )


def record(run: Run, change: str, target: Path, dry_run: bool = False) -> bool:
    """Appends the row for `run` to the table that ends `target`. Returns
    whether a row was added: a run already in the table (by the start of
    its PGN's SHA-256) is not added twice.

    The file is never created or repaired here: it must hold the table's
    heading and end with the table, or the run is refused.
    """
    line = row(run, change)
    try:
        text = target.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as problem:
        raise RecordError(f"cannot read {target}: {problem}") from None
    lines = text.replace("\r\n", "\n").split("\n")
    if HEADER not in lines:
        raise RecordError(f"{target} does not hold the SPRT table (its heading line is missing)")
    while lines and not lines[-1].strip():
        lines.pop()
    if not lines[-1].startswith("|"):
        raise RecordError(f"{target} has text after the table; a row cannot be appended")
    if any(text.endswith(f"| `{run.pgn_sha256[:12]}` |") for text in lines):
        return False
    if not dry_run:
        with target.open("w", encoding="utf-8", newline="\n") as out:
            out.write("\n".join(lines + [line]) + "\n")
    return True


def main(argv: list[str] | None = None) -> int:
    import argparse
    import sys

    parser = argparse.ArgumentParser(
        description="Append the result of an SPRT run to docs/testing/sprt-history.md.")
    parser.add_argument("notes", type=Path, help="the run's .notes.txt (its .log is read from beside it)")
    parser.add_argument("--change", required=True, help="what was tested, in a few words")
    parser.add_argument("--log", type=Path, help="the run's log, if it is not beside the notes")
    parser.add_argument("--file", type=Path, default=HISTORY, help="the history file (default: %(default)s)")
    parser.add_argument("--dry-run", action="store_true", help="show the row and write nothing")
    args = parser.parse_args(argv)
    try:
        run = read_run(args.notes, args.log)
        added = record(run, args.change, args.file, args.dry_run)
    except RecordError as problem:
        print(f"record_sprt.py: {problem}", file=sys.stderr)
        return 2
    print(row(run, args.change))
    print()
    print(summary(run))
    if not added:
        print("record_sprt.py: this run is already in the table; nothing was written", file=sys.stderr)
    elif args.dry_run:
        print("record_sprt.py: dry run; nothing was written", file=sys.stderr)
    if run.outcome == "undecided":
        print("record_sprt.py: undecided is not an acceptance: raise the round cap and run again, "
              "or drop the change", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
