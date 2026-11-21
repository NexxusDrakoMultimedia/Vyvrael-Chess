# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""The opponent engines of Ladder Baseline 2, pinned for Linux.

Which engines, versions and names: docs/testing/ladder-Baseline-2.md and its
ratings files, which are the source of truth. This module records where each
one comes from on Linux and how the names of the ladder map to binaries; the
installer (install_tools.py) acts on it. Vyvræl itself is not in it.

Every source was read on its project's own page or repository on 2026-11-21.
Target: x86-64 with AVX2 and BMI2. Nothing here is built for AVX-512 or for
"the machine it is built on" (-march=native): a build names its architecture.

Change this table only in a reviewed commit, and keep the table in
docs/TESTING.md and docs/DEPENDENCIES.md in step with it.
"""
from __future__ import annotations

# Fields of an entry:
#   name      the engine's name in the baseline, exactly
#   version   what that name means upstream (tag, or what the engine reports)
#   source    the project's own repository or page
#   licence   as its licence file says (SPDX identifier)
#   method    "download" (an official binary) or "build" (official source)
#   url       the file fetched: a binary, an archive of binaries, or a source
#             archive of one exact tag or commit
#   sha256    digest of that file, enforced on download; every entry has one
#   archive   "none" (the file is the binary), "tar.gz" or "zip"
#   binary    path of the engine inside the archive's one top directory (or
#             the member's name in a flat zip), after any build
#   member_sha256  digest of the binary inside an archive holding several
#   beside    files and directories the engine needs next to it, by the same
#             kind of path; they are installed beside the binary
#   build     for method "build": the directory to build in, the command (a
#             list of arguments, no shell), extra environment, and the tools
#             that must be on PATH
#   hash, threads  whether the engine has those UCI options
#   options   fixed UCI options for every entry of the engine, as (name, value)
#             pairs, beside Hash, Threads and the strength settings
#   limit     (lowest, highest) UCI_Elo if it has UCI_LimitStrength, else None
#   variants  the UCI_Elo values the ladder uses: "<name>-<elo>" entries
#   notes     what a reader should know before trusting the entry
ENGINES = [
    {
        "name": "Stockfish 19",
        "version": "sf_19",
        "source": "https://github.com/official-stockfish/Stockfish",
        "licence": "GPL-3.0",
        "method": "download",
        "url": "https://github.com/official-stockfish/Stockfish/releases/download/"
        "sf_19/stockfish-linux-x86-64-universal.tar.gz",
        "sha256": "9defc0d4e55d49c65a6d042f3e571a39fcea499ade6dbe741b53b8c65e03611f",
        "archive": "tar.gz",
        "binary": "stockfish-linux-x86-64-universal",
        "hash": True,
        "threads": True,
        "limit": (1320, 3190),
        "options": [("Move Overhead", 30)],
        "variants": [1320, 1500, 1800, 2000, 2200, 2400],
        "notes": "The only official x86-64 Linux file: one binary that picks its code "
        "path for the processor at run time. There is no separate AVX2/BMI2 file to pin.",
    },
    {
        "name": "Arasan 26.1",
        "version": "v26.1",
        "source": "https://github.com/jdart1/arasan-chess",
        "licence": "MIT",
        "method": "download",
        "url": "https://github.com/jdart1/arasan-chess/releases/download/"
        "v26.1/arasan-linux-binaries-26.1.tar.gz",
        "sha256": "33c0c4f8f70c3ed5e4aa086b9cffd542ad7af616ab4adc9662e9b56fcff91ac5",
        "archive": "tar.gz",
        "binary": "arasanx-64-avx2-bmi2",
        "beside": ["arasanv8-20261005.nnue", "book.bin"],
        "hash": True,
        "threads": True,
        "limit": (1000, 3450),
        "options": [("OwnBook", False), ("Use tablebases", False), ("Move overhead", 30)],
        "variants": [1500, 1800],
        "notes": "The archive also holds an AVX-512 binary, which is not used. The network "
        "file must sit beside the binary. The engine's own book (book.bin) is installed "
        "beside it too, as in Ladder Baseline 1, but switched off with OwnBook, as in "
        "Ladder Baseline 2. UCI_Elo moves in steps of about 24.5: "
        "1500 and 1800 play as 1490 and 1784 on the engine's own scale.",
    },
    {
        "name": "Viridithas 20",
        "version": "v20.0.0",
        "source": "https://github.com/cosmobobak/viridithas",
        "licence": "MIT",
        "method": "download",
        "url": "https://github.com/cosmobobak/viridithas/releases/download/"
        "v20.0.0/viridithas-20-linux-x86-64-v3",
        "sha256": "eb53de2fde546f3ba294407e3e8986dc3859bb1818f2ef4cf2fd85d077779973",
        "archive": "none",
        "binary": "viridithas-20-linux-x86-64-v3",
        "hash": True,
        "threads": True,
        "limit": None,
        "notes": "x86-64-v3 is AVX2 and BMI2 without AVX-512. The digest is the one GitHub "
        "publishes for the file; it was not downloaded when this was recorded.",
    },
    {
        "name": "Reckless 0.9.0",
        "version": "v0.9.0",
        "source": "https://github.com/codedeliveryservice/Reckless",
        "licence": "AGPL-3.0",
        "method": "download",
        "url": "https://github.com/codedeliveryservice/Reckless/releases/download/"
        "v0.9.0/reckless-linux-avx2",
        "sha256": "09ba1634faaffec55d237a7efecfb27d5152f6f1400f24dd63af9bde00a054f6",
        "archive": "none",
        "binary": "reckless-linux-avx2",
        "hash": True,
        "threads": True,
        "limit": None,
        "options": [("MoveOverhead", 30)],
        "notes": "The digest is the one GitHub publishes for the file; it was not "
        "downloaded when this was recorded.",
    },
    {
        "name": "Stash 37.0",
        "version": "v37.0",
        "source": "https://gitlab.com/mhouppin/stash-bot",
        "licence": "GPL-3.0",
        "method": "download",
        "url": "https://gitlab.com/-/project/15050434/uploads/"
        "131cd784128eb9e8e458795daef39f04/stash-37.0-linux-x86_64-bmi2",
        "sha256": "70f8935f8e45cefec1236d3566f3cfab35b03fc78b239db5de670288a777b23c",
        "archive": "none",
        "binary": "stash-37.0-linux-x86_64-bmi2",
        "hash": True,
        "threads": True,
        "limit": None,
        "options": [("MoveOverhead", 30)],
        "notes": "The file linked from the v37.0 release, fetched by GitLab's project-number "
        "address because the address in the release text answers 404 to a script. GitLab "
        "publishes no digest, so this one is what the file had on 2026-11-21.",
    },
    {
        "name": "Princhess 0.22.0",
        "version": "0.22.0",
        "source": "https://github.com/princesslana/princhess",
        "licence": "GPL-3.0",
        "method": "build",
        "url": "https://github.com/princesslana/princhess/archive/"
        "000c92c076246a3385ff2b29aacc227c38eeda8b.tar.gz",
        "sha256": "a21d53cefdcad7b4355aef565a07ca379a29277d9b89f01fe22c41f8e2013883",
        "archive": "tar.gz",
        "binary": "target/release/princhess",
        "build": {
            "dir": ".",
            "command": ["cargo", "rustc", "--release", "--bin", "princhess", "--",
                        "-C", "target-cpu=x86-64-v3"],
            "env": {"PRINCHESS_TARGET_CPU": "x86-64-v3"},
            "tools": ["cargo", "cc", "clang"],
        },
        "hash": True,
        "threads": True,
        "limit": None,
        "notes": "Built from the tag's commit with the project's own `make build` command. "
        "The official Linux binary is not used: its tablebase code was compiled for the "
        "release machine's processor and contains AVX-512 instructions. No digest is "
        "published for the source archive; this one is what GitHub served for the "
        "commit on 2026-11-21. The build "
        "needs libclang as well as cargo and a C compiler, and a build from an archive "
        "may report itself as version 0.0.0-dev: the version comes from git upstream.",
    },
    {
        "name": "Weiss 2.0",
        "version": "v2.0",
        "source": "https://github.com/TerjeKir/weiss",
        "licence": "GPL-3.0",
        "method": "build",
        "url": "https://github.com/TerjeKir/weiss/archive/refs/tags/v2.0.tar.gz",
        "sha256": "ddcd263e493987bd7408a5267bc708f667142e5a3f47b0912cd122f2be643a98",
        "archive": "tar.gz",
        "binary": "src/weiss",
        "build": {
            "dir": "src",
            "command": ["make", "basic", "CC=gcc",
                        "CFLAGS=-std=gnu11 -Wall -Wextra -Wshadow -O3 -flto "
                        "-march=haswell -DUSE_PEXT"],
            "tools": ["gcc", "make"],
        },
        "hash": True,
        "threads": True,
        "limit": None,
        "options": [("NoobBook", False), ("OnlineSyzygy", False)],
        "notes": "No official Linux binary. The makefile's flags are given again with "
        "-march=haswell for -march=native, and without its -Werror: a current gcc "
        "warns about sign comparisons in this 2021 source, and with -Werror those "
        "warnings stopped the build on the maintainer's Linux machine (2026-11-21).",
    },
    {
        "name": "Rodent IV",
        "version": "0.33 (commit e8d84c8)",
        "source": "https://github.com/nescitus/rodent-iv",
        "licence": "GPL-3.0",
        "method": "build",
        "url": "https://github.com/nescitus/rodent-iv/archive/"
        "e8d84c8c8c189a1cf4eb27c578fc573af3b916d2.tar.gz",
        "sha256": "db372b48a8ccf22d9bfbe3e5454cbbccb81dadca0120cbc0abaee2a81a0cb3fe",
        "archive": "tar.gz",
        "binary": "sources/rodentIV",
        "beside": ["personalities", "books"],
        "build": {
            "dir": "sources",
            "command": ["sh", "-c",
                        "g++ -s -w -Wfatal-errors -pipe -DNDEBUG -O3 -fno-rtti "
                        "-finline-functions -fprefetch-loop-arrays -std=c++14 "
                        "-march=haswell -pthread src/*.cpp -o rodentIV -lm"],
            "tools": ["g++"],
        },
        "hash": True,
        "threads": True,
        "limit": (800, 2800),
        "options": [("UseBook", False), ("TimeBuffer", 30)],
        "variants": [1600, 2000],
        "notes": "The repository has no tags or releases, so the pin is a commit, chosen "
        "by the maintainer on 2026-11-21; the engine reports itself as Rodent IV 0.33. "
        "The command is the makefile's `build` rule with -march=haswell and -pthread "
        "added. The engine reads personalities/ and books/ from its own directory, so "
        "the installer puts them beside the installed binary. It switches strength limiting on by default, so the "
        "full-strength entry sets UCI_LimitStrength to false.",
    },
    {
        "name": "Vice 1.1",
        "version": "1.1 (commit 7d4fe19)",
        "source": "https://github.com/bluefeversoft/vice",
        "licence": "MIT",
        "method": "build",
        "url": "https://github.com/bluefeversoft/vice/archive/"
        "7d4fe19282ecb18bab341c64727f3a3489708d8a.tar.gz",
        "sha256": "4bcd7995ab932bd7897fd2455451b78589d95003c224fada66e67114c4706b03",
        "archive": "tar.gz",
        "binary": "Vice11/src/vice",
        "build": {
            "dir": "Vice11/src",
            "command": ["make"],
            "tools": ["gcc", "make"],
        },
        "hash": True,
        "threads": False,
        "limit": None,
        "options": [("Book", False)],
        "notes": "No tags; the commit is the one titled 'initial commit of Vice 1.1 Source'. "
        "Later commits and the main branch are a different engine (Vice 1.2). The makefile "
        "names no architecture, so the build is plain x86-64. The engine's book "
        "(performance.bin) is not placed beside the installed binary.",
    },
    {
        "name": "Fruit 2.1",
        "version": "2.1 (Debian's original tarball, 2.1.dfsg)",
        "source": "https://deb.debian.org/debian/pool/main/f/fruit/",
        "licence": "GPL-2.0",
        "method": "build",
        "url": "https://deb.debian.org/debian/pool/main/f/fruit/fruit_2.1.dfsg.orig.tar.gz",
        "sha256": "9bf6364a15f697ed8f79757e9f0f4e149987ba33693e3b79f2a210a26b9edcc3",
        "archive": "tar.gz",
        "binary": "src/fruit",
        "beside": ["book_small.bin"],
        "build": {
            "dir": "src",
            "command": ["make", "CXX=g++ -march=haswell"],
            "tools": ["g++", "make"],
        },
        "hash": True,
        "threads": False,
        "limit": None,
        "options": [("OwnBook", False)],
        "notes": "Not from the project's own page: the author's site is gone, and this is "
        "the original source as Debian keeps it, accepted by the maintainer on "
        "2026-11-21. The digest is the one in Debian's signed source description. The "
        "readme says to set -march in the makefile; it is passed through CXX instead. "
        "The small book the source ships with is installed beside the binary.",
    },
    {
        "name": "Rustic Alpha 1.5",
        "version": "1.5.0 (branch release/alpha-1/1.5)",
        "source": "https://codeberg.org/mvanthoor/rustic",
        "licence": "GPL-3.0",
        "method": "download",
        "url": "https://codeberg.org/mvanthoor/rustic/releases/download/"
        "alpha-3.0.6/rustic-alpha.zip",
        "sha256": "ac385c81fdc42a2fb1aaa9c8defb5150580990a5bcefbb8009aec4dae89664c6",
        "archive": "zip",
        "binary": "rustic-alpha-1.5-linux64",
        "member_sha256": "9fd0e12691bdf929805d8d03163c737a4657eeb5c9ff17adb434990d9a429967",
        "hash": False,
        "threads": False,
        "limit": None,
        "notes": "One official archive holds all three Rustic Alpha versions. The files are "
        "called linux64 with no processor level stated; they contain no AVX-512. This "
        "version has no UCI options at all.",
    },
    {
        "name": "Rustic Alpha 2.4",
        "version": "alpha-2.4.0",
        "source": "https://codeberg.org/mvanthoor/rustic",
        "licence": "GPL-3.0",
        "method": "download",
        "url": "https://codeberg.org/mvanthoor/rustic/releases/download/"
        "alpha-3.0.6/rustic-alpha.zip",
        "sha256": "ac385c81fdc42a2fb1aaa9c8defb5150580990a5bcefbb8009aec4dae89664c6",
        "archive": "zip",
        "binary": "rustic-alpha-2.4-linux64",
        "member_sha256": "5109e9c3c2cab50d6096c4e1d9893457ec4d34635187824a72cba28a385aa923",
        "hash": True,
        "threads": False,
        "limit": None,
        "notes": "See Rustic Alpha 1.5.",
    },
    {
        "name": "Rustic Alpha 3.0.6",
        "version": "alpha-3.0.6",
        "source": "https://codeberg.org/mvanthoor/rustic",
        "licence": "GPL-3.0",
        "method": "download",
        "url": "https://codeberg.org/mvanthoor/rustic/releases/download/"
        "alpha-3.0.6/rustic-alpha.zip",
        "sha256": "ac385c81fdc42a2fb1aaa9c8defb5150580990a5bcefbb8009aec4dae89664c6",
        "archive": "zip",
        "binary": "rustic-alpha-3.0.6-linux64",
        "member_sha256": "fdab91f1ed45c346730fafac888bf3152208730bd8d3c9c63cb4f240c1c93b48",
        "hash": True,
        "threads": False,
        "limit": None,
        "notes": "See Rustic Alpha 1.5.",
    },
    {
        "name": "CT800 1.46",
        "version": "V1.46 (released 2024-08-29)",
        "source": "https://www.ct800.net/",
        "licence": "GPL-3.0-or-later",
        "method": "build",
        "url": "https://www.ct800.net/downloads/ct800-v1.46.zip",
        "sha256": "61647c7146fd8b3219c36dc987130d92be9f0fe37d494bb41aee192c930aa1c6",
        "archive": "zip",
        "binary": "source/application-uci/ct800",
        "build": {
            "dir": "source/application-uci",
            "command": ["gcc", "-pthread", "-Wall", "-Wextra", "-Wlogical-op",
                        "-Wstrict-prototypes", "-Wshadow", "-O2", "-march=haswell",
                        "-flto=auto", "-std=c99", "-faggressive-loop-optimizations",
                        "-fno-unsafe-loop-optimizations", "-fgcse-sm", "-fgcse-las",
                        "-fgcse-after-reload", "-fno-strict-aliasing", "-fno-strict-overflow",
                        "-fno-common", "-o", "ct800", "play.c", "tuner.c", "kpk.c", "eval.c",
                        "move_gen.c", "hashtables.c", "search.c", "util.c", "book.c",
                        "kpk_table.c", "bookdata.c", "-lrt", "-Wl,-s"],
            "tools": ["gcc"],
        },
        "hash": True,
        "threads": False,
        "limit": (1000, 2250),
        "variants": [1200, 1500, 1800],
        "options": [("OwnBook", False), ("Keep Hash Tables", False),
                    ("Move Overhead [ms]", 30)],
        "notes": "The official zip holds the source, Windows binaries (its CT800_V1.46_x64.exe "
        "is the file of Ladder Baseline 2, by SHA-256) and no Linux binary: its Linux readme "
        "says to build with make_ct800_linux.sh. The command is that script's, with "
        "-march=haswell for -march=native -mtune=native and without -Werror, which the "
        "readme itself names as the workaround when a compiler warns. No digest is "
        "published; this one is what ct800.net served on 2026-11-21. The opening book is "
        "compiled in, so nothing goes beside the binary. As in Ladder Baseline 2 its book "
        "is off, the hash is cleared between games and the move overhead is 30 ms. UCI_Elo "
        "runs from 1000 to a top the engine works out from its speed at start-up: 2250 at "
        "its reference speed of 30,000 nodes a second or slower, more on a faster machine "
        "(its source starts at 2300, rounds down to 50 and keeps 50 in hand). The "
        "ladder's values are far below it. Its notes for 1.46 warn that the strength "
        "limit can play up to 50 Elo stronger, not weaker, at under 15 seconds a move.",
    },
]

# Engines of the baseline that the installer cannot provide on Linux, and why.
NOT_INSTALLABLE = {
    "Orion 1.1": "The official site (orionchess.com) offers Windows programs only and the "
    "source is not published, so there is nothing to download or build for Linux.",
    "Ethereal 12.75": "There is no official Linux binary, and the source of v12.75 does "
    "not run when built with a current compiler: it gives its per-thread data 64-byte "
    "alignment and allocates it with calloc, which does not provide that, so the engine "
    "crashes at the first `go` (seen with gcc 15.2 on 2026-11-21 at every optimisation "
    "level). Building it would mean patching the engine, which the installer does not do.",
    "Nostradamus 0.4": "Not a compiled program: a Python program with a PyTorch model "
    "(github.com/david-carteau/nostradamus, MIT). Its only release file is a Windows "
    "executable, and the model's weights are not in the repository but in a zip on the "
    "author's site that no page there links to and no digest covers. Installing it would "
    "need a Python environment with PyTorch, which the installer does not set up (the "
    "maintainer's decision, 2026-11-21).",
}

# What every engine of the ladder is given, as in the baselines. Each pin's
# `options` add the rest of Ladder Baseline 2's settings: books and online
# tablebases off and 30 ms of move overhead, under the engine's own option
# names (read from each engine's `uci` answer on 2026-11-21).
HASH_MB = 64


def ladder_names() -> list[str]:
    """Every name the installer can provide: each engine, then its limited
    variants, in the table's order."""
    names = []
    for engine in ENGINES:
        names.append(engine["name"])
        names.extend(f"{engine['name']}-{elo}" for elo in engine.get("variants", []))
    return names


def by_name(name: str) -> dict:
    """The entry whose name is `name` (a base name, not a variant)."""
    for engine in ENGINES:
        if engine["name"] == name:
            return engine
    raise KeyError(name)
