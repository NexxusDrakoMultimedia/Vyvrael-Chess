# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""The opening positions of every match, fetched by install_tools.py.

`install_tools.py --openings` downloads UHO_Lichess_4852_v1.epd.zip from the
Stockfish project's books repository at one pinned commit, checks the zip
and the .epd inside it by SHA-256, and puts the .epd under
<prefix>/openings. The match scripts read its path from OPENINGS in
scripts/local.env; the installer prints that line. Nothing is committed to
this repository (see docs/DATA.md).
"""
from __future__ import annotations

import hashlib
import os
import shutil
import tempfile
import zipfile
from pathlib import Path

import install_tools as tools

# Source, licence and digests: docs/DATA.md. Change only in a reviewed commit.
PIN = {
    "name": "UHO_Lichess_4852_v1",
    "version": "commit 37751b08db745aa02babfa74056dd1168b9da803",
    "source": "https://github.com/official-stockfish/books",
    "licence": "CC0-1.0",
    "url": "https://raw.githubusercontent.com/official-stockfish/books/"
    "37751b08db745aa02babfa74056dd1168b9da803/UHO_Lichess_4852_v1.epd.zip",
    "sha256": "4e298f11e8acfa106babe02968f2e61582145e7874c59284690b20b9650e0e07",
    "member": "UHO_Lichess_4852_v1.epd",
    # The file of Ladder Baselines 1 and 2, by SHA-256.
    "member_sha256": "7a7f6470615a69c6cf23d565417701d38732876f480af90d67b42abade35644a",
}
MANIFEST_KEY = "openings"


def openings_path(ctx) -> Path:
    """Where the .epd goes."""
    return ctx.prefix / "openings" / PIN["member"]


def file_digest(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def is_installed(ctx) -> bool:
    """True when the .epd is there, recorded at this pin, and unchanged."""
    path = openings_path(ctx)
    entry = ctx.installed_entry(MANIFEST_KEY)
    return (entry.get("version") == PIN["version"] and path.is_file()
            and file_digest(path) == PIN["member_sha256"])


def extract_member(archive: Path, member: str, dest: Path, sha256: str) -> None:
    """Writes the one member `member` of the zip to `dest`, checking its
    SHA-256 while it is written; a mismatch leaves nothing at `dest`."""
    digest = hashlib.sha256()
    with zipfile.ZipFile(archive) as bundle:
        try:
            info = bundle.getinfo(member)
        except KeyError:
            tools.die(f"{member} is not in {archive.name}")
        with bundle.open(info) as source, open(dest, "wb") as out:
            for chunk in iter(lambda: source.read(1 << 20), b""):
                digest.update(chunk)
                out.write(chunk)
    if digest.hexdigest() != sha256:
        dest.unlink(missing_ok=True)
        tools.die(f"checksum mismatch for {member}\n  expected {sha256}\n"
                  f"  got      {digest.hexdigest()}")


def install_openings(ctx, force: bool) -> Path | None:
    """Installs the opening file and returns its path (None in a dry run).

    Skipped when it is already installed and unchanged, unless forced. The
    file is put in place only once both digests have matched.
    """
    path = openings_path(ctx)
    if is_installed(ctx) and not force:
        tools.say(f"openings: {PIN['member']} already installed (use --force to reinstall)")
    elif ctx.dry:
        tools.say(f"openings: [dry-run] would download {PIN['url']}")
        tools.say(f"  and put {PIN['member']} at {path}")
        return None
    else:
        tools.say(f"openings: {PIN['member']} ({PIN['version']}) -> {path}")
        work = Path(tempfile.mkdtemp(prefix="vyvrael-openings-"))
        try:
            archive = work / "openings.zip"
            tools.download(PIN["url"], archive, PIN["sha256"])
            staged = work / PIN["member"]
            extract_member(archive, PIN["member"], staged, PIN["member_sha256"])
            tools.say("  sha256 of the .epd verified (the file of Ladder Baselines 1 and 2)")
            # Copied next to its place first, then swapped in by one rename,
            # so an interrupted run never leaves a partial file at `path`.
            beside = path.with_name(path.name + ".new")
            try:
                path.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(staged, beside)
                os.replace(beside, path)
            except OSError as problem:
                beside.unlink(missing_ok=True)
                tools.die(f"could not put {PIN['member']} at {path}: {problem}. If a match is"
                          " using the file, run again when it has finished.")
        finally:
            shutil.rmtree(work, ignore_errors=True)
        ctx.record(MANIFEST_KEY, {"version": PIN["version"], "url": PIN["url"],
                                  "sha256": PIN["sha256"], "file": str(path),
                                  "file_sha256": PIN["member_sha256"]})
    tools.say("  for the match scripts, put this line in scripts/local.env:")
    tools.say(f"  OPENINGS={path}")
    return path
