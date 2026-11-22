# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Offline tests for scripts/ladder_install.py.

Nothing is downloaded and no real engine is built: the tests install
made-up engines from archives they write themselves, with the download
replaced by a local copy, and "build" by running this Python. Everything
lives in a temporary directory.

    python3 -m unittest discover -s scripts/tests
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import shutil
import sys
import tarfile
import tempfile
import unittest
import zipfile
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import install_tools  # noqa: E402
import ladder_engines  # noqa: E402
import ladder_install  # noqa: E402

BUILD = "import pathlib; pathlib.Path('engine').write_bytes(b'built ' + pathlib.Path('main.c').read_bytes())"


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class LadderInstallTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name)
        self.served = self.tmp / "served"
        self.served.mkdir()
        self.urls: dict[str, Path] = {}
        self.downloads: list[str] = []
        self.http_allowed: list[bool] = []
        patcher = mock.patch.object(install_tools, "download", self.fake_download)
        patcher.start()
        self.addCleanup(patcher.stop)

    # --- stand-ins ----------------------------------------------------------

    def fake_download(self, url: str, dest: Path, sha256: str | None,
                      allow_http: bool = False) -> str:
        """Copies the file the test serves at `url`, checking the digest as
        the real download does, and records whether plain HTTP was allowed."""
        self.downloads.append(url)
        self.http_allowed.append(allow_http)
        data = self.urls[url].read_bytes()
        if sha256 is not None and sha(data) != sha256:
            install_tools.die(f"checksum mismatch for {url}")
        Path(dest).write_bytes(data)
        return sha(data)

    def serve(self, name: str, data: bytes) -> tuple[str, str]:
        path = self.served / name
        path.write_bytes(data)
        url = f"https://example.invalid/{name}"
        self.urls[url] = path
        return url, sha(data)

    def tarball(self, name: str, files: dict[str, bytes]) -> tuple[str, str]:
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode="w:gz") as tar:
            for path, data in files.items():
                info = tarfile.TarInfo(path)
                info.size = len(data)
                tar.addfile(info, io.BytesIO(data))
        return self.serve(name, buffer.getvalue())

    def ctx(self, dry_run: bool = False):
        args = argparse.Namespace(
            prefix=str(self.tmp / "prefix"), bin_dir=None, engine=None, systemd=False,
            dry_run=dry_run,
        )
        ctx = install_tools.Ctx(args)
        # The Linux pins and the default build, whatever the machine running
        # the tests is and whatever its scripts/local.env says.
        ctx.system, ctx.engine_build = "linux", "avx2"
        return ctx

    def engine(self, name: str = "Test 1.0", **fields) -> dict:
        base = {
            "name": name, "version": "1.0", "source": "https://example.invalid",
            "licence": "MIT", "method": "download", "archive": "none", "binary": "test-bin",
            "hash": True, "threads": True, "limit": None, "notes": "a made-up engine",
        }
        base.update(fields)
        return base

    def install(self, engines: list[dict], only=None, force=False, dry_run=False):
        """Runs install_engines over `engines` and returns (ctx, present,
        failed, everything printed)."""
        ctx = self.ctx(dry_run)
        out = io.StringIO()
        with mock.patch.object(ladder_engines, "ENGINES", engines), \
                redirect_stdout(out), redirect_stderr(out):
            present, failed = ladder_install.install_engines(ctx, only, force)
        return ctx, present, failed, out.getvalue()

    # --- the three kinds of download ----------------------------------------

    def test_a_bare_binary(self):
        url, digest = self.serve("bare", b"ELF bare")
        engine = self.engine(url=url, sha256=digest)
        ctx, present, failed, _ = self.install([engine])
        self.assertEqual((present, failed), ([engine], []))
        program = ctx.prefix / "engines" / "test-1.0" / "test-bin"
        self.assertEqual(program.read_bytes(), b"ELF bare")
        if sys.platform != "win32":
            self.assertTrue(program.stat().st_mode & 0o111, "not executable")
        entry = ctx.installed_entry("engine:Test 1.0")
        self.assertEqual(entry["binary"], str(program))
        self.assertEqual(entry["binary_sha256"], sha(b"ELF bare"))
        self.assertEqual((entry["url"], entry["sha256"]), (url, digest))

    def test_a_binary_in_a_tar_archive_with_files_beside_it(self):
        url, digest = self.tarball("pack.tar.gz", {
            "top/bin/engine-avx2": b"ELF avx2",
            "top/bin/engine-avx512": b"ELF avx512",
            "top/net.nnue": b"net",
            "top/data/a.txt": b"a",
            "top/data/sub/b.txt": b"b",
        })
        engine = self.engine(url=url, sha256=digest, archive="tar.gz",
                             binary="bin/engine-avx2", beside=["net.nnue", "data"])
        ctx, _, failed, _ = self.install([engine])
        self.assertEqual(failed, [])
        home = ctx.prefix / "engines" / "test-1.0"
        self.assertEqual((home / "engine-avx2").read_bytes(), b"ELF avx2")
        self.assertEqual((home / "net.nnue").read_bytes(), b"net")
        self.assertEqual((home / "data" / "sub" / "b.txt").read_bytes(), b"b")
        # Only what the pin names is installed.
        self.assertFalse((home / "engine-avx512").exists())

    def test_one_member_of_a_zip_shared_by_several_engines(self):
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as bundle:
            bundle.writestr("old-linux64", b"ELF old")
            bundle.writestr("new-linux64", b"ELF new")
            bundle.writestr("new-win64.exe", b"MZ")
        url, digest = self.serve("both.zip", buffer.getvalue())
        old = self.engine("Old 1", url=url, sha256=digest, archive="zip", binary="old-linux64",
                          member_sha256=sha(b"ELF old"))
        new = self.engine("New 2", url=url, sha256=digest, archive="zip", binary="new-linux64",
                          member_sha256=sha(b"ELF new"))
        ctx, present, failed, _ = self.install([old, new])
        self.assertEqual((len(present), failed), (2, []))
        self.assertEqual((ctx.prefix / "engines/old-1/old-linux64").read_bytes(), b"ELF old")
        self.assertEqual((ctx.prefix / "engines/new-2/new-linux64").read_bytes(), b"ELF new")
        # The shared archive is fetched once.
        self.assertEqual(self.downloads, [url])

    def test_a_zip_member_with_files_beside_it(self):
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as bundle:
            bundle.writestr("Pack-1.0/engine.exe", b"MZ engine")
            bundle.writestr("Pack-1.0/engine.nn", b"network")
            bundle.writestr("Pack-1.0/logo.jpg", b"not wanted")
        url, digest = self.serve("pack.zip", buffer.getvalue())
        engine = self.engine(url=url, sha256=digest, archive="zip",
                             binary="Pack-1.0/engine.exe", beside=["Pack-1.0/engine.nn"],
                             member_sha256=sha(b"MZ engine"))
        ctx, _, failed, printed = self.install([engine])
        self.assertEqual(failed, [], printed)
        home = ctx.prefix / "engines" / "test-1.0"
        self.assertEqual(sorted(p.name for p in home.iterdir()), ["engine.exe", "engine.nn"])
        self.assertEqual((home / "engine.nn").read_bytes(), b"network")
        self.assertTrue(ladder_install.is_installed(ctx, engine))

    def test_plain_http_is_allowed_only_for_a_pin_that_says_why(self):
        url, digest = self.serve("bare", b"ELF bare")
        self.install([self.engine(url=url, sha256=digest)])
        self.install([self.engine("Old 1", url=url, sha256=digest,
                                  http_reason="the project's site serves only HTTP")])
        self.assertEqual(self.http_allowed, [False, True])

    # --- builds -------------------------------------------------------------

    def source_engine(self, **fields) -> dict:
        url, digest = self.tarball("src.tar.gz", {"proj-1.0/src/main.c": b"source"})
        build = {"dir": "src", "command": [sys.executable, "-c", BUILD],
                 "tools": [Path(sys.executable).name]}
        build.update(fields.pop("build", {}))
        base = dict(url=url, sha256=digest, archive="tar.gz", method="build",
                    binary="src/engine", build=build)
        base.update(fields)
        return self.engine(**base)

    def test_a_build_from_source(self):
        engine = self.source_engine()
        with mock.patch.object(shutil, "which", lambda tool: "/usr/bin/" + tool):
            ctx, _, failed, printed = self.install([engine])
        self.assertEqual(failed, [], printed)
        program = ctx.prefix / "engines" / "test-1.0" / "engine"
        self.assertEqual(program.read_bytes(), b"built source")
        self.assertEqual(ctx.installed_entry("engine:Test 1.0")["command"],
                         engine["build"]["command"])

    def test_a_build_from_a_zip_source_archive(self):
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as bundle:
            bundle.writestr("proj-1.0/src/main.c", b"zipped source")
            bundle.writestr("proj-1.0/readme.txt", b"read me")
        url, digest = self.serve("src.zip", buffer.getvalue())
        engine = self.source_engine(url=url, sha256=digest, archive="zip")
        with mock.patch.object(shutil, "which", lambda tool: "/usr/bin/" + tool):
            ctx, _, failed, printed = self.install([engine])
        self.assertEqual(failed, [], printed)
        program = ctx.prefix / "engines" / "test-1.0" / "engine"
        self.assertEqual(program.read_bytes(), b"built zipped source")

    def test_a_zip_source_archive_may_not_escape(self):
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as bundle:
            bundle.writestr("proj-1.0/src/main.c", b"source")
            bundle.writestr("../outside.c", b"evil")
        url, digest = self.serve("bad.zip", buffer.getvalue())
        engine = self.source_engine(url=url, sha256=digest, archive="zip")
        with mock.patch.object(shutil, "which", lambda tool: "/usr/bin/" + tool):
            ctx, _, failed, printed = self.install([engine])
        # The zip/build path goes through the shared extractor, which refuses
        # the archive; that nothing lands outside its target is checked by
        # test_unsafe_archive_paths_are_refused in test_windows_tools.py.
        self.assertEqual(failed, ["Test 1.0"])
        self.assertIn("unsafe path in archive", printed)
        self.assertFalse((ctx.prefix / "engines" / "test-1.0").exists())

    def test_an_engine_without_a_digest_is_refused(self):
        url, _ = self.serve("bare", b"ELF")
        _, present, failed, printed = self.install([self.engine(url=url, sha256=None)])
        self.assertEqual((present, failed), ([], ["Test 1.0"]))
        self.assertIn("has no SHA-256 in its pin", printed)
        self.assertEqual(self.downloads, [])

    def test_a_download_that_fails_does_not_stop_the_other_engines(self):
        good_url, digest = self.serve("good", b"ELF good")
        gone = self.engine("Gone 1", url="https://example.invalid/gone", sha256="0" * 64)
        good = self.engine("Good 2", url=good_url, sha256=digest)
        real = self.fake_download

        def flaky(url, dest, sha256, allow_http=False):
            if url.endswith("/gone"):
                raise OSError("server answered 404")
            return real(url, dest, sha256, allow_http)

        with mock.patch.object(install_tools, "download", flaky):
            ctx, present, failed, printed = self.install([gone, good])
        self.assertEqual((present, failed), ([good], ["Gone 1"]))
        self.assertIn("error: Gone 1: OSError: server answered 404", printed)
        self.assertTrue((ctx.prefix / "engines" / "good-2" / "test-bin").is_file())

    def test_a_failed_reinstall_leaves_the_working_install_as_it_was(self):
        url, digest = self.serve("bare", b"ELF v1")
        engine = self.engine(url=url, sha256=digest)
        ctx, _, _, _ = self.install([engine])
        program = ctx.prefix / "engines" / "test-1.0" / "test-bin"
        (self.served / "bare").write_bytes(b"ELF tampered")
        ctx, present, failed, _ = self.install([engine], force=True)
        self.assertEqual((present, failed), ([], ["Test 1.0"]))
        # Still on disk, still recorded, so a later run does not fetch again.
        self.assertEqual(program.read_bytes(), b"ELF v1")
        self.assertTrue(ladder_install.is_installed(ctx, engine))

    def test_an_engine_missing_a_file_beside_it_is_installed_again(self):
        url, digest = self.tarball("pack.tar.gz", {"top/engine": b"ELF", "top/net.nnue": b"net"})
        engine = self.engine(url=url, sha256=digest, archive="tar.gz", binary="engine",
                             beside=["net.nnue"])
        ctx, _, _, _ = self.install([engine])
        net = ctx.prefix / "engines" / "test-1.0" / "net.nnue"
        self.assertTrue(ladder_install.is_installed(ctx, engine))
        net.unlink()
        self.assertFalse(ladder_install.is_installed(ctx, engine))
        self.install([engine])
        self.assertEqual(net.read_bytes(), b"net")
        self.assertEqual(len(self.downloads), 2)

    def test_a_dry_run_notes_a_missing_tool_and_changes_nothing(self):
        engine = self.source_engine()
        with mock.patch.object(shutil, "which", lambda tool: "/usr/bin/" + tool):
            ctx, _, _, _ = self.install([engine])
        before = ctx.manifest.read_text()
        with mock.patch.object(shutil, "which", lambda tool: None):
            ctx, present, failed, printed = self.install([engine], force=True, dry_run=True)
        self.assertEqual(failed, [])
        self.assertIn("a real run would skip this engine", printed)
        self.assertEqual(ctx.manifest.read_text(), before)
        self.assertTrue(ladder_install.is_installed(ctx, engine))

    def test_a_missing_build_tool_is_named_and_nothing_is_fetched(self):
        engine = self.source_engine(build={"tools": ["gcc", "no-such-tool-xyz", "make"]})
        with mock.patch.object(shutil, "which",
                               lambda tool: None if "no-such" in tool else "/usr/bin/" + tool):
            ctx, present, failed, printed = self.install([engine])
        self.assertEqual((present, failed), ([], ["Test 1.0"]))
        self.assertIn("needs no-such-tool-xyz, which is not on PATH", printed)
        self.assertIn("nothing was changed", printed)
        self.assertEqual(self.downloads, [])
        self.assertFalse((ctx.prefix / "engines").exists())

    def test_a_failing_build_leaves_nothing_and_does_not_stop_the_others(self):
        bad = self.source_engine(
            build={"command": [sys.executable, "-c", "import sys; print('cc: boom'); sys.exit(3)"]}
        )
        bad["name"] = "Bad 1"
        url, digest = self.serve("good", b"ELF good")
        good = self.engine("Good 2", url=url, sha256=digest)
        with mock.patch.object(shutil, "which", lambda tool: "/usr/bin/" + tool):
            ctx, present, failed, printed = self.install([bad, good])
        self.assertEqual(failed, ["Bad 1"])
        self.assertEqual(present, [good])
        self.assertIn("failed with status 3", printed)
        self.assertIn("cc: boom", printed)
        self.assertFalse((ctx.prefix / "engines" / "bad-1").exists())
        self.assertEqual(ctx.installed_entry("engine:Bad 1"), {})
        self.assertTrue((ctx.prefix / "engines" / "good-2" / "test-bin").is_file())

    def test_a_build_that_makes_no_binary_is_a_failure(self):
        engine = self.source_engine(build={"command": [sys.executable, "-c", "pass"]})
        with mock.patch.object(shutil, "which", lambda tool: "/usr/bin/" + tool):
            _, present, failed, printed = self.install([engine])
        self.assertEqual((present, failed), ([], ["Test 1.0"]))
        self.assertIn("is not there after unpacking and building", printed)

    # --- checks -------------------------------------------------------------

    def test_a_wrong_digest_is_refused(self):
        url, _ = self.serve("bare", b"ELF tampered")
        engine = self.engine(url=url, sha256=sha(b"ELF expected"))
        ctx, present, failed, printed = self.install([engine])
        self.assertEqual((present, failed), ([], ["Test 1.0"]))
        self.assertIn("checksum mismatch", printed)
        self.assertFalse((ctx.prefix / "engines").exists())

    def test_a_wrong_member_digest_is_refused(self):
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as bundle:
            bundle.writestr("prog", b"ELF swapped")
        url, digest = self.serve("z.zip", buffer.getvalue())
        engine = self.engine(url=url, sha256=digest, archive="zip", binary="prog",
                             member_sha256=sha(b"ELF original"))
        ctx, _, failed, printed = self.install([engine])
        self.assertEqual(failed, ["Test 1.0"])
        self.assertIn("checksum mismatch for prog", printed)
        self.assertFalse((ctx.prefix / "engines").exists())

    def test_paths_may_not_leave_the_archive(self):
        url, digest = self.tarball("pack.tar.gz", {"top/engine": b"ELF"})
        for field, value in (("binary", "../../outside"), ("beside", ["../../../etc"])):
            fields = dict(url=url, sha256=digest, archive="tar.gz", binary="engine")
            fields[field] = value
            _, _, failed, printed = self.install([self.engine(**fields)])
            self.assertEqual(failed, ["Test 1.0"], field)
            self.assertIn("leaves the engine's directory", printed)

    def test_a_missing_file_beside_the_binary_is_a_failure(self):
        url, digest = self.tarball("pack.tar.gz", {"top/engine": b"ELF"})
        engine = self.engine(url=url, sha256=digest, archive="tar.gz", binary="engine",
                             beside=["net.nnue"])
        _, _, failed, printed = self.install([engine])
        self.assertEqual(failed, ["Test 1.0"])
        self.assertIn("net.nnue is not in the archive", printed)

    # --- running again, choosing, and dry runs ------------------------------

    def test_a_second_run_installs_nothing_again_unless_forced_or_changed(self):
        url, digest = self.serve("bare", b"ELF v1")
        engine = self.engine(url=url, sha256=digest)
        self.install([engine])
        _, present, _, printed = self.install([engine])
        self.assertEqual(present, [engine])
        self.assertIn("already installed", printed)
        self.assertEqual(len(self.downloads), 1)
        self.install([engine], force=True)
        self.assertEqual(len(self.downloads), 2)
        # A changed pin, or a program that has gone, means installing again.
        url2, digest2 = self.serve("bare2", b"ELF v2")
        ctx, _, _, _ = self.install([self.engine(url=url2, sha256=digest2)])
        self.assertEqual(len(self.downloads), 3)
        program = ctx.prefix / "engines" / "test-1.0" / "test-bin"
        self.assertEqual(program.read_bytes(), b"ELF v2")
        program.unlink()
        self.install([self.engine(url=url2, sha256=digest2)])
        self.assertEqual(len(self.downloads), 4)

    def test_only_chooses_engines_by_ladder_name(self):
        url, digest = self.serve("bare", b"ELF")
        engines = [self.engine("Alpha 1", url=url, sha256=digest, variants=[1500],
                               limit=(1000, 2000)),
                   self.engine("Beta 2", url=url, sha256=digest)]
        _, present, _, _ = self.install(engines, only=["Beta 2"])
        self.assertEqual([engine["name"] for engine in present], ["Beta 2"])
        # A limited entry is the same program as its engine.
        _, present, _, _ = self.install(engines, only=["Alpha 1-1500", "Alpha 1"])
        self.assertEqual([engine["name"] for engine in present], ["Alpha 1"])
        with self.assertRaises(SystemExit):
            self.install(engines, only=["Gamma 3"])

    def test_an_unknown_or_uninstallable_name_is_explained(self):
        ctx = self.ctx()
        out = io.StringIO()
        with redirect_stderr(out), self.assertRaises(SystemExit):
            ladder_install.select(ctx, ["Stockfish 18"])
        self.assertIn("no engine named 'Stockfish 18'", out.getvalue())
        self.assertIn("Stockfish 19", out.getvalue())
        out = io.StringIO()
        with redirect_stderr(out), self.assertRaises(SystemExit):
            ladder_install.select(ctx, ["Orion 1.1"])
        self.assertIn("Orion 1.1 cannot be installed on linux", out.getvalue())
        out = io.StringIO()
        with redirect_stderr(out), self.assertRaises(SystemExit):
            ladder_install.select(ctx, ["Arasan 26.1-1800"])
        self.assertIn("Arasan 26.1-1800 is no longer in the ladder", out.getvalue())
        # The real table: every name of the ladder selects one of its engines.
        for name in ladder_engines.ladder_names():
            self.assertEqual(len(ladder_install.select(ctx, [name])), 1, name)
        self.assertEqual(len(ladder_install.select(ctx, None)), len(ladder_engines.pins("linux")))

    def test_a_dry_run_changes_nothing(self):
        url, digest = self.serve("bare", b"ELF")
        engine = self.engine(url=url, sha256=digest)
        ctx, present, failed, printed = self.install([engine], dry_run=True)
        self.assertEqual((present, failed), ([], []))
        self.assertIn(f"would download {url}", printed)
        self.assertEqual(self.downloads, [])
        self.assertFalse(ctx.prefix.exists())

    # --- the engine list ----------------------------------------------------

    def test_the_engine_list_names_what_is_installed(self):
        url, digest = self.serve("bare", b"ELF")
        limited = self.engine("Limited 1", url=url, sha256=digest, limit=(1000, 3000),
                              variants=[1500, 1800])
        plain = self.engine("Plain 2", url=url, sha256=digest, threads=False)
        bare = self.engine("Bare 3", url=url, sha256=digest, hash=False, threads=False)
        absent = self.engine("Absent 4", url="https://example.invalid/none", sha256=digest)
        ctx, _, _, _ = self.install([limited, plain, bare])
        out = io.StringIO()
        with mock.patch.object(ladder_engines, "ENGINES", [limited, plain, bare, absent]), \
                redirect_stdout(out):
            path = ladder_install.write_engines_json(ctx)
        self.assertEqual(path, ctx.prefix / "engines" / "engines.json")
        self.assertIn("ENGINES_JSON=", out.getvalue())
        entries = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(
            [entry["name"] for entry in entries],
            ["Limited 1", "Limited 1-1500", "Limited 1-1800", "Plain 2", "Bare 3"],
        )
        by_name = {entry["name"]: entry for entry in entries}
        program = ctx.prefix / "engines" / "limited-1" / "test-bin"
        for name in ("Limited 1", "Limited 1-1500", "Limited 1-1800"):
            # One program, three names.
            self.assertEqual(by_name[name]["command"], str(program))
            self.assertEqual(by_name[name]["workingDirectory"], str(program.parent))
            self.assertEqual(by_name[name]["protocol"], "uci")
        self.assertEqual(by_name["Limited 1"]["options"], [
            {"name": "Hash", "value": 64}, {"name": "Threads", "value": 1},
            {"name": "UCI_LimitStrength", "value": False},
        ])
        self.assertEqual(by_name["Limited 1-1500"]["options"], [
            {"name": "Hash", "value": 64}, {"name": "Threads", "value": 1},
            {"name": "UCI_LimitStrength", "value": True}, {"name": "UCI_Elo", "value": 1500},
        ])
        # An engine without an option is not sent it.
        self.assertEqual(by_name["Plain 2"]["options"], [{"name": "Hash", "value": 64}])
        self.assertEqual(by_name["Bare 3"]["options"], [])

    def test_a_pins_fixed_options_go_into_every_entry(self):
        ctx = self.ctx()
        engine = self.engine("Booked 2", options=[("OwnBook", False), ("Move Overhead [ms]", 30)],
                             limit=(1000, 2500), variants=[1200])
        entries = ladder_install.engine_entries(ctx, engine)
        self.assertEqual([entry["name"] for entry in entries], ["Booked 2", "Booked 2-1200"])
        self.assertEqual(
            [(o["name"], o["value"]) for o in entries[1]["options"]],
            [("Hash", 64), ("Threads", 1), ("OwnBook", False), ("Move Overhead [ms]", 30),
             ("UCI_LimitStrength", True), ("UCI_Elo", 1200)])
        self.assertEqual(entries[0]["options"][2:4], entries[1]["options"][2:4])

    def test_the_real_ladder_gives_the_baselines_names_and_values(self):
        ctx = self.ctx()
        entries = []
        for engine in ladder_engines.pins("linux"):
            entries.extend(ladder_install.engine_entries(ctx, engine))
        self.assertEqual([entry["name"] for entry in entries], ladder_engines.ladder_names())
        self.assertEqual(len(entries), 36)
        by_name = {entry["name"]: {o["name"]: o["value"] for o in entry["options"]}
                   for entry in entries}
        self.assertEqual(by_name["Stockfish 19-1320"],
                         {"Hash": 64, "Threads": 1, "Move Overhead": 30,
                          "UCI_LimitStrength": True, "UCI_Elo": 1320})
        self.assertEqual(by_name["Arasan 26.1-1500"]["UCI_Elo"], 1500)
        self.assertNotIn("Arasan 26.1-1800", by_name)  # retired
        self.assertEqual(by_name["Rodent IV-1600"]["UCI_Elo"], 1600)
        # Rodent IV limits itself unless told not to.
        self.assertIs(by_name["Rodent IV"]["UCI_LimitStrength"], False)
        self.assertNotIn("UCI_Elo", by_name["Rodent IV"])
        self.assertEqual(by_name["Rustic Alpha 1.5"], {})
        self.assertEqual(by_name["Vice 1.1"], {"Hash": 64, "Book": False})
        # Engines new in Ladder Baseline 3, with the same settings.
        self.assertEqual(by_name["Blunder 4.0.0"], {})
        self.assertEqual(by_name["Blunder 7.6.0"], {"Hash": 64, "UseBook": False})
        self.assertEqual(by_name["MinimalChess 0.4.1"], {})
        self.assertEqual(by_name["MinimalChess 0.6"], {"Hash": 64})
        self.assertEqual(by_name["Velvet 1.0.0"], {"Hash": 64})
        self.assertEqual(by_name["Tantabus 1.0.2"], {"Hash": 64})
        self.assertEqual(by_name["Inanis 1.0.1"],
                         {"Hash": 64, "Threads": 1, "Move Overhead": 30})
        self.assertEqual(by_name["Viridithas 20"], {"Hash": 64, "Threads": 1})
        # Ladder Baseline 2's settings under each engine's own option names:
        # books off and 30 ms of move overhead.
        self.assertEqual(by_name["Arasan 26.1-1500"]["OwnBook"], False)
        self.assertEqual(by_name["Arasan 26.1"]["Move overhead"], 30)
        self.assertEqual(by_name["Reckless 0.9.0"]["MoveOverhead"], 30)
        self.assertEqual(by_name["Stash 37.0"]["MoveOverhead"], 30)
        self.assertEqual(by_name["Rodent IV-2000"]["TimeBuffer"], 30)
        self.assertIs(by_name["Rodent IV-2000"]["UseBook"], False)
        self.assertIs(by_name["Fruit 2.1"]["OwnBook"], False)
        self.assertIs(by_name["Weiss 2.0"]["NoobBook"], False)
        # CT800 as in Ladder Baseline 2: no Threads option, book off, the
        # hash cleared between games, 30 ms of move overhead.
        ct800 = {"Hash": 64, "OwnBook": False, "Keep Hash Tables": False,
                 "Move Overhead [ms]": 30}
        self.assertEqual(by_name["CT800 1.46"], dict(ct800, UCI_LimitStrength=False))
        self.assertEqual(by_name["CT800 1.46-1200"],
                         dict(ct800, UCI_LimitStrength=True, UCI_Elo=1200))

    def test_a_dry_run_writes_no_engine_list(self):
        ctx = self.ctx(dry_run=True)
        with redirect_stdout(io.StringIO()):
            self.assertIsNone(ladder_install.write_engines_json(ctx))
        self.assertFalse(ctx.prefix.exists())

    def test_directory_names(self):
        self.assertEqual(ladder_install.slug("Rustic Alpha 3.0.6"), "rustic-alpha-3.0.6")
        self.assertEqual(ladder_install.slug("Stockfish 19"), "stockfish-19")
        names = [ladder_install.slug(engine["name"]) for engine in ladder_engines.ENGINES]
        self.assertEqual(len(set(names)), len(names))


if __name__ == "__main__":
    unittest.main()
