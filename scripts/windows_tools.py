# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Installs Cute Chess and Ordo on Windows, for install_tools.py.

Both come as official portable zip archives, pinned below by address and
SHA-256, and are unpacked under the install prefix. Nothing is added to
PATH and nothing needs administrator rights: the installer prints the two
lines to put in scripts/local.env instead.

lichess-bot is not installed on Windows: it runs on the Linux machine only
(docs/LICHESS.md). The ladder's opponent engines are not installed on
Windows either; each project publishes its own Windows program.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

import install_tools as tools

# Read on each project's own release page on 2026-11-21. Cute Chess
# publishes a digest for its file; Ordo's release (2016) has none, so its
# digest is what the file had on that day.
PINS = {
    "cutechess": {
        "version": "v1.5.1",
        "url": "https://github.com/cutechess/cutechess/releases/download/"
        "v1.5.1/cutechess-1.5.1-win64.zip",
        "sha256": "048942ca3473db860cb914fe94108da37051f693f47e368688ec6ec450f924bc",
        "program": "cutechess-1.5.1-win64/cutechess-cli.exe",
        "check": ["--version"],
        "setting": "CUTECHESS_CLI",
    },
    "ordo": {
        "version": "v1.2.6",
        "url": "https://github.com/michiguel/Ordo/releases/download/v1.2.6/ordo-1.2.6-win.zip",
        "sha256": "44dcb6b103e13a39f701baabab12fc503dd5a7c34873face503a9d18b92b2b29",
        "program": "ordo-1.2.6-win/ordo-win64.exe",
        "check": ["-h"],
        "setting": "ORDO",
    },
}

# cutechess-cli.exe needs these, which Windows does not always have. The
# archive carries Microsoft's installer for them (vc_redist.x64.exe), which
# needs administrator rights, so it is reported and never run.
VC_RUNTIME = ("vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll")


def tool_dir(ctx, tool: str) -> Path:
    return ctx.prefix / "windows" / tool


def program_path(ctx, tool: str) -> Path:
    return tool_dir(ctx, tool) / PINS[tool]["program"]


def manifest_key(tool: str) -> str:
    return "windows:" + tool


def is_installed(ctx, tool: str) -> bool:
    entry = ctx.installed_entry(manifest_key(tool))
    pin = PINS[tool]
    return (
        entry.get("url") == pin["url"]
        and entry.get("sha256") == pin["sha256"]
        and program_path(ctx, tool).is_file()
    )


def missing_runtime(system_root: str | None = None) -> list[str]:
    """The Visual C++ runtime files that are not in System32."""
    root = system_root or os.environ.get("SystemRoot") or r"C:\Windows"
    return [name for name in VC_RUNTIME if not (Path(root) / "System32" / name).is_file()]


def install(ctx, tool: str) -> dict | None:
    """Installs one tool and returns what to record, or None in a dry run."""
    pin = PINS[tool]
    dest = tool_dir(ctx, tool)
    tools.say(f"{tool}: {pin['version']} -> {dest}")
    if ctx.dry:
        tools.say(f"  would download {pin['url']}")
        return None
    work = Path(tempfile.mkdtemp(prefix="vyvrael-windows-"))
    try:
        archive = work / "download.zip"
        tools.download(pin["url"], archive, pin["sha256"])
        stage = work / "stage"
        tools.safe_extract_zip(archive, stage)
        if not (stage / pin["program"]).is_file():
            tools.die(f"{pin['program']} is not in the archive")
        tools.replace_dir(stage, dest)
    finally:
        shutil.rmtree(work, ignore_errors=True)
    return {"version": pin["version"], "url": pin["url"], "sha256": pin["sha256"],
            "program": str(program_path(ctx, tool))}


def check(ctx, tool: str) -> bool:
    """Starts the installed program once and shows its first line of output.

    Fails when it cannot be started, does not answer within two minutes, or
    ends with an error status without printing anything, which is what a
    program does that cannot load a library it needs.
    """
    program = program_path(ctx, tool)
    try:
        result = subprocess.run(
            [str(program), *PINS[tool]["check"]], capture_output=True, text=True,
            timeout=120, check=False,
        )
    except (OSError, subprocess.SubprocessError) as problem:
        tools.say(f"  check FAILED: {problem}")
        return False
    lines = (result.stdout or result.stderr or "").strip().splitlines()
    if result.returncode != 0 and not lines:
        tools.say(f"  check FAILED: exit status {result.returncode} and no output")
        return False
    status = f" (exit status {result.returncode})" if result.returncode != 0 else ""
    tools.say(f"  check: {lines[0] if lines else '(no output)'}{status}")
    return True


def install_tools_windows(ctx, wanted: list[str], force: bool) -> list[str]:
    """Installs the wanted tools and returns the names that failed."""
    failed = []
    # The whole list is checked before anything is installed.
    for tool in wanted:
        if tool not in PINS:
            tools.die(
                f"{tool} is not installed on Windows by this script"
                + (": lichess-bot runs on the Linux machine only (docs/LICHESS.md)"
                   if tool == "lichess-bot" else f" (it installs {', '.join(PINS)})")
            )
    for tool in wanted:
        if is_installed(ctx, tool) and not force:
            tools.say(f"{tool}: {PINS[tool]['version']} already installed"
                      " (use --force to reinstall)")
            continue
        info = install(ctx, tool)
        if info is None:
            continue
        if check(ctx, tool):
            ctx.record(manifest_key(tool), info)
        else:
            ctx.forget(manifest_key(tool))
            failed.append(tool)
    if not ctx.dry:
        present = [tool for tool in PINS if is_installed(ctx, tool)]
        if present:
            tools.say("put these lines in scripts/local.env (see scripts/local.env.example):")
            for tool in present:
                tools.say(f"  {PINS[tool]['setting']}={program_path(ctx, tool)}")
        lacking = missing_runtime()
        if "cutechess" in wanted and lacking:
            tools.say(
                f"note: {', '.join(lacking)} not found in System32. cutechess-cli.exe needs"
                " the Microsoft Visual C++ runtime; install it with vc_redist.x64.exe from"
                f" {tool_dir(ctx, 'cutechess')} (it asks for administrator rights)."
            )
    return failed
