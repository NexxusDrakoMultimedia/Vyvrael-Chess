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
    r"^SPRT: llr (-?[\d.]+) \([^)]*\), lbound (-?[\d.]+), ubound (-?[\d.]+)"
    r"(?: - (H[01]) was accepted)?"
)


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
        matches = [found for found in map(pattern.match, lines) if found]
        if not matches:
            raise RecordError(f"the log has no {name} line: nothing can be recorded")
        last[name] = matches[-1]
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
        options=tuple(text.split(": ", 1)[1] for text in given),
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
        pgn_sha256=field(notes, "pgn sha256"),
    )
