# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Offline tests for scripts/windows/match.ps1.

No game is played and the real cutechess-cli is never run: the tests give
the script a stand-in (a PowerShell script) that only answers `-help` and
`-version`, and read the command the script would run from `-DryRun`.
Everything lives in a temporary directory. They are skipped where there is
no PowerShell. The cases mirror scripts/tests/test_match_sh.py: the two
scripts must behave alike.

    python3 -m unittest discover -s scripts/tests
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "windows" / "match.ps1"

HELP_FLAGS = [
    "-engine OPTIONS", "-each OPTIONS", "-concurrency N", "-draw movenumber=NUMBER",
    "-resign movecount=COUNT", "-tournament TYPE", "-games N", "-rounds N",
    "-sprt elo0=ELO0", "-ratinginterval N", "-openings file=FILE", "-pgnout FILE",
    "-recover", "-repeat [N]", "-seeds N", "-srand N",
]


def find_powershell() -> str | None:
    for name in ("pwsh", "powershell"):
        shell = shutil.which(name)
        if not shell:
            continue
        try:
            probe = subprocess.run(
                [shell, "-NoProfile", "-NonInteractive", "-Command", "Write-Output ok"],
                capture_output=True, text=True, timeout=60,
            )
        except (OSError, subprocess.SubprocessError):
            continue
        if probe.stdout.strip() == "ok":
            return shell
    return None


POWERSHELL = find_powershell()


SWITCHES = {"-DryRun", "-Deterministic", "-LowPriority", "-NoLowPriority", "-Help"}


def override(base: list[str], extra: tuple[str, ...]) -> list[str]:
    """`base` without the options that `extra` gives again, then `extra`.
    PowerShell refuses a parameter given twice, where bash takes the last."""
    again = {word for word in extra if word.startswith("-") and not word[1:2].isdigit()}
    kept, skip = [], False
    for word in base:
        if skip:
            skip = False
        elif word in again:
            skip = word not in SWITCHES
        else:
            kept.append(word)
    return kept + list(extra)


def split_command(line: str) -> list[str]:
    """Splits the printed command: arguments holding a space are in quotes."""
    return [word.strip('"') for word in re.findall(r'"[^"]*"|\S+', line)]


