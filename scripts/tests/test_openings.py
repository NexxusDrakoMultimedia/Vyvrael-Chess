# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Offline tests for scripts/openings.py and install_tools.py --openings.

The download is replaced by a local copy of a zip the tests write, with the
pin's digests swapped for that zip's. Nothing is fetched.

    python3 -m unittest discover -s scripts/tests
"""
from __future__ import annotations

import hashlib
import io
import shutil
import sys
import tempfile
import unittest
import zipfile
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import install_tools  # noqa: E402
import openings  # noqa: E402

EPD = b"rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1\n"


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class Fixture:
    """A zip holding a small .epd under the pin's member name, and a pin
    whose digests are that zip's."""

    def __init__(self, tmp: Path, epd: bytes = EPD, member: str | None = None):
        self.zip = tmp / "served.zip"
        with zipfile.ZipFile(self.zip, "w") as bundle:
            bundle.writestr(member or openings.PIN["member"], epd)
        self.pin = dict(openings.PIN, sha256=sha(self.zip.read_bytes()),
                        member_sha256=sha(epd))
        self.fetches = 0

    def download(self, url, dest, sha256, allow_http=False):
        self.fetches += 1
        self_digest = sha(self.zip.read_bytes())
        if self_digest != sha256:
            raise AssertionError("the installer asked for another digest than the pin's")
        shutil.copyfile(self.zip, dest)
        return self_digest


def run(argv, fixture: Fixture) -> str:
    with mock.patch.object(openings, "PIN", fixture.pin), \
            mock.patch.object(install_tools, "download", fixture.download), \
            mock.patch.object(install_tools.subprocess, "run",
                              side_effect=AssertionError("nothing may be started")), \
            mock.patch.object(sys, "platform", "linux"), \
            redirect_stdout(io.StringIO()) as out:
        install_tools.main(argv)
    return out.getvalue()


class OpeningsPin(unittest.TestCase):
    def test_the_pin_is_exact_and_https(self):
        pin = openings.PIN
        self.assertTrue(pin["url"].startswith("https://raw.githubusercontent.com/"
                                              "official-stockfish/books/"))
        # One commit, not a branch: the same bytes every time.
        commit = pin["version"].split()[-1]
        self.assertRegex(commit, r"^[0-9a-f]{40}$")
        self.assertIn(commit, pin["url"])
        self.assertTrue(pin["url"].endswith("/UHO_Lichess_4852_v1.epd.zip"))
        self.assertRegex(pin["sha256"], r"^[0-9a-f]{64}$")
        self.assertEqual(pin["licence"], "CC0-1.0")

    def test_the_epd_is_the_one_of_the_baselines(self):
        # The SHA-256 recorded in docs/testing/ladder-Baseline-2.md.
        self.assertEqual(openings.PIN["member_sha256"],
                         "7a7f6470615a69c6cf23d565417701d38732876f480af90d67b42abade35644a")
        self.assertEqual(openings.PIN["member"], "UHO_Lichess_4852_v1.epd")


