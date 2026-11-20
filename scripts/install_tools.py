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
import posixpath
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
class HttpsOnlyRedirects(urllib.request.HTTPRedirectHandler):
    """Follows a redirect only when it leads to another HTTPS address.

    The standard handler also follows redirects to http:// and ftp://.
    """

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        if not newurl.lower().startswith("https://"):
            die(f"refusing a redirect away from HTTPS: {req.full_url} -> {newurl}")
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def https_opener() -> urllib.request.OpenerDirector:
    """An opener that never leaves HTTPS, whatever the server answers."""
    return urllib.request.build_opener(HttpsOnlyRedirects)


def download(url: str, dest: Path, sha256: str | None) -> str:
    """Downloads `url` to `dest` and returns its SHA-256.

    Only HTTPS is used, for the address given and for every redirect. With
    `sha256` given, a mismatch deletes the file and aborts. Without it the
    digest is printed so it can be pinned.
    """
    if not url.startswith("https://"):
        die(f"refusing to download over anything but HTTPS: {url}")
    say(f"  downloading {url}")
    digest = hashlib.sha256()
    request = urllib.request.Request(url, headers={"User-Agent": "install-tools"})
    with https_opener().open(request, timeout=60) as response:
        # Belt and braces: the address the bytes really come from.
        if not response.geturl().lower().startswith("https://"):
            die(f"refusing to download over anything but HTTPS: {response.geturl()}")
        with open(dest, "wb") as out:
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
    """Extracts a tar archive, refusing entries that would land outside `dest`.

    Every name and link target is checked before anything is extracted, so
    the check cannot see where a link really leads once it exists. Paths are
    therefore also refused when they go through one of the archive's own
    links: that is the only way a chain of individually harmless links can
    climb out of `dest`. Source archives of a git commit never need it.
    """
    dest = dest.resolve()
    dest.mkdir(parents=True, exist_ok=True)

    def check(name: str) -> None:
        target = (dest / name).resolve()
        if target != dest and dest not in target.parents:
            die(f"unsafe path in archive: {name}")

    with tarfile.open(archive) as tar:
        members = tar.getmembers()
        links = {posixpath.normpath(m.name) for m in members if m.issym() or m.islnk()}

        def check_no_link_on_the_way(path: str, what: str) -> None:
            parts = [part for part in path.split("/") if part not in ("", ".")]
            prefix = ""
            for part in parts[:-1]:
                prefix = posixpath.normpath(posixpath.join(prefix, part))
                if prefix in links:
                    die(f"{what} goes through a link in the archive: {path}")

        for member in members:
            check(member.name)
            check_no_link_on_the_way(member.name, "path")
            if member.issym():
                # A symlink target is relative to the link's own directory.
                target = posixpath.join(posixpath.dirname(member.name), member.linkname)
                check(target)
                check_no_link_on_the_way(target, "link target")
            elif member.islnk():
                # A hard link target is relative to the archive root. A hard
                # link to a symlink would repeat that symlink's relative
                # target from another directory, so it is refused too.
                check(member.linkname)
                check_no_link_on_the_way(member.linkname, "link target")
                if posixpath.normpath(member.linkname) in links:
                    die(f"hard link to another link in archive: {member.name}")
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


def remove_path(path: Path) -> None:
    """Removes a file, a link or a whole directory, if it is there."""
    if path.is_symlink() or path.is_file():
        path.unlink()
    elif path.exists():
        shutil.rmtree(path)


def recover_interrupted(dest: Path) -> None:
    """Puts back an install that an interrupted `replace_dir` left aside.

    `replace_dir` renames `dest` to `<dest>.old` and then the new tree to
    `dest`. A run stopped between the two leaves no `dest`, and `<dest>.old`
    is the only copy of the previous install (for lichess-bot, of the user's
    config.yml), so it is moved back instead of being cleared away.
    """
    retired = dest.with_name(dest.name + ".old")
    if retired.is_dir() and not retired.is_symlink() and not os.path.lexists(dest):
        retired.rename(dest)
        say(f"  an earlier run was interrupted: restored {dest} from {retired.name}")


