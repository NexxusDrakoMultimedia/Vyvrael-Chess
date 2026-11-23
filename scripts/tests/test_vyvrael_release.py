# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Offline tests for scripts/vyvrael_release.py (install_tools.py --vyvrael).

Nothing is downloaded: the release's .sha256 file, the download and the
binary's `uci` answer are stubbed, and everything is written to a temporary
directory. The tests that make symbolic links are skipped where the system
does not let this user make them (Windows without Developer Mode); the
feature itself is for the Linux machine.

    python3 -m unittest discover -s scripts/tests
"""
from __future__ import annotations

import argparse
import hashlib
import io
import os
import sys
import tempfile
import unittest
import urllib.error
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import install_tools  # noqa: E402
import vyvrael_release as release  # noqa: E402

BINARY = b"\x7fELF pretend engine"


def symlinks_work() -> bool:
    with tempfile.TemporaryDirectory() as tmp:
        try:
            os.symlink("target", Path(tmp) / "link")
        except (OSError, NotImplementedError):
            return False
    return True


SYMLINKS = symlinks_work()


def ctx_for(prefix: Path, **overrides):
    base = dict(prefix=str(prefix), bin_dir=None, engine=None, dry_run=False, engine_build=None)
    base.update(overrides)
    return install_tools.Ctx(argparse.Namespace(**base))


def quiet(call, *args):
    """Runs `call`, returning (stdout, stderr); SystemExit propagates."""
    out, err = io.StringIO(), io.StringIO()
    with redirect_stdout(out), redirect_stderr(err):
        call(*args)
    return out.getvalue(), err.getvalue()


class Release:
    """Stubs for one release: its .sha256 text, the download, the uci name."""

    def __init__(self, version="0.1.0", name=None, data=BINARY, answers=None):
        self.name = name or f"vyvrael-{version}-avx2"
        self.data = data
        self.digest = hashlib.sha256(data).hexdigest()
        self.answers = answers or f"Vyvræl {version}"
        self.urls = []

    def fetch_text(self, url):
        self.urls.append(url)
        return f"{self.digest}  {self.name}\n"

    def download(self, url, dest, sha256, allow_http=False):
        self.urls.append(url)
        dest.write_bytes(self.data)
        if sha256 != hashlib.sha256(self.data).hexdigest():
            install_tools.die("checksum mismatch")
        return sha256

    def patches(self):
        return (
            mock.patch.object(release, "fetch_text", self.fetch_text),
            mock.patch.object(install_tools, "download", self.download),
            mock.patch.object(release, "uci_name", lambda binary: self.answers),
        )


def run_release(ctx, stub, version="0.1.0", build="avx2"):
    a, b, c = stub.patches()
    with a, b, c:
        return quiet(release.install_release, ctx, version, build)


def failure(call) -> str:
    """Runs `call`, which must end the run with an error; returns stderr."""
    err = io.StringIO()
    with redirect_stdout(io.StringIO()), redirect_stderr(err):
        try:
            call()
        except SystemExit:
            return err.getvalue()
    raise AssertionError("the run did not end with an error")


class Failures(unittest.TestCase):
    """What ends a run before anything is written; no symbolic links needed."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.ctx = ctx_for(Path(self.tmp.name))

    def tearDown(self):
        self.tmp.cleanup()

    def install(self):
        release.install_release(self.ctx, "0.1.0", "avx2")

    def test_a_missing_release_says_why(self):
        def missing(url):
            raise urllib.error.HTTPError(url, 404, "Not Found", None, None)

        with mock.patch.object(release, "fetch_text", missing):
            err = failure(self.install)
        self.assertIn("draft", err)
        self.assertIn("--vyvrael-build plain", err)

    def test_no_network_is_an_error_message_not_a_traceback(self):
        def offline(url):
            raise urllib.error.URLError("Name or service not known")

        with mock.patch.object(release, "fetch_text", offline):
            err = failure(self.install)
        self.assertIn("could not download", err)
        self.assertIn("Name or service not known", err)
        self.assertNotIn("draft", err)

    def test_a_rate_limit_is_not_reported_as_a_missing_release(self):
        def limited(url):
            raise urllib.error.HTTPError(url, 429, "Too Many Requests", None, None)

        with mock.patch.object(release, "fetch_text", limited):
            err = failure(self.install)
        self.assertIn("Too Many Requests", err)
        self.assertNotIn("draft", err)

    def test_engine_named_like_the_release_file_is_refused(self):
        ctx = ctx_for(Path(self.tmp.name),
                      engine=str(Path(self.tmp.name) / "vyvrael-0.1.0-avx2"))
        err = failure(lambda: release.install_release(ctx, "0.1.0", "avx2"))
        self.assertIn("must not be named", err)

    def test_dry_run_through_main_fetches_and_writes_nothing(self):
        stub = Release()
        a, b, c = stub.patches()
        with a, b, c, mock.patch.object(install_tools.sys, "platform", "linux"):
            out, _ = quiet(install_tools.main,
                           ["--prefix", self.tmp.name, "--vyvrael", "0.1.0", "--dry-run"])
        self.assertEqual(stub.urls, [])
        self.assertEqual(list(Path(self.tmp.name).iterdir()), [])
        self.assertIn("[dry-run] would point", out)

    def test_verify_with_vyvrael_is_refused_not_ignored(self):
        with mock.patch.object(install_tools.sys, "platform", "linux"), \
                mock.patch.object(release, "install_release") as install:
            err = failure(lambda: install_tools.main(
                ["--prefix", self.tmp.name, "--vyvrael", "0.1.0", "--verify"]))
        self.assertIn("leave out --verify", err)
        install.assert_not_called()


