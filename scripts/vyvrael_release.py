# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Puts a released Vyvræl binary in place, for install_tools.py.

`--vyvrael VERSION` downloads the Linux binary of that GitHub release and
its `.sha256` file, checks one against the other, checks that the binary
answers `uci` with `id name Vyvræl VERSION`, and only then points the
engine path of lichess-bot's config.yml at it. That path is a symbolic link
(`<prefix>/engines/vyvrael` by default, or the file given with `--engine`),
so config.yml never needs editing for an upgrade. Each version stays beside
the link under its own name, so going back is re-pointing the link.

It never stops, starts or restarts the bot, and never edits config.yml.

On Windows there is no bot: `--vyvrael VERSION` downloads the Windows file
of the release (`.exe`) and its `.sha256` file, makes the same checks, and
keeps it in `<prefix>/engines/vyvrael-VERSION/`, beside the ladder's
engines, for the match scripts and Cute Chess. Nothing is linked there:
Windows lets few users make symbolic links, and matches name a version.
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
from pathlib import Path

import install_tools as tools

REPO = "NexxusDrakoMultimedia/Vyvrael-Chess"
RELEASES = f"https://github.com/{REPO}/releases/download"
# Which file of a release: the primary avx2 build or the plain one. Releases
# before 0.1.0 have the plain build only.
BUILDS = ("avx2", "plain")
VERSION = re.compile(r"^v?(\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?)$")
SHA256_LINE = re.compile(r"^([0-9a-fA-F]{64})\s+\*?(\S+)\s*$")
# Seconds the downloaded binary has to answer `uci`.
UCI_TIMEOUT = 10.0


def parse_version(text: str) -> str:
    """The version without a leading `v`; dies on anything else."""
    match = VERSION.match(text.strip())
    if not match:
        tools.die(f"'{text}' is not a release version such as 0.1.0 or 0.1.0-alpha.2")
    return match.group(1)


def asset_name(version: str, build: str, windows: bool = False) -> str:
    """The file of `version` in the release, for `build`: the Linux file, or
    the Windows one (`.exe`) when `windows`."""
    name = f"vyvrael-{version}-avx2" if build == "avx2" else f"vyvrael-{version}"
    return name + ".exe" if windows else name


def asset_url(version: str, name: str) -> str:
    return f"{RELEASES}/v{version}/{name}"


def parse_sha256(text: str, name: str) -> str:
    """The digest from a `sha256sum` line for the file `name`."""
    match = SHA256_LINE.match(text.strip())
    if not match or match.group(2) != name:
        tools.die(f"{name}.sha256 is not one sha256sum line for {name}")
    return match.group(1).lower()


def fetch_text(url: str) -> str:
    """A small text file over HTTPS only, redirects included."""
    tools.say(f"  downloading {url}")
    request = urllib.request.Request(url, headers={"User-Agent": "install-tools"})
    with tools.https_opener().open(request, timeout=60) as response:
        if not response.geturl().lower().startswith("https://"):
            tools.die(f"refusing to download over anything but HTTPS: {response.geturl()}")
        return response.read(4096).decode("utf-8", errors="replace")