class InstallOpenings(unittest.TestCase):
    def test_installs_the_epd_records_it_and_prints_the_setting(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(Path(tmp))
            prefix = Path(tmp) / "prefix"
            out = run(["--prefix", str(prefix), "--openings"], fixture)
            path = prefix / "openings" / "UHO_Lichess_4852_v1.epd"
            self.assertEqual(path.read_bytes(), EPD)
            self.assertIn(f"OPENINGS={path.resolve()}", out)
            # Only the opening file: no tool was installed.
            self.assertEqual(sorted(p.name for p in prefix.iterdir()),
                             ["manifest.json", "openings"])
            self.assertEqual(sorted(p.name for p in path.parent.iterdir()),
                             ["UHO_Lichess_4852_v1.epd"])
            ctx = install_tools.Ctx(install_tools.build_parser().parse_args(
                ["--prefix", str(prefix)]))
            with mock.patch.object(openings, "PIN", fixture.pin):
                self.assertTrue(openings.is_installed(ctx))

    def test_a_second_run_fetches_nothing_unless_forced(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(Path(tmp))
            prefix = str(Path(tmp) / "prefix")
            run(["--prefix", prefix, "--openings"], fixture)
            out = run(["--prefix", prefix, "--openings"], fixture)
            self.assertEqual(fixture.fetches, 1)
            self.assertIn("already installed", out)
            self.assertIn("OPENINGS=", out)
            run(["--prefix", prefix, "--openings", "--force"], fixture)
            self.assertEqual(fixture.fetches, 2)

    def test_a_changed_file_is_fetched_again(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(Path(tmp))
            prefix = Path(tmp) / "prefix"
            run(["--prefix", str(prefix), "--openings"], fixture)
            (prefix / "openings" / "UHO_Lichess_4852_v1.epd").write_bytes(b"edited\n")
            run(["--prefix", str(prefix), "--openings"], fixture)
            self.assertEqual(fixture.fetches, 2)
            self.assertEqual((prefix / "openings" / "UHO_Lichess_4852_v1.epd").read_bytes(), EPD)

    def test_a_wrong_epd_is_refused_and_leaves_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(Path(tmp))
            fixture.pin["member_sha256"] = "0" * 64
            prefix = Path(tmp) / "prefix"
            err = io.StringIO()
            with redirect_stderr(err), self.assertRaises(SystemExit):
                run(["--prefix", str(prefix), "--openings"], fixture)
            self.assertIn("checksum mismatch for UHO_Lichess_4852_v1.epd", err.getvalue())
            self.assertFalse((prefix / "openings" / "UHO_Lichess_4852_v1.epd").exists())
            self.assertFalse((prefix / "manifest.json").exists())

    def test_a_zip_without_the_epd_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(Path(tmp), member="other.epd")
            err = io.StringIO()
            with redirect_stderr(err), self.assertRaises(SystemExit):
                run(["--prefix", str(Path(tmp) / "prefix"), "--openings"], fixture)
            self.assertIn("UHO_Lichess_4852_v1.epd is not in", err.getvalue())
            prefix = Path(tmp) / "prefix"
            self.assertFalse((prefix / "openings").exists())
            self.assertFalse((prefix / "manifest.json").exists())

    def test_a_failed_swap_leaves_no_new_file_and_says_why(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(Path(tmp))
            prefix = Path(tmp) / "prefix"
            err = io.StringIO()
            with mock.patch.object(openings.os, "replace",
                                   side_effect=PermissionError(13, "in use")), \
                    redirect_stderr(err), self.assertRaises(SystemExit):
                run(["--prefix", str(prefix), "--openings"], fixture)
            self.assertIn("could not put UHO_Lichess_4852_v1.epd", err.getvalue())
            self.assertEqual(list((prefix / "openings").iterdir()), [])
            self.assertFalse((prefix / "manifest.json").exists())

    def test_a_failure_does_not_stop_the_engines(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(Path(tmp))
            fixture.pin["member_sha256"] = "0" * 64
            called = []
            err = io.StringIO()
            with mock.patch.object(install_tools, "install_ladder_engines",
                                   lambda ctx, a: called.append(True) or ([], [])), \
                    redirect_stderr(err), self.assertRaises(SystemExit):
                run(["--prefix", str(Path(tmp) / "prefix"), "--openings", "--engines"],
                    fixture)
            self.assertEqual(called, [True])
            self.assertIn("the opening file was not installed", err.getvalue())

    def test_a_network_error_is_reported_not_a_traceback(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(Path(tmp))
            err = io.StringIO()
            with mock.patch.object(fixture, "download",
                                   side_effect=OSError("network down")), \
                    redirect_stderr(err), self.assertRaises(SystemExit):
                run(["--prefix", str(Path(tmp) / "prefix"), "--openings"], fixture)
            self.assertIn("error: openings: OSError: network down", err.getvalue())

    def test_it_combines_with_a_tool_and_with_verify(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(Path(tmp))
            prefix = str(Path(tmp) / "prefix")
            out = run(["--prefix", prefix, "--openings", "--tool", "cutechess", "--dry-run"],
                      fixture)
            self.assertIn("cutechess", out)
            self.assertIn("openings: [dry-run] would download", out)
            with mock.patch.object(install_tools, "verify_only", lambda ctx, a: None):
                out = run(["--prefix", prefix, "--openings", "--verify"], fixture)
            self.assertEqual(fixture.fetches, 1)
            self.assertIn("OPENINGS=", out)

    def test_a_dry_run_fetches_and_writes_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(Path(tmp))
            prefix = Path(tmp) / "prefix"
            out = run(["--prefix", str(prefix), "--openings", "--dry-run"], fixture)
            self.assertIn("[dry-run] would download", out)
            self.assertEqual(fixture.fetches, 0)
            self.assertFalse(prefix.exists())

    def test_on_windows_it_installs_alone_too(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(Path(tmp))
            prefix = Path(tmp) / "prefix"
            with mock.patch.object(openings, "PIN", fixture.pin), \
                    mock.patch.object(install_tools, "download", fixture.download), \
                    mock.patch.object(sys, "platform", "win32"), \
                    redirect_stdout(io.StringIO()) as out:
                install_tools.main(["--prefix", str(prefix), "--openings"])
            self.assertIn("OPENINGS=", out.getvalue())
            self.assertNotIn("cutechess", out.getvalue().lower())
            self.assertEqual(sorted(p.name for p in prefix.iterdir()),
                             ["manifest.json", "openings"])

    def test_list_shows_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(Path(tmp))
            prefix = str(Path(tmp) / "prefix")
            self.assertRegex(run(["--prefix", prefix, "--list"], fixture),
                             r"openings .*not installed")
            run(["--prefix", prefix, "--openings"], fixture)
            out = run(["--prefix", prefix, "--list"], fixture)
            self.assertRegex(out, r"openings .*UHO_Lichess_4852_v1.epd  installed")


if __name__ == "__main__":
    unittest.main()
