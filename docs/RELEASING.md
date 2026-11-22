# Releasing Vyvræl

The procedure for every release from 0.1.0 on. It is the one place the
steps are written: [`docs/prompts/release.md`](prompts/release.md) (the
`/release-prep` prompt) and the "Releases" section of
[AGENTS.md](../AGENTS.md) point here. What each version needs is in the
"Releases" table of [ROADMAP.md](ROADMAP.md), and the builds and file names
are in its "Release artifacts".

**Who does what:** agents prepare a release on a branch. The maintainer
merges it, creates and pushes the tag, checks the draft and publishes.
Agents never create or push tags and never publish.

**The release workflow** (`.github/workflows/release.yml`, proposed in
[`proposals/release-workflow.md`](proposals/release-workflow.md)) is not
installed yet and has not run. Until it has run once on a test tag, step 4
cannot be relied on.

Copy the checklist below into the release's tracking note or pull request
and tick it as you go.

## 1. Prepare (agents, `/release-prep`)

On a branch `release/<version>` from `main`:

- [ ] Look at every commit since the last version tag, not just the
      latest, and choose the version under Semantic Versioning (major for
      breaking changes, minor for backward-compatible features, patch for
      fixes; before 1.0.0 a minor version may change anything) and the
      ROADMAP's "Releases" table. Say which and why, and wait for the
      maintainer's approval before editing.
- [ ] Bump the version: `Cargo.toml` (`[workspace.package]`), `Cargo.lock`,
      and every document that shows it (the README's status line, the
      sample session in `UCI.md`).
- [ ] `CHANGELOG.md`, following Keep a Changelog: the Unreleased entries
      become a dated section `## [<version>] - YYYY-MM-DD` with a short
      summary, grouped under Added, Changed, Deprecated, Removed, Fixed and
      Security and written for users rather than copied from commit
      messages; a new, empty Unreleased section; the comparison links at
      the bottom. One commit for the bump and the changelog.
- [ ] `docs/releases/<version>.md`, the release notes. Near the top they
      say **which file to take**: `avx2`, the modern and preferred build;
      `pext` where PEXT is fast (Intel, and AMD from Zen 3), from 0.2.0 on;
      the plain file only for older machines. They say **what is measured
      and what is not**, and cite the results file under `docs/testing/`
      and the Ordo rating **for the exact build released**. For 0.1.0 the
      `UCI_Elo` calibration and the rating are **provisional**; say so.
      No strength claim without a measurement behind it.

## 2. Gates (agents, then the maintainer confirms)

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --release`
- [ ] `python3 -m unittest discover -s scripts/tests`
- [ ] Complete games in `cutechess-cli` at 10+0.1 with a build of the
      release commit: no illegal move, crash, stall or loss on time
      (see "Complete games in cutechess-cli" in [TESTING.md](TESTING.md)).
      Recorded in `TESTING.md`.
- [ ] For 0.1.0 and later, the milestone's own requirements from the
      "Releases" table (for 0.1.0: the scripts documented and
      smoke-tested, `UCI_Elo` calibrated, KI-8 resolved, a measured Ordo
      rating in the notes).

## 3. Merge and tag (the maintainer)

- [ ] Review the release branch and merge it into `main`.
- [ ] Create the annotated tag on that merge and push it:

      ```sh
      git tag -a v<version> -m "Vyvræl <version>"
      git push origin main v<version>
      ```

## 4. Build and publish (the workflow, then the maintainer)

- [ ] The tag starts the release workflow. It checks that the tag matches
      `Cargo.toml` and that the changelog and the notes have the version,
      runs the gates, builds the `avx2` and plain binaries for Windows and
      Linux, smoke-tests each, and creates a **draft** release with the
      binaries, their `.sha256` files and the notes.
- [ ] Check the draft: the files are there and named as in "Release
      artifacts"; download them and check each against its `.sha256` file
      (`sha256sum -c <file>.sha256`).
- [ ] Run one binary once (`uci`, `isready`, `go depth 6`, `quit`).
- [ ] Publish the draft.

## 5. After publishing (agents)

- [ ] Commit the results file for the release under `docs/testing/`
      (`results-<version>.md`, with Ordo's output beside it), as
      "Recording results" in [ROADMAP.md](ROADMAP.md) describes, if it is
      not already there.
- [ ] Note in the ROADMAP's "Releases" table that the version was
      released, and update `docs/LICHESS.md` when the bot changes to it.
