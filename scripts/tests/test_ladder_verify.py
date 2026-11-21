# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Offline tests for scripts/ladder_verify.py.

The engines checked here are small Python programs written by the tests:
one that behaves, and several that fail in the ways a real engine can.

    python3 -m unittest discover -s scripts/tests
"""
from __future__ import annotations

import argparse
import io
import sys
import tempfile
import time
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import install_tools  # noqa: E402
import ladder_engines  # noqa: E402
import ladder_install  # noqa: E402
import ladder_verify  # noqa: E402

# A UCI engine in a few lines. BEHAVIOUR picks how it misbehaves.
ENGINE = r'''
import sys, time
behaviour = {behaviour!r}
log = open({log!r}, "a")
for line in sys.stdin:
    line = line.strip()
    log.write(line + "\n"); log.flush()
    if line == "uci":
        if behaviour == "silent":
            time.sleep(600)
        if behaviour == "dies":
            sys.exit(3)
        print("id name Fake"); print("option name Hash type spin default 1 min 1 max 9")
        print("uciok", flush=True)
    elif line == "isready":
        if behaviour == "never ready":
            time.sleep(600)
        print("readyok", flush=True)
    elif line.startswith("go"):
        if behaviour == "thinks for ever":
            time.sleep(600)
        print("info depth 6 score cp 20", flush=True)
        print("bestmove " + ("(none)" if behaviour == "no move" else "e2e4 ponder e7e5"),
              flush=True)
    elif line == "quit":
        if behaviour != "ignores quit":
            break
        time.sleep(600)
'''

QUICK = {"uciok": 3.0, "readyok": 1.0, "bestmove": 1.0}


class LadderVerifyTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name)
        self.log = self.tmp / "heard.txt"

    def engine(self, behaviour: str = "good") -> list[str]:
        script = self.tmp / f"{behaviour.replace(' ', '_')}.py"
        script.write_text(ENGINE.format(behaviour=behaviour, log=str(self.log)), encoding="utf-8")
        return [sys.executable, str(script)]

    def check(self, behaviour: str, options=(), timeouts=QUICK):
        return ladder_verify.check_engine(self.engine(behaviour), None, list(options), timeouts)

    def test_a_working_engine_passes_and_is_told_everything_in_order(self):
        options = [{"name": "Hash", "value": 64}, {"name": "UCI_LimitStrength", "value": True},
                   {"name": "UCI_Elo", "value": 1500}]
        self.assertEqual(self.check("good", options), (True, "bestmove e2e4"))
        self.assertEqual(self.log.read_text().splitlines(), [
            "uci", "setoption name Hash value 64",
            "setoption name UCI_LimitStrength value true", "setoption name UCI_Elo value 1500",
            "isready", "ucinewgame", "position startpos", "go depth 6", "quit",
        ])

    def test_each_way_of_failing_is_named_and_none_hangs(self):
        for behaviour, expected in (
            ("silent", "no `uciok` within 3 s"),
            ("dies", "the engine exited (exit status 3) before `uciok`"),
            ("never ready", "no `readyok` within 1 s"),
            ("thinks for ever", "no `bestmove` within 1 s"),
            ("no move", "answered `bestmove (none)`, which names no move"),
        ):
            started = time.monotonic()
            self.assertEqual(self.check(behaviour), (False, expected), behaviour)
            # The wait, then at most the five seconds given to `quit`.
            self.assertLess(time.monotonic() - started, 20, behaviour)

    def test_an_engine_that_ignores_quit_is_killed(self):
        started = time.monotonic()
        self.assertEqual(self.check("ignores quit"), (True, "bestmove e2e4"))
        self.assertLess(time.monotonic() - started, 20)

    def test_a_program_that_is_not_there(self):
        passed, detail = ladder_verify.check_engine(
            [str(self.tmp / "no-such-engine")], None, [], QUICK
        )
        self.assertFalse(passed)
        self.assertIn("could not be started", detail)

    def test_every_name_of_an_installed_engine_is_checked_and_reported(self):
        args = argparse.Namespace(prefix=str(self.tmp / "prefix"), bin_dir=None, engine=None,
                                  systemd=False, dry_run=False)
        ctx = install_tools.Ctx(args)
        # The Linux pins, whatever the machine running the tests is.
        ctx.system, ctx.engine_build = "linux", "avx2"
        table = [
            {"name": "Good 1", "binary": "good", "hash": True, "threads": False,
             "limit": (1000, 2000), "variants": [1500]},
            {"name": "Broken 2", "binary": "broken", "hash": False, "threads": False,
             "limit": None},
            {"name": "Absent 3", "binary": "absent", "hash": False, "threads": False,
             "limit": None},
        ]
        commands = {"good": self.engine("good"), "broken": self.engine("dies")}

        def fake_check(command, cwd, options, timeouts=None):
            return ladder_verify_check(commands[Path(command[0]).name], None, options, QUICK)

        ladder_verify_check = ladder_verify.check_engine
        out = io.StringIO()
        with mock.patch.object(ladder_engines, "ENGINES", table), \
                mock.patch.object(ladder_install, "is_installed",
                                  lambda ctx, engine: engine["name"] != "Absent 3"), \
                mock.patch.object(ladder_verify, "check_engine", fake_check), \
                redirect_stdout(out):
            failed = ladder_verify.verify_engines(ctx, None)
        self.assertEqual(failed, ["Broken 2"])
        self.assertEqual(out.getvalue().splitlines(), [
            "PASS  Good 1: bestmove e2e4",
            "PASS  Good 1-1500: bestmove e2e4",
            "FAIL  Broken 2: the engine exited (exit status 3) before `uciok`",
            "SKIP  Absent 3: not installed",
        ])
        # The limited name was checked with its own options.
        heard = self.log.read_text()
        self.assertIn("setoption name UCI_LimitStrength value false", heard)
        self.assertIn("setoption name UCI_Elo value 1500", heard)


if __name__ == "__main__":
    unittest.main()
