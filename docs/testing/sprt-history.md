# SPRT history

One row per sequential probability ratio test: what was tested, between
which two builds, on what terms, and how it ended. Every search,
evaluation or network change is accepted only if its row says **H1
accepted** (see [AGENTS.md](../../AGENTS.md) and "Strength matches, SPRT
and ratings" in [TESTING.md](../TESTING.md)).

Rows are appended by `scripts/record_sprt.py` from a run's notes and log,
never typed: `python scripts/record_sprt.py <run>.notes.txt --change
"<what was tested>"` (see "Recording an SPRT result" in TESTING.md).

How to read a row:

- **Machine:** the Windows workstation, unless the change says otherwise.
  Results from different machines are not compared.
- **Test build, base build:** the names used in the games and the first
  twelve digits of each file's SHA-256, with any options the build was
  given. The test build is the change; the base build is what it is
  measured against.
- **Bounds, alpha, beta:** the two Elo hypotheses and the error rates the
  test was run with. `[0, 5]` asks for a gain; `[-5, 0]` asks only that
  nothing was lost.
- **Conditions:** the time control, the hash size, the number of games at
  once and the round cap.
- **Games, W - L - D, Elo:** the last score `cutechess-cli` printed, from
  the test build's side. The Elo figure is a rough estimate with its
  margin, not the size of the gain.
- **LLR:** the final log-likelihood ratio with the bounds it ran between.
- **Outcome:** **H1 accepted** (the change is kept), H0 accepted (it is
  rejected) or undecided (the round cap came first). **Undecided is not a
  pass:** the round cap is raised and the test run again, or the change is
  dropped.
- **Run:** the first twelve digits of the SHA-256 of the run's PGN, which
  identifies the run. The PGN, log and notes are kept by the maintainer
  and are not committed.

| Date | Change | Test build | Base build | Bounds | alpha, beta | Conditions | Games | W - L - D | Elo | LLR | Outcome | Run |
| --- | --- | --- | --- | --- | --- | --- | --: | --- | --- | --- | --- | --- |
| 2026-11-27 | Captures first in the main search, MVV-LVA with promotions (milestone 7, item 1) | Vyvrael dev-db4342c (`a4958414db72`) | Vyvrael dev-d344d0a (`70c372e180a4`) | [0, 10] | 0.05, 0.05 | 10+0.1, Hash 64, 6 at once, cap 5000 rounds | 120 | 96 - 12 - 12 | 301.3 +/- 81.5 | 2.97 (-2.94, 2.94) | **H1 accepted** | `f38fad1831cb` |
| 2026-11-27 | Transposition table in the main search (milestone 7, item 2a) | Vyvrael dev-573363b (`f1a140633b97`) | Vyvrael dev-db4342c (`a4958414db72`) | [0, 10] | 0.05, 0.05 | 10+0.1, Hash 64, 6 at once, cap 5000 rounds | 344 | 183 - 96 - 65 | 89.8 +/- 34.0 | 2.96 (-2.94, 2.94) | **H1 accepted** | `ad16d90abf11` |
| 2026-11-27 | The table's move tried first (milestone 7, item 2b) | Vyvrael dev-7c396b9 (`eed1a7d9b36b`) | Vyvrael dev-573363b (`f1a140633b97`) | [0, 10] | 0.05, 0.05 | 10+0.1, Hash 64, 6 at once, cap 5000 rounds | 242 | 132 - 53 - 57 | 117.7 +/- 39.9 | 2.96 (-2.94, 2.94) | **H1 accepted** | `9710d85acf32` |
| 2026-11-27 | Losing captures last and out of quiescence, by static exchange evaluation (milestone 7, item 3) | Vyvrael dev-8b4967f (`39431e39af0a`) | Vyvrael dev-a39e327 (`5e67944fddba`) | [0, 10] | 0.05, 0.05 | 10+0.1, Hash 64, 6 at once, cap 5000 rounds | 1160 | 486 - 392 - 282 | 28.2 +/- 17.4 | 2.95 (-2.94, 2.94) | **H1 accepted** | `9c8de5ceed4c` |
| 2026-11-27 | Delta pruning in quiescence, margin 200, checking captures exempt (milestone 7, item 4) | Vyvrael dev-4bfd377 (`60344aa981c8`) | Vyvrael dev-8b4967f (`39431e39af0a`) | [0, 10] | 0.05, 0.05 | 10+0.1, Hash 64, 6 at once, cap 5000 rounds | 10000 | 3898 - 3748 - 2354 | 5.2 +/- 5.9 | 0.229 (-2.94, 2.94) | undecided | `d35a397d5e43` |
| 2026-11-28 | Killer moves, two per ply, after the captures that do not lose material (milestone 7, item 6) | Vyvrael dev-6a415b2 (`e5c596054cba`) | Vyvrael dev-3a9fce0 (`f828862c2be4`) | [0, 10] | 0.05, 0.05 | 10+0.1, Hash 64, 6 at once, cap 5000 rounds | 372 | 185 - 102 - 85 | 78.8 +/- 31.6 | 2.95 (-2.94, 2.94) | **H1 accepted** | `f1b57a770a57` |
| 2026-11-28 | Null-move pruning, from depth 3, reduction 2 + depth / 4, verified with one piece left (milestone 7, item 9) | Vyvrael dev-33056b1 (`2fd2d05a113d`) | Vyvrael dev-a62a1b9 (`68154f14e32c`) | [0, 10] | 0.05, 0.05 | 10+0.1, Hash 64, 6 at once, cap 5000 rounds | 198 | 117 - 38 - 43 | 146.8 +/- 45.8 | 2.97 (-2.94, 2.94) | **H1 accepted** | `023be622299e` |
| 2026-11-28 | Late move reductions, quiet moves from the fourth on, from depth 3 (milestone 7, item 10) | Vyvrael dev-ca0a908 (`8b59e112bd92`) | Vyvrael dev-33056b1 (`2fd2d05a113d`) | [0, 5] | 0.05, 0.05 | 10+0.1, Hash 64, 6 at once, cap 5000 rounds | 620 | 309 - 155 - 156 | 88.1 +/- 24.2 | 2.95 (-2.94, 2.94) | **H1 accepted** | `00e8619ba2cf` |
| 2026-11-28 | Reverse futility pruning, up to depth 6, margin 100 a ply, with the null-move verification made to search real moves (milestone 7, item 13) | Vyvrael dev-f2a3124 (`d330663f52b7`) | Vyvrael dev-ca0a908 (`8b59e112bd92`) | [0, 5] | 0.05, 0.05 | 10+0.1, Hash 64, 6 at once, cap 5000 rounds | 698 | 329 - 178 - 191 | 76.4 +/- 22.3 | 2.95 (-2.94, 2.94) | **H1 accepted** | `25da9526092d` |
