# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Offline tests for scripts/linux/rate.sh.

The real Ordo is never run: the script is given a stand-in that answers
`-h` and writes a fixed table. Everything lives in a temporary directory.
They are skipped where there is no bash.

    python3 -m unittest discover -s scripts/tests
"""
from __future__ import annotations

import os
import shlex
import subprocess
import tempfile
import unittest
from pathlib import Path

from test_match_sh import BASH, script_file

SCRIPT = Path(__file__).resolve().parents[1] / "linux" / "rate.sh"

HELP = ["-a <avg>", "-A <player>", "-W", "-s #", "-p <file>", "-P <file>", "-o <file>",
        "-c <file>"]

TABLE = (
    "\n   # PLAYER                :   RATING  ERROR  POINTS  PLAYED   (%)\n"
    "   1 Stockfish 19          :      0.0   ----    44.5      48    93\n"
    "   2 Vyvrael 0.1.0-a1      :  -1900.5  410.0    20.0      48    42\n"
    "   3 Rustic Alpha 1.5      :  -2025.1  422.7    18.0      48    38\n"
    "\nWhite advantage = 4.67 +/- 15.72\n"
)


def game(white: str, black: str) -> str:
    return f'[White "{white}"]\n[Black "{black}"]\n[Result "1-0"]\n\n1. e4 1-0\n\n'


@unittest.skipUnless(BASH, "bash is not available")
class RateShTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name)
        self.first = self.tmp / "ladder one.pgn"
        self.first.write_text(
            game("Stockfish 19", "Vyvrael 0.1.0-a1") + game("Rustic Alpha 1.5", "Stockfish 19"),
            encoding="utf-8",
        )
        self.second = self.tmp / "more.pgn"
        self.second.write_text(game("Vyvrael 0.1.0-a1", "Rustic Alpha 1.5"), encoding="utf-8")
        self.ordo = self.fake_ordo(HELP)

    def fake_ordo(self, flags: list[str], name: str = "ordo", status: int = 0) -> Path:
        """Answers -h; otherwise records its arguments and any list or joined
        input beside itself and writes the table to the -o file."""
        lines = "".join(f" {flag}\\tsomething\\n" for flag in flags)
        record = (self.tmp / f"{name}.args").as_posix()
        seen = (self.tmp / f"{name}.input").as_posix()
        table = TABLE.replace("\n", "\\n")
        return script_file(
            self.tmp / name,
            f"if [ \"$1\" = -h ]; then printf '{lines}'; exit 0; fi\n"
            f"printf '%s\\n' \"$@\" > '{record}'\n"
            "while [ $# -gt 0 ]; do\n"
            "  case \"$1\" in\n"
            f"    -P|-p) cat \"$2\" > '{seen}'; shift 2;;\n"
            f"    -o) printf '%b' '{table}' >\"$2\"; shift 2;;\n"
            "    -c) printf 'csv\\n' > \"$2\"; shift 2;;\n"
            "    *) shift;;\n"
            "  esac\n"
            "done\n"
            f"exit {status}\n",
        )

    def run_script(self, *args: str, env: dict | None = None):
        environment = {
            key: value for key, value in os.environ.items() if not key.startswith("VYVRAEL_")
        }
        environment["VYVRAEL_LOCAL_ENV"] = (self.tmp / "no-local.env").as_posix()
        environment.update(env or {})
        return subprocess.run(
            [BASH, SCRIPT.as_posix(), *args],
            capture_output=True, text=True, env=environment, cwd=self.tmp, timeout=120,
        )

    def refused(self, result, *fragments: str):
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertEqual(result.stdout, "")
        for fragment in fragments:
            self.assertIn(fragment, result.stderr)

    def dry(self, *args: str) -> list[str]:
        result = self.run_script("--ordo", self.ordo.as_posix(), "--dry-run", *args)
        self.assertEqual(result.returncode, 0, result.stderr)
        return shlex.split(result.stdout)

    def test_the_defaults_are_those_of_the_baseline(self):
        stem = self.first.as_posix()[: -len(".pgn")] + "-ratings"
        self.assertEqual(
            self.dry(self.first.as_posix()),
            [self.ordo.as_posix(), "-a", "0", "-A", "Stockfish 19", "-W", "-s", "500",
             "-p", self.first.as_posix(), "-o", stem + ".txt", "-c", stem + ".csv"],
        )

    def test_options(self):
        words = self.dry(
            "--anchor", "Vyvrael 0.1.0-a1", "--anchor-rating", "-1900.5", "--simulations", "0",
            "--out", "out/list", self.first.as_posix(),
        )
        self.assertEqual(
            words[1:],
            ["-a", "-1900.5", "-A", "Vyvrael 0.1.0-a1", "-W", "-p", self.first.as_posix(),
             "-o", "out/list.txt", "-c", "out/list.csv"],
        )

    def test_several_files_go_through_a_list_file(self):
        words = self.dry(self.first.as_posix(), self.second.as_posix())
        stem = self.first.as_posix()[: -len(".pgn")] + "-ratings"
        self.assertEqual(words[words.index("-P") + 1], stem + ".pgn-list.txt")
        self.assertNotIn("-p", words)
        # A dry run writes nothing.
        self.assertFalse(Path(stem + ".pgn-list.txt").exists())

    def test_a_real_run_prints_the_order_and_the_reminder(self):
        result = self.run_script(
            "--ordo", self.ordo.as_posix(), self.first.as_posix(), self.second.as_posix()
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("ORDER (strongest first)", result.stdout)
        order = [line for line in result.stdout.splitlines() if line[:1].isdigit()]
        self.assertEqual(order, ["1. Stockfish 19", "2. Vyvrael 0.1.0-a1", "3. Rustic Alpha 1.5"])
        self.assertIn("Read the order, not the distances", result.stdout)
        self.assertIn("error bars are wide", result.stdout)
        self.assertIn("relative to Stockfish 19 = 0", result.stdout)
        stem = self.first.as_posix()[: -len(".pgn")] + "-ratings"
        self.assertTrue(Path(stem + ".txt").is_file())
        self.assertTrue(Path(stem + ".csv").is_file())
        # Ordo was given both files through its list, which is then removed.
        listed = (self.tmp / "ordo.input").read_text(encoding="utf-8").splitlines()
        self.assertEqual(listed, [self.first.as_posix(), self.second.as_posix()])
        self.assertFalse(Path(stem + ".pgn-list.txt").exists())

    def test_an_ordo_without_a_list_option_gets_one_joined_file(self):
        old = self.fake_ordo([flag for flag in HELP if not flag.startswith("-P")], "old-ordo")
        result = self.run_script(
            "--ordo", old.as_posix(), self.first.as_posix(), self.second.as_posix()
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        arguments = (self.tmp / "old-ordo.args").read_text(encoding="utf-8").splitlines()
        self.assertNotIn("-P", arguments)
        joined = (self.tmp / "old-ordo.input").read_text(encoding="utf-8")
        self.assertEqual(joined.count("[White "), 3)
        self.assertIn('[White "Vyvrael 0.1.0-a1"]\n[Black "Rustic Alpha 1.5"]', joined)

    def test_the_anchor_must_have_played(self):
        result = self.run_script(
            "--ordo", self.ordo.as_posix(), "--anchor", "Stockfish 18", self.first.as_posix()
        )
        self.refused(result, "the anchor 'Stockfish 18' played in none of the games",
                     "Rustic Alpha 1.5, Stockfish 19, Vyvrael 0.1.0-a1")
        # In one of several files is enough.
        self.dry("--anchor", "Stockfish 19", self.second.as_posix(), self.first.as_posix())
        result = self.run_script(
            "--ordo", self.ordo.as_posix(), self.second.as_posix()
        )
        self.refused(result, "the anchor 'Stockfish 19' played in none of the games")

    def test_missing_files_and_bad_values(self):
        self.refused(
            self.run_script("--ordo", self.ordo.as_posix(), "nowhere.pgn"),
            "PGN file not found: nowhere.pgn",
        )
        for option, value, message in (
            ("--anchor-rating", "high", "--anchor-rating must be a number"),
            ("--simulations", "-1", "--simulations must be a whole number"),
            ("--anchor", "", "--anchor may not be empty"),
        ):
            result = self.run_script(
                "--ordo", self.ordo.as_posix(), option, value, self.first.as_posix()
            )
            self.refused(result, message)
        self.refused(self.run_script("--frobnicate", self.first.as_posix()), "unknown option")
        self.refused(self.run_script(self.first.as_posix(), "--out"), "--out needs a value")
        none = self.run_script()
        self.assertEqual(none.returncode, 2)
        self.assertIn("Usage:", none.stderr)
        self.assertIn("rate.sh [options]", self.run_script("--help").stdout)

    def test_a_missing_ordo_is_an_error_but_only_a_warning_in_a_dry_run(self):
        missing = (self.tmp / "no-ordo").as_posix()
        dry = self.run_script("--ordo", missing, "--dry-run", self.first.as_posix())
        self.assertEqual(dry.returncode, 0, dry.stderr)
        self.assertIn("warning: ordo not found", dry.stderr)
        self.refused(
            self.run_script("--ordo", missing, self.first.as_posix()),
            "ordo not found", "docs/TESTING.md",
        )

    def test_an_ordo_without_a_needed_option_or_that_fails_is_refused(self):
        bare = self.fake_ordo([flag for flag in HELP if not flag.startswith("-A")], "bare-ordo")
        self.refused(
            self.run_script("--ordo", bare.as_posix(), self.first.as_posix()),
            "does not list the option -A",
        )
        broken = self.fake_ordo(HELP, "broken-ordo", status=4)
        self.refused(
            self.run_script("--ordo", broken.as_posix(), self.first.as_posix()),
            "ordo failed with status 4",
        )

    def test_ordo_comes_from_the_local_file(self):
        local = self.tmp / "local.env"
        local.write_text(f"ORDO={self.ordo.as_posix()}\n", encoding="utf-8")
        result = self.run_script(
            "--dry-run", self.first.as_posix(), env={"VYVRAEL_LOCAL_ENV": local.as_posix()}
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(shlex.split(result.stdout)[0], self.ordo.as_posix())
        self.assertEqual(result.stderr, "")


if __name__ == "__main__":
    unittest.main()
