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
