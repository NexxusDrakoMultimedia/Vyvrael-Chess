# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Installs the ladder's opponent engines on Linux, for install_tools.py.

What is installed is fixed by the table in ladder_engines.py. Each engine
goes into its own directory under <prefix>/engines, outside the repository.
An engine is fetched (and built, where there is no official binary) in a
temporary directory and swapped into place only when it is complete, so a
failed download or build leaves no half-installed engine behind.

Nothing here runs with sudo. A build tool that is missing is reported by
name and the engine is skipped.
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

import install_tools as tools
import ladder_engines

BUILD_TIMEOUT = 3600  # seconds; the largest build is a Rust engine with its dependencies


def system_of(ctx) -> str:
    """The system whose pins apply: "linux" or "windows" (set on the run's
    context; Linux when it says nothing)."""
    return getattr(ctx, "system", None) or "linux"


def build_of(ctx) -> str:
    """The build of engines to install: "avx2" (the default) or "pext"."""
    return getattr(ctx, "engine_build", None) or "avx2"


def active_pins(ctx) -> list[dict]:
    """The pins of every engine installable for this run's system and build."""
    return ladder_engines.pins(system_of(ctx), build_of(ctx))


def slug(name: str) -> str:
    """A directory name for an engine: "Rustic Alpha 3.0.6" -> "rustic-alpha-3.0.6"."""
    return re.sub(r"[^a-z0-9.]+", "-", name.lower()).strip("-")


def engine_dir(ctx, engine: dict) -> Path:
    return ctx.prefix / "engines" / slug(engine["name"])


def installed_binary(ctx, engine: dict) -> Path:
    """Where the engine's program is once installed."""
    return engine_dir(ctx, engine) / Path(engine["binary"]).name


def manifest_key(engine: dict) -> str:
    return "engine:" + engine["name"]


