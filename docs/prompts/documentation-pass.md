# Documentation pass prompt

The default prompt for a full documentation pass on Vyvræl. `AGENTS.md`
points here from "Documentation upkeep", and Claude Code exposes it as
`/docs-pass`. Edit the wording here only; nothing else holds a copy.

Where this prompt and `AGENTS.md` disagree, `AGENTS.md` wins.

## Prompt

Do a full documentation pass across this project: docs, code comments,
docstrings, help text, and example configs.

First, inventory everything and show me a short plan (overlaps,
contradictions, stale parts, gaps) before editing. Make sure changes are
revertible.

Then verify every claim against the code (commands, flags, env vars, config
keys, paths, versions, API signatures), fixing mismatches. Merge duplicated
content into one canonical place and link to it elsewhere. Keep terminology
and structure consistent, and keep each file's existing format.

Fill gaps only with facts you can confirm from the code (setup, config
reference, architecture, examples, troubleshooting, testing, contributing).
Don't invent anything; list uncertainties as open questions.

Write plainly and naturally, like a person explaining to a colleague. Cut
filler and stock AI phrasing. Remove all em dashes.

Don't change runtime behavior, rename files, or push. Preserve working links
and samples.

Before finishing, confirm no em dashes remain, links resolve, documented
commands work, and tests still pass. Then give me a brief report: files
changed, what was merged or added, what you couldn't verify, and open
questions.
