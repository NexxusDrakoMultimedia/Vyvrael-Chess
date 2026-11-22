# Lichess build history

What the bot account [Vyvrael-Chess](https://lichess.org/@/Vyvrael-Chess)
played with each release of Vyvræl. One section per build, from the moment
the bot started running it to the moment it was stopped for the next
upgrade.

> **This is a record, not a measurement of strength.** The games are casual
> challenges from a handful of people. Their Lichess ratings are a
> different scale from the project's ladder, and a few games against a few
> opponents say nothing reliable about a rating. Strength is measured only
> with the match scripts (see [TESTING.md](TESTING.md)). The bot's own
> "3000" in the games is a figure Lichess shows for it, not a rating it
> earned: it has played no rated games.

## How a section is made

1. **Before stopping the bot for an upgrade**, the maintainer exports the
   bot's games from Lichess as one PGN (the "pre-release PGN") and hands it
   to the agent. The export is done by hand on the Lichess website. The
   agent never calls the Lichess API (see [LICHESS.md](LICHESS.md)).
2. The build's window runs from the first game after the previous
   pre-release PGN up to this export. Games already recorded for an earlier
   build are left out.
3. The PGN is not committed. The section records its file name, size,
   SHA-256 and game count, and lists every game by its Lichess link, with
   the date, Vyvræl's colour, the opponent's Lichess rating at the time, the
   result, the length and how the game ended.
4. Opponents are not named here. Their accounts are visible on Lichess
   through the game links, but this repository does not need to repeat
   them. The maintainer's own account is called that.

## 0.1.0-alpha.1

- **Build:** the 0.1.0-alpha.1 release, run on the Linux machine by
  lichess-bot. Which binary the bot ran is the maintainer's record; it is
  not in the PGN.
- **Window:** from the first game, 2026-11-20 17:13 UTC, to the pre-release
  export on 2026-11-22. The last game was 2026-11-21 19:30 UTC.
- **Pre-release PGN:** `lichess_Vyvrael-Chess_2026-11-22.pgn`, 7 games,
  3,851 bytes, SHA-256
  `60d832ff02948db70519fe511c3fccb33133a64c2a3a6672b309289a43c96642`.
  Not committed.
- **Settings seen in the games:** casual blitz only, 3+2 (180+2) in every
  game, standard chess, as the checklist in LICHESS.md asks.

| Date (UTC) | Game | Vyvræl | Opponent (Lichess rating) | Result | Moves | Ended by |
| --- | --- | --- | --- | --- | --: | --- |
| 2026-11-20 17:13 | [jk7XDkqP](https://lichess.org/game/jk7XDkqP) | White | The maintainer's account (777) | Won | 13 | Mate (13. Bd3#) |
| 2026-11-20 21:46 | [AV8ymKJm](https://lichess.org/game/AV8ymKJm) | Black | Opponent A (1375) | Won | 28 | Resignation |
| 2026-11-20 21:50 | [Oxn2YIvQ](https://lichess.org/game/Oxn2YIvQ) | White | Opponent A (1375) | Won | 22 | Resignation |
| 2026-11-20 21:55 | [LMY8TMBH](https://lichess.org/game/LMY8TMBH) | Black | Opponent A (1375) | Won | 26 | Resignation |
| 2026-11-21 19:26 | [w3qd6rBz](https://lichess.org/game/w3qd6rBz) | White | Opponent B (1399) | Won | 2 | Resignation |
| 2026-11-21 19:28 | [gUSrp7fn](https://lichess.org/game/gUSrp7fn) | Black | Opponent B (1399) | Won | 11 | Resignation |
| 2026-11-21 19:30 | [qkvM7v0w](https://lichess.org/game/qkvM7v0w) | Black | Opponent B (1399) | Won | 10 | Resignation |

"Resignation" means the PGN says `Termination "Normal"` and the last move is
not mate: the opponent resigned. No game in the export was lost on time
or drawn.

**Total:** 7 games, 7 wins, 3 opponents (the maintainer included).

What the games show:

- **The same openings every time.** As White, Vyvræl played 1. d4 and
  2. Nc3 in all three games. As Black against 1. e4 it answered 1... Nc6
  and 2... d5 in all four. 0.1.0-alpha.1 has no move randomisation without
  its strength-limiting options, so it repeats its openings. 0.1.0-alpha.2
  added `Variety`, which the bot's configuration
  sets to 10 (see step 12 of the checklist in LICHESS.md), so later
  sections should show more varied openings.
- **No game lost on time:** all 7 ended normally. A Lichess export does
  not show aborted games or the engine's own output, so the PGN cannot
  rule out aborts or engine errors; those would be in lichess-bot's logs.
- **Too few games, against too few opponents,** to say anything about
  strength. Two opponents played three games each, and three games ended
  by move 11.
