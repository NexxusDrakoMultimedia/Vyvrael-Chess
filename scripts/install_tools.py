#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Vyvræl tooling installer for Linux: Cute Chess, Ordo and lichess-bot.

Installs the three external programs the project uses for strength testing
and for playing on Lichess. They are separate programs: nothing is vendored
into the repository or linked into the engine. Everything goes under a
per-user prefix outside the repository.

Standard library only, Python 3.9+ (lichess-bot itself needs 3.11+).
Safe to re-run; use --force to reinstall.

    python3 scripts/install_tools.py --list
    python3 scripts/install_tools.py --dry-run
    python3 scripts/install_tools.py                      # cutechess + ordo
    python3 scripts/install_tools.py --tool lichess-bot --engine path/to/vyvrael

Every tool is pinned to an exact version in PINS below, so two runs install
the same thing. Updating a tool means editing PINS in a reviewed commit.

It never: writes your Lichess token anywhere (the only token file it makes is
an owner-only template that sets nothing), upgrades the account to BOT (irreversible; run
`vyvrael-lichess-bot -u` yourself), or enables or starts any service.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shlex
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
from pathlib import Path

# What gets installed. Change these only in a reviewed commit, and record the
# new versions in docs/DEPENDENCIES.md.
#
# `sha256` is the digest of the downloaded file. When it is set, a mismatch
# aborts the install. Cute Chess publishes digests for its release files.
# Ordo and lichess-bot are fetched as source archives of one exact commit;
# GitHub publishes no digest for those, so theirs start as None: the commit
# hash fixes what is fetched, and the installer prints the digest it saw so
# it can be recorded here, after which it is enforced.
PINS = {
    "cutechess": {
        "repo": "cutechess/cutechess",
        "version": "v1.5.1",
        "url": "https://github.com/cutechess/cutechess/releases/download/"
        "v1.5.1/Cute_Chess-1.5.1-x86_64.AppImage",
        "sha256": "d9448693e45bd57f1aeb32c46e94466894cd7cc5b6937effd285a02e871387b5",
        "licence": "GPL-3.0",
    },
    "ordo": {
        "repo": "michiguel/Ordo",
        "version": "v1.2.6",
        "commit": "17eec774f2e4b9fdd2b1b38739f55ea221fb851a",
        "sha256": None,
        "licence": "GPL-3.0",
    },
    "lichess-bot": {
        "repo": "lichess-bot-devs/lichess-bot",
        "version": "2026.11.16.2",
        "commit": "06f4f10b5a5a70549b9fc5f8f759a532723b5dab",
        "sha256": None,
        "licence": "AGPL-3.0",
    },
}
TOOLS = list(PINS)
DEFAULT_TOOLS = ["cutechess", "ordo"]
CODELOAD = "https://codeload.github.com"


def say(msg: str) -> None:
    print(msg, flush=True)


def die(msg: str) -> None:
    print(f"error: {msg}", file=sys.stderr)
    sys.exit(1)


def source_url(tool: str) -> str:
    """The source archive of the pinned commit of `tool`."""
    pin = PINS[tool]
    return f"{CODELOAD}/{pin['repo']}/tar.gz/{pin['commit']}"


# ---------------------------------------------------------------- network
def download(url: str, dest: Path, sha256: str | None) -> str:
    """Downloads `url` to `dest` and returns its SHA-256.

    With `sha256` given, a mismatch deletes the file and aborts. Without it
    the digest is printed so it can be pinned.
    """
    if not url.startswith("https://"):
        die(f"refusing to download over anything but HTTPS: {url}")
    say(f"  downloading {url}")
    digest = hashlib.sha256()
    request = urllib.request.Request(url, headers={"User-Agent": "install-tools"})
    with urllib.request.urlopen(request, timeout=60) as response, open(dest, "wb") as out:
        for chunk in iter(lambda: response.read(1 << 20), b""):
            digest.update(chunk)
            out.write(chunk)
    seen = digest.hexdigest()
    if sha256 is None:
        say(f"  WARNING: no pinned digest for this file. sha256 seen: {seen}")
        say("           Record it in PINS to have it checked from now on.")
    elif seen != sha256.lower():
        dest.unlink(missing_ok=True)
        die(f"checksum mismatch for {url}\n  expected {sha256}\n  got      {seen}")
    else:
        say("  sha256 verified")
    return seen


