# Release prompt

The default prompt for preparing a Vyvræl release. `AGENTS.md` points here
from "Releases", and Claude Code exposes it as `/release-prep`. Edit the
wording here only; nothing else holds a copy.

Where this prompt and `AGENTS.md` disagree, `AGENTS.md` wins.

## Prompt

Prepare the next release. Look at the commits since the last version tag (or
the last changelog entry if there are no tags), not just the latest commit,
and decide the bump under Semantic Versioning: major for breaking changes,
minor for backward-compatible features, patch for fixes. Tell me the version
you chose and why before editing.

Update the version everywhere it's defined (manifest, lockfile, version
constants, docs) and keep them consistent. In the changelog, move the
Unreleased entries into a new dated section for the version, following Keep a
Changelog: group changes under Added, Changed, Deprecated, Removed, Fixed,
and Security, write entries for users rather than copying commit messages,
and update the comparison links at the bottom.

Don't tag, push, or publish. Commit the bump as a single commit, and finish
with a short summary of what changed.
