# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Offline tests for scripts/ladder_engines.py: the pins of the ladder's
opponent engines. Nothing is downloaded.

    python3 -m unittest discover -s scripts/tests
"""
from __future__ import annotations

import csv
import re
import sys
import unittest
from pathlib import Path, PurePosixPath

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ladder_engines  # noqa: E402

BASELINE = Path(__file__).resolve().parents[2] / "docs/testing/ladder-Baseline-2-ratings.csv"


def baseline_names() -> list[str]:
    """The names in the ratings file of Ladder Baseline 2, the reference
    ladder. Names are compared without surrounding spaces: in Ladder
    Baseline 1 "Stockfish 19-2400 " carried a trailing one, which the
    maintainer dropped on 2026-11-21."""
    with open(BASELINE, encoding="utf-8", newline="") as handle:
        return [row["PLAYER"].strip() for row in csv.DictReader(handle)]


def all_pins() -> list[dict]:
    """Every pin the table gives: each system, each build."""
    return [pin for system in ladder_engines.SYSTEMS for build in ladder_engines.BUILDS
            for pin in ladder_engines.pins(system, build)]


class LadderEnginesTest(unittest.TestCase):
    def test_the_names_are_exactly_those_of_the_baseline(self):
        baseline = baseline_names()
        self.assertEqual(len(baseline), 30)
        provided = ladder_engines.ladder_names()
        left_out = list(ladder_engines.NOT_INSTALLABLE["linux"])
        retired = list(ladder_engines.RETIRED)
        self.assertEqual(len(set(provided)), len(provided), "a name is given twice")
        new = list(ladder_engines.NEW_IN_BASELINE_3)
        # Every name of the baseline is either provided, listed as left out
        # with a reason or retired with a reason; besides them only the
        # engines new in Ladder Baseline 3 are provided: no invented engines.
        self.assertEqual(sorted(provided + left_out + retired), sorted(baseline + new))
        self.assertEqual(len(new), 10)
        self.assertFalse(set(new) & set(baseline))
        self.assertEqual(left_out, ["Orion 1.1", "Ethereal 12.75", "Nostradamus 0.4"])
        self.assertEqual(retired, ["Arasan 26.1-1800"])
        for reason in [*ladder_engines.NOT_INSTALLABLE["linux"].values(),
                       *ladder_engines.RETIRED.values()]:
            self.assertGreater(len(reason), 40)
        # No generated name carries a stray space.
        for name in provided:
            self.assertEqual(name, name.strip())

    def test_windows_provides_every_name_not_retired(self):
        provided = ladder_engines.ladder_names("windows")
        self.assertEqual(len(set(provided)), len(provided), "a name is given twice")
        self.assertEqual(sorted(provided + list(ladder_engines.RETIRED)),
                         sorted(baseline_names() + ladder_engines.NEW_IN_BASELINE_3))
        self.assertEqual(ladder_engines.NOT_INSTALLABLE["windows"], {})
        self.assertEqual(len(ladder_engines.pins("windows")), 27)
        # Windows takes official programs only; nothing is built there.
        for engine in ladder_engines.pins("windows"):
            self.assertEqual(engine["method"], "download", engine["name"])
            self.assertNotIn("build", engine, engine["name"])
            self.assertTrue(engine["binary"].endswith(".exe"), engine["name"])

    def test_an_engine_missing_on_a_system_says_why(self):
        for system in ladder_engines.SYSTEMS:
            pinned = {p["name"] for p in ladder_engines.pins(system)}
            for engine in ladder_engines.ENGINES:
                if engine["name"] not in pinned:
                    self.assertIn(engine["name"], ladder_engines.NOT_INSTALLABLE[system])
            for name in ladder_engines.NOT_INSTALLABLE[system]:
                self.assertNotIn(name, pinned, f"{name} on {system}")

    def test_the_limited_entries_are_variants_not_binaries(self):
        binaries = [engine["name"] for engine in ladder_engines.pins("linux")]
        self.assertEqual(len(binaries), 24)
        for name in binaries:
            self.assertIsNone(re.search(r"-\d{4}$", name), name)
        variants = {
            engine["name"]: engine.get("variants", []) for engine in ladder_engines.pins("linux")
        }
        self.assertEqual(variants["Stockfish 19"], [1320, 1500, 1800, 2000, 2200, 2400])
        self.assertEqual(variants["Arasan 26.1"], [1500])
        self.assertEqual(variants["Rodent IV"], [1600, 2000])
        self.assertEqual(variants["CT800 1.46"], [1200, 1500, 1800])
        self.assertEqual(sum(len(elos) for elos in variants.values()), 12)

    def test_a_variant_needs_an_engine_that_can_be_limited_that_far(self):
        for engine in all_pins():
            variants = engine.get("variants", [])
            if not variants:
                continue
            self.assertIsNotNone(engine["limit"], engine["name"])
            lowest, highest = engine["limit"]
            for elo in variants:
                self.assertTrue(lowest <= elo <= highest, f"{engine['name']}-{elo}")

    def test_every_source_is_https_and_pinned(self):
        for engine in all_pins():
            name = engine["name"]
            if engine.get("http_reason"):
                # Plain HTTP only where the project serves nothing else, and
                # then the pin says why; the digest below is checked anyway.
                self.assertTrue(engine["url"].startswith(("https://", "http://")), name)
                self.assertGreater(len(engine["http_reason"]), 40, name)
            else:
                self.assertTrue(engine["url"].startswith("https://"), name)
            web = ("https://", "http://") if engine.get("http_reason") else ("https://",)
            self.assertTrue(engine["source"].startswith(web), name)
            self.assertIn(engine["method"], ("download", "build"), name)
            self.assertIn(engine["archive"], ("none", "tar.gz", "zip", "7z"), name)
            self.assertRegex(engine["licence"],
                             r"^(GPL-[23]\.0(-or-later)?|AGPL-3\.0|MIT|MPL-2\.0|Freeware)$", name)
            if engine["licence"] == "Freeware":
                # Closed source: the notes must say what its terms allow.
                self.assertIn("redistribution", engine["notes"], name)
            self.assertGreater(len(engine["notes"]), 10, name)
            # Nothing is fetched without a digest, source archives included.
            self.assertRegex(engine["sha256"] or "", r"^[0-9a-f]{64}$", name)
            if "member_sha256" in engine:
                self.assertRegex(engine["member_sha256"], r"^[0-9a-f]{64}$", name)
            # A download that is the binary itself must have a digest.
            if engine["method"] == "download":
                self.assertIsNotNone(engine["sha256"], name)

    def test_paths_stay_inside_the_install(self):
        for engine in all_pins():
            paths = [engine["binary"], *engine.get("beside", [])]
            if "build" in engine:
                paths.append(engine["build"]["dir"])
            for text in paths:
                path = PurePosixPath(text)
                self.assertFalse(path.is_absolute(), text)
                self.assertNotIn("..", path.parts, text)

    def test_nothing_is_built_for_this_machine_or_for_avx512(self):
        for engine in all_pins():
            text = " ".join(
                [engine["url"], engine["binary"]]
                + engine.get("build", {}).get("command", [])
                + list(engine.get("build", {}).get("env", {}).values())
            ).lower()
            for banned in ("native", "avx512", "avx-512", "x86-64-v4", "znver", "skylake-avx"):
                self.assertNotIn(banned, text, engine["name"])

    def test_builds_say_what_they_need(self):
        for engine in all_pins():
            if engine["method"] != "build":
                self.assertNotIn("build", engine, engine["name"])
                continue
            build = engine["build"]
            self.assertTrue(build["command"], engine["name"])
            self.assertTrue(build["tools"], engine["name"])
            self.assertTrue(all(isinstance(word, str) for word in build["command"]))
            # A compiled engine names its architecture, unless its own
            # makefile names none (Vice, plain x86-64).
            flags = " ".join(build["command"] + list(build.get("env", {}).values()))
            if engine["name"] != "Vice 1.1":
                self.assertRegex(flags, r"haswell|x86-64-v3", engine["name"])

    def test_fixed_options_do_not_repeat_the_ones_set_for_every_engine(self):
        for engine in all_pins():
            names = [name for name, _ in engine.get("options", [])]
            self.assertEqual(len(set(names)), len(names), engine["name"])
            for name, value in engine.get("options", []):
                self.assertNotIn(name, ("Hash", "Threads", "UCI_LimitStrength", "UCI_Elo"),
                                 engine["name"])
                self.assertIsInstance(value, (bool, int, str), engine["name"])

    def test_the_default_build_avoids_pext_and_the_pext_build_uses_it(self):
        for system in ladder_engines.SYSTEMS:
            avx2 = {p["name"]: p for p in ladder_engines.pins(system, "avx2")}
            pext = {p["name"]: p for p in ladder_engines.pins(system, "pext")}
            self.assertEqual(list(avx2), list(pext), system)
            for name, pin in avx2.items():
                text = " ".join([pin["url"], pin["binary"]]
                                + pin.get("build", {}).get("command", [])).lower()
                for word in ("pext", "bmi2", "use_pext"):
                    self.assertNotIn(word, text, f"{name} on {system}")
        linux_avx2 = {p["name"]: p for p in ladder_engines.pins("linux", "avx2")}
        linux_pext = {p["name"]: p for p in ladder_engines.pins("linux", "pext")}
        changed = sorted(n for n in linux_avx2 if linux_avx2[n] != linux_pext[n])
        self.assertEqual(changed, ["Arasan 26.1", "Inanis 1.0.1", "Stash 37.0", "Weiss 2.0"])
        self.assertEqual(linux_pext["Arasan 26.1"]["binary"], "arasanx-64-avx2-bmi2")
        self.assertIn("-DUSE_PEXT", " ".join(linux_pext["Weiss 2.0"]["build"]["command"]))
        self.assertTrue(linux_pext["Stash 37.0"]["binary"].endswith("-bmi2"))
        self.assertTrue(linux_pext["Inanis 1.0.1"]["url"].endswith("_popcnt_bmi2.zip"))

    def test_a_pin_is_resolved_per_system_and_build(self):
        engine = {
            "name": "Both 1", "version": "1", "licence": "MIT", "hash": True,
            "method": "build", "url": "https://example.invalid/src.tar.gz", "sha256": "a" * 64,
            "archive": "tar.gz", "binary": "src/both", "build": {"command": ["make"]},
            "pext": {"build": {"command": ["make", "PEXT=1"]}},
            "windows": {"method": "download", "url": "https://example.invalid/both.exe",
                        "sha256": "b" * 64, "archive": "none", "binary": "both-avx2.exe",
                        "pext": {"url": "https://example.invalid/both-pext.exe",
                                 "binary": "both-pext.exe"}},
        }
        linux = ladder_engines.pin(engine, "linux", "avx2")
        self.assertEqual(linux["build"], {"command": ["make"]})
        self.assertNotIn("windows", linux)
        self.assertEqual(ladder_engines.pin(engine, "linux", "pext")["build"]["command"],
                         ["make", "PEXT=1"])
        windows = ladder_engines.pin(engine, "windows", "avx2")
        self.assertEqual((windows["binary"], windows["method"]), ("both-avx2.exe", "download"))
        self.assertNotIn("build", windows)  # Linux's source fields do not leak
        self.assertEqual(windows["hash"], True)  # the shared fields do
        pext = ladder_engines.pin(engine, "windows", "pext")
        self.assertEqual((pext["binary"], pext["sha256"]), ("both-pext.exe", "b" * 64))
        windows_only = dict(engine, linux=None)
        self.assertIsNone(ladder_engines.pin(windows_only, "linux"))
        self.assertIsNone(ladder_engines.pin({"name": "Linux 1"}, "windows"))
        with self.assertRaises(ValueError):
            ladder_engines.pin(engine, "macos")

    def test_lookup(self):
        self.assertEqual(ladder_engines.by_name("Vice 1.1")["version"], "1.1 (commit 7d4fe19)")
        with self.assertRaises(KeyError):
            ladder_engines.by_name("Stockfish 19-1500")
        self.assertEqual(ladder_engines.HASH_MB, 64)


if __name__ == "__main__":
    unittest.main()