# ---------------------------------------------------------------- archives
def safe_extract(archive: Path, dest: Path) -> None:
    """Extracts a tar archive, refusing entries that would land outside `dest`."""
    dest = dest.resolve()
    dest.mkdir(parents=True, exist_ok=True)

    def check(name: str) -> None:
        target = (dest / name).resolve()
        if target != dest and dest not in target.parents:
            die(f"unsafe path in archive: {name}")

    with tarfile.open(archive) as tar:
        for member in tar.getmembers():
            check(member.name)
            if member.issym():
                # A symlink target is relative to the link's own directory.
                check(os.path.join(os.path.dirname(member.name), member.linkname))
            elif member.islnk():
                # A hard link target is relative to the archive root.
                check(member.linkname)
            if member.isdev():
                die(f"device file in archive: {member.name}")
        if hasattr(tarfile, "data_filter"):
            tar.extractall(dest, filter="data")
        else:
            tar.extractall(dest)


def single_subdir(path: Path) -> Path:
    """The only directory inside `path`, or `path` itself."""
    kids = list(path.iterdir())
    return kids[0] if len(kids) == 1 and kids[0].is_dir() else path


def make_exec(path: Path) -> None:
    path.chmod(path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def replace_dir(new: Path, dest: Path) -> None:
    """Puts the directory `new` in place of `dest`.

    The new tree is copied next to `dest` first and only then swapped in, so
    a failure while copying leaves the existing install untouched.
    """
    staged = dest.with_name(dest.name + ".new")
    retired = dest.with_name(dest.name + ".old")
    for leftover in (staged, retired):
        if leftover.exists():
            shutil.rmtree(leftover)
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(new, staged, symlinks=True)
    if dest.exists():
        dest.rename(retired)
    staged.rename(dest)
    if retired.exists():
        shutil.rmtree(retired)


# ---------------------------------------------------------------- paths
class Ctx:
    """Where things go, worked out once from the command line."""

    def __init__(self, args):
        self.args = args
        data_home = os.environ.get("XDG_DATA_HOME") or str(Path.home() / ".local/share")
        if args.prefix:
            self.prefix = Path(args.prefix).expanduser().resolve()
        else:
            self.prefix = Path(data_home) / "vyvrael-tools"
        # An explicit --prefix keeps everything under it, launchers included.
        # Without one, launchers go where a user's PATH usually already looks.
        if args.bin_dir:
            self.bin = Path(args.bin_dir).expanduser().resolve()
        elif args.prefix:
            self.bin = self.prefix / "bin"
        else:
            self.bin = Path.home() / ".local/bin"
        self.dry = args.dry_run
        self.manifest = self.prefix / "manifest.json"

    def _read_manifest(self) -> dict:
        try:
            data = json.loads(self.manifest.read_text())
        except (OSError, ValueError):
            return {}
        return data if isinstance(data, dict) else {}

    def installed_version(self, tool: str) -> str | None:
        entry = self._read_manifest().get(tool)
        return entry.get("version") if isinstance(entry, dict) else None

    def record(self, tool: str, info: dict) -> None:
        manifest = self._read_manifest()
        manifest[tool] = info
        self.manifest.parent.mkdir(parents=True, exist_ok=True)
        self.manifest.write_text(json.dumps(manifest, indent=2) + "\n")

    def shim(self, name: str, target: Path, args: list[str] = (), env: dict = None) -> Path:
        """Writes a launcher `name` in the bin directory that runs `target`."""
        self.bin.mkdir(parents=True, exist_ok=True)
        path = self.bin / name
        path.write_text(shim_text(target, list(args), env or {}))
        make_exec(path)
        return path


def shim_text(target: Path, args: list[str], env: dict) -> str:
    """The text of a POSIX shell launcher. Every path and argument is quoted."""
    assignments = "".join(f"{key}={shlex.quote(value)} " for key, value in env.items())
    fixed = "".join(shlex.quote(arg) + " " for arg in args)
    return f'#!/bin/sh\n{assignments}exec {shlex.quote(str(target))} {fixed}"$@"\n'


# ---------------------------------------------------------------- installers
def install_cutechess(ctx: Ctx) -> None:
    pin = PINS["cutechess"]
    dest = ctx.prefix / "cutechess"
    name = pin["url"].rsplit("/", 1)[-1]
    say(f"cutechess {pin['version']}")
    if ctx.dry:
        return say(f"  [dry-run] would install {pin['url']} -> {dest}")
    with tempfile.TemporaryDirectory() as tmp:
        staged = Path(tmp) / "cutechess"
        staged.mkdir()
        app = staged / name
        download(pin["url"], app, pin["sha256"])
        make_exec(app)
        replace_dir(staged, dest)
    app = dest / name
    # EXTRACT_AND_RUN lets the AppImage run without libfuse2, which recent
    # Ubuntu releases do not install. The AppImage bundles both programs; the
    # command-line one is selected with a leading "cli" argument. If the
    # check below fails, run the AppImage with --appimage-help to see how
    # this release exposes cutechess-cli.
    env = {"APPIMAGE_EXTRACT_AND_RUN": "1"}
    ctx.shim("cutechess-cli", app, ["cli"], env)
    ctx.shim("cutechess", app, [], env)
    ctx.record("cutechess", {"version": pin["version"], "sha256": pin["sha256"], "path": str(dest)})


def install_ordo(ctx: Ctx) -> None:
    pin = PINS["ordo"]
    dest = ctx.prefix / "ordo"
    url = source_url("ordo")
    say(f"ordo {pin['version']} (building from source, commit {pin['commit'][:12]})")
    missing = [tool for tool in ("gcc", "make") if not shutil.which(tool)]
    if ctx.dry:
        say(f"  [dry-run] would build {url} -> {dest}")
        if missing:
            say(f"  [dry-run] note: {', '.join(missing)} not found; a real run would stop here")
        return
    if missing:
        die(f"{', '.join(missing)} not found; run: sudo apt install build-essential")
    with tempfile.TemporaryDirectory() as tmp:
        archive = Path(tmp) / "ordo.tar.gz"
        seen = download(url, archive, pin["sha256"])
        safe_extract(archive, Path(tmp) / "src")
        src = single_subdir(Path(tmp) / "src")
        subprocess.run(["make", "-C", str(src)], check=True)
        built = src / "ordo"
        if not built.is_file():
            die("make finished but did not produce ./ordo")
        staged = Path(tmp) / "ordo-install"
        staged.mkdir()
        shutil.copy2(built, staged / "ordo")
        make_exec(staged / "ordo")
        replace_dir(staged, dest)
    ctx.shim("ordo", dest / "ordo")
    ctx.record(
        "ordo",
        {"version": pin["version"], "commit": pin["commit"], "sha256": seen, "path": str(dest)},
    )


def engine_location(args) -> tuple[str, str]:
    """(directory, file name) of the engine for lichess-bot's configuration."""
    if args.engine:
        engine = Path(args.engine).expanduser().resolve()
        if not engine.is_file():
            say(f"  warning: no engine at {engine} yet (build Vyvræl first; config still written)")
        return str(engine.parent), engine.name
    return "./engines/", "vyvrael"


def patch_config(text: str, engine_dir: str, engine_name: str) -> str:
    """Sets the engine directory and name and blanks the token in the text of
    lichess-bot's `config.yml.default`."""

    def quoted(value: str) -> str:
        return json.dumps(value)  # a YAML-safe double-quoted string

    text, dirs = re.subn(
        r'(?m)^(  dir:\s*)"[^"]*"', lambda m: m.group(1) + quoted(engine_dir), text, count=1
    )
    text, names = re.subn(
        r'(?m)^(  name:\s*)"[^"]*"', lambda m: m.group(1) + quoted(engine_name), text, count=1
    )
    text, tokens = re.subn(r'(?m)^token:\s*"[^"]*"', 'token: ""', text, count=1)
    if (dirs, names, tokens) != (1, 1, 1):
        die("config.yml.default has changed layout; edit config.yml by hand (dir, name, token)")
    return text


def systemd_quote(value: str) -> str:
    """Quotes one argument for a systemd `ExecStart=` line.

    systemd expands `%` specifiers and `$NAME` / `${NAME}` variables there,
    even inside double quotes, so both characters are doubled.
    """
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    escaped = escaped.replace("%", "%%").replace("$", "$$")
    return f'"{escaped}"'


def systemd_unit(python: Path, script: Path, workdir: Path, env_file: Path) -> str:
    """The text of the user service that runs lichess-bot."""
    start = f"{systemd_quote(str(python))} {systemd_quote(str(script))}"
    return (
        "[Unit]\n"
        "Description=Vyvrael on Lichess (lichess-bot)\n"
        "After=network-online.target\n"
        "Wants=network-online.target\n\n"
        "[Service]\n"
        "Environment=PYTHONUNBUFFERED=1\n"
        f"EnvironmentFile={str(env_file).replace('%', '%%')}\n"
        f"WorkingDirectory={str(workdir).replace('%', '%%')}\n"
        f"ExecStart={start}\n"
        "Restart=always\n"
        "RestartSec=10\n\n"
        "[Install]\n"
        "WantedBy=default.target\n"
    )


def write_private(path: Path, text: str) -> None:
    """Writes a file only its owner can read: it holds, or will hold, a token."""
    path.parent.mkdir(parents=True, exist_ok=True)
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w") as out:
        out.write(text)
    path.chmod(0o600)


def install_lichess_bot(ctx: Ctx) -> None:
    pin = PINS["lichess-bot"]
    dest = ctx.prefix / "lichess-bot"
    url = source_url("lichess-bot")
    say(f"lichess-bot {pin['version']} (commit {pin['commit'][:12]})")
    if sys.version_info < (3, 11):
        die("lichess-bot needs Python 3.11 or newer; this is " + sys.version.split()[0])
    if ctx.dry:
        return say(f"  [dry-run] would install {url} -> {dest}")

    config = dest / "config.yml"
    kept_config = config.read_text() if config.exists() else None
    with tempfile.TemporaryDirectory() as tmp:
        archive = Path(tmp) / "lichess-bot.tar.gz"
        seen = download(url, archive, pin["sha256"])
        safe_extract(archive, Path(tmp) / "src")
        src = single_subdir(Path(tmp) / "src")
        # Carry the user's configuration and the virtual environment across,
        # inside the staged tree, so the swap below is all or nothing.
        if kept_config is not None:
            write_private(src / "config.yml", kept_config)
        if (dest / "venv").exists():
            shutil.copytree(dest / "venv", src / "venv", symlinks=True)
        replace_dir(src, dest)

    if not (dest / "venv").exists():
        subprocess.run([sys.executable, "-m", "venv", str(dest / "venv")], check=True)
    python = dest / "venv/bin/python"
    subprocess.run(
        [str(python), "-m", "pip", "install", "--quiet", "-r", str(dest / "requirements.txt")],
        check=True,
    )

    if kept_config is not None:
        say("  kept your existing config.yml")
    else:
        engine_dir, engine_name = engine_location(ctx.args)
        template = (dest / "config.yml.default").read_text()
        write_private(config, patch_config(template, engine_dir, engine_name))
        say(f"  wrote config.yml (engine {Path(engine_dir, engine_name)}, token blank)")

    ctx.shim("vyvrael-lichess-bot", python, [str(dest / "lichess-bot.py")])
    if ctx.args.systemd:
        write_systemd_unit(dest, python)
    ctx.record(
        "lichess-bot",
        {"version": pin["version"], "commit": pin["commit"], "sha256": seen, "path": str(dest)},
    )


# The assignment is commented out on purpose. lichess-bot replaces the token
# from config.yml whenever LICHESS_BOT_TOKEN is present in its environment,
# empty or not, so a bare `LICHESS_BOT_TOKEN=` line would blank a token kept
# in config.yml as soon as the service is used.
TOKEN_ENV_TEMPLATE = (
    "# Lichess API token for the vyvrael-lichess-bot service (scope: bot:play).\n"
    "# To use it, remove the leading '# ' from the last line and put the token\n"
    "# after the '='. Leave the line commented out to use the token from\n"
    "# config.yml instead: while this variable is set, even to nothing, it\n"
    "# replaces that one.\n"
    "# LICHESS_BOT_TOKEN=\n"
)


def write_systemd_unit(dest: Path, python: Path) -> None:
    """Writes a user service and a token file template. Does not enable either.

    systemd only looks for user units under the user's configuration
    directory, so these two files are the one thing that cannot live under
    --prefix. That is why they are written only when --systemd is given.
    An existing token file is never rewritten.
    """
    config_home = Path(os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config")
    env_file = config_home / "vyvrael/lichess-token.env"
    if not env_file.exists():
        write_private(env_file, TOKEN_ENV_TEMPLATE)
        state = "sets nothing until you edit it"
    else:
        env_file.chmod(0o600)
        state = "left as it was"
    unit = config_home / "systemd/user/vyvrael-lichess-bot.service"
    unit.parent.mkdir(parents=True, exist_ok=True)
    unit.write_text(systemd_unit(python, dest / "lichess-bot.py", dest, env_file))
    say(f"  wrote {unit} (NOT enabled or started)")
    say(f"  token file: {env_file} ({state}; only you can read it)")


INSTALLERS = {
    "cutechess": install_cutechess,
    "ordo": install_ordo,
    "lichess-bot": install_lichess_bot,
}


# ---------------------------------------------------------------- checks
def verify(ctx: Ctx, tool: str) -> None:
    """Runs the installed tool once and shows its first line of output."""
    commands = {"cutechess": ["cutechess-cli", "--version"], "ordo": ["ordo", "-h"]}
    if tool not in commands:
        return
    name, *rest = commands[tool]
    try:
        result = subprocess.run(
            [str(ctx.bin / name), *rest], capture_output=True, text=True, timeout=120, check=False
        )
        lines = (result.stdout or result.stderr).strip().splitlines()
        say(f"  check: {lines[0] if lines else '(no output)'}")
    except (OSError, subprocess.SubprocessError) as problem:
        say(f"  check failed: {problem}")


def path_hint(ctx: Ctx) -> None:
    on_path = str(ctx.bin) in os.environ.get("PATH", "").split(os.pathsep)
    if on_path:
        say(f"launchers are in {ctx.bin}, which is already on your PATH")
    else:
        say(f'launchers are in {ctx.bin}; add it to your PATH, for example in ~/.profile:')
        say(f'  export PATH="{ctx.bin}:$PATH"')


LICHESS_NEXT_STEPS = """
lichess-bot next steps (manual by design):
  1. Use a FRESH Lichess account with zero games.
  2. Create an API token with only the bot:play scope. Give it to the bot
     through the LICHESS_BOT_TOKEN environment variable (with --systemd:
     uncomment that line in ~/.config/vyvrael/lichess-token.env and put the
     token after the '='). Never commit it. While the variable is set, even
     to nothing, it replaces any token in config.yml.
  3. Build Vyvræl, then run once: vyvrael-lichess-bot -u
     That upgrades the account to BOT. It is IRREVERSIBLE.
  4. Afterwards: vyvrael-lichess-bot
     (with --systemd: systemctl --user enable --now vyvrael-lichess-bot)"""


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--tool",
        action="append",
        choices=TOOLS + ["all"],
        help="repeatable; default: cutechess and ordo",
    )
    parser.add_argument("--prefix", help="install root (default: ~/.local/share/vyvrael-tools)")
    parser.add_argument("--bin-dir", help="where launchers go")
    parser.add_argument("--engine", help="path to the Vyvræl binary, for lichess-bot's config")
    parser.add_argument(
        "--systemd",
        action="store_true",
        help="also write a user service for lichess-bot under ~/.config (not enabled)",
    )
    parser.add_argument("--force", action="store_true", help="reinstall the pinned version")
    parser.add_argument("--list", action="store_true", help="show pinned and installed versions")
    parser.add_argument("--dry-run", action="store_true", help="show what would be done")
    return parser


def main(argv: list[str] | None = None) -> None:
    args = build_parser().parse_args(argv)
    ctx = Ctx(args)
    if args.list:
        for tool in TOOLS:
            pin = PINS[tool]
            have = ctx.installed_version(tool) or "not installed"
            say(f"{tool:12} pinned {pin['version']:14} installed: {have}  ({pin['licence']})")
        say(f"prefix {ctx.prefix}\nbin    {ctx.bin}")
        return
    if not sys.platform.startswith("linux") and not args.dry_run:
        die("this installer supports Linux only; see docs/TESTING.md for other systems")

    wanted = args.tool or DEFAULT_TOOLS
    if "all" in wanted:
        wanted = TOOLS
    for tool in wanted:
        have = ctx.installed_version(tool)
        if have == PINS[tool]["version"] and not args.force and not args.dry_run:
            say(f"{tool}: {have} already installed (use --force to reinstall)")
            continue
        INSTALLERS[tool](ctx)
        if not args.dry_run:
            verify(ctx, tool)
    if not args.dry_run:
        path_hint(ctx)
        if "lichess-bot" in wanted:
            say(LICHESS_NEXT_STEPS)


if __name__ == "__main__":
    main()