def replace_dir(new: Path, dest: Path) -> None:
    """Puts the directory `new` in place of `dest`.

    The new tree is copied next to `dest` first and only then swapped in, so
    a failure while copying leaves the existing install untouched. The swap
    is two renames; one interrupted between them is undone by the next call
    (see `recover_interrupted`).
    """
    staged = dest.with_name(dest.name + ".new")
    retired = dest.with_name(dest.name + ".old")
    recover_interrupted(dest)
    for leftover in (staged, retired):
        remove_path(leftover)
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(new, staged, symlinks=True)
    if dest.exists():
        dest.rename(retired)
    try:
        staged.rename(dest)
    except OSError:
        if retired.exists():
            retired.rename(dest)
        raise
    remove_path(retired)


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

    def installed_entry(self, tool: str) -> dict:
        """What the manifest records about `tool`; empty when nothing usable."""
        entry = self._read_manifest().get(tool)
        return entry if isinstance(entry, dict) else {}

    def installed_version(self, tool: str) -> str | None:
        return self.installed_entry(tool).get("version")

    def record(self, tool: str, info: dict) -> None:
        manifest = self._read_manifest()
        manifest[tool] = info
        self.manifest.parent.mkdir(parents=True, exist_ok=True)
        self.manifest.write_text(json.dumps(manifest, indent=2) + "\n")

    def shim(
        self, name: str, target: Path, args: list[str] = (), env: dict = None, cwd: Path = None
    ) -> Path:
        """Writes a launcher `name` in the bin directory that runs `target`."""
        self.bin.mkdir(parents=True, exist_ok=True)
        path = self.bin / name
        path.write_text(shim_text(target, list(args), env or {}, cwd))
        make_exec(path)
        return path


def shim_text(target: Path, args: list[str], env: dict, cwd: Path | None = None) -> str:
    """The text of a POSIX shell launcher. Every path and argument is quoted.

    With `cwd` the launcher changes into that directory first, and does not
    start the program if it cannot.
    """
    assignments = "".join(f"{key}={shlex.quote(value)} " for key, value in env.items())
    fixed = "".join(shlex.quote(arg) + " " for arg in args)
    enter = f"cd {shlex.quote(str(cwd))} && " if cwd is not None else ""
    return f'#!/bin/sh\n{enter}{assignments}exec {shlex.quote(str(target))} {fixed}"$@"\n'


# ---------------------------------------------------------------- launchers
def tool_files(ctx: Ctx, tool: str) -> list[Path]:
    """The installed files the launchers of `tool` run, program first."""
    dest = ctx.prefix / tool
    if tool == "cutechess":
        return [dest / PINS["cutechess"]["url"].rsplit("/", 1)[-1]]
    if tool == "ordo":
        return [dest / "ordo"]
    return [dest / "lichess-bot.py", dest / "venv/bin/python"]


def write_launchers(ctx: Ctx, tool: str) -> None:
    """Writes the launchers of an installed `tool` and, for lichess-bot with
    --systemd, its user service.

    Kept apart from the installers so that a re-run of an installed tool can
    still honour --bin-dir and --systemd without reinstalling anything.
    """
    dest = ctx.prefix / tool
    files = tool_files(ctx, tool)
    if tool == "cutechess":
        # EXTRACT_AND_RUN lets the AppImage run without libfuse2, which
        # recent Ubuntu releases do not install. The AppImage bundles both
        # programs; the command-line one is selected with a leading "cli"
        # argument. If the check after installing fails, run the AppImage
        # with --appimage-help to see how this release exposes cutechess-cli.
        env = {"APPIMAGE_EXTRACT_AND_RUN": "1"}
        ctx.shim("cutechess-cli", files[0], ["cli"], env)
        ctx.shim("cutechess", files[0], [], env)
    elif tool == "ordo":
        ctx.shim("ordo", files[0])
    else:
        script, python = files
        # lichess-bot reads ./config.yml and resolves relative paths in it
        # against the current directory, so the launcher runs it from its
        # own directory, as the systemd unit does with WorkingDirectory.
        ctx.shim("vyvrael-lichess-bot", python, [str(script)], cwd=dest)
        if ctx.args.systemd:
            write_systemd_unit(dest, python)


def is_installed(ctx: Ctx, tool: str) -> bool:
    """Whether the pinned `tool` is installed: recorded as exactly what PINS
    names (version, commit, and digest where one is pinned) and still on disk.
    """
    pin = PINS[tool]
    entry = ctx.installed_entry(tool)
    if entry.get("version") != pin["version"]:
        return False
    if "commit" in pin and entry.get("commit") != pin["commit"]:
        return False
    if pin["sha256"] is not None and entry.get("sha256") != pin["sha256"]:
        return False
    return all(path.exists() for path in tool_files(ctx, tool))


def engine_option_note(ctx: Ctx) -> None:
    """Says that --engine did nothing, when it was given beside a kept config."""
    if ctx.args.engine:
        config = ctx.prefix / "lichess-bot/config.yml"
        say("  note: --engine is only used when config.yml is first written. To change the")
        say(f"  engine, edit dir and name under engine: in {config}")


def refresh_installed(ctx: Ctx, tool: str) -> None:
    """What a run still does for a tool that is already installed.

    The launchers are rewritten, so --bin-dir takes effect and a deleted
    launcher comes back, and --systemd writes the service. Nothing is
    downloaded and the install itself is not touched.
    """
    service = " and the user service" if tool == "lichess-bot" and ctx.args.systemd else ""
    if ctx.dry:
        say(f"  [dry-run] would rewrite its launchers in {ctx.bin}{service}")
    else:
        write_launchers(ctx, tool)
        say(f"  launchers rewritten in {ctx.bin}")
    if tool == "lichess-bot":
        engine_option_note(ctx)


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
    write_launchers(ctx, "cutechess")
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
    write_launchers(ctx, "ordo")
    ctx.record(
        "ordo",
        {"version": pin["version"], "commit": pin["commit"], "sha256": seen, "path": str(dest)},
    )


