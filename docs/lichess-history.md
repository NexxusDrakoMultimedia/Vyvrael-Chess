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
   to the agent. The export is done by hand on the Lichess website, with
   clock times, opening names and comments switched on, so the PGN carries
   `[%clk ...]` times and the `ECO` and `Opening` tags. The agent never
   calls the Lichess API (see [LICHESS.md](LICHESS.md)).
2. The build's window runs from the first game after the previous
   pre-release PGN up to this export. Games already recorded for an earlier
   build are left out.
3. The PGN is not committed. The section records its file name, size,
   SHA-256 and game count, and lists every game by its Lichess link, with
   the date, Vyvræl's colour, the opponent's Lichess rating at the time, the
   opening, the result, the length, how the game ended and Vyvræl's clock
   after its last move.
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
  9,940 bytes, SHA-256
  `d24d881ce07600f8106022de8b8252fb679a028f12d6e2610ba70633e7861978`.
  Not committed. It replaces a first export of the same name and the same
  7 games (3,851 bytes) that had no clock times or opening tags.
- **Settings seen in the games:** casual blitz only, 3+2 (180+2) in every
  game, standard chess, as the checklist in LICHESS.md asks.

| Date (UTC) | Game | Vyvræl | Opponent (Lichess rating) | Opening (ECO) | Result | Moves | Ended by | Vyvræl's clock |
| --- | --- | --- | --- | --- | --- | --: | --- | --: |
| 2026-11-20 17:13 | [jk7XDkqP](https://lichess.org/game/jk7XDkqP) | White | The maintainer's account (777) | Rapport-Jobava System (D01) | Won | 13 | Mate (13. Bd3#) | 2:15 |
| 2026-11-20 21:46 | [AV8ymKJm](https://lichess.org/game/AV8ymKJm) | Black | Opponent A (1375) | Nimzowitsch Defence, Scandinavian Variation, Advance (B00) | Won | 28 | Resignation | 1:15 |
| 2026-11-20 21:50 | [Oxn2YIvQ](https://lichess.org/game/Oxn2YIvQ) | White | Opponent A (1375) | Indian Defence, Maddigan Gambit (A45) | Won | 22 | Resignation | 1:31 |
| 2026-11-20 21:55 | [LMY8TMBH](https://lichess.org/game/LMY8TMBH) | Black | Opponent A (1375) | Nimzowitsch Defence, Scandinavian Variation, Advance (B00) | Won | 26 | Resignation | 1:18 |
| 2026-11-21 19:26 | [w3qd6rBz](https://lichess.org/game/w3qd6rBz) | White | Opponent B (1399) | Queen's Pawn Game, Chigorin Variation (A45) | Won | 2 | Resignation | 2:54 |
| 2026-11-21 19:28 | [gUSrp7fn](https://lichess.org/game/gUSrp7fn) | Black | Opponent B (1399) | Nimzowitsch Defence, Scandinavian Variation, Bogoljubow (B00) | Won | 11 | Resignation | 2:10 |
| 2026-11-21 19:30 | [qkvM7v0w](https://lichess.org/game/qkvM7v0w) | Black | Opponent B (1399) | Nimzowitsch Defence, Scandinavian Variation, Exchange (B00) | Won | 10 | Resignation | 2:20 |

"Resignation" means the PGN ends with Lichess's "White resigns." or
"Black resigns." comment. No game in the export was lost on time or
drawn. "Vyvræl's clock" is its `[%clk]` time after its last move, out of
3:00 plus 2 seconds a move. The opening names are Lichess's `Opening`
tags, shortened.

**Total:** 7 games, 7 wins, 3 opponents (the maintainer included).

What the games show:

- **The same openings every time.** As White, Vyvræl played 1. d4 and
  2. Nc3 in all three games. As Black against 1. e4 it answered 1... Nc6
  and 2... d5 in all four. 0.1.0-alpha.1 has no move randomisation without
  its strength-limiting options, so it repeats its openings. 0.1.0-alpha.2
  added `Variety`, which the bot's configuration
  sets to 10 (see step 12 of the checklist in LICHESS.md), so later
  sections should show more varied openings. The opponents' replies,
  not Vyvræl's, are what gave the games different opening names.
- **Steady, unhurried clock use.** Vyvræl spent about 6 seconds a move
  (counting the increment back), a little less as its clock ran down, and
  never fell below 1:15. Its moves in the opening took as long as its
  moves later on: it has no opening book, so it searches from move 1.
- **No game lost on time:** all 7 ended normally. A Lichess export does
  not show aborted games or the engine's own output, so the PGN cannot
  rule out aborts or engine errors; those would be in lichess-bot's logs.
- **One draw offer.** Opponent B offered a draw with 1... Nf6 in
  w3qd6rBz. The game went on, and they resigned after 2. Nc3.
- **Too few games, against too few opponents,** to say anything about
  strength. Two opponents played three games each, and three games ended
  by move 11.

## 0.1.0-alpha.2

- **Build:** the 0.1.0-alpha.2 release, run on the Linux machine by
  lichess-bot. Its settings are the maintainer's record; the checklist in
  LICHESS.md suggests `Variety: 10` from this release on.
- **Window:** from the upgrade after the alpha.1 pre-release export on
  2026-11-22 to the upgrade to 0.1.0 on 2026-11-23.
- **Games:** none. The maintainer confirmed on 2026-11-23 that the bot
  played no games with 0.1.0-alpha.2, so this section has no pre-release
  PGN and nothing to show about `Variety` in play yet.

## 0.1.0

- **Build:** the 0.1.0 release, its Linux `avx2` file
  (`vyvrael-0.1.0-avx2`), run on the Linux machine by lichess-bot. The
  maintainer rebuilt that machine for this upgrade and put the binary in
  place with `install_tools.py --vyvrael 0.1.0`.
- **Window:** from the upgrade on 2026-11-23 to the pre-release export on
  2026-11-28, made before the bot is stopped for 0.2.0. The first game was
  2026-11-23 17:21 UTC, the last 2026-11-24 18:45 UTC. The PGN does not
  say which build played; that the game of 2026-11-23 was played after
  the upgrade rests on the maintainer's statement of that day that
  0.1.0-alpha.2 played none.
- **Pre-release PGN:** `lichess_Vyvrael-Chess_2026-11-28.pgn`, 9 games,
  12,681 bytes, SHA-256
  `58f91202e019520d6739afcfabc0cb9a3f1fa1577e61f29173574a6aeb87ee70`.
  Not committed. Seven of its games are those of the 0.1.0-alpha.1
  section; the two below are new.
- **Settings seen in the games:** casual games of standard chess, one
  rapid at 7+3 (420+3) and one blitz at 3+2 (180+2). The rapid game is
  the first the bot played at another time control than 3+2.

| Date (UTC) | Game | Vyvræl | Opponent (Lichess rating) | Opening (ECO) | Result | Moves | Ended by | Vyvræl's clock |
| --- | --- | --- | --- | --- | --- | --: | --- | --: |
| 2026-11-23 17:21 | [cMLrxU4V](https://lichess.org/game/cMLrxU4V) | Black | Opponent C (1479) | Four Knights Game, Italian Variation (C50) | Won | 23 | Mate (23... Qf2#) | 3:58 |
| 2026-11-24 18:45 | [OoV5D8lA](https://lichess.org/game/OoV5D8lA) | Black | Opponent C (1567) | Nimzowitsch Defence, Scandinavian Variation, Exchange, Marshall Gambit (B00) | Won | 6 | Opponent's time ran out | 2:43 |

The columns are as in the 0.1.0-alpha.1 section. The two ratings of
Opponent C are Lichess's rapid and blitz ratings, which are separate.

**Total:** 2 games, 2 wins, 1 opponent.

What the games show:

- **1... Nc6 again, then two different second moves.** Against 1. e4
  Vyvræl answered 1... Nc6 in both games, as 0.1.0-alpha.1 did in all
  four of its games as Black. After 2. Nc3 it played 2... Nf6, after
  2. d4 again 2... d5. Two games cannot show whether `Variety` changed
  anything: the opponent's second move differed, which alone explains
  the different replies.
- **The rapid game was won by an attack on the king:** a bishop given up
  on f2 at move 11, and mate on f2 at move 23 with queen and rook. It is
  one game against one opponent and says nothing about strength.
- **More time used at the longer control.** At 7+3 Vyvræl spent about 11
  seconds a move (counting the increment back) and finished with 3:58
  of 7:00. At 3+2 it spent about 5 seconds a move, as before.
- **One game ended on the opponent's clock:** in OoV5D8lA the opponent
  had 2:10 after their sixth move and made no seventh; Lichess records
  "Black wins on time" after 6... Nxc2+. Vyvræl lost no game on time.
- **Vyvræl did not play White in this window.**
- As before, an export shows neither aborted games nor the engine's own
  output; those would be in lichess-bot's logs.

## 0.2.0

- **Build:** the 0.2.0 release, once it is published and the bot is
  upgraded to it.
- **Window:** opens with that upgrade.
- **Games:** recorded when the window closes.
