# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Offline tests for scripts/install_tools.py.

They exercise the pure helpers only: nothing is downloaded, built or
installed, and every file they touch lives in a temporary directory. The
installer targets Linux, so tests that check generated text build their paths
with PurePosixPath and pass on any system.

    python3 -m unittest discover -s scripts/tests
"""
from __future__ import annotations

import argparse
import io
import os
import re
import stat
import sys
import tarfile
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path, PurePosixPath
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import install_tools  # noqa: E402


def args(**overrides) -> argparse.Namespace:
    base = dict(prefix=None, bin_dir=None, engine=None, systemd=False, dry_run=False)
    base.update(overrides)
    return argparse.Namespace(**base)


def tar_with(
    path: Path,
    members: list[tuple[str, bytes]],
    links: list[tuple[str, str]] = (),
    hard_links: list[tuple[str, str]] = (),
) -> None:
    """Writes an archive: symlinks first, then hard links, then files."""
    with tarfile.open(path, "w:gz") as tar:
        for kind, pairs in ((tarfile.SYMTYPE, links), (tarfile.LNKTYPE, hard_links)):
            for name, target in pairs:
                info = tarfile.TarInfo(name)
                info.type = kind
                info.linkname = target
                tar.addfile(info)
        for name, data in members:
            info = tarfile.TarInfo(name)
            info.size = len(data)
            tar.addfile(info, io.BytesIO(data))


class Pins(unittest.TestCase):
    def test_every_tool_is_pinned_to_something_exact(self):
        self.assertEqual(set(install_tools.PINS), {"cutechess", "ordo", "lichess-bot"})
        for tool, pin in install_tools.PINS.items():
            self.assertTrue(pin["version"], tool)
            self.assertRegex(pin["repo"], r"^[\w.-]+/[\w.-]+$", tool)
            self.assertTrue(pin["licence"], tool)
            # Either a release file with a digest, or an exact commit.
            if "commit" in pin:
                self.assertRegex(pin["commit"], r"^[0-9a-f]{40}$", tool)
            else:
                self.assertRegex(pin["sha256"], r"^[0-9a-f]{64}$", tool)
            if pin.get("sha256") is not None:
                self.assertRegex(pin["sha256"], r"^[0-9a-f]{64}$", tool)

    def test_urls_are_https_and_contain_the_pin(self):
        cute = install_tools.PINS["cutechess"]
        self.assertTrue(cute["url"].startswith("https://github.com/cutechess/cutechess/"))
        self.assertIn(cute["version"], cute["url"])
        for tool in ("ordo", "lichess-bot"):
            url = install_tools.source_url(tool)
            self.assertTrue(url.startswith("https://codeload.github.com/"), url)
            self.assertTrue(url.endswith("/tar.gz/" + install_tools.PINS[tool]["commit"]), url)

    def test_default_tools_do_not_include_the_bot(self):
        self.assertEqual(install_tools.DEFAULT_TOOLS, ["cutechess", "ordo"])


class Download(unittest.TestCase):
    def test_refuses_plain_http(self):
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(SystemExit):
            install_tools.download("http://example.invalid/x", Path(tmp) / "x", None)

    def test_plain_http_needs_a_digest_and_stays_on_the_web(self):
        with tempfile.TemporaryDirectory() as tmp:
            for url, digest in (("http://example.invalid/x", None),
                                ("ftp://example.invalid/x", "a" * 64)):
                with self.assertRaises(SystemExit, msg=url), redirect_stderr(io.StringIO()):
                    install_tools.download(url, Path(tmp) / "x", digest, allow_http=True)
        request = install_tools.urllib.request.Request("http://example.invalid/x")
        handler = install_tools.WebOnlyRedirects()
        for newurl in ("ftp://example.invalid/x", "file:///etc/passwd"):
            with self.assertRaises(SystemExit, msg=newurl), redirect_stderr(io.StringIO()):
                handler.redirect_request(request, None, 302, "Found", {}, newurl)
        followed = handler.redirect_request(request, None, 302, "Found", {},
                                            "http://other.invalid/y")
        self.assertEqual(followed.full_url, "http://other.invalid/y")

    def test_plain_http_with_a_digest_is_checked(self):
        # No network: the opener answers from memory with the wrong bytes.
        response = mock.MagicMock()
        response.__enter__.return_value = response
        response.geturl.return_value = "http://example.invalid/x"
        response.read.side_effect = [b"other bytes", b""]
        opener = mock.MagicMock()
        opener.open.return_value = response
        with tempfile.TemporaryDirectory() as tmp, \
                mock.patch.object(install_tools.urllib.request, "build_opener",
                                  return_value=opener), \
                redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()), \
                self.assertRaises(SystemExit):
            install_tools.download("http://example.invalid/x", Path(tmp) / "x", "a" * 64,
                                   allow_http=True)

    def redirect(self, newurl: str):
        request = install_tools.urllib.request.Request("https://example.invalid/x")
        handler = install_tools.HttpsOnlyRedirects()
        return handler.redirect_request(request, None, 302, "Found", {}, newurl)

    def test_refuses_a_redirect_that_leaves_https(self):
        for newurl in ("http://example.invalid/x", "ftp://example.invalid/x", "file:///etc/passwd"):
            with self.assertRaises(SystemExit, msg=newurl), redirect_stderr(io.StringIO()):
                self.redirect(newurl)

    def test_follows_a_redirect_that_stays_on_https(self):
        followed = self.redirect("https://other.invalid/y")
        self.assertEqual(followed.full_url, "https://other.invalid/y")

    def test_the_opener_uses_the_https_only_handler(self):
        handlers = install_tools.https_opener().handlers
        redirectors = [
            h for h in handlers if isinstance(h, install_tools.urllib.request.HTTPRedirectHandler)
        ]
        self.assertEqual([type(h) for h in redirectors], [install_tools.HttpsOnlyRedirects])

    def test_refuses_a_response_that_did_not_come_over_https(self):
        # No network: the opener is replaced by one that answers from memory.
        response = mock.MagicMock()
        response.__enter__.return_value = response
        response.geturl.return_value = "http://example.invalid/x"
        response.read.return_value = b""
        opener = mock.Mock()
        opener.open.return_value = response
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(
            install_tools, "https_opener", return_value=opener
        ), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit):
                install_tools.download("https://example.invalid/x", Path(tmp) / "x", None)
            self.assertFalse((Path(tmp) / "x").exists())
            response.read.assert_not_called()

    def test_downloads_through_the_opener_and_checks_the_digest(self):
        response = mock.MagicMock()
        response.__enter__.return_value = response
        response.geturl.return_value = "https://other.invalid/y"
        response.read.side_effect = [b"hello", b""]
        opener = mock.Mock()
        opener.open.return_value = response
        digest = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(
            install_tools, "https_opener", return_value=opener
        ), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            seen = install_tools.download("https://example.invalid/x", Path(tmp) / "x", digest)
            self.assertEqual(seen, digest)
            self.assertEqual((Path(tmp) / "x").read_bytes(), b"hello")
            with self.assertRaises(SystemExit):
                response.read.side_effect = [b"other", b""]
                install_tools.download("https://example.invalid/x", Path(tmp) / "y", digest)
            self.assertFalse((Path(tmp) / "y").exists())


class Extract(unittest.TestCase):
    def test_extracts_a_normal_archive(self):
        with tempfile.TemporaryDirectory() as tmp:
            archive = Path(tmp) / "a.tar.gz"
            tar_with(archive, [("pkg/readme.txt", b"hello")])
            install_tools.safe_extract(archive, Path(tmp) / "out")
            self.assertEqual((Path(tmp) / "out/pkg/readme.txt").read_bytes(), b"hello")
            self.assertEqual(install_tools.single_subdir(Path(tmp) / "out").name, "pkg")

    def test_rejects_paths_that_escape(self):
        for name in ("../evil.txt", "pkg/../../evil.txt"):
            with tempfile.TemporaryDirectory() as tmp:
                archive = Path(tmp) / "a.tar.gz"
                tar_with(archive, [(name, b"x")])
                with self.assertRaises(SystemExit, msg=name):
                    install_tools.safe_extract(archive, Path(tmp) / "out")
                self.assertFalse((Path(tmp) / "evil.txt").exists())

    def test_rejects_a_symlink_that_points_outside(self):
        with tempfile.TemporaryDirectory() as tmp:
            archive = Path(tmp) / "a.tar.gz"
            tar_with(archive, [], links=[("pkg/link", "../../outside")])
            with self.assertRaises(SystemExit):
                install_tools.safe_extract(archive, Path(tmp) / "out")

    def refused_before_extraction(self, **archive) -> None:
        """Asserts the installer's own check stops the archive.

        Extraction is replaced by a mock, so this holds on Pythons without
        `tarfile.data_filter` too, where nothing else would stop it.
        """
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "a.tar.gz"
            tar_with(path, archive.pop("members", []), **archive)
            with mock.patch.object(tarfile.TarFile, "extractall") as extractall:
                with self.assertRaises(SystemExit), redirect_stderr(io.StringIO()):
                    install_tools.safe_extract(path, Path(tmp) / "out")
                extractall.assert_not_called()

    def test_rejects_a_chain_of_links_that_climbs_out(self):
        # Each link is harmless alone: `up` is the archive root, and `up2`
        # reads as pkg/ until `up` exists, when it becomes the parent.
        self.refused_before_extraction(
            members=[("pkg/up2/evil.txt", b"x")],
            links=[("pkg/up", ".."), ("pkg/up2", "up/..")],
        )

    def test_rejects_a_link_target_that_goes_through_a_link(self):
        self.refused_before_extraction(links=[("pkg/up", ".."), ("pkg/up2", "up/..")])

    def test_rejects_a_file_under_a_link(self):
        self.refused_before_extraction(
            members=[("pkg/dir/file.txt", b"x")], links=[("pkg/dir", ".")]
        )

    def test_rejects_a_hard_link_to_a_symlink(self):
        # A copy of `a/b/up` at the top would point two levels above it.
        self.refused_before_extraction(
            links=[("a/b/up", "../..")], hard_links=[("copy", "a/b/up")]
        )

    def test_accepts_links_that_stay_inside(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "a.tar.gz"
            tar_with(
                path,
                [("pkg/readme.txt", b"hello"), ("pkg/sub/file.txt", b"x")],
                links=[("pkg/link", "readme.txt"), ("pkg/sub/up", "../readme.txt")],
                hard_links=[("pkg/same", "pkg/readme.txt")],
            )
            with mock.patch.object(tarfile.TarFile, "extractall") as extractall:
                install_tools.safe_extract(path, Path(tmp) / "out")
                extractall.assert_called_once()

    def fake_tar(self, listing: str, names: str):
        """A stand-in for Windows' tar: answers -tvf and -tf from text and
        records an extraction instead of doing one."""
        calls = []

        def run(words, **_):
            calls.append(words)
            out = {"-tvf": listing, "-tf": names}.get(words[1], "")
            return mock.Mock(returncode=0, stdout=out, stderr="")
        return run, calls

    def test_7z_entries_that_escape_or_link_are_refused_before_extracting(self):
        cases = [
            ("-rw-r--r-- 0 0 0 1 Jan 1 2020 top/ok\n", "top/ok\n../evil\n"),
            ("-rw-r--r-- 0 0 0 1 Jan 1 2020 top/ok\n", "top/ok\nC:/evil\n"),
            ("lrwxrwxrwx 0 0 0 0 Jan 1 2020 top/link -> /etc\n", "top/link\n"),
        ]
        for listing, names in cases:
            run, calls = self.fake_tar(listing, names)
            with tempfile.TemporaryDirectory() as tmp, \
                    mock.patch.object(install_tools.subprocess, "run", run), \
                    redirect_stderr(io.StringIO()), self.assertRaises(SystemExit, msg=names):
                install_tools.safe_extract_7z(Path(tmp) / "a.7z", Path(tmp) / "out",
                                              tar=Path("tar.exe"))
            self.assertNotIn("-xf", [words[1] for words in calls])

    def test_a_safe_7z_is_extracted_into_the_destination(self):
        run, calls = self.fake_tar("drwxr-xr-x 0 0 0 0 Jan 1 2020 top/\n"
                                   "-rw-r--r-- 0 0 0 1 Jan 1 2020 top/engine.exe\n",
                                   "top/\ntop/engine.exe\n")
        with tempfile.TemporaryDirectory() as tmp, \
                mock.patch.object(install_tools.subprocess, "run", run):
            install_tools.safe_extract_7z(Path(tmp) / "a.7z", Path(tmp) / "out",
                                          tar=Path("tar.exe"))
            self.assertEqual(calls[-1][1:4], ["-xf", str(Path(tmp) / "a.7z"), "-C"])

    def test_without_windows_tar_a_7z_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp, \
                mock.patch.object(install_tools, "windows_tar", lambda: None), \
                redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            install_tools.safe_extract_7z(Path(tmp) / "a.7z", Path(tmp) / "out")

    @unittest.skipUnless(install_tools.windows_tar(), "needs Windows' own tar")
    def test_a_real_7z_with_windows_tar(self):
        tar = install_tools.windows_tar()
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "src" / "top"
            source.mkdir(parents=True)
            (source / "engine.exe").write_bytes(b"MZ engine")
            archive = Path(tmp) / "a.7z"
            install_tools.subprocess.run(
                [str(tar), "--format", "7zip", "-cf", str(archive), "-C",
                 str(Path(tmp) / "src"), "top"], check=True)
            install_tools.safe_extract_7z(archive, Path(tmp) / "out")
            self.assertEqual((Path(tmp) / "out/top/engine.exe").read_bytes(), b"MZ engine")

    def test_single_subdir_falls_back_to_the_directory_itself(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "a").mkdir()
            (root / "b").mkdir()
            self.assertEqual(install_tools.single_subdir(root), root)


class ReplaceDir(unittest.TestCase):
    def test_swaps_in_the_new_tree_and_cleans_up(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "new").mkdir()
            (root / "new/file").write_text("new")
            dest = root / "install/tool"
            dest.mkdir(parents=True)
            (dest / "file").write_text("old")
            (dest / "stale").write_text("gone")
            install_tools.replace_dir(root / "new", dest)
            self.assertEqual((dest / "file").read_text(), "new")
            self.assertFalse((dest / "stale").exists())
            self.assertEqual(sorted(p.name for p in dest.parent.iterdir()), ["tool"])

    def test_a_failed_copy_leaves_the_old_install_alone(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dest = root / "tool"
            dest.mkdir()
            (dest / "file").write_text("old")
            with self.assertRaises(OSError):
                install_tools.replace_dir(root / "does-not-exist", dest)
            self.assertEqual((dest / "file").read_text(), "old")

    def interrupted(self, root: Path) -> Path:
        """The state left by a run stopped between the swap's two renames:
        no install, the previous one under `.old`, the new one under `.new`."""
        dest = root / "tool"
        (root / "tool.old").mkdir()
        (root / "tool.old/config.yml").write_text("mine")
        (root / "tool.new").mkdir()
        (root / "tool.new/file").write_text("half done")
        return dest

    def test_recovers_the_old_install_after_an_interrupted_swap(self):
        with tempfile.TemporaryDirectory() as tmp, redirect_stdout(io.StringIO()) as out:
            dest = self.interrupted(Path(tmp))
            install_tools.recover_interrupted(dest)
            self.assertEqual((dest / "config.yml").read_text(), "mine")
            self.assertFalse((Path(tmp) / "tool.old").exists())
            self.assertIn("interrupted", out.getvalue())

    def test_recovery_leaves_a_present_install_alone(self):
        with tempfile.TemporaryDirectory() as tmp, redirect_stdout(io.StringIO()) as out:
            dest = self.interrupted(Path(tmp))
            dest.mkdir()
            (dest / "config.yml").write_text("current")
            install_tools.recover_interrupted(dest)
            self.assertEqual((dest / "config.yml").read_text(), "current")
            self.assertTrue((Path(tmp) / "tool.old").is_dir())
            self.assertEqual(out.getvalue(), "")

    def test_replace_recovers_before_it_clears_leftovers(self):
        # The copy fails, so what is left shows what happened before it: the
        # only copy of the old install must be back in place, not deleted.
        with tempfile.TemporaryDirectory() as tmp, redirect_stdout(io.StringIO()):
            dest = self.interrupted(Path(tmp))
            with self.assertRaises(OSError):
                install_tools.replace_dir(Path(tmp) / "does-not-exist", dest)
            self.assertEqual((dest / "config.yml").read_text(), "mine")

    def test_replace_after_an_interrupted_swap_ends_clean(self):
        with tempfile.TemporaryDirectory() as tmp, redirect_stdout(io.StringIO()):
            root = Path(tmp)
            dest = self.interrupted(root)
            (root / "new").mkdir()
            (root / "new/file").write_text("new")
            install_tools.replace_dir(root / "new", dest)
            self.assertEqual((dest / "file").read_text(), "new")
            self.assertEqual(sorted(p.name for p in root.iterdir()), ["new", "tool"])

    def test_leftovers_that_are_plain_files_do_not_stop_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "new").mkdir()
            (root / "new/file").write_text("new")
            dest = root / "tool"
            dest.mkdir()
            (root / "tool.new").write_text("stray")
            (root / "tool.old").write_text("stray")
            install_tools.replace_dir(root / "new", dest)
            self.assertEqual((dest / "file").read_text(), "new")
            self.assertEqual(sorted(p.name for p in root.iterdir()), ["new", "tool"])

    def test_a_failed_swap_puts_the_old_install_back(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "new").mkdir()
            dest = root / "tool"
            dest.mkdir()
            (dest / "file").write_text("old")
            rename = Path.rename

            def fail_on_the_new_tree(self, target):
                if self.name == "tool.new":
                    raise OSError("simulated")
                return rename(self, target)

            with mock.patch.object(Path, "rename", fail_on_the_new_tree):
                with self.assertRaises(OSError):
                    install_tools.replace_dir(root / "new", dest)
            self.assertEqual((dest / "file").read_text(), "old")


class Paths(unittest.TestCase):
    def test_prefix_keeps_everything_under_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            ctx = install_tools.Ctx(args(prefix=tmp))
            root = Path(tmp).resolve()
            self.assertEqual(ctx.prefix, root)
            self.assertEqual(ctx.bin, root / "bin")
            self.assertEqual(ctx.manifest, root / "manifest.json")

    def test_bin_dir_overrides(self):
        with tempfile.TemporaryDirectory() as tmp:
            ctx = install_tools.Ctx(args(prefix=tmp, bin_dir=str(Path(tmp) / "elsewhere")))
            self.assertEqual(ctx.bin, (Path(tmp) / "elsewhere").resolve())

    def test_manifest_round_trip_and_garbage(self):
        with tempfile.TemporaryDirectory() as tmp:
            ctx = install_tools.Ctx(args(prefix=tmp))
            self.assertIsNone(ctx.installed_version("ordo"))
            ctx.record("ordo", {"version": "v1.2.6"})
            ctx.record("cutechess", {"version": "v1.5.1"})
            self.assertEqual(ctx.installed_version("ordo"), "v1.2.6")
            self.assertEqual(ctx.installed_version("cutechess"), "v1.5.1")
            ctx.manifest.write_text("not json")
            self.assertIsNone(ctx.installed_version("ordo"))


class EngineBuild(unittest.TestCase):
    def test_the_order_is_argument_environment_file_default(self):
        with tempfile.TemporaryDirectory() as tmp:
            local = Path(tmp) / "local.env"
            local.write_text("# a comment\nCONCURRENCY=3\nENGINE_BUILD=pext\n", encoding="utf-8")
            empty = Path(tmp) / "none.env"
            with mock.patch.dict(os.environ, {}, clear=False):
                os.environ.pop("VYVRAEL_ENGINE_BUILD", None)
                self.assertEqual(install_tools.engine_build(args(), empty), "avx2")
                self.assertEqual(install_tools.engine_build(args(), local), "pext")
                os.environ["VYVRAEL_ENGINE_BUILD"] = "avx2"
                self.assertEqual(install_tools.engine_build(args(), local), "avx2")
                self.assertEqual(
                    install_tools.engine_build(args(engine_build="pext"), empty), "pext")

    def test_an_unknown_build_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            local = Path(tmp) / "local.env"
            local.write_text("ENGINE_BUILD=native\n", encoding="utf-8")
            with mock.patch.dict(os.environ, {}, clear=False), \
                    redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                os.environ.pop("VYVRAEL_ENGINE_BUILD", None)
                install_tools.engine_build(args(), local)

    def test_the_file_may_have_a_byte_order_mark_or_be_utf16(self):
        with tempfile.TemporaryDirectory() as tmp:
            local = Path(tmp) / "local.env"
            text = "ENGINE_BUILD=pext\r\nCONCURRENCY=6\r\n"
            for data in (b"\xef\xbb\xbf" + text.encode("utf-8"),
                         text.encode("utf-16"),        # with its byte-order mark
                         text.encode("utf-8")):
                local.write_bytes(data)
                self.assertEqual(install_tools.local_setting("ENGINE_BUILD", local), "pext")
                self.assertEqual(install_tools.local_setting("CONCURRENCY", local), "6")
            local.write_bytes(b"ENGINE_BUILD=\xff\xff\n")
            with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                install_tools.local_setting("ENGINE_BUILD", local)

    def test_the_file_follows_vyvrael_local_env(self):
        with tempfile.TemporaryDirectory() as tmp:
            local = Path(tmp) / "elsewhere.env"
            local.write_text("ENGINE_BUILD=pext\n", encoding="utf-8")
            with mock.patch.dict(os.environ, {"VYVRAEL_LOCAL_ENV": str(local)}):
                self.assertEqual(install_tools.local_setting("ENGINE_BUILD"), "pext")
                self.assertIsNone(install_tools.local_setting("OPENINGS"))


class Shims(unittest.TestCase):
    def test_quotes_paths_arguments_and_environment(self):
        text = install_tools.shim_text(
            PurePosixPath("/opt/my tools/app"), ["cli", "two words"], {"KEY": "a b"}
        )
        self.assertEqual(
            text, "#!/bin/sh\nKEY='a b' exec '/opt/my tools/app' cli 'two words' \"$@\"\n"
        )

    def test_plain_launcher(self):
        text = install_tools.shim_text(PurePosixPath("/usr/bin/ordo"), [], {})
        self.assertEqual(text, '#!/bin/sh\nexec /usr/bin/ordo "$@"\n')

    def test_launcher_can_start_in_a_directory(self):
        # lichess-bot loads ./config.yml, so its launcher must not depend on
        # where it is called from.
        text = install_tools.shim_text(
            PurePosixPath("/opt/my tools/lichess-bot/venv/bin/python"),
            ["/opt/my tools/lichess-bot/lichess-bot.py"],
            {},
            PurePosixPath("/opt/my tools/lichess-bot"),
        )
        self.assertEqual(
            text,
            "#!/bin/sh\ncd '/opt/my tools/lichess-bot' && "
            "exec '/opt/my tools/lichess-bot/venv/bin/python' "
            "'/opt/my tools/lichess-bot/lichess-bot.py' \"$@\"\n",
        )

    def test_shim_passes_the_directory_through(self):
        with tempfile.TemporaryDirectory() as tmp:
            ctx = install_tools.Ctx(args(prefix=tmp))
            home = PurePosixPath("/opt/lichess-bot")
            path = ctx.shim("bot", PurePosixPath("/opt/python"), ["x.py"], cwd=home)
            self.assertEqual(
                path.read_text(), '#!/bin/sh\ncd /opt/lichess-bot && exec /opt/python x.py "$@"\n'
            )

    def test_shim_is_written_executable(self):
        with tempfile.TemporaryDirectory() as tmp:
            ctx = install_tools.Ctx(args(prefix=tmp))
            path = ctx.shim("ordo", Path("/usr/bin/ordo"))
            self.assertEqual(path, Path(tmp).resolve() / "bin/ordo")
            if os.name != "nt":
                self.assertTrue(path.stat().st_mode & stat.S_IXUSR)


CONFIG_DEFAULT = '''token: "xxxxxxxxxxxxxxxxxxxxxx"    # Lichess OAuth2 Token.
url: "https://lichess.org/"

engine:
  dir: "./engines/"
  name: "engine_name"
  protocol: "uci"
'''


class Config(unittest.TestCase):
    def test_sets_engine_and_blanks_the_token(self):
        text = install_tools.patch_config(CONFIG_DEFAULT, "/home/me/vyvrael/target/release", "vyvrael")
        self.assertIn('token: ""', text)
        self.assertNotIn("xxxxxxxx", text)
        self.assertIn('  dir: "/home/me/vyvrael/target/release"', text)
        self.assertIn('  name: "vyvrael"', text)
        self.assertIn('  protocol: "uci"', text)

    def test_awkward_paths_stay_valid_yaml_strings(self):
        text = install_tools.patch_config(CONFIG_DEFAULT, 'C:\\dir with "quotes"', "name")
        self.assertIn('  dir: "C:\\\\dir with \\"quotes\\""', text)

    def test_unknown_layout_is_an_error_not_a_guess(self):
        with self.assertRaises(SystemExit):
            install_tools.patch_config("engine:\n  path: x\n", "d", "n")

    def test_engine_location(self):
        with tempfile.TemporaryDirectory() as tmp:
            engine = Path(tmp) / "vyvrael"
            engine.write_text("")
            where = install_tools.engine_location(args(engine=str(engine)), Path(tmp) / "prefix")
            self.assertEqual(where, (str(engine.resolve().parent), "vyvrael"))

    def test_default_engine_location_is_outside_the_replaced_directory(self):
        # A reinstall replaces <prefix>/lichess-bot, so the engine the default
        # configuration points at must not live in there, nor depend on the
        # current directory.
        prefix = PurePosixPath("/home/me/tools")
        directory, name = install_tools.engine_location(args(), prefix)
        self.assertEqual((directory, name), ("/home/me/tools/engines", "vyvrael"))
        self.assertNotIn(prefix / "lichess-bot", PurePosixPath(directory, name).parents)


class CarryOver(unittest.TestCase):
    """What a lichess-bot reinstall brings across from the old directory."""

    def trees(self, root: Path) -> tuple[Path, Path]:
        old, new = root / "old", root / "new"
        for tree in (old, new):
            (tree / "lib").mkdir(parents=True)
            (tree / "engines").mkdir()
        (old / "lichess-bot.py").write_text("old release")
        (old / "lib/old_module.py").write_text("old release")
        (old / "config.yml").write_text("token: mine")
        (old / "venv/bin").mkdir(parents=True)
        (old / "venv/bin/python").write_text("venv")
        (old / "game_records").mkdir()
        (old / "game_records/one.pgn").write_text("1. e4")
        (old / "book.bin").write_text("book")
        (old / "__pycache__").mkdir()
        (old / "engines/in-release-dir").write_text("x")
        (new / "lichess-bot.py").write_text("new release")
        return old, new

    def test_keeps_config_venv_and_the_users_own_top_level_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            old, new = self.trees(Path(tmp))
            carried = install_tools.carry_over(old, new)
            self.assertEqual(carried, ["book.bin", "config.yml", "game_records", "venv"])
            self.assertEqual((new / "config.yml").read_text(), "token: mine")
            self.assertEqual((new / "venv/bin/python").read_text(), "venv")
            self.assertEqual((new / "game_records/one.pgn").read_text(), "1. e4")
            self.assertEqual((new / "book.bin").read_text(), "book")
            if os.name != "nt":
                self.assertEqual(stat.S_IMODE((new / "config.yml").stat().st_mode), 0o600)

    def test_what_the_release_provides_is_not_merged(self):
        with tempfile.TemporaryDirectory() as tmp:
            old, new = self.trees(Path(tmp))
            install_tools.carry_over(old, new)
            self.assertEqual((new / "lichess-bot.py").read_text(), "new release")
            self.assertFalse((new / "lib/old_module.py").exists())
            self.assertFalse((new / "engines/in-release-dir").exists())
            self.assertFalse((new / "__pycache__").exists())

    def test_config_and_venv_win_over_files_of_the_same_name_in_the_release(self):
        with tempfile.TemporaryDirectory() as tmp:
            old, new = self.trees(Path(tmp))
            (new / "config.yml").write_text("from the release")
            (new / "venv").mkdir()
            (new / "venv/stray").write_text("x")
            install_tools.carry_over(old, new)
            self.assertEqual((new / "config.yml").read_text(), "token: mine")
            self.assertFalse((new / "venv/stray").exists())

    def test_a_first_install_carries_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "new").mkdir()
            self.assertEqual(install_tools.carry_over(Path(tmp) / "missing", Path(tmp) / "new"), [])

    def test_the_whole_reinstall_keeps_them_through_the_swap(self):
        with tempfile.TemporaryDirectory() as tmp:
            old, new = self.trees(Path(tmp))
            install_tools.carry_over(old, new)
            install_tools.replace_dir(new, old)
            self.assertEqual((old / "lichess-bot.py").read_text(), "new release")
            self.assertEqual((old / "config.yml").read_text(), "token: mine")
            self.assertEqual((old / "game_records/one.pgn").read_text(), "1. e4")


class Systemd(unittest.TestCase):
    def test_quoting(self):
        self.assertEqual(install_tools.systemd_quote("/a b/c"), '"/a b/c"')
        self.assertEqual(install_tools.systemd_quote('say "hi"'), '"say \\"hi\\""')
        self.assertEqual(install_tools.systemd_quote("100%"), '"100%%"')

    def test_dollar_signs_are_not_left_for_systemd_to_expand(self):
        self.assertEqual(install_tools.systemd_quote("/a/${X}/b"), '"/a/$${X}/b"')
        self.assertEqual(install_tools.systemd_quote("$HOME"), '"$$HOME"')
        unit = install_tools.systemd_unit(
            PurePosixPath("/opt/${X}/venv/bin/python"),
            PurePosixPath("/opt/${X}/lichess-bot.py"),
            PurePosixPath("/opt/${X}"),
            PurePosixPath("/home/me/.config/vyvrael/lichess-token.env"),
        )
        self.assertIn(
            'ExecStart="/opt/$${X}/venv/bin/python" "/opt/$${X}/lichess-bot.py"\n', unit
        )
        # Only command lines are expanded; a path setting keeps its dollar.
        self.assertIn("WorkingDirectory=/opt/${X}\n", unit)

    def test_unit_quotes_exec_start_and_is_not_enabled_by_writing(self):
        unit = install_tools.systemd_unit(
            PurePosixPath("/opt/my tools/venv/bin/python"),
            PurePosixPath("/opt/my tools/lichess-bot.py"),
            PurePosixPath("/opt/my tools"),
            PurePosixPath("/home/me/.config/vyvrael/lichess-token.env"),
        )
        self.assertIn(
            'ExecStart="/opt/my tools/venv/bin/python" "/opt/my tools/lichess-bot.py"\n', unit
        )
        self.assertIn("WorkingDirectory=/opt/my tools\n", unit)
        self.assertIn("EnvironmentFile=/home/me/.config/vyvrael/lichess-token.env\n", unit)
        self.assertEqual(len(re.findall(r"(?m)^\[(Unit|Service|Install)\]$", unit)), 3)

    def test_unit_gives_up_on_a_bot_that_keeps_failing(self):
        unit = install_tools.systemd_unit(
            PurePosixPath("/opt/venv/bin/python"),
            PurePosixPath("/opt/lichess-bot.py"),
            PurePosixPath("/opt"),
            PurePosixPath("/home/me/.config/vyvrael/lichess-token.env"),
        )
        sections = dict(
            (name, body)
            for name, body in re.findall(r"(?ms)^\[(\w+)\]\n(.*?)(?=^\[|\Z)", unit)
        )
        # Restarted after a crash, but never after a clean stop, and not for ever.
        self.assertIn("Restart=on-failure\n", sections["Service"])
        self.assertNotIn("Restart=always", unit)
        self.assertIn("RestartSec=30\n", sections["Service"])
        # The start limit belongs to [Unit]; in [Service] systemd ignores it.
        self.assertIn("StartLimitIntervalSec=600\n", sections["Unit"])
        self.assertIn("StartLimitBurst=3\n", sections["Unit"])
        self.assertNotIn("StartLimit", sections["Service"])


class TokenFile(unittest.TestCase):
    """The service's token file, written under a temporary XDG_CONFIG_HOME."""

    def write(self, tmp: str) -> Path:
        with mock.patch.dict(os.environ, {"XDG_CONFIG_HOME": tmp}), redirect_stdout(io.StringIO()):
            install_tools.write_systemd_unit(Path(tmp) / "lichess-bot", Path(tmp) / "python")
        return Path(tmp) / "vyvrael/lichess-token.env"

    def test_template_does_not_set_the_variable(self):
        # lichess-bot overrides config.yml's token whenever the variable is
        # present, so the template must not define it, not even as empty.
        with tempfile.TemporaryDirectory() as tmp:
            env_file = self.write(tmp)
            lines = env_file.read_text().splitlines()
            active = [line for line in lines if line.strip() and not line.lstrip().startswith("#")]
            self.assertEqual(active, [])
            self.assertIn("# LICHESS_BOT_TOKEN=", lines)
            self.assertTrue((Path(tmp) / "systemd/user/vyvrael-lichess-bot.service").is_file())
            if os.name != "nt":
                self.assertEqual(stat.S_IMODE(env_file.stat().st_mode), 0o600)

    def test_an_existing_token_file_is_not_rewritten(self):
        with tempfile.TemporaryDirectory() as tmp:
            env_file = Path(tmp) / "vyvrael/lichess-token.env"
            env_file.parent.mkdir()
            env_file.write_text("LICHESS_BOT_TOKEN=placeholder\n")
            self.write(tmp)
            self.assertEqual(env_file.read_text(), "LICHESS_BOT_TOKEN=placeholder\n")


