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
        patcher = mock.patch.object(install_tools, "download", self.fake_download)
        patcher.start()
        self.addCleanup(patcher.stop)

    # --- stand-ins ----------------------------------------------------------

    def fake_download(self, url: str, dest: Path, sha256: str | None) -> str:
        """Copies the file the test serves at `url`, checking the digest as
        the real download does."""
        self.downloads.append(url)
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
        return install_tools.Ctx(args)

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
        out = io.StringIO()
        with redirect_stderr(out), self.assertRaises(SystemExit):
            ladder_install.select(["Stockfish 18"])
        self.assertIn("no engine named 'Stockfish 18'", out.getvalue())
        self.assertIn("Stockfish 19", out.getvalue())
        out = io.StringIO()
        with redirect_stderr(out), self.assertRaises(SystemExit):
            ladder_install.select(["Orion 1.1"])
        self.assertIn("Orion 1.1 cannot be installed", out.getvalue())
        # The real table: every name of the ladder selects one of its engines.
        for name in ladder_engines.ladder_names():
            self.assertEqual(len(ladder_install.select([name])), 1, name)
        self.assertEqual(len(ladder_install.select(None)), len(ladder_engines.ENGINES))

    def test_a_dry_run_changes_nothing(self):
        url, digest = self.serve("bare", b"ELF")
        engine = self.engine(url=url, sha256=digest)
        ctx, present, failed, printed = self.install([engine], dry_run=True)
        self.assertEqual((present, failed), ([], []))
        self.assertIn(f"would download {url}", printed)
        self.assertEqual(self.downloads, [])
        self.assertFalse(ctx.prefix.exists())

    def test_directory_names(self):
        self.assertEqual(ladder_install.slug("Rustic Alpha 3.0.6"), "rustic-alpha-3.0.6")
        self.assertEqual(ladder_install.slug("Stockfish 19"), "stockfish-19")
        names = [ladder_install.slug(engine["name"]) for engine in ladder_engines.ENGINES]
        self.assertEqual(len(set(names)), len(names))


if __name__ == "__main__":
    unittest.main()
