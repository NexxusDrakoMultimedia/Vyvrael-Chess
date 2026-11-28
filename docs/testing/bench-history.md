# Bench history

One row per measured build: the node count of `vyvrael bench`, its speed,
and the speed of perft. The node count is a fingerprint of what the search
does and must be the same on every run of a build; a change of it is a
search change. Speeds are for comparing builds measured the same way on the
same machine, nothing else. See "Bench" in [TESTING.md](../TESTING.md).

How each row is measured, unless its notes say otherwise:

- **Machine:** the Windows workstation (Windows 11, Ryzen 7 5700X), idle (no
  match or build running), as the maintainer confirmed for each row.
- **Build:** `cargo build --release` with the profile in `Cargo.toml`
  (`lto = true`, one codegen unit, `panic = "abort"`), no `target-cpu`
  setting: a plain x86-64 build.
- **Bench:** `vyvrael bench` (seven positions, depth 4), run five times; the
  median speed is given. The node count was the same in all five runs.
- **Perft:** `vyvrael perft 6` from the start position (119,060,324 nodes),
  run three times; the median is given.

| Date | Commit | Change | Bench nodes | Bench speed (nodes/s) | Perft 6 speed (nodes/s) | Notes |
| --- | --- | --- | --: | --: | --: | --- |
| 2026-11-22 | `f99cf07` | `bench` added (queued task 1) | 16,611,416 | 2,604,889 | 26,942,137 (4,419 ms) | First row. rustc 1.100.0. Before milestone 7: no move ordering in the main search and no transposition table. |
| 2026-11-27 | `db4342c` | The main search tries captures first (MVV-LVA with promotions; milestone 7, item 1) | 459,926 | 2,662,793 | not measured again (move generation is unchanged) | Search change: every pinned score and line is unchanged, only the counts fall. Speed is the median of three runs, not idle-checked, and of little meaning now that the bench takes 0.17 s. Kiwipete at depth 3: 46,307 nodes, from 4,513,442. |
| 2026-11-27 | `573363b` | The transposition table, with its cutoffs in the main search (milestone 7, item 2a) | 394,939 | 3,446,383 | not measured again | Search change. Each bench position gets a new 16 MiB table. Start position at depth 6: 1,297,093 nodes, from 2,403,736. One run of the plain build; the bench is too short for its speed to mean much. |
| 2026-11-27 | `7c396b9` | The table's move tried first, at the root too (milestone 7, item 2b) | 206,097 | 3,317,093 | not measured again | Search change. Start position at depth 6: 235,247 nodes, from 1,297,093; the first move gives 50.5% of the main search's cutoffs there, from 30.9%. |
| 2026-11-27 | `8b4967f` | Captures that lose material by static exchange evaluation: last in the main search, left out of quiescence (milestone 7, item 3) | 77,407 | 2,946,761 | not measured again | Search change. Every pinned score and line is unchanged. Start position at depth 6: 176,611 nodes, from 235,247; the first move gives 55.8% of the main search's cutoffs there, from 50.5%. One run; the bench is now far too short to time. |
