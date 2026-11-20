# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Offline tests for scripts/windows/rate.ps1.

The real Ordo is never run: the script is given a stand-in (a PowerShell
script) that answers `-h` and writes a fixed table. Everything lives in a
temporary directory. They are skipped where there is no PowerShell. The
cases mirror scripts/tests/test_rate_sh.py.

    python3 -m unittest discover -s scripts/tests
"""
from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from test_match_ps1 import POWERSHELL, split_command
from test_rate_sh import HELP, TABLE, game

SCRIPT = Path(__file__).resolve().parents[1] / "windows" / "rate.ps1"


@unittest.skipUnless(POWERSHELL, "PowerShell is not available")
class RatePs1Test(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name).resolve()
        self.first = self.tmp / "ladder one.pgn"
        self.first.write_text(
            game("Stockfish 19", "Vyvrael 0.1.0-a1") + game("Rustic Alpha 1.5", "Stockfish 19"),
            encoding="utf-8",
        )
        self.second = self.tmp / "more.pgn"
        self.second.write_text(game("Vyvrael 0.1.0-a1", "Rustic Alpha 1.5"), encoding="utf-8")
        self.ordo = self.fake_ordo(HELP)

    def fake_ordo(self, flags: list[str], name: str = "ordo.ps1", status: int = 0) -> Path:
        """Answers -h; otherwise records its arguments and any list or joined
        input beside itself and writes the table to the -o file."""
        listing = "\n".join(f" {flag}\tsomething" for flag in flags)
        stem = name[: -len(".ps1")]
        record = self.tmp / f"{stem}.args"
        seen = self.tmp / f"{stem}.input"
        path = self.tmp / name
        path.write_text(
            "if ($args[0] -ceq '-h') {\n@'\n" + listing + "\n'@\nexit 0 }\n"
            f"[IO.File]::WriteAllLines('{record}', [string[]] $args)\n"
            "for ($i = 0; $i -lt $args.Count; $i++) {\n"
            "  $next = $args[$i + 1]\n"
            f"  if ($args[$i] -cin @('-p', '-P')) {{ Copy-Item -LiteralPath $next '{seen}' }}\n"
            "  if ($args[$i] -ceq '-o') { [IO.File]::WriteAllText($next, @'\n"
            + TABLE + "\n'@) }\n"
            "  if ($args[$i] -ceq '-c') { [IO.File]::WriteAllText($next, 'csv') }\n"
            "}\n"
            f"exit {status}\n",
            encoding="utf-8",
        )
        return path

    def run_script(self, *args: str, env: dict | None = None):
        environment = {
            key: value for key, value in os.environ.items() if not key.startswith("VYVRAEL_")
        }
        environment["VYVRAEL_LOCAL_ENV"] = str(self.tmp / "no-local.env")
        environment.update(env or {})
        return subprocess.run(
            [POWERSHELL, "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
             "-File", str(SCRIPT), *args],
            capture_output=True, text=True, env=environment, cwd=self.tmp, timeout=180,
        )

    def refused(self, result, *fragments: str):
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertEqual(result.stdout.strip(), "")
        for fragment in fragments:
            self.assertIn(fragment, result.stderr)

    def dry(self, *args: str) -> list[str]:
        result = self.run_script("-Ordo", str(self.ordo), "-DryRun", *args)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return split_command(result.stdout.strip())

    def stem(self) -> str:
        return str(self.first)[: -len(".pgn")] + "-ratings"

    def test_the_defaults_are_those_of_the_baseline(self):
        self.assertEqual(
            self.dry(str(self.first)),
            [str(self.ordo), "-a", "0", "-A", "Stockfish 19", "-W", "-s", "500",
             "-p", str(self.first), "-o", self.stem() + ".txt", "-c", self.stem() + ".csv"],
        )

    def test_options(self):
        words = self.dry(
            "-Anchor", "Vyvrael 0.1.0-a1", "-AnchorRating", "-1900.5", "-Simulations", "0",
            "-Out", "out/list", str(self.first),
        )
        self.assertEqual(
            words[1:],
            ["-a", "-1900.5", "-A", "Vyvrael 0.1.0-a1", "-W", "-p", str(self.first),
             "-o", "out/list.txt", "-c", "out/list.csv"],
        )

    def test_several_files_go_through_a_list_file(self):
        words = self.dry(str(self.first), str(self.second))
        self.assertEqual(words[words.index("-P") + 1], self.stem() + ".pgn-list.txt")
        self.assertNotIn("-p", words)
        self.assertFalse(Path(self.stem() + ".pgn-list.txt").exists())

    def test_a_real_run_prints_the_order_and_the_reminder(self):
        result = self.run_script("-Ordo", str(self.ordo), str(self.first), str(self.second))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("ORDER (strongest first)", result.stdout)
        order = [line for line in result.stdout.splitlines() if line[:1].isdigit()]
        self.assertEqual(order, ["1. Stockfish 19", "2. Vyvrael 0.1.0-a1", "3. Rustic Alpha 1.5"])
        self.assertIn("Read the order, not the distances", result.stdout)
        self.assertIn("error bars are wide", result.stdout)
        self.assertIn("relative to Stockfish 19 = 0", result.stdout)
        self.assertTrue(Path(self.stem() + ".txt").is_file())
        self.assertTrue(Path(self.stem() + ".csv").is_file())
        listed = (self.tmp / "ordo.input").read_text(encoding="utf-8").splitlines()
        self.assertEqual(listed, [str(self.first), str(self.second)])
        self.assertFalse(Path(self.stem() + ".pgn-list.txt").exists())

    def test_an_ordo_without_a_list_option_gets_one_joined_file(self):
        old = self.fake_ordo([flag for flag in HELP if not flag.startswith("-P")], "old-ordo.ps1")
        result = self.run_script("-Ordo", str(old), str(self.first), str(self.second))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        arguments = (self.tmp / "old-ordo.args").read_text(encoding="utf-8").splitlines()
        self.assertNotIn("-P", arguments)
        joined = (self.tmp / "old-ordo.input").read_text(encoding="utf-8")
        self.assertEqual(joined.count("[White "), 3)
        self.assertIn('[White "Vyvrael 0.1.0-a1"]\n[Black "Rustic Alpha 1.5"]', joined)

    def test_the_anchor_must_have_played(self):
        result = self.run_script(
            "-Ordo", str(self.ordo), "-Anchor", "Stockfish 18", str(self.first)
        )
        self.refused(result, "the anchor 'Stockfish 18' played in none of the games",
                     "Rustic Alpha 1.5, Stockfish 19, Vyvrael 0.1.0-a1")
        self.dry("-Anchor", "Stockfish 19", str(self.second), str(self.first))
        self.refused(
            self.run_script("-Ordo", str(self.ordo), str(self.second)),
            "the anchor 'Stockfish 19' played in none of the games",
        )

    def test_missing_files_and_bad_values(self):
        self.refused(
            self.run_script("-Ordo", str(self.ordo), "nowhere.pgn"),
            "PGN file not found: nowhere.pgn",
        )
        for option, value, message in (
            ("-AnchorRating", "high", "-AnchorRating must be a number"),
            ("-Simulations", "many", "-Simulations must be a whole number"),
        ):
            result = self.run_script("-Ordo", str(self.ordo), option, value, str(self.first))
            self.refused(result, message)
        # An option that does not exist is refused by PowerShell itself.
        unknown = self.run_script("-Frobnicate", str(self.first))
        self.assertNotEqual(unknown.returncode, 0)
        self.assertIn("Frobnicate", unknown.stderr)
        self.assertEqual(unknown.stdout.strip(), "")
        none = self.run_script()
        self.assertEqual(none.returncode, 2)
        self.assertIn("Usage:", none.stderr)
        self.assertIn("rate.ps1 [options]", self.run_script("-Help").stdout)

    def test_a_missing_ordo_is_an_error_but_only_a_warning_in_a_dry_run(self):
        missing = str(self.tmp / "no-ordo.exe")
        dry = self.run_script("-Ordo", missing, "-DryRun", str(self.first))
        self.assertEqual(dry.returncode, 0, dry.stderr)
        self.assertIn("warning: ordo not found", dry.stderr)
        self.refused(
            self.run_script("-Ordo", missing, str(self.first)),
            "ordo not found", "docs/TESTING.md",
        )

    def test_an_ordo_without_a_needed_option_or_that_fails_is_refused(self):
        bare = self.fake_ordo(
            [flag for flag in HELP if not flag.startswith("-A")], "bare-ordo.ps1"
        )
        self.refused(
            self.run_script("-Ordo", str(bare), str(self.first)), "does not list the option -A"
        )
        broken = self.fake_ordo(HELP, "broken-ordo.ps1", status=4)
        self.refused(
            self.run_script("-Ordo", str(broken), str(self.first)), "ordo failed with status 4"
        )

    def test_a_table_from_an_earlier_run_does_not_pass_for_this_one(self):
        Path(self.stem() + ".txt").write_text("   1 Old Engine : 0.0\n", encoding="utf-8")
        listing = "\n".join(f" {flag}\tsomething" for flag in HELP)
        idle = self.tmp / "idle-ordo.ps1"
        idle.write_text(
            "if ($args[0] -ceq '-h') {\n@'\n" + listing + "\n'@\n}\nexit 0\n", encoding="utf-8"
        )
        result = self.run_script("-Ordo", str(idle), str(self.first))
        self.refused(result, "ordo wrote no table")
        self.assertFalse(Path(self.stem() + ".txt").exists())

    def test_ordo_comes_from_the_local_file(self):
        local = self.tmp / "local.env"
        local.write_text(f"ORDO={self.ordo}\n", encoding="utf-8")
        result = self.run_script(
            "-DryRun", str(self.first), env={"VYVRAEL_LOCAL_ENV": str(local)}
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(split_command(result.stdout.strip())[0], str(self.ordo))
        self.assertEqual(result.stderr, "")


if __name__ == "__main__":
    unittest.main()