def uci_name(binary: Path) -> str:
    """The `id name` line the binary prints after `uci`."""
    try:
        result = subprocess.run(
            [str(binary)], input="uci\nquit\n", capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=UCI_TIMEOUT,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        tools.die(f"{binary.name} did not run: {error}")
    for line in result.stdout.splitlines():
        if line.startswith("id name "):
            return line[len("id name "):].strip()
    tools.die(f"{binary.name} did not answer uci with an id name line")


def link_path(ctx) -> Path:
    """Where lichess-bot's config.yml finds the engine: the path given with
    --engine, else `<prefix>/engines/vyvrael` (as install_tools writes it).

    Only the directory is resolved: resolving the whole path would follow
    the link to the version it points at now.
    """
    if ctx.args.engine:
        given = Path(ctx.args.engine).expanduser()
        return given.parent.resolve() / given.name
    return ctx.prefix / "engines" / "vyvrael"


def point_link(link: Path, target_name: str) -> Path | None:
    """Points `link` at the file `target_name` beside it, in one rename, so
    there is something at `link` at every moment.

    A plain file already at `link` (a binary copied there by hand) is first
    copied to `<link>.before-link` rather than lost, and that path returned.
    """
    kept = None
    if link.exists() and not link.is_symlink():
        kept = link.with_name(link.name + ".before-link")
        if os.path.lexists(kept):
            tools.die(f"{link} is a plain file and {kept.name} already exists; move one away")
        shutil.copy2(link, kept)
    staged = link.with_name(link.name + ".new-link")
    tools.remove_path(staged)
    os.symlink(target_name, staged)  # relative: it survives moving the directory
    os.replace(staged, link)
    return kept


def fetch_error(error: OSError, url: str, name: str, version: str) -> str:
    """The message for a download that failed: a hint for a missing file,
    the plain reason otherwise (no network, a rate limit, a server error)."""
    if isinstance(error, urllib.error.HTTPError) and error.code == 404:
        return (
            f"no {name} in release v{version} (404). Check the version; a release"
            " that is still a draft cannot be downloaded, and releases before 0.1.0"
            " have no avx2 file (use --vyvrael-build plain)"
        )
    reason = getattr(error, "reason", None) or error
    return f"could not download {url}: {reason}"


def fetch_checked(version: str, name: str, target: Path) -> None:
    """Downloads the release file `name` to `target`, checked against its
    `.sha256` file and its `uci` answer; nothing is left behind on failure."""
    url = asset_url(version, name)
    try:
        expected = parse_sha256(fetch_text(url + ".sha256"), name)
    except OSError as error:  # URLError, HTTPError and timeouts are OSErrors
        tools.die(fetch_error(error, url + ".sha256", name, version))
    target.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=target.parent) as tmp:
        staged = Path(tmp) / name
        try:
            tools.download(url, staged, expected)
        except OSError as error:
            tools.die(fetch_error(error, url, name, version))
        tools.make_exec(staged)
        answered = uci_name(staged)
        if answered.split()[-1:] != [version]:
            tools.die(f"{name} calls itself '{answered}', not version {version}")
        tools.say(f"  check: id name {answered}")
        os.replace(staged, target)


def windows_dir(ctx, version: str) -> Path:
    """Where a release is kept on Windows: beside the ladder's engines."""
    return ctx.prefix / "engines" / f"vyvrael-{version}"


def install_windows(ctx, version: str, build: str) -> None:
    """The Windows side of --vyvrael; see the module docstring."""
    name = asset_name(version, build, windows=True)
    target = windows_dir(ctx, version) / name
    tools.say(f"Vyvræl {version} ({build} build) for Windows")
    if ctx.dry:
        tools.say(f"  [dry-run] would download {asset_url(version, name)} -> {target}")
        return
    fetch_checked(version, name, target)
    tools.say(f"  {target}")
    tools.say("  add it to the Cute Chess engine list to use it in matches")


def install_release(ctx, version_text: str, build: str, windows: bool | None = None) -> None:
    """Downloads, checks and links in the release; see the module docstring.

    `windows` defaults to the system this runs on.
    """
    version = parse_version(version_text)
    if windows is None:
        windows = sys.platform == "win32"
    if windows:
        install_windows(ctx, version, build)
        return
    name = asset_name(version, build)
    link = link_path(ctx)
    target = link.with_name(name)
    url = asset_url(version, name)
    tools.say(f"Vyvræl {version} ({build} build) for lichess-bot")
    if target.name == link.name:
        tools.die(f"--engine must not be named {name}: that is the release file itself")
    if ctx.dry:
        tools.say(f"  [dry-run] would download {url} -> {target}")
        tools.say(f"  [dry-run] would point {link} -> {name}")
        return

    fetch_checked(version, name, target)
    kept = point_link(link, name)
    if kept:
        tools.say(f"  the file that was at {link} is now {kept.name}")
    tools.say(f"  {link} -> {name}")
    tools.say("  restart lichess-bot to use it; config.yml is unchanged")


def describe_windows(ctx) -> str:
    """One line for --list on Windows: the releases kept, if any."""
    found = sorted(p.name for p in (ctx.prefix / "engines").glob("vyvrael-*/vyvrael-*.exe"))
    if not found:
        return "vyvrael      no release under engines/"
    return "vyvrael      " + ", ".join(found)


def describe_link(ctx) -> str:
    """One line for --list: where the engine link points, if anywhere."""
    link = link_path(ctx)
    if link.is_symlink():
        return f"vyvrael      {link} -> {os.readlink(link)}"
    if link.is_file():
        return f"vyvrael      {link} (a plain file, not a link)"
    return f"vyvrael      no engine at {link}"