class Private(unittest.TestCase):
    def test_token_file_is_owner_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "deep/token.env"
            install_tools.write_private(path, "LICHESS_BOT_TOKEN=\n")
            self.assertEqual(path.read_text(), "LICHESS_BOT_TOKEN=\n")
            if os.name != "nt":
                self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)


def run_main(argv: list[str]) -> str:
    """Runs the installer's `main` offline and returns what it printed.

    Downloading and running programs are replaced by mocks that fail the
    test, so a mistake here can never fetch, build or start anything. The
    system is given as Linux, whatever the tests run on: these tests are of
    the Linux installer, and on Windows `main` takes another path (tested in
    test_windows_tools.py).
    """
    forbidden = AssertionError("the test tried to leave the sandbox")
    with mock.patch.object(install_tools, "download", side_effect=forbidden), mock.patch.object(
        install_tools.subprocess, "run", side_effect=forbidden
    ), mock.patch.object(sys, "platform", "linux"), redirect_stdout(io.StringIO()) as out:
        install_tools.main(argv)
    return out.getvalue()


def record_as_installed(prefix: str, tool: str) -> None:
    """Makes `prefix` look as if the pinned version of `tool` were installed."""
    pin = install_tools.PINS[tool]
    ctx = install_tools.Ctx(args(prefix=prefix))
    for path in install_tools.tool_files(ctx, tool):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("")
    info = {"version": pin["version"], "sha256": pin["sha256"], "path": str(ctx.prefix / tool)}
    if "commit" in pin:
        info["commit"] = pin["commit"]
    ctx.record(tool, info)


