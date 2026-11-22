# Playing on Lichess

How Vyvræl is to be run as a Lichess bot, who does what, and the rules the
bot keeps. The plan and the rules here were set by the maintainer on
2026-11-20.

**Status:** the maintainer upgraded the account to BOT after tagging
`v0.1.0-alpha.1`, and the bot has played casual games through lichess-bot
since 2026-11-20. [lichess-history.md](lichess-history.md) records them.

**AI coding agents do not touch Lichess.** No agent requests, stores, logs
or commits an API token, calls the Lichess API, runs the BOT upgrade,
launches lichess-bot or adds it to CI. Every step in the checklist below is
the maintainer's. If a task seems to need the token or the account, the
agent stops and says so.

## The account

- Username: **Vyvrael-Chess**, <https://lichess.org/@/Vyvrael-Chess>. It
  belongs to the maintainer. The username is public and not a secret.
- **Observed by the maintainer on 2026-11-20** from the profile page (a dated
  observation, not a current fact, and not verified by anyone else): created
  2026-11-19; 0 games in every time control and 0 games on the Games tab; not
  yet a BOT; already a member of the "Lichess Bots" team; profile 66%
  complete.
- **Before the upgrade, nobody played a game on this account**: not against
  a person, not against the Lichess AI, not by accepting a challenge. One
  game would have made the upgrade impossible for good. The upgrade is done;
  this rule stays here as the record of how it was done.
- **The games the bot played, build by build,** are recorded in
  [lichess-history.md](lichess-history.md). Before stopping the bot to
  upgrade it to a new release, the maintainer exports its games as a
  pre-release PGN by hand on the Lichess website; that export closes the
  old build's section.

## Policy

- **Casual games only, from the first release.** After tagging
  `v0.1.0-alpha.1`, the maintainer upgraded the account to BOT and has run
  it with lichess-bot in casual mode only since 2026-11-20.
- **Rated play stays off until the maintainer says so.** The earliest point
  is after 0.2.0 (milestone 7), when the engine's strength has been measured
  (milestone 6) and its search heuristics have each been accepted by SPRT,
  and after the branding in [ROADMAP.md](ROADMAP.md) is done. Before
  enabling it, the maintainer re-reads the Terms of Service and the Bot API
  documentation, because Lichess may change them.
- **Strength is measured only on the project's own ladder.** Milestone 6
  measured 0.1.0-alpha.2 there ([results-0.1.0-alpha.2.md](testing/results-0.1.0-alpha.2.md)):
  an ordering against other engines, not a Lichess, CCRL or FIDE rating.
  Nothing in the bot's profile quotes it as one.
- **lichess-bot only ever runs on the maintainer's Linux machine**, the one
  that is always on. It is not installed or started on the Windows
  workstation, which is for development and for watching games.
- lichess-bot is a separate program, installed and run by the maintainer. It
  is not vendored, not linked and not a Cargo dependency. Its pinned version
  and licence are in [DEPENDENCIES.md](DEPENDENCIES.md).

## Facts from the Lichess Bot API documentation

From <https://lichess.org/api>, Bot section, as given to the project by the
maintainer on 2026-11-20:

- The upgrade is `POST https://lichess.org/api/bot/account/upgrade` with a
  Bearer token.
- The API page does not name the token scope the upgrade needs. **The scope
  is `bot:play`, as confirmed by the maintainer.** The maintainer re-checks
  it against lichess-bot's own instructions when installing. The token is
  created with that scope and no other.
- The account cannot have played any game before becoming a Bot account.
  **The upgrade is irreversible**, and afterwards the account can only play
  as a Bot.
- After the upgrade, `title` in the "Get my profile" response reads `BOT`.
- Only Bot accounts can use the Bot API, which allows engine play. Moves are
  submitted in UCI format.
- The documentation points to the official lichess-bot client as the
  alternative to calling the endpoint directly.

## Bot rules

**Supplied by the maintainer** from Lichess's bot documentation. They have
not been verified by the project. **The maintainer re-checks them against
the live documentation before going live.**

- Bots can only play challenge games: pools and tournaments are off-limits.
- Bots cannot play UltraBullet (1/4+0), because it needs too many requests;
  0+1 and 1/2+0 are allowed.
- Bots must follow the Lichess Terms of Service, in particular on
  sandbagging, constant aborting and boosting.
