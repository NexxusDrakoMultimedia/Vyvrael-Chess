# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
"""The opponent engines of Ladder Baseline 2, pinned for Linux and Windows.

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
#   pext      fields that replace the above for the engine's PEXT build, where
#             it has one; without it the one build serves both (see pin())
#   windows   the source fields for Windows, with their own "pext" (see pin())
#   options   fixed UCI options for every entry of the engine, as (name, value)
#             pairs, beside Hash, Threads and the strength settings
#   limit     (lowest, highest) UCI_Elo if it has UCI_LimitStrength, else None
#   variants  the UCI_Elo values the ladder uses: "<name>-<elo>" entries
#   notes     what a reader should know before trusting the entry
ENGINES = [
    {
        "name": "Stockfish 19",
        "windows": {
            "method": "download",
            "url": "https://github.com/official-stockfish/Stockfish/releases/download/"
            "sf_19/stockfish-windows-x86-64-universal.zip",
            "sha256": "3c8bf1f9ea66a09350a40df4f632288285ac206d99f33ab5842c408fc30b48a7",
            "archive": "zip",
            "binary": "stockfish/stockfish-windows-x86-64-universal.exe",
            "member_sha256": "45bc8e4969147db9c2eb533810637994619bff0eacc81ccfd9854394901bcbd0",
        },
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
        "windows": {
            "method": "download",
            "url": "https://github.com/jdart1/arasan-chess/releases/download/"
            "v26.1/arasan26.1.zip",
            "sha256": "57af9ca4d8a9f57662421c1b54652b246b02cce011f9e2ed26c2252813420690",
            "archive": "zip",
            "binary": "arasanx-64-avx2.exe",
            "member_sha256": "67ff0441343bd5f6b9b066593b741a9947ff0c376dc7387dbc171b084c2aa73c",
            "beside": ["arasanv8-20261005.nnue", "book.bin"],
            "pext": {
                "binary": "arasanx-64-avx2-bmi2.exe",
                "member_sha256": "cca0e46c7a1d18d4fe407827d96da2f02753e29ac62dc6748a6fbf83ca7985e0",
            },
        },
        "version": "v26.1",
        "source": "https://github.com/jdart1/arasan-chess",
        "licence": "MIT",
        "method": "download",
        "url": "https://github.com/jdart1/arasan-chess/releases/download/"
        "v26.1/arasan-linux-binaries-26.1.tar.gz",
        "sha256": "33c0c4f8f70c3ed5e4aa086b9cffd542ad7af616ab4adc9662e9b56fcff91ac5",
        "archive": "tar.gz",
        "binary": "arasanx-64-avx2",
        "pext": {"binary": "arasanx-64-avx2-bmi2"},
        "beside": ["arasanv8-20261005.nnue", "book.bin"],
        "hash": True,
        "threads": True,
        "limit": (1000, 3450),
        "options": [("OwnBook", False), ("Use tablebases", False), ("Move overhead", 30)],
        "variants": [1500, 1800],
        "notes": "One archive holds the avx2 build and the avx2-bmi2 build, whose sliding-"
        "piece attacks use PEXT (the project's Makefile: BUILD_TYPE=avx2-bmi2 adds -DBMI2), "
        "so the build only changes which file is installed. The avx2 file's name follows "
        "the Makefile's naming and has not yet been installed from this archive on Linux. "
        "The archive also holds an AVX-512 binary, which is not used. The network "
        "file must sit beside the binary. The engine's own book (book.bin) is installed "
        "beside it too, as in Ladder Baseline 1, but switched off with OwnBook, as in "
        "Ladder Baseline 2. UCI_Elo moves in steps of about 24.5: "
        "1500 and 1800 play as 1490 and 1784 on the engine's own scale.",
    },
    {
        "name": "Viridithas 20",
        "windows": {
            "method": "download",
            "url": "https://github.com/cosmobobak/viridithas/releases/download/"
            "v20.0.0/viridithas-20-win-x86-64-v3.exe",
            "sha256": "c7fa86b2b46fe1c34b0ea3715ad3b4a61d3d984ec63d50b1d6b4f187332fbc51",
            "archive": "none",
            "binary": "viridithas-20-win-x86-64-v3.exe",
        },
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
        "windows": {
            "method": "download",
            "url": "https://github.com/codedeliveryservice/Reckless/releases/download/"
            "v0.9.0/reckless-windows-avx2.exe",
            "sha256": "b74ead5648cfa7a7a9f51d04566cf00f56dcf90dacd1252990906223bf1891b8",
            "archive": "none",
            "binary": "reckless-windows-avx2.exe",
        },
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
        "windows": {
            "method": "download",
            "url": "https://gitlab.com/-/project/15050434/uploads/"
            "9e7866a80b3e6434c8b78d87f3953543/stash-37.0-windows-x86_64-popcnt.exe",
            "sha256": "e86edfdafa35416ceba57f81ad4d74f637bd582b8fdd8f6a1f189a163d0a06d8",
            "archive": "none",
            "binary": "stash-37.0-windows-x86_64-popcnt.exe",
            "pext": {
                "url": "https://gitlab.com/-/project/15050434/uploads/"
                "4e00da44722df643ff0de2be377bdd47/stash-37.0-windows-x86_64-bmi2.exe",
                "sha256": "7cd0e8bb0bf48e819e4acf3fe9323a51e9e5b1813d073ff295d802fcd8fd7bb3",
                "binary": "stash-37.0-windows-x86_64-bmi2.exe",
            },
        },
        "version": "v37.0",
        "source": "https://gitlab.com/mhouppin/stash-bot",
        "licence": "GPL-3.0",
        "method": "download",
        "url": "https://gitlab.com/-/project/15050434/uploads/"
        "826ccefa3ce9e480250fc57b34000001/stash-37.0-linux-x86_64-popcnt",
        "sha256": "8643547bfe9505d580275f48ce77c67d36f5c6325e97dadc5260bd74d16dd286",
        "archive": "none",
        "binary": "stash-37.0-linux-x86_64-popcnt",
        "pext": {
            "url": "https://gitlab.com/-/project/15050434/uploads/"
            "131cd784128eb9e8e458795daef39f04/stash-37.0-linux-x86_64-bmi2",
            "sha256": "70f8935f8e45cefec1236d3566f3cfab35b03fc78b239db5de670288a777b23c",
            "binary": "stash-37.0-linux-x86_64-bmi2",
        },
        "hash": True,
        "threads": True,
        "limit": None,
        "options": [("MoveOverhead", 30)],
        "notes": "The file linked from the v37.0 release, fetched by GitLab's project-number "
        "address because the address in the release text answers 404 to a script. GitLab "
        "publishes no digest, so these are what the files had on 2026-11-21. Stash has no "
        "AVX2 build: the default is its popcnt build, and the PEXT build is its bmi2 build "
        "(ARCH=x86-64-bmi2, -DUSE_PEXT).",
    },
    {
        "name": "Princhess 0.22.0",
        "windows": {
            "method": "download",
            "url": "https://github.com/princesslana/princhess/releases/download/"
            "0.22.0/princhess.exe",
            "sha256": "7a95e41985483b8689e77cb32ff4687f41f56f91317b9615fa5c2edeabc09b81",
            "archive": "none",
            "binary": "princhess.exe",
        },
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
        "windows": {
            "method": "download",
            "url": "https://github.com/TerjeKir/weiss/releases/download/"
            "v2.0/Weiss-2.0-windows-collection.zip",
            "sha256": "2bbc728d8eb9ac1e60c2cc57b843ca43a4887854cc790db8f8fe238873c255ca",
            "archive": "zip",
            "binary": "weiss-popcnt.exe",
            "member_sha256": "c56623c15c777442eb2658eb0915b494726c085a5ad4222609ea98a5ed1e848b",
            "pext": {
                "binary": "weiss-pext.exe",
                "member_sha256": "5bc2262139ae0712b8589c95b0cd343d536cf22a9a6ca4ab98e73c03ce8f0eed",
            },
        },
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
                        "-march=haswell"],
            "tools": ["gcc", "make"],
        },
        "pext": {"build": {
            "dir": "src",
            "command": ["make", "basic", "CC=gcc",
                        "CFLAGS=-std=gnu11 -Wall -Wextra -Wshadow -O3 -flto "
                        "-march=haswell -DUSE_PEXT"],
            "tools": ["gcc", "make"],
        }},
        "hash": True,
        "threads": True,
        "limit": None,
        "options": [("NoobBook", False), ("OnlineSyzygy", False)],
        "notes": "No official Linux binary. The makefile's flags are given again with "
        "-march=haswell for -march=native, and without its -Werror: a current gcc "
        "warns about sign comparisons in this 2021 source, and with -Werror those "
        "warnings stopped the build on the maintainer's Linux machine (2026-11-21). The "
        "PEXT build adds -DUSE_PEXT, as the makefile's own pext build does.",
    },
    {
        "name": "Rodent IV",
        "windows": {
            "method": "download",
            "url": "http://pkoziol.cal24.pl/rodent/FinalRodent.7z",
            "http_reason": "The author's own site serves the Windows release over plain "
            "HTTP only: its HTTPS certificate is for another host name (2026-11-21).",
            "sha256": "b118bc21f846b3ef41cf58f60d0039d8715fe7dbc6670aab5dbcdfc0770bd6c2",
            "archive": "7z",
            "binary": "rodent-iv-x64.exe",
            "member_sha256": "39c879b2dfdaf0252e94db547397d95422fee76ea8bafa584bfdc03e4d648de5",
            "beside": ["personalities", "books"],
            "version": "FinalRodent.7z (the author's last Windows release, 2019)",
            "source": "http://pkoziol.cal24.pl/rodent/rodent.htm",
            "notes": "The author's own Windows release, a 7-Zip archive unpacked with "
            "Windows' tar; its rodent-iv-x64.exe is the file of Ladder Baseline 2, by "
            "SHA-256, and the personalities and books it reads sit beside it. It is served "
            "over plain HTTP only (see http_reason); the digests are what it served on "
            "2026-11-21. It switches strength limiting on by default, so the full-strength "
            "entry sets UCI_LimitStrength to false.",
        },
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
        "windows": {
            "method": "download",
            "url": "https://www.bluefeversoft.com/downloads/Vice11.zip",
            "sha256": "795b53997f94a12bd236be5b5b3847acf912a73e689f432f96ca16cb8ccfa6b6",
            "archive": "zip",
            "binary": "Vice11/vice11_win32.exe",
            "member_sha256": "92827b547b32b7a71b28ec41c99bf1f6aaaf17fa5bb3265232bb015fb5f93a8a",
        },
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
        "windows": {
            "method": "download",
            "url": "https://raw.githubusercontent.com/Warpten/Fruit-2.1/"
            "09627df3835f205c92ece013219c7a99ac4a023a/fruit_21.exe",
            "sha256": "7f321a960887af6c52b5768a2c5b5a85720358398a8790fd8deb281fe2cb2565",
            "archive": "none",
            "binary": "fruit_21.exe",
            "version": "2.1 (fruit_21.exe at commit 09627df)",
            "source": "https://github.com/Warpten/Fruit-2.1",
            "notes": "The author's site is gone and there is no official Windows download. "
            "This is the Fruit 2.1 distribution as kept in github.com/Warpten/Fruit-2.1 "
            "(GPL-2.0), chosen by the maintainer on 2026-11-21, pinned to a commit; its "
            "fruit_21.exe is the file of Ladder Baseline 2, by SHA-256. Its book is off, so "
            "book_small.bin is not installed beside it.",
        },
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
        "windows": {
            "method": "download",
            "url": "https://codeberg.org/mvanthoor/rustic/releases/download/"
            "alpha-3.0.6/rustic-alpha.zip",
            "sha256": "ac385c81fdc42a2fb1aaa9c8defb5150580990a5bcefbb8009aec4dae89664c6",
            "archive": "zip",
            "binary": "rustic-alpha-1.5-win64.exe",
            "member_sha256": "2ee1c05f25380c9be12f4d2deeb3279fa66fe2e2b3f104f545a19aea7ed63c18",
        },
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
        "windows": {
            "method": "download",
            "url": "https://codeberg.org/mvanthoor/rustic/releases/download/"
            "alpha-3.0.6/rustic-alpha.zip",
            "sha256": "ac385c81fdc42a2fb1aaa9c8defb5150580990a5bcefbb8009aec4dae89664c6",
            "archive": "zip",
            "binary": "rustic-alpha-2.4-win64.exe",
            "member_sha256": "0780316684b43af4666eec542b8e8670a79155ae0e6de87bb6e3c3b5d1600e53",
        },
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
        "windows": {
            "method": "download",
            "url": "https://codeberg.org/mvanthoor/rustic/releases/download/"
            "alpha-3.0.6/rustic-alpha.zip",
            "sha256": "ac385c81fdc42a2fb1aaa9c8defb5150580990a5bcefbb8009aec4dae89664c6",
            "archive": "zip",
            "binary": "rustic-alpha-3.0.6-win64.exe",
            "member_sha256": "fa8c651196c05bfd670c25fbad3712983eb85fc29d68593f7ee841ce5ec2f95e",
        },
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
        "windows": {
            "method": "download",
            "url": "https://www.ct800.net/downloads/ct800-v1.46.zip",
            "sha256": "61647c7146fd8b3219c36dc987130d92be9f0fe37d494bb41aee192c930aa1c6",
            "archive": "zip",
            "binary": "ct800-v1.46/binaries/Windows/CT800_V1.46_x64.exe",
            "member_sha256": "dc0c1f6e49c56898815c5232b0d05db39412428597b66c14fc1b6c7b43d9405c",
        },
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
    # Engines with a Windows program only; NOT_INSTALLABLE["linux"] says why.
    {
        "name": "Ethereal 12.75",
        "linux": None,
        "windows": {
            "method": "download",
            "url": "https://github.com/AndyGrant/Ethereal/releases/download/"
            "v12.75/Ethereal12.75-x64-popcnt-avx2.exe",
            "sha256": "5e891c6d5f0775ab71a1f200d49bac7f69e8c6653673cc9c779ee74768648791",
            "archive": "none",
            "binary": "Ethereal12.75-x64-popcnt-avx2.exe",
            "pext": {
                "url": "https://github.com/AndyGrant/Ethereal/releases/download/"
                "v12.75/Ethereal12.75-x64-pext-avx2.exe",
                "sha256": "22f12797fd43e220047b41c2f6373a20cabd893895a6e0a3c273339b911430e2",
                "binary": "Ethereal12.75-x64-pext-avx2.exe",
            },
        },
        "version": "v12.75",
        "source": "https://github.com/AndyGrant/Ethereal",
        "licence": "GPL-3.0",
        "hash": True,
        "threads": True,
        "limit": None,
        "options": [("MoveOverhead", 30)],
        "notes": "Official Windows binaries of the v12.75 release: popcnt-avx2 and "
        "pext-avx2. GitHub publishes no digest for these 2021 files; these are what it "
        "served on 2026-11-21, and the pext file is the one of Ladder Baseline 2.",
    },
    {
        "name": "Orion 1.1",
        "linux": None,
        "windows": {
            "method": "download",
            "url": "https://www.orionchess.com/download/Orion-v1.1.zip",
            "sha256": "7c109a3a6d476a1fea8582833bd0023334e995b3324376a9ef04ea5a731a43ac",
            "archive": "zip",
            "binary": "Orion-v1.1/orion64-v1.1-pop-avx.exe",
            "member_sha256": "712ba9848e84c9a7e92eb84553f6e56e4a6241e0d2f7ade86f5a18ed9ecf9083",
            "beside": ["Orion-v1.1/orion64-v1.1.nn"],
        },
        "version": "1.1",
        "source": "https://www.orionchess.com/",
        "licence": "Freeware",
        "hash": True,
        "threads": True,
        "limit": None,
        "notes": "Closed source; the author's site allows free use and unmodified "
        "redistribution, and the installer only downloads it. The zip holds three builds "
        "(plain, popcnt, popcnt-avx) and no AVX2 or PEXT one; pop-avx is the fastest and "
        "the one of Ladder Baseline 2, for both builds. Its network, orion64-v1.1.nn, is "
        "the default of its NN_FILE option and sits beside it. No digest is published; "
        "this one is what orionchess.com served on 2026-11-21.",
    },
    {
        "name": "Nostradamus 0.4",
        "linux": None,
        "windows": {
            "method": "download",
            "url": "https://github.com/david-carteau/nostradamus/releases/download/"
            "v0.4/nostradamus-v0.4.exe",
            "sha256": "485d3b606ad8784e6435b08f32e9a1f789562c7abc6064ce807868e64d1cb5e2",
            "archive": "none",
            "binary": "nostradamus-v0.4.exe",
        },
        "version": "v0.4",
        "source": "https://github.com/david-carteau/nostradamus",
        "licence": "MIT",
        "hash": False,
        "threads": False,
        "limit": None,
        "notes": "A Python program with a small PyTorch model, packed by PyInstaller into "
        "one Windows file (about 156 MB) with its model inside; the digest is the one "
        "GitHub publishes, and the file is the one of Ladder Baseline 2. It offers a Hash "
        "option fixed at 0 and nothing else to set, and reports no score, depth or time. "
        "Its first start unpacks itself and can be slow.",
    },
]

# Engines of the baseline that the installer cannot provide, per system, and why.
NOT_INSTALLABLE = {"linux": {
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
}, "windows": {}}

# What every engine of the ladder is given, as in the baselines. Each pin's
# `options` add the rest of Ladder Baseline 2's settings: books and online
# tablebases off and 30 ms of move overhead, under the engine's own option
# names (read from each engine's `uci` answer on 2026-11-21).
HASH_MB = 64


SYSTEMS = ("linux", "windows")
BUILDS = ("avx2", "pext")

# The fields of an entry that say where its program comes from. At the top
# level they describe the Linux program; an entry's "windows" section gives
# its own set for Windows.
SOURCE_FIELDS = ("method", "url", "sha256", "archive", "binary", "member_sha256",
                 "beside", "build")


def pin(engine: dict, system: str = "linux", build: str = "avx2") -> dict | None:
    """The engine's pin for one system and build, as one flat entry, or None
    if the engine cannot be installed there.

    On Linux the entry's own fields are the pin (an entry with "linux": None
    exists on Windows only). On Windows its "windows" section replaces the
    source fields. With build "pext", a "pext" section (at the top level for
    Linux, inside "windows" for Windows) replaces fields again: the PEXT
    build of an engine that offers one. Engines without one have a single
    build, used for both.
    """
    if system not in SYSTEMS or build not in BUILDS:
        raise ValueError(f"unknown system or build: {system}, {build}")
    if system == "linux":
        if "linux" in engine and engine["linux"] is None:
            return None
        flat = {k: v for k, v in engine.items() if k not in ("linux", "windows", "pext")}
        override = engine.get("pext")
    else:
        section = engine.get("windows")
        if section is None:
            return None
        flat = {k: v for k, v in engine.items()
                if k not in SOURCE_FIELDS + ("linux", "windows", "pext")}
        flat.update({k: v for k, v in section.items() if k != "pext"})
        override = section.get("pext")
    if build == "pext" and override:
        flat.update(override)
    return flat


def pins(system: str = "linux", build: str = "avx2") -> list[dict]:
    """Every engine that can be installed on `system`, as flat pins."""
    return [p for p in (pin(engine, system, build) for engine in ENGINES) if p is not None]


def ladder_names(system: str = "linux") -> list[str]:
    """Every name the installer can provide on `system`: each engine, then
    its limited variants, in the table's order."""
    names = []
    for engine in pins(system):
        names.append(engine["name"])
        names.extend(f"{engine['name']}-{elo}" for elo in engine.get("variants", []))
    return names


def by_name(name: str) -> dict:
    """The entry whose name is `name` (a base name, not a variant)."""
    for engine in ENGINES:
        if engine["name"] == name:
            return engine
    raise KeyError(name)