class Fixture:
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name).resolve()
        self.test_build = self.tmp / "vyvrael-test.exe"
        self.test_build.write_bytes(b"test")
        self.base_build = self.tmp / "vyvrael-base.exe"
        self.base_build.write_bytes(b"base")
        self.openings = self.tmp / "openings.epd"
        self.openings.write_text("8/8/8/8/8/8/8/K6k w - - 0 1\n", encoding="utf-8")
        self.cutechess = self.fake_cutechess(HELP_FLAGS)
        self.opponent = self.tmp / "weak engine.exe"
        self.opponent.write_bytes(b"weak")
        self.engines_json = self.tmp / "engines.json"
        self.engines_json.write_text(json.dumps([
            {
                "name": "Stockfish 19-1500",
                "command": self.opponent.name,
                "protocol": "uci",
                "workingDirectory": str(self.tmp),
                "options": [
                    {"name": "Hash", "value": 16},
                    {"name": "Threads", "value": 1},
                    {"name": "UCI_LimitStrength", "value": True},
                    {"name": "UCI_Elo", "value": 1500},
                ],
            },
            {"name": "Vice 1.1", "command": str(self.opponent), "protocol": "uci"},
            {"name": "Gone 1.0", "command": str(self.tmp / "missing.exe")},
        ]), encoding="utf-8")

    def fake_cutechess(
        self, flags: list[str], name: str = "cutechess-cli.ps1", play: str = "exit 0"
    ) -> Path:
        """A stand-in that answers -help and -version; anything else runs `play`."""
        listing = "\n".join(f"  {flag}\tsomething" for flag in flags)
        path = self.tmp / name
        path.write_text(
            "if ($args[0] -eq '-help') {\n@'\n" + listing + "\n'@\nexit 0 }\n"
            "if ($args[0] -eq '-version') { 'cutechess-cli 9.9.9'; 'more'; exit 0 }\n"
            + play + "\n",
            encoding="utf-8",
        )
        return path

    def run_script(self, *args: str, env: dict | None = None):
        environment = {
            key: value for key, value in os.environ.items() if not key.startswith("VYVRAEL_")
        }
        environment["VYVRAEL_LOCAL_ENV"] = str(self.tmp / "no-local.env")
        environment["VYVRAEL_MATCH_DATE"] = "2026-01-02"
        environment["VYVRAEL_PHYSICAL_CORES"] = "4"
        environment.update(env or {})
        return subprocess.run(
            [POWERSHELL, "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
             "-File", str(SCRIPT), *args],
            capture_output=True, text=True, env=environment, cwd=self.tmp, timeout=180,
        )

    def builds(self) -> list[str]:
        return [
            "-Test", str(self.test_build), "-TestName", "Vyvrael dev-abc1234",
            "-Base", str(self.base_build), "-BaseName", "Vyvrael 0.1.0-a1",
        ]

    def common(self) -> list[str]:
        return ["-Openings", str(self.openings), "-Cutechess", str(self.cutechess), "-DryRun"]

    BOUNDS = ["-Elo0", "0", "-Elo1", "5", "-Alpha", "0.05", "-Beta", "0.05"]

    def sprt(self, *extra: str, **kwargs):
        base = [*self.builds(), *self.BOUNDS, "-Rounds", "500", *self.common()]
        return self.run_script("sprt", *override(base, extra), **kwargs)

    def gauntlet(self, *extra: str, **kwargs):
        base = ["-Test", str(self.test_build), "-TestName", "Vyvrael 0.1.0-a1",
                "-Rounds", "20", "-EnginesJson", str(self.engines_json), *self.common()]
        return self.run_script("gauntlet", *override(base, extra), **kwargs)

    def command(self, result) -> list[str]:
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        lines = [line for line in result.stdout.splitlines() if line and not line.startswith("#")]
        self.assertEqual(len(lines), 1, result.stdout)
        return split_command(lines[0])

    def refused(self, result, *fragments: str):
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertEqual(result.stdout.strip(), "")
        for fragment in fragments:
            self.assertIn(fragment, result.stderr)