- Bot developers are advised to play casual only while testing their bot's
  logic, and to avoid breaking the Terms of Service.

## Terms of Service

<https://lichess.org/terms-of-service>, as read by the maintainer on
2026-11-20. They contain no clauses specific to bots; the bot rules are the
ones above. They do say that:

- using engines during a live game is cheating, with listed exceptions;
- artificially boosting or sandbagging a rating is cheating;
- administrators may ban or close accounts at their discretion.

## Checklist for the maintainer

Do these in order, yourself. Do not hand any of them to an AI agent.

Steps 1 to 5 were the one-time BOT upgrade, which is done. They stay here
as the record of how it was done. Steps 6 to 14 apply again whenever
lichess-bot is installed or set up afresh.

1. **Re-check the account.** On the profile page, confirm it still shows
   zero games in every time control and on the Games tab, and no BOT title.
   If a game has been played, stop: this account cannot be upgraded.
2. **Do not start or accept any game** between now and the upgrade.
3. **Create a personal API token** on your own account with only the
   `bot:play` scope ("Play games with the bot API"). Keep it only in your
   local configuration or environment. Never paste it into a chat, an issue,
   a commit or an AI agent.
4. **Upgrade the account** to BOT with the request above. This cannot be
   undone. The other way is lichess-bot's `-u` flag (confirm it in the
   README of the version you install), but not at this point: lichess-bot
   is not installed or configured yet, and it is expected to start playing
   straight after upgrading, with whatever `config.yml` says. If you
   prefer `-u`, do steps 6 to 13 first, then upgrade with it and carry on
   at step 14.
5. **Verify** that `title` reads `BOT` in the "Get my profile" response, or
   that the profile page shows the BOT title.
6. **Install lichess-bot separately**, outside this repository, **on the
   Linux machine only**. `scripts/install_tools.py --tool lichess-bot`
   installs the pinned version there (see [TESTING.md](TESTING.md)); it does
   not upgrade the account and does not start anything. The engine binary
   it runs is a Linux build of the release.
7. **Copy `config.yml.default` to a local `config.yml`.** It is git-ignored
   here (`/config.yml`, `/lichess-bot/`) and must stay out of every
   repository. The installer already does this copy, with an empty token.
   It also fills in `dir:` and `name:` under `engine:`: the binary given
   with `--engine`, or else `<prefix>/engines/vyvrael`, a directory it
   creates. With `--systemd` it writes a token file,
   `~/.config/vyvrael/lichess-token.env`, holding a commented-out
   `LICHESS_BOT_TOKEN=` line; a token put there stays out of `config.yml`
   (see [TESTING.md](TESTING.md)).
8. **Casual only.** Under `challenge:`, reduce `modes:` to the single entry
   `casual` (delete `rated`). Leave `matchmaking: allow_matchmaking: false`.
   If you ever switch matchmaking on, set `matchmaking: challenge_mode:` to
   `"casual"` first; its default is `"random"`, which sends rated challenges
   too.
9. **Conservative time controls.** Under `challenge:`, take `bullet` out of
   `time_controls:` and keep `blitz`, `rapid` and `classical`; keep
   `variants:` at `standard` only; keep `concurrency: 1`. Set
   `min_base: 180` and `min_increment: 2`, so every game has an
   increment. Never accept UltraBullet (lichess-bot's default file says bots
   may not play it and offers no such entry).
10. **Challenge games only.** lichess-bot plays challenges; do not enter
    the account in tournaments or pools by any other means.
11. **Point it at the engine.** Under `engine:`, check that `dir:` and
    `name:` (set by the installer in step 7) lead to the release binary,
    keep `protocol: "uci"`, and set `ponder: false`
    (Vyvræl does not ponder; the default is `true`).
12. **Set the engine options.** Under `engine: uci_options:`, keep only
    options Vyvræl offers: `Move Overhead: 100`, `Threads: 1`, `Hash: 64`,
    and, from 0.1.0-alpha.2 on, `Variety: 10`. Delete `SyzygyPath` and
    `UCI_ShowWDL`, which Vyvræl does not have, and change the default
    `Threads: 4`, which Vyvræl refuses. Leave `UCI_LimitStrength` out, so
    the bot plays at full strength, and leave `Seed` out, so every game takes
    its own seed (logged in lichess-bot's engine output). `Variety: 10` lets
    the bot vary its first twelve moves among moves checked to be within 10
    centipawns of its best, so it does not repeat the same games; a release
    before 0.1.0-alpha.2 has no `Variety` and refuses it.
13. **Keep lichess-bot's own `move_overhead: 2000`** (top level, in
    milliseconds). See "Move overhead" below.
