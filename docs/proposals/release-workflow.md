# Proposal: the release workflow

**Status: a proposal, not installed and not yet run.** Agents do not edit
`.github/workflows`; the maintainer reviews this proposal and puts it in
place. Do not call it working until it has run once on a test tag (see
"Trying it" below).

The workflow is [`release-workflow.yml`](release-workflow.yml). From 0.1.0
it builds the release binaries that were built by hand up to 0.1.0-alpha.2,
and it creates a **draft** GitHub release with them. It never publishes: the
maintainer checks the draft and publishes it. The whole procedure around it
is in [RELEASING.md](../RELEASING.md).

## What it does

It runs only when a tag matching `v*.*.*` is pushed: never for a branch
push or a pull request. The maintainer creates and pushes the tag. Four
jobs, each starting only if the one before it passed:

1. **check-version** (Ubuntu): the tag without its `v` must equal the
   version in `Cargo.toml`'s `[workspace.package]`, `CHANGELOG.md` must have
   a dated section `## [<version>] - YYYY-MM-DD`, and
   `docs/releases/<version>.md` must exist. Otherwise the run fails before
   anything is built.
2. **gates** (Ubuntu): the same checks as `ci.yml`: `cargo fmt --check`,
   `cargo clippy -D warnings`, `cargo test --workspace --release` and the
   script tests.
3. **build**, four jobs (Windows on `windows-latest` and Linux on
   `ubuntu-latest`, each in two builds):

   | Build | Flags | Windows file | Linux file |
   | --- | --- | --- | --- |
   | `avx2` | `-C target-cpu=x86-64-v3` | `vyvrael-<version>-avx2.exe` | `vyvrael-<version>-avx2` |
   | plain | none (any x86-64) | `vyvrael-<version>.exe` | `vyvrael-<version>` |

   The names are those of "Release artifacts" in
   [ROADMAP.md](../ROADMAP.md). Never `target-cpu=native`, never an
   AVX-512 build. The `pext` build is added in milestone 7 (0.2.0), when the
   engine gets sliding-piece attacks by PEXT; a comment in the matrix marks
   where. Each binary is then:
   - **smoke-tested** before anything else is done with it: it is sent
     `uci`, `isready`, `position startpos` and `go depth 6`, and after ten
     seconds `quit`. The run fails unless it answers with
     `id name Vyvræl <version>`, `uciok`, `readyok`, an `info depth 6` line
     and a legal-looking `bestmove`;
   - given a **SHA-256 file** beside it, `<file>.sha256`, in the usual
     `sha256sum` format (`<hash>  <file>`), checked once with
     `sha256sum -c`;
   - kept as a **workflow artifact** for 14 days.
4. **draft-release** (Ubuntu): gathers the four binaries and four checksum
   files, checks every checksum again, and creates a **draft** release for
   the tag with the runner's `gh` command. Its title is `Vyvræl <version>`,
   its text is `docs/releases/<version>.md`, and a version with a
   pre-release part (such as `0.2.0-rc.1`) is marked as a pre-release.

## Permissions and secrets

- **`contents: read`** for the whole workflow: every job can check out the
  code and nothing more.
- **`contents: write`** for the **draft-release** job only, because creating
  a release and uploading its files writes to the repository's releases.
  No other job and no other permission gets more.
- **No secrets** beyond the built-in `GITHUB_TOKEN`, which GitHub gives
  every run and limits to the permissions above. No Lichess token, no call
  to Lichess, no lichess-bot.
- The checkouts keep no credentials (`persist-credentials: false`), so the
  token is not left in the working copy where the build runs.

## Supply chain

- **Third-party actions are pinned to full commit SHAs**, with the version
  in a comment: `actions/checkout` v7.0.1
  (`3d3c42e5aac5ba805825da76410c181273ba90b1`), `actions/upload-artifact`
  v7.0.2 (`cf430e030ddbb5b0abf93d22962f4752f3646cd9`) and
  `actions/download-artifact` v8.0.2
  (`9000827ccba6bdab643e8b6fd33ac0654aef8333`), the latest releases on
  2026-11-22, read from GitHub's API. No other action is used; the release
  is made with `gh`, which the runners carry.
- **No cache** of any kind: every build starts from a clean checkout and a
  clean `target/`, with incremental compilation off, so nothing from an
  earlier run can reach a release build.
- **Network:** the engine crate has no dependencies, so `cargo` fetches
  nothing beyond the toolchain that `rust-toolchain.toml` names; the
  actions are fetched by GitHub. `cargo build --locked` refuses to change
  `Cargo.lock`.
- **Binaries are never committed:** they exist only as workflow artifacts
  and as files of the draft release.

## Choices worth a look

- **Windows links the C runtime statically** (`+crt-static`). Rust on
  Windows uses Microsoft's toolchain on GitHub's runners, and without this a
  user who lacks the Visual C++ runtime could not start the program. The
  hand-made builds up to alpha.2 used the GNU toolchain and did not need it.
- **Linux is built on `ubuntu-latest`**, as asked. The program then needs a
  C library (glibc) at least as new as that Ubuntu's; it may not start on
  an older distribution. If that matters, the Linux jobs can run on an older
  Ubuntu image or build a fully static binary (the `musl` target) instead;
  that is the maintainer's choice.
- **The smoke test waits ten seconds** before `quit`, because a `quit` that
  arrives while the engine is still searching ends the search early, and
  depth 6 from the start position takes about a second.
- **The gates run once, on Ubuntu**, as in `ci.yml`. The Windows binaries
  are not put through the test suite on Windows; their smoke test is what
  runs there.

## Installing it

1. Read [`release-workflow.yml`](release-workflow.yml).
2. Copy it to `.github/workflows/release.yml`, unchanged, and commit that
   on a branch of its own; merge it into `main`.
3. In the repository's settings, under Actions, keep "Workflow permissions"
   at read-only by default: the workflow asks for write access itself, on
   the one job that needs it.

## Trying it

Before relying on it for 0.1.0, run it once on a test tag:

1. On a throwaway branch, set a test version such as `0.1.0-rc.0` in
   `Cargo.toml` (and `Cargo.lock`), add a dated `## [0.1.0-rc.0]` section to
   `CHANGELOG.md` and a `docs/releases/0.1.0-rc.0.md`.
2. Tag that commit `v0.1.0-rc.0` and push the tag.
3. Watch the run: the four jobs pass, and a **draft** pre-release appears
   with eight files. Download one binary and check it against its
   `.sha256` file; run it once.
4. Delete the draft release, the tag (locally and on GitHub) and the branch.

Until that has been done, this document stays marked as a proposal and
`RELEASING.md` says so.