@unittest.skipUnless(POWERSHELL, "PowerShell is not available")
class MatchPs1Test(Fixture, unittest.TestCase):
    def test_sprt_command(self):
        words = self.command(self.sprt())
        self.assertEqual(words[0], str(self.cutechess))
        joined = " ".join(words)
        self.assertIn("-sprt elo0=0 elo1=5 alpha=0.05 beta=0.05", joined)
        self.assertIn("-each tc=10+0.1 option.Hash=64", joined)
        self.assertIn(f"-openings file={self.openings} format=epd order=random", joined)
        self.assertIn("-games 2 -repeat -rounds 500", joined)
        self.assertIn("-resign movecount=3 score=700 twosided=true", joined)
        self.assertIn("-draw movenumber=34 movecount=10 score=20", joined)
        self.assertIn("-recover", words)
        self.assertIn("-ratinginterval 10", joined)
        self.assertIn("-concurrency 2", joined)  # four cores minus two
        self.assertNotIn("-tournament", words)
        self.assertNotIn("-srand", words)
        self.assertEqual(words.count("-engine"), 2)
        first = words.index("-engine")
        self.assertEqual(
            words[first + 1 : first + 5],
            ["name=Vyvrael dev-abc1234", f"cmd={self.test_build}", "proto=uci",
             "option.Threads=1"],
        )
        self.assertIn("name=Vyvrael 0.1.0-a1", words)
        self.assertEqual(words[-2], "-pgnout")
        self.assertEqual(
            Path(words[-1]).as_posix(), "matches/2026-01-02-sprt-Vyvrael-dev-abc1234.pgn"
        )

    def test_head_to_head_has_no_sprt(self):
        words = self.command(
            self.run_script("h2h", *self.builds(), "-Rounds", "50", *self.common())
        )
        self.assertNotIn("-sprt", words)
        self.assertIn("-rounds 50", " ".join(words))

    def test_gauntlet_takes_opponents_from_the_engine_list(self):
        words = self.command(self.gauntlet("-Opponent", "Stockfish 19-1500;Vice 1.1"))
        self.assertIn("-tournament gauntlet -seeds 1", " ".join(words))
        self.assertEqual(words.count("-engine"), 3)
        self.assertEqual(words[words.index("-engine") + 1], "name=Vyvrael 0.1.0-a1")
        second = [index for index, word in enumerate(words) if word == "-engine"][1]
        self.assertEqual(
            words[second + 1 : second + 8],
            ["name=Stockfish 19-1500", f"cmd={self.opponent}", "proto=uci",
             f"dir={self.tmp}", "option.Threads=1", "option.UCI_LimitStrength=true",
             "option.UCI_Elo=1500"],
        )
        self.assertNotIn("option.Hash=16", words)
        self.assertIn("name=Vice 1.1", words)

    def test_optional_settings(self):
        result = self.sprt(
            "-Tc", "60+0.6", "-Hash", "128", "-Concurrency", "3", "-Seed", "7",
            "-Deterministic", "-LowPriority", "-Label", "run2", "-OutDir", "out dir",
        )
        self.assertIn("# at BelowNormal priority", result.stdout)
        words = self.command(result)
        joined = " ".join(words)
        self.assertIn("-each tc=60+0.6 option.Hash=128", joined)
        self.assertIn("-concurrency 3", joined)
        self.assertIn("-srand 7", joined)
        # Both builds get a fixed Seed and no Variety; the old option is gone.
        self.assertEqual(words.count("option.Seed=822286742"), 2)
        self.assertEqual(words.count("option.Variety=0"), 2)
        self.assertNotIn("DeterministicRNG", joined)
        self.assertEqual(
            Path(words[-1]).as_posix(), "out dir/2026-01-02-sprt-Vyvrael-dev-abc1234-run2.pgn"
        )

    def engines(self, words: list[str]) -> list[list[str]]:
        """The arguments of each -engine, up to the next flag."""
        starts = [index for index, word in enumerate(words) if word == "-engine"]
        groups = []
        for start in starts:
            end = start + 1
            while end < len(words) and not words[end].startswith("-"):
                end += 1
            groups.append(words[start + 1 : end])
        return groups

    def test_options_go_to_their_own_build(self):
        words = self.command(self.sprt(
            "-TestOption", "UCI_LimitStrength=true;UCI_Elo=1500",
            "-BaseOption", "Move Overhead=50",
        ))
        test, base = self.engines(words)
        # After the script's own settings, in the order given.
        self.assertEqual(test[3:], ["option.Threads=1", "option.UCI_LimitStrength=true",
                                    "option.UCI_Elo=1500"])
        self.assertEqual(base[3:], ["option.Threads=1", "option.Move Overhead=50"])

    def test_a_gauntlet_gives_options_to_the_build_under_test_only(self):
        words = self.command(self.gauntlet("-Opponent", "Vice 1.1", "-TestOption", "Seed=5"))
        test, opponent = self.engines(words)
        self.assertIn("option.Seed=5", test)
        self.assertNotIn("option.Seed=5", opponent)
        self.refused(
            self.gauntlet("-Opponent", "Vice 1.1", "-BaseOption", "Seed=5"),
            "-BaseOption is for sprt and h2h modes",
        )

    def test_bad_options_are_refused(self):
        # A semicolon separates options here, as it does opponents.
        for given, message in (
            ("UCI_Elo", "must be NAME=VALUE"),
            ("=1500", "must be NAME=VALUE"),
            ("UCI_Elo=", "must be NAME=VALUE"),
            ("UCI_Elo=15=00", "may hold only one '='"),
            ("UCI_Elo='1500'", "may not contain quotes or ';'"),
            (" UCI_Elo=1500", "may not start or end with a space"),
            ("Threads=4", "the script sets Threads"),
            ("hash=128", "the script sets hash"),
        ):
            with self.subTest(given=given):
                self.refused(self.sprt("-TestOption", given), f"-TestOption '{given}'", message)
        # Windows may re-encode a non-ASCII argument, so only the reason is checked.
        self.refused(self.sprt("-TestOption", "UCI_Elo=1500\u00e9"), "must be plain ASCII")
        # A line break, even at the end, would put a second command into the
        # engine's input.
        for given in ("UCI_Elo=1500\nquit", "UCI_Elo=1500\n"):
            with self.subTest(given=given):
                self.refused(self.sprt("-TestOption", given), "must be plain ASCII")
        self.refused(
            self.sprt("-TestOption", "SyzygyPath=C:\\tb dir\\"),
            "may not end with a backslash",
        )
        self.refused(
            self.sprt("-BaseOption", "UCI_Elo=1500;uci_elo=1600"),
            "-BaseOption: the option uci_elo is given twice",
        )
        self.refused(
            self.sprt("-Deterministic", "-TestOption", "Seed=5"),
            "-Deterministic already sets Seed",
        )

    def test_settings_come_from_the_local_file_then_environment_then_arguments(self):
        local = self.tmp / "local.env"
        local.write_text(
            "# a comment\n\nCONCURRENCY=3\nLOW_PRIORITY=true\n"
            f"OPENINGS={self.openings}\nCUTECHESS_CLI={self.cutechess}\nMATCH_DIR=from file\n",
            encoding="utf-8",
        )
        env = {"VYVRAEL_LOCAL_ENV": str(local)}
        base = ["sprt", *self.builds(), *self.BOUNDS, "-Rounds", "9", "-DryRun"]
        result = self.run_script(*base, env=env)
        self.assertIn("# at BelowNormal priority", result.stdout)
        words = self.command(result)
        self.assertIn("-concurrency 3", " ".join(words))
        self.assertTrue(Path(words[-1]).as_posix().startswith("from file/"))
        env["VYVRAEL_CONCURRENCY"] = "4"
        self.assertIn("-concurrency 4", " ".join(self.command(self.run_script(*base, env=env))))
        result = self.run_script(*base, "-Concurrency", "1", "-NoLowPriority", env=env)
        self.assertNotIn("BelowNormal", result.stdout)
        self.assertIn("-concurrency 1", " ".join(self.command(result)))

    def test_sprt_bounds_have_no_defaults(self):
        for missing in ("-Elo0", "-Elo1", "-Alpha", "-Beta"):
            bounds = {"-Elo0": "0", "-Elo1": "5", "-Alpha": "0.05", "-Beta": "0.05"}
            del bounds[missing]
            flat = [word for pair in bounds.items() for word in pair]
            result = self.run_script(
                "sprt", *self.builds(), *flat, "-Rounds", "10", *self.common()
            )
            self.refused(result, f"{missing} is required in sprt mode")
        self.refused(self.sprt("-Elo1", "five"), "-Elo1 must be a number")

    def test_a_round_cap_is_required(self):
        self.refused(
            self.run_script("h2h", *self.builds(), *self.common()), "-Rounds is required"
        )
        for bad in ("0", "many", "1.5"):
            result = self.run_script("h2h", *self.builds(), "-Rounds", bad, *self.common())
            self.refused(result, "-Rounds must be a whole number above zero")

    def test_missing_engines_are_refused(self):
        self.test_build.unlink()
        self.refused(self.sprt(), "the test build (-Test) not found")
        self.test_build.write_bytes(b"test")
        self.base_build.unlink()
        self.refused(self.sprt(), "the base build (-Base) not found")

    def test_name_collisions_are_refused(self):
        result = self.run_script(
            "h2h", "-Test", str(self.test_build), "-TestName", "Vyvrael dev",
            "-Base", str(self.base_build), "-BaseName", "Vyvrael dev", "-Rounds", "5",
            *self.common(),
        )
        self.refused(result, "the two builds have the same name")
        result = self.run_script(
            "gauntlet", "-Test", str(self.test_build), "-TestName", "Vice 1.1", "-Rounds", "5",
            "-EnginesJson", str(self.engines_json), "-Opponent", "Vice 1.1", *self.common(),
        )
        self.refused(result, "same name as the opponent 'Vice 1.1'")
        self.refused(
            self.gauntlet("-Opponent", "Vice 1.1;Vice 1.1"), "opponent 'Vice 1.1' is listed twice"
        )

    def test_build_names_must_be_plain_ascii(self):
        for name in ("Vyvræl dev", "Vyvrael=dev", "Vyvrael 'dev'"):
            result = self.run_script(
                "h2h", "-Test", str(self.test_build), "-TestName", name,
                "-Base", str(self.base_build), "-BaseName", "Vyvrael base", "-Rounds", "5",
                *self.common(),
            )
            self.refused(result, "-TestName")

    def test_gauntlet_opponents_must_be_known_and_present(self):
        self.refused(self.gauntlet("-Opponent", "Nobody 9"), 'no engine named "Nobody 9"',
                     "Stockfish 19-1500")
        self.refused(self.gauntlet("-Opponent", "Gone 1.0"), "opponent engine not found")
        self.refused(self.gauntlet(), "needs at least one -Opponent")
        result = self.run_script(
            "gauntlet", "-Test", str(self.test_build), "-TestName", "Vyvrael dev",
            "-Rounds", "5", "-Opponent", "Vice 1.1", *self.common(),
        )
        self.refused(result, "gauntlet mode needs the engine list")

    def test_concurrency_never_exceeds_the_physical_cores(self):
        self.assertIn("-concurrency 4", " ".join(self.command(self.sprt("-Concurrency", "4"))))
        self.refused(self.sprt("-Concurrency", "5"), "more than the 4 physical cores")
        self.refused(self.sprt("-Concurrency", "0"), "-Concurrency must be a whole number")

    def test_numbers_with_a_leading_zero_are_decimal(self):
        result = self.run_script(
            "h2h", *self.builds(), "-Rounds", "08", "-Hash", "064", "-Concurrency", "02",
            *self.common(),
        )
        joined = " ".join(self.command(result))
        self.assertIn("-rounds 8 ", joined)
        self.assertIn("option.Hash=64", joined)
        self.assertIn("-concurrency 2", joined)

    def test_a_bad_low_priority_setting_is_refused_unless_an_argument_replaces_it(self):
        env = {"VYVRAEL_LOW_PRIORITY": "yes"}
        self.refused(self.sprt(env=env), "LOW_PRIORITY must be true or false, got 'yes'")
        self.assertIn("BelowNormal", self.sprt("-LowPriority", env=env).stdout)
        result = self.sprt("-NoLowPriority", env=env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("BelowNormal", result.stdout)

    def test_openings_are_required(self):
        result = self.run_script(
            "h2h", *self.builds(), "-Rounds", "5", "-Cutechess", str(self.cutechess), "-DryRun"
        )
        self.refused(result, "no opening file")
        self.refused(self.sprt("-Openings", "nowhere.epd"), "opening file not found")

    def test_a_missing_cutechess_is_an_error_but_only_a_warning_in_a_dry_run(self):
        missing = str(self.tmp / "no-such-cutechess.exe")
        dry = self.sprt("-Cutechess", missing)
        self.assertEqual(dry.returncode, 0, dry.stderr)
        self.assertIn("warning: cutechess-cli not found", dry.stderr)
        real = self.run_script(
            "sprt", *self.builds(), *self.BOUNDS, "-Rounds", "5",
            "-Openings", str(self.openings), "-Cutechess", missing,
        )
        self.refused(real, "cutechess-cli not found", "docs/TESTING.md")

    def test_a_cutechess_without_a_needed_option_is_refused(self):
        old = self.fake_cutechess(
            [flag for flag in HELP_FLAGS if "-sprt" not in flag], "old-cli.ps1"
        )
        self.refused(self.sprt("-Cutechess", str(old)), "does not list the option -sprt")
        result = self.run_script(
            "h2h", *self.builds(), "-Rounds", "5", "-Openings", str(self.openings),
            "-Cutechess", str(old), "-DryRun",
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_bad_usage(self):
        self.refused(self.run_script("tournament"), "unknown mode 'tournament'")
        self.refused(self.sprt("-Opponent", "Vice 1.1"), "-Opponent is for gauntlet mode")
        result = self.run_script(
            "h2h", *self.builds(), "-Rounds", "5", "-Elo0", "0", *self.common()
        )
        self.refused(result, "are for sprt mode")
        self.refused(self.sprt("-Label", "a b"), "-Label may only hold")
        unknown = self.sprt("-Frobnicate")
        self.assertNotEqual(unknown.returncode, 0)
        self.assertIn("Frobnicate", unknown.stderr)
        usage = self.run_script("-Help")
        self.assertEqual(usage.returncode, 0)
        self.assertIn("match.ps1 sprt", usage.stdout)
        none = self.run_script()
        self.assertEqual(none.returncode, 2)
        self.assertIn("Usage:", none.stderr)

    def test_an_existing_pgn_is_not_added_to(self):
        out = self.tmp / "matches"
        out.mkdir()
        (out / "2026-01-02-h2h-Vyvrael-dev-abc1234.pgn").write_text("", encoding="utf-8")
        result = self.run_script(
            "h2h", *self.builds(), "-Rounds", "5", "-Openings", str(self.openings),
            "-Cutechess", str(self.cutechess),
        )
        self.refused(result, "already exists")


@unittest.skipUnless(POWERSHELL, "PowerShell is not available")
class MatchPs1RunTest(Fixture, unittest.TestCase):
    """What a real run leaves behind, with a stand-in that plays no game."""

    def play(self, script: str, *mode_args: str, env: dict | None = None):
        fake = self.fake_cutechess(HELP_FLAGS, "playing-cli.ps1", script)
        return self.run_script(
            *mode_args, "-Rounds", "5", "-Openings", str(self.openings),
            "-Cutechess", str(fake), env=env,
        )

    def sprt_run(self, script: str, env: dict | None = None):
        return self.play(script, "sprt", *self.builds(), *self.BOUNDS, env=env)

    # A stand-in's lines that write two games where -pgnout says.
    WRITE_PGN = (
        "$out = $args[[array]::IndexOf($args, '-pgnout') + 1]\n"
        "[IO.File]::WriteAllText($out, \"[Event `\"one`\"]`n`n1. e4 *`n`n"
        "[Event `\"two`\"]`n`n1. d4 *`n\")\n"
    )

    def test_notes_record_the_repository_the_games_and_the_pgn(self):
        result = self.sprt_run(
            self.WRITE_PGN
            + "'SPRT: llr 2.97 (100.8%), lbound -2.94, ubound 2.94 - H1 was accepted'\n"
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        notes = self.read(".notes.txt")
        # The repository the script came from: its commit, clean or dirty.
        self.assertRegex(notes, r"repository: [0-9a-f]{40} \((clean|dirty)\)\n")
        self.assertIn("engines:    no engine list used", notes)
        # At the end: the games in the PGN and its digest.
        pgn = self.tmp / "matches" / "2026-01-02-sprt-Vyvrael-dev-abc1234.pgn"
        self.assertIn("games:      2\n", notes)
        self.assertIn(f"pgn sha256: {hashlib.sha256(pgn.read_bytes()).hexdigest()}\n", notes)
        # Nothing but the run's own files was written in the output folder.
        self.assertEqual(
            sorted(p.name for p in (self.tmp / "matches").iterdir()),
            [f"2026-01-02-sprt-Vyvrael-dev-abc1234{s}" for s in (".log", ".notes.txt", ".pgn")],
        )

    def test_without_git_the_repository_is_unknown_and_no_pgn_is_said(self):
        result = self.sprt_run(
            "'nothing'", env={"VYVRAEL_GIT": str(self.tmp / "no-git.exe")}
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        notes = self.read(".notes.txt")
        self.assertIn("repository: unknown (no git, or not a git repository)\n", notes)
        self.assertIn("games:      0 (no PGN was written)\n", notes)
        self.assertIn("pgn sha256: none\n", notes)

    def test_notes_record_the_options_given(self):
        result = self.play(
            "'nothing'", "sprt", *self.builds(), *self.BOUNDS,
            "-TestOption", "UCI_LimitStrength=true;UCI_Elo=1500",
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(
            "options given (-TestOption, -BaseOption):\n"
            "  Vyvrael dev-abc1234: UCI_LimitStrength=true; UCI_Elo=1500\n"
            "  Vyvrael 0.1.0-a1: none\n",
            self.read(".notes.txt"),
        )

    def read(self, suffix: str) -> str:
        stem = "2026-01-02-sprt-Vyvrael-dev-abc1234"
        return (self.tmp / "matches" / (stem + suffix)).read_text(encoding="utf-8")

    def test_notes_log_and_an_accepted_sprt(self):
        result = self.sprt_run(
            "'Score of A vs B: 10 - 2 - 3'\n"
            "'SPRT: llr 1.20 (40.8%), lbound -2.94, ubound 2.94'\n"
            "'SPRT: llr 2.97 (100.8%), lbound -2.94, ubound 2.94 - H1 was accepted'\n"
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("Score of A vs B: 10 - 2 - 3", result.stdout)
        self.assertIn("Score of A vs B: 10 - 2 - 3", self.read(".log"))
        self.assertIn("SPRT final: SPRT: llr 2.97 (100.8%)", result.stdout)
        self.assertIn("SPRT outcome: H1 accepted", result.stdout)
        self.assertIn("LLR 2.97", result.stdout)
        notes = self.read(".notes.txt")
        for expected in (
            "mode:       sprt", "cutechess:  cutechess-cli 9.9.9", "Vyvrael dev-abc1234",
            "Vyvrael 0.1.0-a1", str(self.test_build), "-sprt elo0=0 elo1=5",
            "started:    20", "ended:      20", "status:     0", "outcome:    H1 accepted",
            f"openings:   {self.openings}",
            hashlib.sha256(b"test").hexdigest(), hashlib.sha256(b"base").hexdigest(),
        ):
            self.assertIn(expected, notes)

    def test_a_rejected_sprt(self):
        result = self.sprt_run(
            "'SPRT: llr -2.95 (-100.3%), lbound -2.94, ubound 2.94 - H0 was accepted'"
        )
        self.assertIn("SPRT outcome: H0 accepted", result.stdout)
        self.assertIn("outcome:    H0 accepted", self.read(".notes.txt"))

    def test_an_undecided_sprt(self):
        result = self.sprt_run("'SPRT: llr 0.51 (17.3%), lbound -2.94, ubound 2.94'")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("SPRT outcome: undecided: the round cap was reached first (LLR 0.51)",
                      result.stdout)

    def test_a_run_without_an_sprt_line_concludes_nothing(self):
        result = self.sprt_run("'nothing useful'")
        self.assertIn("SPRT outcome: no SPRT line in the log", result.stdout)

    def test_a_failing_cutechess_fails_the_script_and_is_recorded(self):
        result = self.sprt_run("'engine crashed'\nexit 3")
        self.assertEqual(result.returncode, 3, result.stdout + result.stderr)
        notes = self.read(".notes.txt")
        self.assertIn("status:     3", notes)
        self.assertIn("cutechess-cli failed with status 3", notes)

    def test_gauntlet_notes_list_every_opponent(self):
        result = self.play(
            "'done'", "gauntlet", "-Test", str(self.test_build), "-TestName", "Vyvrael 0.1.0-a1",
            "-EnginesJson", str(self.engines_json), "-Opponent", "Stockfish 19-1500;Vice 1.1",
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertNotIn("SPRT", result.stdout)
        notes = (self.tmp / "matches" / "2026-01-02-gauntlet-Vyvrael-0.1.0-a1.notes.txt"
                 ).read_text(encoding="utf-8")
        for expected in ("Stockfish 19-1500", "Vice 1.1", str(self.opponent),
                         hashlib.sha256(b"weak").hexdigest(), "outcome:    finished"):
            self.assertIn(expected, notes)
        # A byte-for-byte copy of the engine list sits beside the notes.
        copy = self.tmp / "matches" / "2026-01-02-gauntlet-Vyvrael-0.1.0-a1.engines.json"
        self.assertEqual(copy.read_bytes(), self.engines_json.read_bytes())
        self.assertIn(f"engines:    {self.engines_json} (copied to ", notes)


WINDOWS_POWERSHELL = shutil.which("powershell") if sys.platform == "win32" else None


@unittest.skipUnless(WINDOWS_POWERSHELL, "needs Windows PowerShell 5.1")
class MatchPs1StderrTest(Fixture, unittest.TestCase):
    """A real program that writes to stderr, under Windows PowerShell 5.1,
    which turns such lines into error records. cutechess-cli prints warnings
    there during ordinary matches; they must not end the run."""

    def test_warnings_on_stderr_do_not_end_the_match(self):
        (self.tmp / "help.txt").write_text(
            "".join(f"  {flag}\tsomething\n" for flag in HELP_FLAGS), encoding="ascii"
        )
        stub = self.tmp / "noisy-cli.cmd"
        stub.write_text(
            "@echo off\r\n"
            'if "%~1"=="-help" ( echo warning while helping 1>&2 & type "%~dp0help.txt" & exit /b 0 )\r\n'
            'if "%~1"=="-version" ( echo cutechess-cli 9.9.9 & exit /b 0 )\r\n'
            "echo Warning: Illegal PV move e2e4 from A 1>&2\r\n"
            "echo SPRT: llr 2.95 (100.3%%), lbound -2.94, ubound 2.94 - H1 was accepted\r\n"
            "exit /b 0\r\n",
            encoding="ascii",
        )
        environment = {
            key: value for key, value in os.environ.items() if not key.startswith("VYVRAEL_")
        }
        environment.update(
            VYVRAEL_LOCAL_ENV=str(self.tmp / "no-local.env"), VYVRAEL_MATCH_DATE="2026-01-02",
            VYVRAEL_PHYSICAL_CORES="4",
        )
        result = subprocess.run(
            [WINDOWS_POWERSHELL, "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
             "-File", str(SCRIPT), "sprt", *self.builds(), *self.BOUNDS, "-Rounds", "5",
             "-Openings", str(self.openings), "-Cutechess", str(stub)],
            capture_output=True, text=True, env=environment, cwd=self.tmp, timeout=180,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("Warning: Illegal PV move e2e4 from A", result.stdout)
        self.assertIn("SPRT outcome: H1 accepted", result.stdout)
        stem = self.tmp / "matches" / "2026-01-02-sprt-Vyvrael-dev-abc1234"
        self.assertIn("Illegal PV move", Path(str(stem) + ".log").read_text(encoding="utf-8"))
        notes = Path(str(stem) + ".notes.txt").read_text(encoding="utf-8")
        self.assertIn("status:     0", notes)
        self.assertIn("outcome:    H1 accepted", notes)


if __name__ == "__main__":
    unittest.main()