class Names(unittest.TestCase):
    def test_versions_with_or_without_v_and_prereleases(self):
        self.assertEqual(release.parse_version("0.1.0"), "0.1.0")
        self.assertEqual(release.parse_version("v0.1.0"), "0.1.0")
        self.assertEqual(release.parse_version("0.1.0-alpha.2"), "0.1.0-alpha.2")

    def test_anything_else_is_refused(self):
        for text in ("latest", "0.1", "../0.1.0", "0.1.0/x", "0.1.0 rm"):
            with self.assertRaises(SystemExit, msg=text):
                quiet(release.parse_version, text)

    def test_file_names_and_urls_follow_the_release_artifacts(self):
        self.assertEqual(release.asset_name("0.1.0", "avx2"), "vyvrael-0.1.0-avx2")
        self.assertEqual(release.asset_name("0.1.0-alpha.2", "plain"), "vyvrael-0.1.0-alpha.2")
        url = release.asset_url("0.1.0", "vyvrael-0.1.0-avx2")
        self.assertEqual(
            url, "https://github.com/NexxusDrakoMultimedia/Vyvrael-Chess/releases/download/"
            "v0.1.0/vyvrael-0.1.0-avx2")

    def test_sha256_line_must_name_the_file(self):
        digest = "ab" * 32
        self.assertEqual(release.parse_sha256(f"{digest}  vyvrael-0.1.0\n", "vyvrael-0.1.0"),
                         digest)
        self.assertEqual(release.parse_sha256(f"{digest.upper()} *vyvrael-0.1.0", "vyvrael-0.1.0"),
                         digest)
        for text in (f"{digest}  other", "nonsense", f"{digest}  vyvrael-0.1.0\nmore lines"):
            with self.assertRaises(SystemExit, msg=text):
                quiet(release.parse_sha256, text, "vyvrael-0.1.0")


class LinkPath(unittest.TestCase):
    def test_default_is_the_engine_path_install_tools_writes(self):
        with tempfile.TemporaryDirectory() as tmp:
            ctx = ctx_for(Path(tmp))
            self.assertEqual(release.link_path(ctx), ctx.prefix / "engines" / "vyvrael")

    @unittest.skipUnless(SYMLINKS, "this system does not let the user make symbolic links")
    def test_an_existing_link_given_with_engine_is_not_followed(self):
        with tempfile.TemporaryDirectory() as tmp:
            link = Path(tmp).resolve() / "vyvrael"
            os.symlink("vyvrael-0.1.0-alpha.2", link)
            ctx = ctx_for(Path(tmp), engine=str(link))
            self.assertEqual(release.link_path(ctx), link)


