# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Offline tests for scripts/record_sprt.py.

The notes and logs are written by the tests in the two forms the match
scripts leave them in. No match is played and nothing is fetched.

    python3 -m unittest discover -s scripts/tests
"""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import record_sprt  # noqa: E402

TEST_SHA = "a" * 64
BASE_SHA = "b" * 64
PGN_SHA = "0123456789abcdef" * 4

# The command as match.sh writes it (printf %q) and as match.ps1 does
# (arguments that hold a space in double quotes).
COMMAND_SH = (
    "/opt/cutechess-cli -engine name=Vyvrael\\ dev-1a2b3c4 cmd=/b/vyvrael proto=uci "
    "-engine name=Vyvrael\\ 0.1.0 cmd=/b/vyvrael-0.1.0 proto=uci -each tc=10+0.1 option.Hash=64 "
    "-openings file=/o/UHO.epd format=epd order=random -games 2 -repeat -rounds 5000 "
    "-recover -concurrency 3 -ratinginterval 10 -sprt elo0=0 elo1=5 alpha=0.05 beta=0.05 "
    "-pgnout /m/run.pgn "
)
COMMAND_PS1 = (
    'C:\\tools\\cutechess-cli.exe -engine "name=Vyvrael dev-1a2b3c4" cmd=C:\\b\\vyvrael.exe proto=uci '
    '-engine "name=Vyvrael 0.1.0" cmd=C:\\b\\vyvrael-0.1.0.exe proto=uci -each tc=10+0.1 option.Hash=64 '
    "-openings file=C:\\o\\UHO.epd format=epd order=random -games 2 -repeat -rounds 5000 "
    "-recover -concurrency 6 -ratinginterval 10 -sprt elo0=-5 elo1=0 alpha=0.05 beta=0.05 "
    "-pgnout C:\\m\\run.pgn"
)


def notes(command=COMMAND_SH, mode="sprt", status="0", outcome="H1 accepted: the test build gains at least elo0 (LLR 2.95)",
          ended=True, state="(clean)") -> str:
    text = f"""Vyvrael match run notes
mode:       {mode}
started:    2026-11-28T09:15:00Z
repository: 1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b {state}
engines:    no engine list used
host cores: 8 physical, 6 games at once
cutechess:  cutechess-cli 1.5.1
openings:   /o/UHO.epd
pgn:        /m/run.pgn

engines (name, file, SHA-256):
  Vyvrael dev-1a2b3c4
    /b/vyvrael
    {TEST_SHA}
  Vyvrael 0.1.0
    /b/vyvrael-0.1.0
    {BASE_SHA}

options given (--test-option, --base-option):
  Vyvrael dev-1a2b3c4: Move Overhead=100; Variety=0
  Vyvrael 0.1.0: none

command:
  {command}
"""
    if ended:
        text += f"""
ended:      2026-11-28T11:02:10Z
status:     {status}
outcome:    {outcome}
games:      1234
pgn sha256: {PGN_SHA}
"""
    return text


def log(final="SPRT: llr 2.95 (100.3%), lbound -2.94, ubound 2.94 - H1 was accepted") -> str:
    return f"""Started game 1 of 10000 (Vyvrael dev-1a2b3c4 vs Vyvrael 0.1.0)
Score of Vyvrael dev-1a2b3c4 vs Vyvrael 0.1.0: 4 - 3 - 3  [0.550] 10
Elo difference: 34.9 +/- 190.1, LOS: 64.7 %, DrawRatio: 30.0 %
SPRT: llr 0.02 (0.7%), lbound -2.94, ubound 2.94
Warning: some line cutechess-cli prints in between
Score of Vyvrael dev-1a2b3c4 vs Vyvrael 0.1.0: 520 - 431 - 281  [0.536] 1232
Elo difference: 25.1 +/- 17.0, LOS: 99.8 %, DrawRatio: 22.8 %
{final}
Finished match
"""


class Files(unittest.TestCase):
    """A temporary folder with one run's notes and log in it."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)

    def run_files(self, notes_text: str, log_text: str | None, newline="\n", bom=False) -> Path:
        path = self.dir / "run.notes.txt"
        data = notes_text.replace("\n", newline).encode("utf-8")
        path.write_bytes((b"\xef\xbb\xbf" if bom else b"") + data)
        if log_text is not None:
            (self.dir / "run.log").write_bytes(log_text.replace("\n", newline).encode("utf-8"))
        return path

    def refused(self, notes_text: str, log_text: str | None) -> str:
        with self.assertRaises(record_sprt.RecordError) as caught:
            record_sprt.read_run(self.run_files(notes_text, log_text))
        return str(caught.exception)


