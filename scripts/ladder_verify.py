# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""Checks that an installed engine really runs, for install_tools.py.

Each entry of the engine list is started, greeted with `uci`, given its
options, asked `isready`, and made to search the start position to depth 6.
Every wait has a time limit, so an engine that hangs cannot hang the
installer, and whatever happens the engine's process is ended.
"""
from __future__ import annotations

import queue
import re
import subprocess
import threading
import time

import install_tools as tools
import ladder_engines
import ladder_install

# Seconds to wait for each answer. Generous: a first start may unpack a
# network, and depth 6 is instant for every engine of the ladder.
TIMEOUTS = {"uciok": 20.0, "readyok": 30.0, "bestmove": 60.0}

MOVE = re.compile(r"^[a-h][1-8][a-h][1-8][qrbn]?$")


class Conversation:
    """A started engine, with its output read on a thread so that every wait
    can time out."""

    def __init__(self, command: list[str], cwd: str | None):
        self.process = subprocess.Popen(
            command, cwd=cwd, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True, encoding="utf-8", errors="replace",
        )
        self.lines: queue.Queue = queue.Queue()
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self) -> None:
        for line in self.process.stdout:
            self.lines.put(line.rstrip("\r\n"))
        self.lines.put(None)  # the engine closed its output

    def send(self, line: str) -> None:
        try:
            self.process.stdin.write(line + "\n")
            self.process.stdin.flush()
        except OSError:
            pass  # the engine is gone; the next wait reports it

    def wait_for(self, word: str, seconds: float) -> str:
        """The first line that starts with `word`.

        Raises RuntimeError if the engine exits or the time runs out first.
        """
        deadline = time.monotonic() + seconds
        while True:
            left = deadline - time.monotonic()
            if left <= 0:
                raise RuntimeError(f"no `{word}` within {seconds:g} s")
            try:
                line = self.lines.get(timeout=left)
            except queue.Empty:
                raise RuntimeError(f"no `{word}` within {seconds:g} s") from None
            if line is None:
                raise RuntimeError(f"the engine exited{self._status()} before `{word}`")
            if line.split(" ", 1)[0] == word:
                return line

    def _status(self) -> str:
        """How the engine ended, for the message: its exit status, or the
        signal that killed it (a crash shows as a signal on Linux)."""
        try:
            code = self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            return ""
        if code < 0:
            return f" (killed by signal {-code})"
        return f" (exit status {code})"

    def close(self) -> None:
        self.send("quit")
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=10)
        for stream in (self.process.stdin, self.process.stdout):
            try:
                stream.close()
            except OSError:
                pass


def check_engine(
    command: list[str], cwd: str | None, options: list[dict], timeouts: dict | None = None
) -> tuple[bool, str]:
    """Starts an engine and takes it through `uci`, its options, `isready`
    and `go depth 6`. Returns whether it passed and one line saying what
    happened.
    """
    limits = dict(TIMEOUTS, **(timeouts or {}))
    try:
        engine = Conversation(command, cwd)
    except OSError as problem:
        return False, f"could not be started: {problem}"
    try:
        engine.send("uci")
        engine.wait_for("uciok", limits["uciok"])
        for option in options:
            value = option["value"]
            if isinstance(value, bool):
                value = "true" if value else "false"
            engine.send(f"setoption name {option['name']} value {value}")
        engine.send("isready")
        engine.wait_for("readyok", limits["readyok"])
        engine.send("ucinewgame")
        engine.send("position startpos")
        engine.send("go depth 6")
        words = engine.wait_for("bestmove", limits["bestmove"]).split()
        move = words[1] if len(words) > 1 else ""
        if not MOVE.match(move):
            return False, f"answered `{' '.join(words)}`, which names no move"
        return True, f"bestmove {move}"
    except RuntimeError as problem:
        return False, str(problem)
    finally:
        engine.close()


def verify_engines(ctx, only: list[str] | None) -> list[str]:
    """Checks every installed ladder engine (or those of --only), each under
    all its names, prints PASS or FAIL per name and returns the names that
    failed. An engine that is not installed is reported and skipped."""
    failed = []
    for engine in ladder_install.select(ctx, only):
        if not ladder_install.is_installed(ctx, engine):
            tools.say(f"SKIP  {engine['name']}: not installed")
            continue
        for entry in ladder_install.engine_entries(ctx, engine):
            passed, detail = check_engine(
                [entry["command"]], entry["workingDirectory"], entry["options"]
            )
            tools.say(f"{'PASS' if passed else 'FAIL'}  {entry['name']}: {detail}")
            if not passed:
                failed.append(entry["name"])
    return failed