@unittest.skipUnless(SYMLINKS, "this system does not let the user make symbolic links")
class Install(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.ctx = ctx_for(Path(self.tmp.name))
        self.engines = self.ctx.prefix / "engines"

    def tearDown(self):
        self.tmp.cleanup()

    def test_downloads_checks_and_points_the_link(self):
        stub = Release()
        out, _ = run_release(self.ctx, stub)
        link = self.engines / "vyvrael"
        self.assertEqual(os.readlink(link), "vyvrael-0.1.0-avx2")  # relative
        self.assertEqual(link.read_bytes(), BINARY)
        self.assertTrue(os.access(self.engines / "vyvrael-0.1.0-avx2", os.X_OK)
                        or sys.platform == "win32")
        self.assertEqual(stub.urls[0], release.asset_url("0.1.0", "vyvrael-0.1.0-avx2.sha256"))
        self.assertIn("config.yml is unchanged", out)
        self.assertEqual(sorted(p.name for p in self.engines.iterdir()),
                         ["vyvrael", "vyvrael-0.1.0-avx2"])  # no staging left behind

    def test_an_upgrade_repoints_and_keeps_the_old_version(self):
        run_release(self.ctx, Release("0.1.0-alpha.2", name="vyvrael-0.1.0-alpha.2"),
                    "0.1.0-alpha.2", "plain")
        run_release(self.ctx, Release(data=b"new"))
        self.assertEqual(os.readlink(self.engines / "vyvrael"), "vyvrael-0.1.0-avx2")
        self.assertTrue((self.engines / "vyvrael-0.1.0-alpha.2").is_file())

    def test_a_plain_file_at_the_link_is_moved_aside_not_lost(self):
        self.engines.mkdir(parents=True)
        (self.engines / "vyvrael").write_bytes(b"copied by hand")
        out, _ = run_release(self.ctx, Release())
        self.assertEqual((self.engines / "vyvrael.before-link").read_bytes(), b"copied by hand")
        self.assertTrue((self.engines / "vyvrael").is_symlink())
        self.assertIn("vyvrael.before-link", out)

    def test_a_binary_of_another_version_is_refused_and_nothing_changes(self):
        run_release(self.ctx, Release("0.1.0-alpha.2", name="vyvrael-0.1.0-alpha.2"),
                    "0.1.0-alpha.2", "plain")
        with self.assertRaises(SystemExit):
            run_release(self.ctx, Release(answers="Vyvræl 0.1.0-alpha.2"))
        self.assertEqual(os.readlink(self.engines / "vyvrael"), "vyvrael-0.1.0-alpha.2")
        self.assertFalse((self.engines / "vyvrael-0.1.0-avx2").exists())

    def test_a_wrong_digest_leaves_nothing_behind(self):
        stub = Release()
        stub.digest = "0" * 64
        with self.assertRaises(SystemExit):
            run_release(self.ctx, stub)
        self.assertEqual(list(self.engines.iterdir()), [])

    def test_a_failed_binary_download_is_an_error_message_and_leaves_nothing(self):
        stub = Release()

        def unavailable(url, dest, sha256, allow_http=False):
            dest.write_bytes(b"part")
            raise urllib.error.HTTPError(url, 503, "Service Unavailable", None, None)

        stub.download = unavailable
        a, b, c = stub.patches()
        with a, b, c:
            err = failure(lambda: release.install_release(self.ctx, "0.1.0", "avx2"))
        self.assertIn("could not download", err)
        self.assertNotIn("draft", err)
        self.assertEqual(list(self.engines.iterdir()), [])

    def test_a_link_that_cannot_be_swapped_leaves_the_plain_file_in_place(self):
        self.engines.mkdir(parents=True)
        link = self.engines / "vyvrael"
        link.write_bytes(b"copied by hand")
        (self.engines / "vyvrael-0.1.0-avx2").write_bytes(BINARY)
        with mock.patch.object(release.os, "replace", side_effect=PermissionError("no")), \
                self.assertRaises(PermissionError):
            release.point_link(link, "vyvrael-0.1.0-avx2")
        self.assertFalse(link.is_symlink())
        self.assertEqual(link.read_bytes(), b"copied by hand")

    def test_list_shows_where_the_link_points(self):
        self.assertIn("no engine at", release.describe_link(self.ctx))
        run_release(self.ctx, Release())
        self.assertIn("-> vyvrael-0.1.0-avx2", release.describe_link(self.ctx))


@unittest.skipIf(sys.platform == "win32", "runs a shell script as the engine")
class UciName(unittest.TestCase):
    def test_reads_the_id_name_line(self):
        with tempfile.TemporaryDirectory() as tmp:
            engine = Path(tmp) / "engine"
            engine.write_text("#!/bin/sh\nread x\necho 'id name Vyvræl 0.1.0'\necho uciok\n")
            install_tools.make_exec(engine)
            self.assertEqual(release.uci_name(engine), "Vyvræl 0.1.0")

    def test_no_id_name_is_an_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            engine = Path(tmp) / "engine"
            engine.write_text("#!/bin/sh\necho hello\n")
            install_tools.make_exec(engine)
            with self.assertRaises(SystemExit):
                quiet(release.uci_name, engine)


class Main(unittest.TestCase):
    def test_refused_on_windows(self):
        with tempfile.TemporaryDirectory() as tmp, \
                mock.patch.object(install_tools.sys, "platform", "win32"), \
                self.assertRaises(SystemExit):
            quiet(install_tools.main, ["--prefix", tmp, "--vyvrael", "0.1.0"])

    def test_vyvrael_alone_installs_no_tool(self):
        with tempfile.TemporaryDirectory() as tmp, \
                mock.patch.object(install_tools.sys, "platform", "linux"), \
                mock.patch.object(release, "install_release") as install, \
                mock.patch.dict(install_tools.INSTALLERS, {
                    tool: mock.Mock(side_effect=AssertionError(tool))
                    for tool in install_tools.TOOLS}):
            quiet(install_tools.main, ["--prefix", tmp, "--vyvrael", "v0.1.0",
                                       "--vyvrael-build", "plain"])
        install.assert_called_once()
        self.assertEqual(install.call_args.args[1:], ("v0.1.0", "plain"))


if __name__ == "__main__":
    unittest.main()