class Reading(Files):
    def test_a_run_of_match_sh(self):
        run = record_sprt.read_run(self.run_files(notes(), log()))
        self.assertEqual(run.date, "2026-11-28")
        self.assertEqual((run.commit, run.dirty), ("1a2b3c4d5e6f", False))
        self.assertEqual(run.test, ("Vyvrael dev-1a2b3c4", TEST_SHA))
        self.assertEqual(run.base, ("Vyvrael 0.1.0", BASE_SHA))
        self.assertEqual(run.options, ("Move Overhead=100; Variety=0", "none"))
        self.assertEqual((run.elo0, run.elo1, run.alpha, run.beta), ("0", "5", "0.05", "0.05"))
        self.assertEqual((run.tc, run.hash_mb, run.rounds, run.concurrency), ("10+0.1", "64", "5000", "3"))
        # The last score line, not the first, and not the PGN's game count.
        self.assertEqual((run.wins, run.losses, run.draws, run.games), (520, 431, 281, 1232))
        self.assertEqual((run.elo, run.margin), ("25.1", "17.0"))
        self.assertEqual((run.llr, run.lbound, run.ubound), ("2.95", "-2.94", "2.94"))
        self.assertEqual(run.outcome, "H1 accepted")
        self.assertEqual(run.pgn_sha256, PGN_SHA)

    def test_a_run_of_match_ps1(self):
        """Carriage returns, a byte-order mark, quoted arguments, a negative bound."""
        text = notes(COMMAND_PS1, outcome="H0 accepted: no gain shown; the change is rejected (LLR -2.97)",
                     state="(dirty)")
        final = "SPRT: llr -2.97 (-101.0%), lbound -2.94, ubound 2.94 - H0 was accepted"
        run = record_sprt.read_run(self.run_files(text, log(final), newline="\r\n", bom=True))
        self.assertEqual((run.elo0, run.elo1, run.concurrency), ("-5", "0", "6"))
        self.assertEqual((run.outcome, run.llr, run.dirty), ("H0 accepted", "-2.97", True))
        self.assertEqual(run.test[0], "Vyvrael dev-1a2b3c4")

    def test_an_undecided_run_is_read_as_undecided(self):
        text = notes(outcome="undecided: the round cap was reached first (LLR 1.83)")
        run = record_sprt.read_run(self.run_files(text, log("SPRT: llr 1.83 (62.3%), lbound -2.94, ubound 2.94")))
        self.assertEqual(run.outcome, "undecided")
        self.assertIn(run.outcome, record_sprt.OUTCOMES)

    def test_the_log_can_be_named(self):
        path = self.run_files(notes(), None)
        other = self.dir / "elsewhere.txt"
        other.write_text(log(), encoding="utf-8")
        self.assertEqual(record_sprt.read_run(path, other).wins, 520)


class Refusals(Files):
    def test_what_is_not_an_sprt(self):
        self.assertIn("gauntlet, not an SPRT", self.refused(notes(mode="gauntlet"), log()))
        self.assertIn("not the notes of a match run", self.refused("something else\n", log()))

    def test_a_run_that_did_not_finish(self):
        self.assertIn("interrupted", self.refused(notes(ended=False), log()))
        said = "cutechess-cli failed with status 1; undecided: the round cap was reached first (LLR 0.4)"
        self.assertIn("failed (status 1)", self.refused(notes(status="1", outcome=said), log()))

    def test_a_log_without_a_result(self):
        self.assertIn("cannot read", self.refused(notes(), None))
        self.assertIn("no score line", self.refused(notes(), "Started game 1 of 10\n"))
        without = "\n".join(line for line in log().splitlines() if not line.startswith("SPRT:"))
        self.assertIn("no sprt line", self.refused(notes(), without))

    def test_notes_and_log_that_disagree(self):
        text = notes(outcome="H0 accepted: no gain shown; the change is rejected (LLR -2.97)")
        self.assertIn("but the log's last SPRT line means H1 accepted", self.refused(text, log()))
        undecided = "SPRT: llr 1.83 (62.3%), lbound -2.94, ubound 2.94"
        self.assertIn("means undecided", self.refused(notes(), log(undecided)))

    def test_a_command_without_bounds(self):
        self.assertIn("no elo0", self.refused(notes(COMMAND_SH.replace("elo0=0 ", "")), log()))


if __name__ == "__main__":
    unittest.main()
