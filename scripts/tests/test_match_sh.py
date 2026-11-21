# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Offline tests for scripts/linux/match.sh.

No game is played and the real cutechess-cli is never run: the tests give
the script a stand-in that only answers `-help`, and read the command the
script would run from `--dry-run`. Everything lives in a temporary directory.
They are skipped where there is no bash.

    python3 -m unittest discover -s scripts/tests
"""
from __future__ import annotations

import hashlib
import json
import os
import shlex
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "linux" / "match.sh"

# Every option of cutechess-cli the script may use, as its -help lists them.
HELP_FLAGS = [
    "-engine OPTIONS", "-each OPTIONS", "-concurrency N", "-draw movenumber=NUMBER",
    "-resign movecount=COUNT", "-tournament TYPE", "-games N", "-rounds N",
    "-sprt elo0=ELO0", "-ratinginterval N", "-openings file=FILE", "-pgnout FILE",
    "-recover", "-repeat [N]", "-seeds N", "-srand N",
]


def find_bash() -> str | None:
    bash = shutil.which("bash")
    if not bash:
        return None
    try:
        probe = subprocess.run(
            [bash, "-c", "echo ok"], capture_output=True, text=True, timeout=30
        )
    except (OSError, subprocess.SubprocessError):
        return None
    return bash if probe.stdout.strip() == "ok" else None


BASH = find_bash()


def script_file(path: Path, text: str) -> Path:
    """Writes an executable shell script."""
    path.write_text("#!/bin/sh\n" + text, encoding="utf-8", newline="\n")
    path.chmod(path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return path


class Fixture:
    """Stand-in builds, openings, engine list and cutechess-cli in a temporary
    directory, and the helpers to run the script against them."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name)
        self.test_build = script_file(self.tmp / "vyvrael-test", "exit 0\n")
        self.base_build = script_file(self.tmp / "vyvrael-base", "exit 0\n")
        self.openings = self.tmp / "openings.epd"
        self.openings.write_text("8/8/8/8/8/8/8/K6k w - - 0 1\n", encoding="utf-8")
        self.cutechess = self.fake_cutechess(HELP_FLAGS)
        # Four physical cores on two threads each, whatever the real machine.
        bin_dir = self.tmp / "bin"
        bin_dir.mkdir()
        cores = "".join(f"{core},0\\n" for core in (0, 1, 2, 3, 0, 1, 2, 3))
        script_file(bin_dir / "lscpu", f"printf '# core,socket\\n{cores}'\n")
        self.bin_dir = bin_dir
        self.opponent = script_file(self.tmp / "weak engine", "exit 0\n")
        self.engines_json = self.tmp / "engines.json"
        self.engines_json.write_text(json.dumps([
            {
                "name": "Stockfish 19-1500",
                "command": self.opponent.as_posix(),
                "protocol": "uci",
                "workingDirectory": self.tmp.as_posix(),
                "options": [
                    {"name": "Hash", "value": 16},
                    {"name": "Threads", "value": 1},
                    {"name": "UCI_LimitStrength", "value": True},
                    {"name": "UCI_Elo", "value": 1500},
                ],
            },
            {"name": "Vice 1.1", "command": self.opponent.as_posix(), "protocol": "uci"},
            {"name": "Gone 1.0", "command": (self.tmp / "missing").as_posix()},
        ]), encoding="utf-8")

    def fake_cutechess(
        self, flags: list[str], name: str = "cutechess-cli", play: str = "exit 0\n"
    ) -> Path:
        """A stand-in that answers -help and -version; anything else runs `play`."""
        lines = "".join(f"  {flag}\\tsomething\\n" for flag in flags)
        return script_file(
            self.tmp / name,
            f"case \"$1\" in\n-help) printf '{lines}'; exit 0;;\n"
            f"-version) echo 'cutechess-cli 9.9.9'; echo 'more'; exit 0;;\nesac\n{play}",
        )

    def run_script(self, *args: str, env: dict | None = None):
        environment = {
            key: value for key, value in os.environ.items() if not key.startswith("VYVRAEL_")
        }
        environment["PATH"] = f"{self.bin_dir.as_posix()}{os.pathsep}{environment['PATH']}"
        environment["VYVRAEL_LOCAL_ENV"] = (self.tmp / "no-local.env").as_posix()
        environment["VYVRAEL_PYTHON"] = Path(sys.executable).as_posix()
        environment["VYVRAEL_MATCH_DATE"] = "2026-01-02"
        environment.update(env or {})
        return subprocess.run(
            [BASH, SCRIPT.as_posix(), *args],
            capture_output=True, text=True, env=environment, cwd=self.tmp, timeout=120,
        )

    def builds(self) -> list[str]:
        return [
            "--test", self.test_build.as_posix(), "--test-name", "Vyvrael dev-abc1234",
            "--base", self.base_build.as_posix(), "--base-name", "Vyvrael 0.1.0-a1",
        ]

    def common(self) -> list[str]:
        return [
            "--openings", self.openings.as_posix(),
            "--cutechess", self.cutechess.as_posix(), "--dry-run",
        ]

    def sprt(self, *extra: str, **kwargs):
        bounds = ["--elo0", "0", "--elo1", "5", "--alpha", "0.05", "--beta", "0.05"]
        return self.run_script(
            "sprt", *self.builds(), *bounds, "--rounds", "500", *self.common(), *extra, **kwargs
        )

    def command(self, result) -> list[str]:
        self.assertEqual(result.returncode, 0, result.stderr)
        return shlex.split(result.stdout)

    def refused(self, result, *fragments: str):
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertEqual(result.stdout, "")
        for fragment in fragments:
            self.assertIn(fragment, result.stderr)