class DryRun(unittest.TestCase):
    def test_reports_already_installed_like_a_real_run(self):
        for tool in install_tools.TOOLS:
            with tempfile.TemporaryDirectory() as tmp:
                record_as_installed(tmp, tool)
                out = run_main(["--prefix", tmp, "--dry-run", "--tool", tool])
                version = install_tools.PINS[tool]["version"]
                self.assertIn(f"{tool}: {version} already installed", out)
                self.assertNotIn("would install", out, tool)
                self.assertNotIn("would build", out, tool)

    def test_reports_an_install_when_nothing_is_installed(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = run_main(["--prefix", tmp, "--dry-run", "--tool", "cutechess"])
            self.assertIn("[dry-run] would install", out)
            self.assertNotIn("already installed", out)

    def test_force_is_reported_as_a_reinstall(self):
        with tempfile.TemporaryDirectory() as tmp:
            record_as_installed(tmp, "cutechess")
            out = run_main(["--prefix", tmp, "--dry-run", "--force", "--tool", "cutechess"])
            self.assertIn("[dry-run] would install", out)

    def test_writes_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            # The default tools; lichess-bot would stop on a Python before 3.11.
            out = run_main(["--prefix", tmp, "--dry-run"])
            self.assertIn("cutechess", out)
            self.assertIn("ordo", out)
            self.assertEqual(list(Path(tmp).iterdir()), [])


class Installed(unittest.TestCase):
    """When a tool counts as installed."""

    def ctx(self, tmp: str):
        return install_tools.Ctx(args(prefix=tmp))

    def test_recorded_pin_with_its_files_is_installed(self):
        for tool in install_tools.TOOLS:
            with tempfile.TemporaryDirectory() as tmp:
                self.assertFalse(install_tools.is_installed(self.ctx(tmp), tool), tool)
                record_as_installed(tmp, tool)
                self.assertTrue(install_tools.is_installed(self.ctx(tmp), tool), tool)

    def test_another_version_is_not(self):
        with tempfile.TemporaryDirectory() as tmp:
            record_as_installed(tmp, "ordo")
            ctx = self.ctx(tmp)
            ctx.record("ordo", dict(ctx.installed_entry("ordo"), version="v0"))
            self.assertFalse(install_tools.is_installed(ctx, "ordo"))

    def test_the_same_version_at_another_commit_is_not(self):
        with tempfile.TemporaryDirectory() as tmp:
            record_as_installed(tmp, "lichess-bot")
            ctx = self.ctx(tmp)
            ctx.record("lichess-bot", dict(ctx.installed_entry("lichess-bot"), commit="0" * 40))
            self.assertFalse(install_tools.is_installed(ctx, "lichess-bot"))

    def test_the_same_version_with_another_pinned_digest_is_not(self):
        with tempfile.TemporaryDirectory() as tmp:
            record_as_installed(tmp, "cutechess")
            ctx = self.ctx(tmp)
            ctx.record("cutechess", dict(ctx.installed_entry("cutechess"), sha256="0" * 64))
            self.assertFalse(install_tools.is_installed(ctx, "cutechess"))

    def test_a_recorded_tool_whose_files_are_gone_is_not(self):
        for tool in install_tools.TOOLS:
            for index in range(len(install_tools.tool_files(self.ctx("."), tool))):
                with tempfile.TemporaryDirectory() as tmp:
                    record_as_installed(tmp, tool)
                    install_tools.tool_files(self.ctx(tmp), tool)[index].unlink()
                    self.assertFalse(install_tools.is_installed(self.ctx(tmp), tool), tool)


class Rerun(unittest.TestCase):
    """A real run over tools that are already installed.

    `main` refuses to run for real off Linux, so the platform is patched;
    the installers are replaced by mocks, so nothing can be fetched.
    """

    def run_installed(self, tmp: str, argv: list[str], tools=install_tools.TOOLS, check=True):
        """Runs `main` with prefix <tmp>/prefix and configuration home
        <tmp>/config, after recording `tools` there as installed. A mocked
        installer returns MOCK_RECORD and the check answers `check`."""
        for tool in tools:
            record_as_installed(self.prefix(tmp), tool)
        installers = {
            tool: mock.Mock(return_value=dict(self.MOCK_RECORD)) for tool in install_tools.TOOLS
        }
        with mock.patch.object(sys, "platform", "linux"), mock.patch.dict(
            install_tools.INSTALLERS, installers
        ), mock.patch.dict(
            os.environ, {"XDG_CONFIG_HOME": str(Path(tmp) / "config")}
        ), mock.patch.object(install_tools, "verify", return_value=check):
            out = run_main(["--prefix", self.prefix(tmp), *argv])
        return out, installers

    MOCK_RECORD = {"version": "from the mocked installer"}

    def manifest(self, tmp: str) -> dict:
        ctx = install_tools.Ctx(args(prefix=self.prefix(tmp)))
        return {tool: ctx.installed_entry(tool) for tool in install_tools.TOOLS}

    def test_a_tool_is_recorded_once_its_check_passed(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.run_installed(tmp, ["--tool", "ordo"], tools=[])
            self.assertEqual(self.manifest(tmp)["ordo"], self.MOCK_RECORD)

    def test_a_failed_check_is_an_error_and_records_nothing(self):
        with tempfile.TemporaryDirectory() as tmp, redirect_stderr(io.StringIO()) as err:
            with self.assertRaises(SystemExit) as stopped:
                self.run_installed(tmp, ["--tool", "ordo"], tools=[], check=False)
            self.assertEqual(stopped.exception.code, 1)
            self.assertIn("the check failed for ordo", err.getvalue())
            self.assertEqual(self.manifest(tmp)["ordo"], {})

    def test_a_failed_check_after_a_reinstall_drops_the_old_record(self):
        # Otherwise the next run would call the broken tool "already installed".
        with tempfile.TemporaryDirectory() as tmp, redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit):
                self.run_installed(tmp, ["--force", "--tool", "ordo"], check=False)
            manifest = self.manifest(tmp)
            self.assertEqual(manifest["ordo"], {})
            self.assertFalse(
                install_tools.is_installed(install_tools.Ctx(args(prefix=self.prefix(tmp))), "ordo")
            )
            # The other tools' records are untouched.
            self.assertEqual(
                manifest["cutechess"]["version"], install_tools.PINS["cutechess"]["version"]
            )

    def test_one_failed_check_does_not_stop_the_other_tools(self):
        with tempfile.TemporaryDirectory() as tmp, redirect_stderr(io.StringIO()) as err:
            with self.assertRaises(SystemExit):
                self.run_installed(tmp, [], tools=[], check=False)
            self.assertIn("the check failed for cutechess, ordo", err.getvalue())

    def prefix(self, tmp: str) -> str:
        return str(Path(tmp).resolve() / "prefix")

    def test_rewrites_launchers_into_a_new_bin_dir(self):
        with tempfile.TemporaryDirectory() as tmp:
            bin_dir = Path(tmp).resolve() / "elsewhere"
            out, installers = self.run_installed(tmp, ["--bin-dir", str(bin_dir)])
            self.assertEqual(
                sorted(p.name for p in bin_dir.iterdir()), ["cutechess", "cutechess-cli", "ordo"]
            )
            for installer in installers.values():
                installer.assert_not_called()
            self.assertIn("ordo: v1.2.6 already installed", out)
            # The closing hint names the directory the launchers really are in.
            self.assertIn(f"launchers are in {bin_dir}", out)

    def test_restores_a_deleted_launcher(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.run_installed(tmp, ["--tool", "ordo"])
            launcher = Path(self.prefix(tmp)) / "bin/ordo"
            text = launcher.read_text()
            self.assertIn("exec ", text)
            launcher.unlink()
            self.run_installed(tmp, ["--tool", "ordo"])
            self.assertEqual(launcher.read_text(), text)

    def test_systemd_is_honoured_without_force(self):
        with tempfile.TemporaryDirectory() as tmp:
            out, installers = self.run_installed(tmp, ["--tool", "lichess-bot", "--systemd"])
            installers["lichess-bot"].assert_not_called()
            unit = Path(tmp) / "config/systemd/user/vyvrael-lichess-bot.service"
            self.assertIn("ExecStart=", unit.read_text())
            self.assertTrue((Path(tmp) / "config/vyvrael/lichess-token.env").is_file())
            launcher = (Path(self.prefix(tmp)) / "bin/vyvrael-lichess-bot").read_text()
            self.assertIn("\ncd ", launcher)
            self.assertIn("NOT enabled or started", out)

    def test_no_service_without_systemd(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.run_installed(tmp, ["--tool", "lichess-bot"])
            self.assertTrue((Path(self.prefix(tmp)) / "bin/vyvrael-lichess-bot").is_file())
            self.assertFalse((Path(tmp) / "config").exists())

    def test_says_that_engine_was_not_applied(self):
        with tempfile.TemporaryDirectory() as tmp:
            argv = ["--tool", "lichess-bot", "--engine", str(Path(tmp) / "vyvrael")]
            out, _ = self.run_installed(tmp, argv)
            self.assertIn("--engine is only used when config.yml is first written", out)
            out, _ = self.run_installed(tmp, ["--tool", "lichess-bot"])
            self.assertNotIn("--engine is only used", out)

    def test_a_stale_record_runs_the_installer(self):
        with tempfile.TemporaryDirectory() as tmp:
            record_as_installed(self.prefix(tmp), "ordo")
            ctx = install_tools.Ctx(args(prefix=self.prefix(tmp)))
            ctx.record("ordo", dict(ctx.installed_entry("ordo"), commit="0" * 40))
            out, installers = self.run_installed(tmp, ["--tool", "ordo"], tools=[])
            installers["ordo"].assert_called_once()
            self.assertNotIn("already installed", out)

    def test_a_dry_run_only_says_what_it_would_rewrite(self):
        with tempfile.TemporaryDirectory() as tmp:
            argv = ["--dry-run", "--tool", "lichess-bot", "--systemd"]
            out, _ = self.run_installed(tmp, argv)
            self.assertIn("[dry-run] would rewrite its launchers in", out)
            self.assertIn("and the user service", out)
            self.assertFalse((Path(self.prefix(tmp)) / "bin").exists())
            self.assertFalse((Path(tmp) / "config").exists())


class Checks(unittest.TestCase):
    """The check run after an install. The program itself is mocked."""

    def verify(self, tool: str, **run) -> tuple[bool, str, mock.Mock]:
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(
            install_tools.subprocess, "run", **run
        ) as runner, redirect_stdout(io.StringIO()) as out:
            passed = install_tools.verify(install_tools.Ctx(args(prefix=tmp)), tool)
        return passed, out.getvalue(), runner

    def finished(self, status: int, stdout: str = "", stderr: str = ""):
        return install_tools.subprocess.CompletedProcess([], status, stdout, stderr)

    def test_passes_when_the_program_runs_and_exits_with_zero(self):
        result = self.finished(0, "cutechess-cli 1.5.1\nmore\n")
        passed, out, runner = self.verify("cutechess", return_value=result)
        self.assertIs(passed, True)
        self.assertEqual(out, "  check: cutechess-cli 1.5.1\n")
        command = runner.call_args.args[0]
        self.assertEqual((Path(command[0]).name, command[1:]), ("cutechess-cli", ["--version"]))

    def test_fails_when_the_shell_could_not_execute_or_find_the_program(self):
        for status, message in ((126, "Exec format error"), (127, "not found")):
            result = self.finished(status, stderr=message + "\n")
            passed, out, _ = self.verify("ordo", return_value=result)
            self.assertIs(passed, False, status)
            self.assertIn("check FAILED", out)
            self.assertIn(f"status {status}", out)
            self.assertIn(message, out)

    def test_any_other_exit_status_counts_as_started_and_is_shown(self):
        # Nobody has confirmed that these commands end with status 0.
        for status in (1, 2, 125, 128, 255, -11):
            result = self.finished(status, "ordo 1.2.6\n")
            passed, out, _ = self.verify("ordo", return_value=result)
            self.assertIs(passed, True, status)
            self.assertEqual(out, f"  check: ordo 1.2.6 (exit status {status})\n")

    def test_fails_when_the_program_cannot_be_started_or_hangs(self):
        timeout = install_tools.subprocess.TimeoutExpired("ordo", 120)
        for problem in (OSError("no such file"), timeout):
            passed, out, _ = self.verify("ordo", side_effect=problem)
            self.assertIs(passed, False, problem)
            self.assertIn("check FAILED", out)

    def test_a_tool_without_a_check_passes_without_running_anything(self):
        passed, out, runner = self.verify("lichess-bot")
        self.assertIs(passed, True)
        self.assertEqual(out, "")
        runner.assert_not_called()

    def test_forget_removes_only_that_tool(self):
        with tempfile.TemporaryDirectory() as tmp:
            ctx = install_tools.Ctx(args(prefix=tmp))
            ctx.forget("ordo")  # nothing recorded, nothing written
            self.assertFalse(ctx.manifest.exists())
            ctx.record("ordo", {"version": "v1.2.6"})
            ctx.record("cutechess", {"version": "v1.5.1"})
            ctx.forget("ordo")
            self.assertIsNone(ctx.installed_version("ordo"))
            self.assertEqual(ctx.installed_version("cutechess"), "v1.5.1")


class Machine(unittest.TestCase):
    """The pinned Cute Chess file is an x86_64 AppImage."""

    def install(self, machine: str, dry_run: bool) -> str:
        forbidden = AssertionError("the test tried to leave the sandbox")
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(
            install_tools.platform, "machine", return_value=machine
        ), mock.patch.object(install_tools, "download", side_effect=forbidden), redirect_stdout(
            io.StringIO()
        ) as out:
            ctx = install_tools.Ctx(args(prefix=tmp, dry_run=dry_run))
            self.assertIsNone(install_tools.install_cutechess(ctx))
            self.assertEqual(list(Path(tmp).iterdir()), [])
        return out.getvalue()

    def test_a_real_run_stops_before_downloading_on_another_machine(self):
        with self.assertRaises(SystemExit), redirect_stderr(io.StringIO()) as err:
            self.install("aarch64", dry_run=False)
        self.assertIn("x86_64 only; this machine is aarch64", err.getvalue())

    def test_a_dry_run_says_a_real_run_would_stop(self):
        out = self.install("aarch64", dry_run=True)
        self.assertIn("this machine is aarch64, not x86_64", out)

    def test_x86_64_is_accepted_under_both_of_its_names(self):
        for machine in ("x86_64", "AMD64"):
            out = self.install(machine, dry_run=True)
            self.assertIn("[dry-run] would install", out)
            self.assertNotIn("not x86_64", out)


class CommandLine(unittest.TestCase):
    def test_parser_defaults(self):
        parsed = install_tools.build_parser().parse_args([])
        self.assertIsNone(parsed.tool)
        self.assertFalse(parsed.systemd)
        self.assertFalse(parsed.force)
        self.assertFalse(parsed.dry_run)

    def test_unknown_tool_is_rejected(self):
        with self.assertRaises(SystemExit):
            install_tools.build_parser().parse_args(["--tool", "stockfish"])


if __name__ == "__main__":
    unittest.main()