def engine_location(args, prefix: Path) -> tuple[str, str]:
    """(directory, file name) of the engine for lichess-bot's configuration.

    Without --engine it is `<prefix>/engines/vyvrael`: an absolute path
    outside lichess-bot's own directory, which a reinstall replaces.
    """
    if args.engine:
        engine = Path(args.engine).expanduser().resolve()
        if not engine.is_file():
            say(f"  warning: no engine at {engine} yet (build Vyvræl first; config still written)")
        return str(engine.parent), engine.name
    return str(prefix / "engines"), "vyvrael"


# What a reinstall of lichess-bot always brings across from the old directory.
KEPT_ALWAYS = ("config.yml", "venv")


def carry_over(old: Path, new: Path) -> list[str]:
    """Copies what a reinstall must not lose from `old` into the staged `new`.

    That is config.yml and the virtual environment, plus every top-level
    entry of the old directory that the new release does not provide: those
    can only be the user's own files (game records, opening books, notes).
    Entries the release does provide are not merged, so the pinned version
    of them wins. Returns the names carried across, sorted.
    """
    carried = []
    if not old.is_dir():
        return carried
    for entry in sorted(old.iterdir()):
        target = new / entry.name
        always = entry.name in KEPT_ALWAYS
        if entry.name == "__pycache__" or (os.path.lexists(target) and not always):
            continue
        remove_path(target)
        if entry.is_dir() and not entry.is_symlink():
            shutil.copytree(entry, target, symlinks=True)
        else:
            shutil.copy2(entry, target, follow_symlinks=False)
        carried.append(entry.name)
    if "config.yml" in carried and not (new / "config.yml").is_symlink():
        (new / "config.yml").chmod(0o600)  # it may hold a token
    return carried


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

    # Before looking for the user's configuration: an interrupted earlier run
    # may have left the whole install, config.yml included, under `.old`.
    recover_interrupted(dest)
    config = dest / "config.yml"
    reinstall = dest.is_dir()
    with tempfile.TemporaryDirectory() as tmp:
        archive = Path(tmp) / "lichess-bot.tar.gz"
        seen = download(url, archive, pin["sha256"])
        safe_extract(archive, Path(tmp) / "src")
        src = single_subdir(Path(tmp) / "src")
        # Carry the user's configuration, the virtual environment and the
        # user's own files across, inside the staged tree, so the swap below
        # is all or nothing.
        carried = carry_over(dest, src)
        replace_dir(src, dest)
    kept_config = "config.yml" in carried

    if not (dest / "venv").exists():
        subprocess.run([sys.executable, "-m", "venv", str(dest / "venv")], check=True)
    python = dest / "venv/bin/python"
    subprocess.run(
        [str(python), "-m", "pip", "install", "--quiet", "-r", str(dest / "requirements.txt")],
        check=True,
    )

    if kept_config:
        say("  kept your existing config.yml")
        engine_option_note(ctx)
    else:
        engine_dir, engine_name = engine_location(ctx.args, ctx.prefix)
        template = (dest / "config.yml.default").read_text()
        write_private(config, patch_config(template, engine_dir, engine_name))
        say(f"  wrote config.yml (engine {Path(engine_dir, engine_name)}, token blank)")
        if not ctx.args.engine:
            # lichess-bot refuses to start when the engine directory is missing.
            Path(engine_dir).mkdir(parents=True, exist_ok=True)
            say(f"  put the Vyvræl binary at {Path(engine_dir, engine_name)}")
    own = [name for name in carried if name not in KEPT_ALWAYS]
    if own:
        say(f"  kept your own files: {', '.join(own)}")
    if reinstall:
        say("  everything else in that directory was replaced by the pinned release:")
        say("  edits to its files, and files added inside its directories, are gone")

    write_launchers(ctx, "lichess-bot")
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
  3. Build Vyvræl. Without --engine, a new config.yml expects the binary at
     engines/vyvrael under the install prefix: outside lichess-bot's own
     directory, which a reinstall replaces. Then run once:
       vyvrael-lichess-bot -u
     That upgrades the account to BOT. It is IRREVERSIBLE.
  4. Afterwards: vyvrael-lichess-bot
     (with --systemd: systemctl --user enable --now vyvrael-lichess-bot)
  The launcher works from any directory: it changes into lichess-bot's own
  directory first, so config.yml there is the one used and a relative path
  given on its command line is read from there too."""


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
        # No dry-run exception here: a dry run reports what a real run would
        # do, and a real run skips a tool that is already installed.
        if is_installed(ctx, tool) and not args.force:
            say(f"{tool}: {PINS[tool]['version']} already installed (use --force to reinstall)")
            refresh_installed(ctx, tool)
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