14. **Start the bot and watch the first games** from start to finish. Stop
    it at the first sign of trouble (see "No constant aborting").

The Lichess Bots team membership is already done.

### What could not be verified

- The key names above were read on 2026-11-20 from `config.yml.default` of
  the pinned lichess-bot commit
  (`06f4f10b5a5a70549b9fc5f8f759a532723b5dab`). **If you install another
  version, read its `config.yml.default` and use its names.**
- No one has yet run Vyvræl under lichess-bot. How lichess-bot reacts to an
  option the engine refuses, and that the settings above are enough for a
  clean first game, are unconfirmed until step 14.
- "10+0.1" is the time control of the project's own matches (ten seconds
  plus a tenth of a second per move). Lichess increments are whole seconds,
  so it cannot be set there; the values in step 9 (three minutes and two
  seconds a move as the fastest game) stand in its place, as the maintainer
  confirmed on 2026-11-21.
- The bot rules and the Terms of Service points are the maintainer's
  reading, recorded here as given.

### Move overhead

Two settings keep time back, and they add up:

- `engine: uci_options: Move Overhead` is Vyvræl's own option (see
  [UCI.md](UCI.md)): milliseconds the engine keeps back from each time
  limit. **Start at 100**, which is also lichess-bot's default. The engine's
  default of 30 suits a match tool on the same machine; a bot's moves cross
  the network twice.
- `move_overhead` at the top level of `config.yml` is lichess-bot's: it
  takes that much off the clock before telling the engine how much time
  there is. Its default is 2000.

Why err on the large side: a bot that loses on time is worse than one that
thinks a little less, and Vyvræl always finishes its first search depth
before it looks at the clock (known issue KI-10 in
[ROADMAP.md](ROADMAP.md)), which in rare positions takes far longer than
usual. If the bot still loses on time, raise the values; lower them only
after many games without a time loss.

That known issue applies to the casual bot for as long as it runs a build
from before milestone 7, which is where it is fixed: in a position with very
many captures the engine can overrun its time for the move. Until then
`Move Overhead` and watching for losses on time are the whole mitigation.

## Anti-abuse

These follow the Terms of Service points above.

- **Engine use.** The account is a BOT, run only by the engine through
  lichess-bot. Nobody plays on it by hand, and nobody uses it, or any
  engine, to assist a human account.
- **No constant aborting.** If the engine crashes or fails to start,
  lichess-bot may accept challenge after challenge and abort each one. So:
  - watch the first games, and afterwards look at the log regularly;
  - the signs are repeated engine errors or tracebacks in lichess-bot's
    output, games on the profile that end as "Aborted" after zero or one
    move, and time losses in the opening;
  - **stop the bot instead of letting it run**: Ctrl-C in its terminal, or
    `systemctl --user stop vyvrael-lichess-bot` if it runs as the service
    the installer writes. Fix the cause before starting it again.
  - The service restarts the bot only after a failure, waits 30 seconds
    each time, and gives up after three failed starts in ten minutes
    (`systemctl --user status vyvrael-lichess-bot` then shows it as
    failed). That limits the damage; it does not replace looking. Do not
    enable the service until the bot has played cleanly from a terminal.
- **No boosting.** No rated games against the maintainer's own accounts or
  bots, and no arranged results.
- **No sandbagging.** On a rated account, do not weaken the engine with
  `UCI_LimitStrength` or `UCI_Elo` unless the profile says so. This is the
  project's recommendation, not a quotation from the Terms of Service; the
  maintainer decides.

## Suggested profile bio

Short and honest: the engine and its version, the tagline, casual only,
and that strength is measured only on the project's own engine ladder, not
as a Lichess rating. For example:

> Vyvræl 0.1.0-alpha.1: a dragon-flavoured UCI chess engine in Rust, built
> with AI and playing for the attack. Casual games only; strength measured
> only against the project's own engine ladder.
> Source: github.com/NexxusDrakoMultimedia/Vyvrael-Chess

Update the version when the bot's binary changes. The profile's flag is
Australia (see the branding section of [ROADMAP.md](ROADMAP.md)).
