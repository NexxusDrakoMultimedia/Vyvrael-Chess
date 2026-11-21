# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Offline tests for scripts/windows_tools.py.

Nothing is downloaded and no real tool is run: the download is replaced by
a zip the test writes, and the program check by a stand-in. Everything
lives in a temporary directory, so the tests run on any system.

    python3 -m unittest discover -s scripts/tests
"""
from __future__ import annotations

import argparse
import hashlib
import io
import sys
import tempfile
import unittest
import zipfile
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import install_tools  # noqa: E402
import windows_tools  # noqa: E402


def zip_bytes(files: dict[str, bytes]) -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as bundle:
        for name, data in files.items():
            bundle.writestr(name, data)
    return buffer.getvalue()


class WindowsToolsTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name)
        self.archives = {
            "cutechess": zip_bytes({
                "cutechess-1.5.1-win64/cutechess-cli.exe": b"MZ cli",
                "cutechess-1.5.1-win64/Qt6Core.dll": b"MZ qt",
                "cutechess-1.5.1-win64/vc_redist.x64.exe": b"MZ redist",
            }),
            "ordo": zip_bytes({"ordo-1.2.6-win/ordo-win64.exe": b"MZ ordo"}),
        }
        self.downloads: list[str] = []
        self.pins = {
            tool: dict(pin, sha256=hashlib.sha256(self.archives[tool]).hexdigest())
            for tool, pin in windows_tools.PINS.items()
        }
        for target, name, value in (
            (windows_tools, "PINS", self.pins),
            (install_tools, "download", self.fake_download),
            (windows_tools, "check", lambda ctx, tool: True),
            (windows_tools, "missing_runtime", lambda system_root=None: []),
        ):
            patcher = mock.patch.object(target, name, value)
            patcher.start()
            self.addCleanup(patcher.stop)

    def fake_download(self, url: str, dest: Path, sha256: str | None) -> str:
        self.downloads.append(url)
        tool = "cutechess" if "cutechess" in url else "ordo"
        data = self.archives[tool]
        if hashlib.sha256(data).hexdigest() != sha256:
            install_tools.die(f"checksum mismatch for {url}")
        Path(dest).write_bytes(data)
        return sha256

    def ctx(self, dry_run: bool = False):
        return install_tools.Ctx(argparse.Namespace(
            prefix=str(self.tmp / "prefix"), bin_dir=None, engine=None, systemd=False,
            dry_run=dry_run,
        ))

    def run_install(self, wanted, force=False, dry_run=False):
        ctx = self.ctx(dry_run)
        out = io.StringIO()
        with redirect_stdout(out), redirect_stderr(out):
            failed = windows_tools.install_tools_windows(ctx, wanted, force)
        return ctx, failed, out.getvalue()

    def test_the_real_pins(self):
        for tool, pin in windows_tools.PINS.items():
            self.assertTrue(pin["url"].startswith("https://github.com/"), tool)
            self.assertTrue(pin["url"].endswith(".zip"), tool)
            self.assertRegex(pin["sha256"], r"^[0-9a-f]{64}$", tool)
            self.assertTrue(pin["program"].endswith(".exe"), tool)
            # The same versions as the Linux pins.
            self.assertEqual(pin["version"], install_tools.PINS[tool]["version"], tool)

    def test_both_tools_are_unpacked_and_their_settings_printed(self):
        ctx, failed, printed = self.run_install(["cutechess", "ordo"])
        self.assertEqual(failed, [])
        cli = ctx.prefix / "windows/cutechess/cutechess-1.5.1-win64/cutechess-cli.exe"
        ordo = ctx.prefix / "windows/ordo/ordo-1.2.6-win/ordo-win64.exe"
        self.assertEqual(cli.read_bytes(), b"MZ cli")
        # The whole archive is kept: the program needs the files beside it.
        self.assertTrue((cli.parent / "Qt6Core.dll").is_file())
        self.assertEqual(ordo.read_bytes(), b"MZ ordo")
        self.assertIn(f"CUTECHESS_CLI={cli}", printed)
        self.assertIn(f"ORDO={ordo}", printed)
        self.assertIn("scripts/local.env", printed)
        self.assertEqual(ctx.installed_entry("windows:ordo")["program"], str(ordo))

    def test_a_second_run_installs_nothing_again_unless_forced(self):
        self.run_install(["ordo"])
        _, _, printed = self.run_install(["ordo"])
        self.assertIn("already installed", printed)
        self.assertEqual(len(self.downloads), 1)
        self.run_install(["ordo"], force=True)
        self.assertEqual(len(self.downloads), 2)

    def test_a_tool_that_does_not_start_is_not_recorded(self):
        with mock.patch.object(windows_tools, "check", lambda ctx, tool: False):
            ctx, failed, _ = self.run_install(["ordo"])
        self.assertEqual(failed, ["ordo"])
        self.assertEqual(ctx.installed_entry("windows:ordo"), {})

    def test_the_check_fails_a_program_that_dies_silently(self):
        ctx = self.ctx()
        program = windows_tools.program_path(ctx, "ordo")
        program.parent.mkdir(parents=True)
        program.write_bytes(b"")

        really_run = windows_tools.subprocess.run

        def run_as(code: str):
            # Runs this Python in place of the installed program.
            return lambda command, **kwargs: really_run([sys.executable, "-c", code], **kwargs)

        out = io.StringIO()
        with redirect_stdout(out):
            with mock.patch.object(windows_tools.subprocess, "run",
                                   run_as("import sys; sys.exit(53)")):
                silent = WindowsToolsTest.real_check(ctx, "ordo")
            with mock.patch.object(windows_tools.subprocess, "run",
                                   run_as("import sys; print('Ordo 1.2.6'); sys.exit(1)")):
                talks = WindowsToolsTest.real_check(ctx, "ordo")
            with mock.patch.object(windows_tools.subprocess, "run",
                                   run_as("print('cutechess-cli 1.5.1')")):
                fine = WindowsToolsTest.real_check(ctx, "ordo")
        self.assertEqual((silent, talks, fine), (False, True, True))
        self.assertIn("check FAILED: exit status 53 and no output", out.getvalue())
        self.assertIn("check: Ordo 1.2.6 (exit status 1)", out.getvalue())

    real_check = staticmethod(windows_tools.check)

    def test_all_means_both_tools_and_the_list_is_checked_first(self):
        prefix = str(self.tmp / "prefix")
        out = io.StringIO()
        with mock.patch.object(sys, "platform", "win32"), redirect_stdout(out):
            install_tools.main(["--prefix", prefix, "--tool", "all"])
        self.assertIn("CUTECHESS_CLI=", out.getvalue())
        self.assertIn("ORDO=", out.getvalue())
        # A name that cannot be installed stops the run before anything is fetched.
        self.downloads.clear()
        with redirect_stderr(io.StringIO()), redirect_stdout(io.StringIO()), \
                self.assertRaises(SystemExit):
            windows_tools.install_tools_windows(
                self.ctx(), ["cutechess", "lichess-bot"], True
            )
        self.assertEqual(self.downloads, [])

    def test_list_on_windows_shows_the_windows_tools(self):
        prefix = str(self.tmp / "prefix")
        with mock.patch.object(sys, "platform", "win32"), redirect_stdout(io.StringIO()):
            install_tools.main(["--prefix", prefix, "--tool", "ordo"])
        out = io.StringIO()
        with mock.patch.object(sys, "platform", "win32"), redirect_stdout(out):
            install_tools.main(["--prefix", prefix, "--list"])
        lines = out.getvalue().splitlines()
        self.assertRegex(lines[0], r"^cutechess +pinned v1\.5\.1 +not installed$")
        self.assertRegex(lines[1], r"^ordo +pinned v1\.2\.6 +installed$")
        self.assertIn("on Linux only", out.getvalue())

    def test_a_wrong_digest_is_refused(self):
        self.pins["ordo"]["sha256"] = "0" * 64
        with self.assertRaises(SystemExit):
            self.run_install(["ordo"])
        self.assertFalse((self.tmp / "prefix" / "windows").exists())

    def test_an_archive_without_the_program_is_refused(self):
        self.archives["ordo"] = zip_bytes({"ordo-1.2.6-win/README.txt": b"hello"})
        self.pins["ordo"]["sha256"] = hashlib.sha256(self.archives["ordo"]).hexdigest()
        with self.assertRaises(SystemExit):
            self.run_install(["ordo"])

    def test_unsafe_archive_paths_are_refused(self):
        for name in ("../outside.exe", "a/../../outside.exe", "/absolute.exe", "C:/x.exe"):
            archive = self.tmp / "bad.zip"
            archive.write_bytes(zip_bytes({name: b"x"}))
            with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit, msg=name):
                install_tools.safe_extract_zip(archive, self.tmp / "out")
        self.assertFalse((self.tmp / "outside.exe").exists())

    def test_a_missing_visual_c_runtime_is_reported_not_installed(self):
        with mock.patch.object(windows_tools, "missing_runtime",
                               lambda system_root=None: ["vcruntime140_1.dll"]):
            _, failed, printed = self.run_install(["cutechess"])
        self.assertEqual(failed, [])
        self.assertIn("vcruntime140_1.dll not found in System32", printed)
        self.assertIn("vc_redist.x64.exe", printed)
        self.assertIn("administrator rights", printed)

    def test_the_runtime_check_looks_in_system32(self):
        root = self.tmp / "Windows"
        (root / "System32").mkdir(parents=True)
        (root / "System32" / "msvcp140.dll").write_bytes(b"")
        real = WindowsToolsTest.real_missing_runtime
        self.assertEqual(real(str(root)), ["vcruntime140.dll", "vcruntime140_1.dll"])

    real_missing_runtime = staticmethod(windows_tools.missing_runtime)

    def test_lichess_bot_is_refused_on_windows(self):
        out = io.StringIO()
        with redirect_stderr(out), redirect_stdout(io.StringIO()), self.assertRaises(SystemExit):
            windows_tools.install_tools_windows(self.ctx(), ["lichess-bot"], False)
        self.assertIn("lichess-bot runs on the Linux machine only", out.getvalue())

    def test_a_dry_run_changes_nothing(self):
        ctx, failed, printed = self.run_install(["cutechess", "ordo"], dry_run=True)
        self.assertEqual(failed, [])
        self.assertIn("would download", printed)
        self.assertEqual(self.downloads, [])
        self.assertFalse(ctx.prefix.exists())

    def test_main_takes_the_windows_path_on_windows_for_tools_and_engines(self):
        prefix = str(self.tmp / "prefix")
        out = io.StringIO()
        with mock.patch.object(sys, "platform", "win32"), redirect_stdout(out):
            install_tools.main(["--prefix", prefix, "--tool", "ordo"])
        self.assertIn("ORDO=", out.getvalue())
        # --engines takes the Windows pins of the chosen build, and alone it
        # installs no tool. A dry run downloads nothing.
        self.downloads.clear()
        for build, wanted, unwanted in (
                ("avx2", "Ethereal12.75-x64-popcnt-avx2.exe", "pext-avx2"),
                ("pext", "Ethereal12.75-x64-pext-avx2.exe", "popcnt-avx2")):
            out = io.StringIO()
            with mock.patch.object(sys, "platform", "win32"), redirect_stdout(out):
                install_tools.main(["--prefix", prefix, "--engines", "--dry-run",
                                    "--engine-build", build])
            printed = out.getvalue()
            self.assertIn(f"ladder engines for windows, {build} build", printed)
            self.assertIn(wanted, printed)
            self.assertNotIn(unwanted, printed)
            self.assertIn("https://www.orionchess.com/download/Orion-v1.1.zip", printed)
            self.assertNotIn("cutechess", printed.lower())
        self.assertEqual(self.downloads, [])


if __name__ == "__main__":
    unittest.main()
