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
from pathlib import Path, PurePosixPath

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import install_tools  # noqa: E402


def args(**overrides) -> argparse.Namespace:
    base = dict(prefix=None, bin_dir=None, engine=None, systemd=False, dry_run=False)
    base.update(overrides)
    return argparse.Namespace(**base)


def tar_with(path: Path, members: list[tuple[str, bytes]], links: list[tuple[str, str]] = ()) -> None:
    with tarfile.open(path, "w:gz") as tar:
        for name, data in members:
            info = tarfile.TarInfo(name)
            info.size = len(data)
            tar.addfile(info, io.BytesIO(data))
        for name, target in links:
            info = tarfile.TarInfo(name)
            info.type = tarfile.SYMTYPE
            info.linkname = target
            tar.addfile(info)


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
        self.assertEqual(install_tools.engine_location(args()), ("./engines/", "vyvrael"))
        with tempfile.TemporaryDirectory() as tmp:
            engine = Path(tmp) / "vyvrael"
            engine.write_text("")
            where = install_tools.engine_location(args(engine=str(engine)))
            self.assertEqual(where, (str(engine.resolve().parent), "vyvrael"))


class Systemd(unittest.TestCase):
    def test_quoting(self):
        self.assertEqual(install_tools.systemd_quote("/a b/c"), '"/a b/c"')
        self.assertEqual(install_tools.systemd_quote('say "hi"'), '"say \\"hi\\""')
        self.assertEqual(install_tools.systemd_quote("100%"), '"100%%"')

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
