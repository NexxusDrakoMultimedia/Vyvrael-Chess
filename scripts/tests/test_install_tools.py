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