@unittest.skipUnless(BASH, "bash is not available")
class MatchShTest(Fixture, unittest.TestCase):
    """The command the script would run, read from --dry-run, and what it
    refuses."""

    # --- the command each mode builds ---------------------------------------

    def test_sprt_command(self):
        words = self.command(self.sprt())
        self.assertEqual(words[0], self.cutechess.as_posix())
        joined = " ".join(words)
        self.assertIn("-sprt elo0=0 elo1=5 alpha=0.05 beta=0.05", joined)
        self.assertIn("-each tc=10+0.1 option.Hash=64", joined)
        self.assertIn(f"-openings file={self.openings.as_posix()} format=epd order=random", joined)
        self.assertIn("-games 2 -repeat -rounds 500", joined)
        self.assertIn("-resign movecount=3 score=700 twosided=true", joined)
        self.assertIn("-draw movenumber=34 movecount=10 score=20", joined)
        self.assertIn("-recover", words)
        self.assertIn("-ratinginterval 10", joined)
        self.assertNotIn("-tournament", words)
        self.assertNotIn("-srand", words)
        self.assertNotIn("nice", words)
        # Two engines, each one argument per setting, with the name intact.
        self.assertEqual(words.count("-engine"), 2)
        first = words.index("-engine")
        self.assertEqual(
            words[first + 1 : first + 5],
            ["name=Vyvrael dev-abc1234", f"cmd={self.test_build.as_posix()}", "proto=uci",
             "option.Threads=1"],
        )
        self.assertIn("name=Vyvrael 0.1.0-a1", words)
        self.assertEqual(words[-2:], ["-pgnout", "matches/2026-01-02-sprt-Vyvrael-dev-abc1234.pgn"])

    def test_head_to_head_has_no_sprt(self):
        result = self.run_script("h2h", *self.builds(), "--rounds", "50", *self.common())
        words = self.command(result)
        self.assertNotIn("-sprt", words)
        self.assertIn("-rounds 50", " ".join(words))
        self.assertEqual(words[-1], "matches/2026-01-02-h2h-Vyvrael-dev-abc1234.pgn")

    def gauntlet(self, *extra: str, **kwargs):
        return self.run_script(
            "gauntlet", "--test", self.test_build.as_posix(), "--test-name", "Vyvrael 0.1.0-a1",
            "--rounds", "20", "--engines-json", self.engines_json.as_posix(),
            *self.common(), *extra, **kwargs,
        )

    def test_gauntlet_takes_opponents_from_the_engine_list(self):
        words = self.command(self.gauntlet(
            "--opponent", "Stockfish 19-1500", "--opponent", "Vice 1.1"
        ))
        joined = " ".join(words)
        self.assertIn("-tournament gauntlet -seeds 1", joined)
        self.assertEqual(words.count("-engine"), 3)
        # The build under test comes first: it is the seed.
        self.assertEqual(words[words.index("-engine") + 1], "name=Vyvrael 0.1.0-a1")
        second = [index for index, word in enumerate(words) if word == "-engine"][1]
        self.assertEqual(
            words[second + 1 : second + 8],
            ["name=Stockfish 19-1500", f"cmd={self.opponent.as_posix()}", "proto=uci",
             f"dir={self.tmp.as_posix()}", "option.Threads=1",
             "option.UCI_LimitStrength=true", "option.UCI_Elo=1500"],
        )
        # The list's own Hash is left out: -each sets one size for everyone.
        self.assertNotIn("option.Hash=16", words)
        self.assertIn("name=Vice 1.1", words)

    def test_optional_settings(self):
        words = self.command(self.sprt(
            "--tc", "60+0.6", "--hash", "128", "--concurrency", "2", "--seed", "7",
            "--deterministic", "--low-priority", "--label", "run2", "--out-dir", "out dir",
        ))
        joined = " ".join(words)
        self.assertEqual(words[:3], ["nice", "-n", "10"])
        self.assertIn("-each tc=60+0.6 option.Hash=128", joined)
        self.assertIn("-concurrency 2", joined)
        self.assertIn("-srand 7", joined)
        # Both builds get a fixed Seed and no Variety; the old option is gone.
        self.assertEqual(words.count("option.Seed=822286742"), 2)
        self.assertEqual(words.count("option.Variety=0"), 2)
        self.assertNotIn("DeterministicRNG", joined)
        self.assertEqual(words[-1], "out dir/2026-01-02-sprt-Vyvrael-dev-abc1234-run2.pgn")

    # --- settings from the environment and the local file -------------------

    def test_default_concurrency_is_physical_cores_minus_two(self):
        self.assertIn("-concurrency 2", " ".join(self.command(self.sprt())))

    def test_settings_come_from_the_local_file_then_environment_then_arguments(self):
        local = self.tmp / "local.env"
        local.write_text(
            "# a comment\n\nCONCURRENCY=3\nLOW_PRIORITY=true\n"
            f"OPENINGS={self.openings.as_posix()}\n"
            f"CUTECHESS_CLI={self.cutechess.as_posix()}\nMATCH_DIR=from file\n",
            encoding="utf-8",
        )
        env = {"VYVRAEL_LOCAL_ENV": local.as_posix()}
        bounds = ["--elo0", "0", "--elo1", "5", "--alpha", "0.05", "--beta", "0.05"]
        base = ["sprt", *self.builds(), *bounds, "--rounds", "9", "--dry-run"]
        words = self.command(self.run_script(*base, env=env))
        self.assertEqual(words[:3], ["nice", "-n", "10"])
        self.assertIn("-concurrency 3", " ".join(words))
        self.assertTrue(words[-1].startswith("from file/"))
        # The environment beats the file, and an argument beats both.
        env["VYVRAEL_CONCURRENCY"] = "4"
        self.assertIn("-concurrency 4", " ".join(self.command(self.run_script(*base, env=env))))
        words = self.command(self.run_script(
            *base, "--concurrency", "1", "--no-low-priority", env=env
        ))
        self.assertIn("-concurrency 1", " ".join(words))
        self.assertNotIn("nice", words)

    # --- refusals -----------------------------------------------------------

    def test_sprt_bounds_have_no_defaults(self):
        for missing in ("--elo0", "--elo1", "--alpha", "--beta"):
            bounds = {"--elo0": "0", "--elo1": "5", "--alpha": "0.05", "--beta": "0.05"}
            del bounds[missing]
            flat = [word for pair in bounds.items() for word in pair]
            result = self.run_script(
                "sprt", *self.builds(), *flat, "--rounds", "10", *self.common()
            )
            self.refused(result, f"{missing} is required in sprt mode")
        self.refused(self.sprt("--elo1", "five"), "--elo1 must be a number")

    def test_a_round_cap_is_required(self):
        result = self.run_script("h2h", *self.builds(), *self.common())
        self.refused(result, "--rounds is required")
        for bad in ("0", "-3", "many", "1.5"):
            result = self.run_script("h2h", *self.builds(), "--rounds", bad, *self.common())
            self.refused(result, "--rounds must be a whole number above zero")

    def test_missing_engines_are_refused(self):
        self.test_build.unlink()
        self.refused(self.sprt(), "the test build (--test) not found")
        script_file(self.test_build, "exit 0\n")
        self.base_build.unlink()
        self.refused(self.sprt(), "the base build (--base) not found")

    def test_name_collisions_are_refused(self):
        result = self.run_script(
            "h2h", "--test", self.test_build.as_posix(), "--test-name", "Vyvrael dev",
            "--base", self.base_build.as_posix(), "--base-name", "Vyvrael dev",
            "--rounds", "5", *self.common(),
        )
        self.refused(result, "the two builds have the same name")
        result = self.run_script(
            "gauntlet", "--test", self.test_build.as_posix(), "--test-name", "Vice 1.1",
            "--rounds", "5", "--engines-json", self.engines_json.as_posix(),
            "--opponent", "Vice 1.1", *self.common(),
        )
        self.refused(result, "same name as the opponent 'Vice 1.1'")
        self.refused(
            self.gauntlet("--opponent", "Vice 1.1", "--opponent", "Vice 1.1"),
            "opponent 'Vice 1.1' is listed twice",
        )

    def test_build_names_must_be_plain_ascii(self):
        for name in ("Vyvræl dev", 'Vyvrael "dev"', "Vyvrael=dev", " Vyvrael"):
            result = self.run_script(
                "h2h", "--test", self.test_build.as_posix(), "--test-name", name,
                "--base", self.base_build.as_posix(), "--base-name", "Vyvrael base",
                "--rounds", "5", *self.common(),
            )
            self.refused(result, "--test-name")

    def test_plain_names_are_accepted_whatever_the_locale(self):
        # On Linux a UTF-8 locale made the name check refuse plain ASCII.
        names = ["Vyvrael 0.1.0-a1", "Vyvrael dev-1a2b3c4", "A", "Vyvrael (test) ~x #2"]
        for locale in ("C", "C.UTF-8", "en_US.UTF-8", "en_AU.UTF-8", "de_DE.UTF-8"):
            env = {"LC_ALL": locale, "LANG": locale}
            for name in names:
                result = self.run_script(
                    "h2h", "--test", self.test_build.as_posix(), "--test-name", name,
                    "--base", self.base_build.as_posix(), "--base-name", "Vyvrael base",
                    "--rounds", "5", *self.common(), env=env,
                )
                self.assertEqual(result.returncode, 0, f"{locale} {name!r}: {result.stderr}")
                self.assertIn(f"name={name}", shlex.split(result.stdout))
            result = self.run_script(
                "h2h", "--test", self.test_build.as_posix(), "--test-name", "Vyvræl dev",
                "--base", self.base_build.as_posix(), "--base-name", "Vyvrael base",
                "--rounds", "5", *self.common(), env=env,
            )
            self.refused(result, "must be plain ASCII")

    def test_gauntlet_opponents_must_be_known_and_present(self):
        self.refused(self.gauntlet("--opponent", "Nobody 9"), 'no engine named "Nobody 9"',
                     "Stockfish 19-1500")
        self.refused(self.gauntlet("--opponent", "Gone 1.0"), "opponent engine not found")
        self.refused(self.gauntlet(), "needs at least one --opponent")
        result = self.run_script(
            "gauntlet", "--test", self.test_build.as_posix(), "--test-name", "Vyvrael dev",
            "--rounds", "5", "--opponent", "Vice 1.1", *self.common(),
        )
        self.refused(result, "gauntlet mode needs the engine list")

    def test_concurrency_never_exceeds_the_physical_cores(self):
        self.assertIn("-concurrency 4", " ".join(self.command(self.sprt("--concurrency", "4"))))
        self.refused(self.sprt("--concurrency", "5"), "more than the 4 physical cores")
        self.refused(self.sprt("--concurrency", "0"), "--concurrency must be a whole number")

    def test_the_core_count_can_be_given(self):
        env = {"VYVRAEL_PHYSICAL_CORES": "8"}
        self.assertIn("-concurrency 6", " ".join(self.command(self.sprt(env=env))))
        self.refused(self.sprt("--concurrency", "9", env=env), "more than the 8 physical cores")

    def test_an_lscpu_that_fails_means_the_cores_are_unknown(self):
        script_file(self.bin_dir / "lscpu", "exit 1\n")
        words = self.command(self.sprt())
        # Unknown cores: one game at a time by default, and no limit to check.
        self.assertIn("-concurrency 1", " ".join(words))
        self.assertIn("-concurrency 9", " ".join(self.command(self.sprt("--concurrency", "9"))))

    def test_a_byte_order_mark_in_the_local_file_is_ignored(self):
        local = self.tmp / "local.env"
        local.write_bytes(b"\xef\xbb\xbfCONCURRENCY=3\r\nMATCH_DIR=from file\r\n")
        words = self.command(self.sprt(env={"VYVRAEL_LOCAL_ENV": local.as_posix()}))
        self.assertIn("-concurrency 3", " ".join(words))
        self.assertTrue(words[-1].startswith("from file/"))

    def test_numbers_with_a_leading_zero_are_decimal(self):
        result = self.run_script(
            "h2h", *self.builds(), "--rounds", "08", "--hash", "064", "--concurrency", "02",
            *self.common(),
        )
        self.assertEqual(result.stderr, "")
        joined = " ".join(self.command(result))
        self.assertIn("-rounds 8 ", joined)
        self.assertIn("option.Hash=64", joined)
        self.assertIn("-concurrency 2", joined)

    def test_a_bad_low_priority_setting_is_refused_unless_an_argument_replaces_it(self):
        env = {"VYVRAEL_LOW_PRIORITY": "yes"}
        self.refused(self.sprt(env=env), "LOW_PRIORITY must be true or false, got 'yes'")
        self.assertEqual(self.command(self.sprt("--low-priority", env=env))[0], "nice")
        self.assertNotIn("nice", self.command(self.sprt("--no-low-priority", env=env)))

    def test_openings_are_required(self):
        result = self.run_script(
            "h2h", *self.builds(), "--rounds", "5", "--cutechess", self.cutechess.as_posix(),
            "--dry-run",
        )
        self.refused(result, "no opening file")
        self.refused(self.sprt("--openings", "nowhere.epd"), "opening file not found")

    def test_a_missing_cutechess_is_an_error_but_only_a_warning_in_a_dry_run(self):
        missing = (self.tmp / "no-such-cutechess").as_posix()
        dry = self.sprt("--cutechess", missing)
        self.assertEqual(dry.returncode, 0, dry.stderr)
        self.assertIn("warning: cutechess-cli not found", dry.stderr)
        self.assertEqual(shlex.split(dry.stdout)[0], missing)
        bounds = ["--elo0", "0", "--elo1", "5", "--alpha", "0.05", "--beta", "0.05"]
        real = self.run_script(
            "sprt", *self.builds(), *bounds, "--rounds", "5",
            "--openings", self.openings.as_posix(), "--cutechess", missing,
        )
        self.refused(real, "cutechess-cli not found", "docs/TESTING.md")

    def test_a_cutechess_without_a_needed_option_is_refused(self):
        old = self.fake_cutechess([flag for flag in HELP_FLAGS if "-sprt" not in flag], "old-cli")
        self.refused(self.sprt("--cutechess", old.as_posix()), "does not list the option -sprt")
        # The same tool is fine for a mode that does not use that option.
        result = self.run_script(
            "h2h", *self.builds(), "--rounds", "5", "--openings", self.openings.as_posix(),
            "--cutechess", old.as_posix(), "--dry-run",
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_bad_usage(self):
        self.refused(self.run_script("tournament"), "unknown mode 'tournament'")
        self.refused(self.sprt("--frobnicate"), "unknown option '--frobnicate'")
        self.refused(self.sprt("--tc"), "--tc needs a value")
        self.refused(self.sprt("--opponent", "Vice 1.1"), "--opponent is for gauntlet mode")
        result = self.run_script(
            "h2h", *self.builds(), "--rounds", "5", "--elo0", "0", *self.common()
        )
        self.refused(result, "are for sprt mode")
        self.refused(self.sprt("--label", "a b"), "--label may only hold")
        usage = self.run_script("--help")
        self.assertEqual(usage.returncode, 0)
        self.assertIn("match.sh sprt", usage.stdout)
        none = self.run_script()
        self.assertEqual(none.returncode, 2)
        self.assertIn("Usage:", none.stderr)

    def test_an_existing_pgn_is_not_added_to(self):
        out = self.tmp / "matches"
        out.mkdir()
        (out / "2026-01-02-h2h-Vyvrael-dev-abc1234.pgn").write_text("", encoding="utf-8")
        result = self.run_script(
            "h2h", *self.builds(), "--rounds", "5", "--openings", self.openings.as_posix(),
            "--cutechess", self.cutechess.as_posix(),
        )
        self.refused(result, "already exists")


@unittest.skipUnless(BASH, "bash is not available")
class MatchShRunTest(Fixture, unittest.TestCase):
    """What a real run leaves behind, with a stand-in that plays no game."""

    def play(self, script: str, *mode_args: str):
        fake = self.fake_cutechess(HELP_FLAGS, "playing-cli", script)
        return self.run_script(
            *mode_args, "--rounds", "5", "--openings", self.openings.as_posix(),
            "--cutechess", fake.as_posix(),
        )

    def sprt_run(self, script: str):
        bounds = ["--elo0", "0", "--elo1", "5", "--alpha", "0.05", "--beta", "0.05"]
        return self.play(script, "sprt", *self.builds(), *bounds)

    def read(self, suffix: str) -> str:
        stem = "2026-01-02-sprt-Vyvrael-dev-abc1234"
        return (self.tmp / "matches" / (stem + suffix)).read_text(encoding="utf-8")

    def test_notes_log_and_an_accepted_sprt(self):
        result = self.sprt_run(
            "echo 'Score of A vs B: 10 - 2 - 3'\n"
            "echo 'SPRT: llr 1.20 (40.8%), lbound -2.94, ubound 2.94'\n"
            "echo 'SPRT: llr 2.97 (100.8%), lbound -2.94, ubound 2.94 - H1 was accepted'\n"
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        # The progress is shown as it comes and kept in the log.
        self.assertIn("Score of A vs B: 10 - 2 - 3", result.stdout)
        self.assertIn("Score of A vs B: 10 - 2 - 3", self.read(".log"))
        # The last SPRT line decides, and the outcome is said in words.
        self.assertIn("SPRT final: SPRT: llr 2.97 (100.8%)", result.stdout)
        self.assertIn("SPRT outcome: H1 accepted", result.stdout)
        self.assertIn("LLR 2.97", result.stdout)
        notes = self.read(".notes.txt")
        for expected in (
            "mode:       sprt", "cutechess:  cutechess-cli 9.9.9", "Vyvrael dev-abc1234",
            "Vyvrael 0.1.0-a1", self.test_build.as_posix(), "-sprt elo0=0 elo1=5",
            "started:    20", "ended:      20", "status:     0", "outcome:    H1 accepted",
            f"openings:   {self.openings.as_posix()}",
        ):
            self.assertIn(expected, notes)
        self.assertIn(hashlib.sha256(self.test_build.read_bytes()).hexdigest(), notes)

    def test_a_rejected_sprt(self):
        result = self.sprt_run(
            "echo 'SPRT: llr -2.95 (-100.3%), lbound -2.94, ubound 2.94 - H0 was accepted'\n"
        )
        self.assertIn("SPRT outcome: H0 accepted", result.stdout)
        self.assertIn("outcome:    H0 accepted", self.read(".notes.txt"))

    def test_an_undecided_sprt(self):
        result = self.sprt_run("echo 'SPRT: llr 0.51 (17.3%), lbound -2.94, ubound 2.94'\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("SPRT outcome: undecided: the round cap was reached first (LLR 0.51)",
                      result.stdout)

    def test_a_run_without_an_sprt_line_concludes_nothing(self):
        result = self.sprt_run("echo 'nothing useful'\n")
        self.assertIn("SPRT outcome: no SPRT line in the log", result.stdout)

    def test_a_failing_cutechess_fails_the_script_and_is_recorded(self):
        result = self.sprt_run("echo 'engine crashed'\nexit 3\n")
        self.assertEqual(result.returncode, 3, result.stderr)
        notes = self.read(".notes.txt")
        self.assertIn("status:     3", notes)
        self.assertIn("cutechess-cli failed with status 3", notes)

    def test_gauntlet_notes_list_every_opponent(self):
        result = self.play(
            "echo done\n", "gauntlet", "--test", self.test_build.as_posix(),
            "--test-name", "Vyvrael 0.1.0-a1", "--engines-json", self.engines_json.as_posix(),
            "--opponent", "Stockfish 19-1500", "--opponent", "Vice 1.1",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("SPRT", result.stdout)
        notes = (self.tmp / "matches" / "2026-01-02-gauntlet-Vyvrael-0.1.0-a1.notes.txt"
                 ).read_text(encoding="utf-8")
        for expected in ("Stockfish 19-1500", "Vice 1.1", self.opponent.as_posix(),
                         "outcome:    finished"):
            self.assertIn(expected, notes)


if __name__ == "__main__":
    unittest.main()
