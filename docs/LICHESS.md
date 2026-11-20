# Playing on Lichess

How Vyvræl is to be run as a Lichess bot, who does what, and the rules the
bot keeps. The plan and the rules here were set by the maintainer on
2026-11-20.

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
- **Until the upgrade, nobody plays a game on this account**: not against a
  person, not against the Lichess AI, not by accepting a challenge. One game
  makes the upgrade impossible for good. Puzzles are not games.

## Policy

- **Casual games only, from the first release.** After the maintainer tags
  `v0.1.0-alpha.1`, the maintainer upgrades the account to BOT and runs it
  with lichess-bot in casual mode only.
- **Rated play stays off until the maintainer says so.** The earliest point
  is after 0.2.0 (milestone 7), when the engine's strength has been measured
  (milestone 6) and its search heuristics have each been accepted by SPRT,
  and after the branding in [ROADMAP.md](ROADMAP.md) is done. Before
  enabling it, the maintainer re-reads the Terms of Service and the Bot API
  documentation, because Lichess may change them.
- **Strength is unmeasured.** Nothing in the bot's profile or anywhere else
  says how strong it is until milestone 6 has measured it.
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

1. **Re-check the account.** On the profile page, confirm it still shows
   zero games in every time control and on the Games tab, and no BOT title.
   If a game has been played, stop: this account cannot be upgraded.
2. **Do not start or accept any game** between now and the upgrade.
3. **Create a personal API token** on your own account with only the
   `bot:play` scope ("Play games with the bot API"). Keep it only in your
   local configuration or environment. Never paste it into a chat, an issue,
   a commit or an AI agent.
4. **Upgrade the account** to BOT, either with the request above or by
   letting lichess-bot do it (its `-u` flag; confirm the flag in the README
   of the version you install). This cannot be undone.
5. **Verify** that `title` reads `BOT` in the "Get my profile" response, or
   that the profile page shows the BOT title.
6. **Install lichess-bot separately**, outside this repository. On Linux,
   `scripts/install_tools.py --tool lichess-bot` installs the pinned version
   (see [TESTING.md](TESTING.md)); it does not upgrade the account and does
   not start anything.
7. **Copy `config.yml.default` to a local `config.yml`.** It is git-ignored
   here (`/config.yml`, `/lichess-bot/`) and must stay out of every
   repository. The installer already does this copy, with an empty token.
8. **Casual only.** Under `challenge:`, reduce `modes:` to the single entry
   `casual` (delete `rated`). Leave `matchmaking: allow_matchmaking: false`.
   If you ever switch matchmaking on, set `matchmaking: challenge_mode:` to
   `"casual"` first; its default is `"random"`, which sends rated challenges
   too.
9. **Conservative time controls.** Under `challenge:`, take `bullet` out of
   `time_controls:` and keep `blitz`, `rapid` and `classical`; keep
   `variants:` at `standard` only; keep `concurrency: 1`. A suggested start
   is `min_base: 180` and `min_increment: 2`, so every game has an
   increment. Never accept UltraBullet (lichess-bot's default file says bots
   may not play it and offers no such entry).
10. **Challenge games only.** lichess-bot plays challenges; do not enter
    the account in tournaments or pools by any other means.
11. **Point it at the engine.** Under `engine:`, set `dir:` and `name:` to
    the release binary, keep `protocol: "uci"`, and set `ponder: false`
    (Vyvræl does not ponder; the default is `true`).
12. **Set the engine options.** Under `engine: uci_options:`, keep only
    options Vyvræl offers: `Move Overhead: 100`, `Threads: 1`, `Hash: 64`.
    Delete `SyzygyPath` and `UCI_ShowWDL`, which Vyvræl does not have, and
    change the default `Threads: 4`, which Vyvræl refuses. Leave
    `UCI_LimitStrength` out, so the bot plays at full strength.
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
  so it cannot be set there; the values in step 9 are a suggestion in its
  place, for the maintainer to decide.
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
  - Known risk: that service is written with `Restart=always`, so a bot
    that keeps failing is restarted every ten seconds. Do not enable the
    service until the bot has played cleanly from a terminal.
- **No boosting.** No rated games against the maintainer's own accounts or
  bots, and no arranged results.
- **No sandbagging.** On a rated account, do not weaken the engine with
  `UCI_LimitStrength` or `UCI_Elo` unless the profile says so. This is the
  project's recommendation, not a quotation from the Terms of Service; the
  maintainer decides.

## Suggested profile bio

Short and honest: the engine and its version, the tagline, casual only,
strength not yet measured. For example:

> Vyvræl 0.1.0-alpha.1: a dragon-flavoured UCI chess engine in Rust, built
> with AI and playing for the attack. Casual games only; strength still
> being measured. Source: github.com/NexxusDrakoMultimedia/Vyvrael-Chess

Update the version when the bot's binary changes. The profile's flag is
Australia (see the branding section of [ROADMAP.md](ROADMAP.md)).