def file_digest(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def is_installed(ctx, engine: dict) -> bool:
    """Whether this exact pin is installed: recorded with the same address,
    digest and build command, and with its program still on disk."""
    entry = ctx.installed_entry(manifest_key(engine))
    home = engine_dir(ctx, engine)
    return (
        entry.get("url") == engine["url"]
        and entry.get("sha256") == engine["sha256"]
        and entry.get("command") == engine.get("build", {}).get("command")
        # The same program: another build may come from the same archive.
        and entry.get("binary") == str(installed_binary(ctx, engine))
        and entry.get("binary_sha256", engine.get("member_sha256"))
        == engine.get("member_sha256", entry.get("binary_sha256"))
        and installed_binary(ctx, engine).is_file()
        # What the engine needs beside it (a network, a book, personalities).
        and all((home / Path(extra).name).exists() for extra in engine.get("beside", []))
    )


def missing_tools(engine: dict) -> list[str]:
    """The build tools this engine needs that are not on PATH."""
    return [tool for tool in engine.get("build", {}).get("tools", []) if not shutil.which(tool)]


def select(ctx, only: list[str] | None) -> list[dict]:
    """The pins to install: all of this system's, or those named with --only.

    A limited entry ("Stockfish 19-1500") selects its engine, since it is the
    same program. An unknown name ends the run with the names that exist.
    """
    available = active_pins(ctx)
    if not only:
        return available
    system = system_of(ctx)
    names = ladder_engines.ladder_names(system)
    unavailable = ladder_engines.NOT_INSTALLABLE[system]
    chosen = []
    for name in only:
        base = re.sub(r"-\d+$", "", name) if name in names else name
        engine = next((p for p in available if p["name"] == base), None)
        if engine is None:
            if name in ladder_engines.RETIRED:
                tools.die(f"{name} is no longer in the ladder: {ladder_engines.RETIRED[name]}")
            if name in unavailable:
                tools.die(f"{name} cannot be installed on {system}: {unavailable[name]}")
            known = ", ".join(p["name"] for p in available)
            tools.die(f"no engine named '{name}' in the ladder. Known: {known}")
        if engine not in chosen:
            chosen.append(engine)
    return chosen


def extract_zip_member(archive: Path, member: str, dest: Path) -> None:
    """Writes the one file `member` of a zip archive to `dest`.

    Only that member is read, by its exact name, so nothing else in the
    archive can be written anywhere.
    """
    with zipfile.ZipFile(archive) as bundle:
        try:
            info = bundle.getinfo(member)
        except KeyError:
            tools.die(f"{member} is not in {archive.name}")
        if info.is_dir():
            tools.die(f"{member} in {archive.name} is a directory, not a program")
        with bundle.open(info) as source, open(dest, "wb") as out:
            shutil.copyfileobj(source, out)


def inside(root: Path, relative: str) -> Path:
    """`relative` under `root`, refusing anything that would leave it."""
    root = root.resolve()
    path = (root / relative).resolve()
    if path != root and root not in path.parents:
        tools.die(f"path leaves the engine's directory: {relative}")
    return path


def run_build(engine: dict, tree: Path, log: Path) -> None:
    """Runs the engine's build command in its build directory.

    The command is a list of arguments, started without a shell. Its output
    goes to `log`; on failure the last lines are shown.
    """
    build = engine["build"]
    where = inside(tree, build["dir"])
    environment = dict(os.environ)
    environment.update(build.get("env", {}))
    tools.say(f"  building: {' '.join(build['command'])}")
    with open(log, "w", encoding="utf-8", errors="replace") as out:
        try:
            result = subprocess.run(
                build["command"], cwd=where, env=environment, stdout=out,
                stderr=subprocess.STDOUT, timeout=BUILD_TIMEOUT, check=False,
            )
        except subprocess.TimeoutExpired:
            tools.die(f"the build of {engine['name']} took longer than {BUILD_TIMEOUT} s")
        except OSError as problem:
            tools.die(f"the build of {engine['name']} could not start: {problem}")
    if result.returncode != 0:
        tail = log.read_text(encoding="utf-8", errors="replace").splitlines()[-15:]
        tools.die(
            f"the build of {engine['name']} failed with status {result.returncode}."
            " Last lines of its output:\n  " + "\n  ".join(tail)
        )


def install_engine(ctx, engine: dict, fetched: dict) -> dict | None:
    """Installs one engine and returns what to record, or None in a dry run.

    `fetched` maps an address to the file already downloaded in this run:
    the three Rustic Alpha versions share one archive.
    """
    name = engine["name"]
    dest = engine_dir(ctx, engine)
    how = "download" if engine["method"] == "download" else "build from source"
    tools.say(f"{name}: {engine['version']} ({how}) -> {dest}")
    if not engine.get("sha256"):
        tools.die(f"{name} has no SHA-256 in its pin; nothing is fetched without one")
    lacking = missing_tools(engine)
    if lacking:
        problem = (
            f"{name} is built from source and needs {', '.join(lacking)}, which"
            f" {'is' if len(lacking) == 1 else 'are'} not on PATH"
        )
        if ctx.dry:
            # A dry run reports and goes on; it does not fail.
            tools.say(f"  note: {problem}; a real run would skip this engine")
        else:
            tools.die(
                f"{problem}. Install {'it' if len(lacking) == 1 else 'them'} and run"
                " again; nothing was changed."
            )
    if ctx.dry:
        tools.say(f"  would download {engine['url']}")
        if engine["method"] == "build":
            tools.say(f"  would build: {' '.join(engine['build']['command'])}")
        return None

    work = Path(tempfile.mkdtemp(prefix="vyvrael-engine-"))
    try:
        if engine["url"] not in fetched:
            download = work / "download"
            # Plain HTTP only for a pin that says why; the digest is checked.
            tools.download(engine["url"], download, engine["sha256"],
                           allow_http=bool(engine.get("http_reason")))
            kept = Path(fetched["dir"]) / hashlib.sha256(engine["url"].encode()).hexdigest()
            shutil.move(str(download), kept)
            fetched[engine["url"]] = kept
        archive = fetched[engine["url"]]

        stage = work / "stage"
        stage.mkdir()
        program = stage / Path(engine["binary"]).name
        if engine["archive"] == "none":
            shutil.copyfile(archive, program)
        elif engine["archive"] == "zip" and engine["method"] == "download":
            # An archive of ready binaries: only the named members are read,
            # the program and the files it needs beside it.
            extract_zip_member(archive, engine["binary"], program)
            for extra in engine.get("beside", []):
                extract_zip_member(archive, extra, stage / Path(extra).name)
        else:
            # A source archive (tar.gz or zip), or a tar.gz or 7z of binaries:
            # the whole tree is unpacked, with the same checks for each kind.
            tree_root = work / "tree"
            if engine["archive"] == "zip":
                tools.safe_extract_zip(archive, tree_root)
            elif engine["archive"] == "7z":
                tools.safe_extract_7z(archive, tree_root)
            else:
                tools.safe_extract(archive, tree_root)
            tree = tools.single_subdir(tree_root)
            if engine["method"] == "build":
                run_build(engine, tree, work / "build.log")
            built = inside(tree, engine["binary"])
            if not built.is_file():
                tools.die(f"{name}: {engine['binary']} is not there after unpacking and building")
            shutil.copyfile(built, program)
            for extra in engine.get("beside", []):
                source = inside(tree, extra)
                if source.is_dir():
                    shutil.copytree(source, stage / source.name, symlinks=False)
                elif source.is_file():
                    shutil.copyfile(source, stage / source.name)
                else:
                    tools.die(f"{name}: {extra} is not in the archive")
        if "member_sha256" in engine and file_digest(program) != engine["member_sha256"]:
            tools.die(
                f"checksum mismatch for {engine['binary']} of {name}\n"
                f"  expected {engine['member_sha256']}\n  got      {file_digest(program)}"
            )
        tools.make_exec(program)
        digest = file_digest(program)
        tools.replace_dir(stage, dest)
    finally:
        shutil.rmtree(work, ignore_errors=True)
    tools.say(f"  installed {installed_binary(ctx, engine)} (sha256 {digest})")
    return {
        "version": engine["version"],
        "url": engine["url"],
        "sha256": engine["sha256"],
        "command": engine.get("build", {}).get("command"),
        "binary": str(installed_binary(ctx, engine)),
        "binary_sha256": digest,
    }


def install_engines(ctx, only: list[str] | None, force: bool) -> tuple[list[dict], list[str]]:
    """Installs the selected engines.

    Returns the engines now installed and the names of those that failed.
    One engine failing (a missing build tool, a build error, a checksum that
    does not match, a download that does not answer) does not stop the
    others: its message is printed and this run records nothing for it. An
    earlier install of the same engine is left as it was, on disk and in the
    record: a failure happens before anything is swapped into place.
    """
    chosen = select(ctx, only)
    fetched = {"dir": tempfile.mkdtemp(prefix="vyvrael-fetched-")}
    present, failed = [], []
    try:
        for engine in chosen:
            if is_installed(ctx, engine) and not force:
                tools.say(f"{engine['name']}: already installed (use --force to reinstall)")
                present.append(engine)
                continue
            try:
                info = install_engine(ctx, engine, fetched)
            except SystemExit:
                failed.append(engine["name"])
                continue
            except Exception as problem:  # noqa: BLE001 - one engine must not stop the rest
                # A network error, a broken archive, a full disk.
                print(f"error: {engine['name']}: {type(problem).__name__}: {problem}",
                      file=sys.stderr)
                failed.append(engine["name"])
                continue
            if info is not None:
                ctx.record(manifest_key(engine), info)
                present.append(engine)
    finally:
        shutil.rmtree(fetched["dir"], ignore_errors=True)
    return present, failed


# ---------------------------------------------------------------- engine list
def engine_entries(ctx, engine: dict) -> list[dict]:
    """The engine-list entries of one engine: itself at full strength, then
    one per limited variant, named as in the ladder.

    Every entry gets the baseline's settings where the engine has the
    option: Hash 64 and one thread. Then come the pin's own fixed options
    (`options`), such as a book switched off. An engine that can be limited
    is told so explicitly in every entry, because at least one (Rodent IV)
    limits its strength unless told not to. Pondering is not an engine
    option here: the match scripts never switch it on.
    """
    program = installed_binary(ctx, engine)

    def entry(name: str, elo: int | None) -> dict:
        options = []
        if engine["hash"]:
            options.append({"name": "Hash", "value": ladder_engines.HASH_MB})
        if engine["threads"]:
            options.append({"name": "Threads", "value": 1})
        for option_name, value in engine.get("options", []):
            options.append({"name": option_name, "value": value})
        if engine["limit"] is not None:
            options.append({"name": "UCI_LimitStrength", "value": elo is not None})
            if elo is not None:
                options.append({"name": "UCI_Elo", "value": elo})
        return {
            "name": name,
            "command": str(program),
            "workingDirectory": str(program.parent),
            "protocol": "uci",
            "options": options,
        }

    entries = [entry(engine["name"], None)]
    entries += [entry(f"{engine['name']}-{elo}", elo) for elo in engine.get("variants", [])]
    return entries


def engines_json_path(ctx) -> Path:
    return ctx.prefix / "engines" / "engines.json"


def write_engines_json(ctx) -> Path | None:
    """Writes the engine list of every ladder engine installed under this
    prefix, in the format of Cute Chess's engines.json, and returns its path
    (None in a dry run). scripts/linux/match.sh reads it for gauntlets.

    It is written afresh each time from what is installed, so it never names
    an engine that is not there.
    """
    path = engines_json_path(ctx)
    entries = []
    for engine in active_pins(ctx):
        if is_installed(ctx, engine):
            entries.extend(engine_entries(ctx, engine))
    if ctx.dry:
        tools.say(f"would write the engine list {path}")
        return None
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(entries, indent=2) + "\n", encoding="utf-8")
    tools.say(f"engine list: {path} ({len(entries)} entries)")
    tools.say(f"  for the match scripts, set ENGINES_JSON={path} in scripts/local.env")
    return path
