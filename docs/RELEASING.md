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

**The release workflow** (`.github/workflows/release.yml`, described in
[`proposals/release-workflow.md`](proposals/release-workflow.md)) is
installed and was tried once on a test tag (`v0.1.0-rc.0`, 2026-11-22):
every job passed and it made a draft with the eight files. 0.1.0 is its
first real release.

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
      and the Ordo rating **for the release's source**. For 0.1.0 the
      `UCI_Elo` calibration and the rating are **provisional**; say so.
      No strength claim without a measurement behind it.
      **Which build is measured:** the released binaries only exist once
      the workflow has run (step 4), after the notes are fixed in the
      tagged commit. So the rating is measured beforehand, on the Windows
      workstation, with an `avx2` build (`-C target-cpu=x86-64-v3`) of
      the release's source, and the notes say so: same source and build
      level as the released `avx2` file, built by hand with the local
      toolchain, not the workflow's binary itself. Its search is the
      same, which the bench count shows; only its speed may differ
      slightly.
      **Links:** the workflow publishes this file as the body of the
      release (`gh release create --notes-file`), and relative links do
      not resolve on a release page. Write every link to a file in the
      repository as an absolute one, pinned to the tag:
      `https://github.com/NexxusDrakoMultimedia/Vyvrael-Chess/blob/v<version>/<path>`.

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
      smoke-tested, `UCI_Elo` calibrated, KI-8 measured for the build
      released, a measured Ordo rating in the notes).

## 3. Merge and tag (the maintainer)

- [ ] Review the release branch and merge it into `main`.
- [ ] For 0.2.0, before the tag: install the proposed `pext` change of
      the release workflow (copy `docs/proposals/release-workflow.yml`
      over `.github/workflows/release.yml`, keeping the installed file's
      header comment). Without it the release has no `pext` files, which
      its notes promise.
- [ ] Create the annotated tag on that merge and push it:

      ```sh
      git tag -a v<version> -m "Vyvræl <version>"
      git push origin main v<version>
      ```

## 4. Build and publish (the workflow, then the maintainer)

- [ ] The tag starts the release workflow. It checks that the tag matches
      `Cargo.toml`, that the changelog has a dated section for the version
      and that its notes file `docs/releases/<version>.md` exists,
      runs the gates, builds the `avx2` and plain binaries for Windows and
      Linux (and the `pext` ones, once the maintainer has installed the
      change proposed for 0.2.0 in
      [`proposals/release-workflow.md`](proposals/release-workflow.md)),
      smoke-tests each, and creates a **draft** release with the
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
