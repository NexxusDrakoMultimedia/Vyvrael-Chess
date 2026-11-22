# Release prompt

The default prompt for preparing a Vyvræl release. `AGENTS.md` points here
from "Releases", and Claude Code exposes it as `/release-prep`. The steps
themselves are written once, in [`docs/RELEASING.md`](../RELEASING.md);
this prompt only says which of them an agent carries out.

Where this prompt and `AGENTS.md` disagree, `AGENTS.md` wins.

## Prompt

Prepare the next release by following steps 1 ("Prepare") and 2 ("Gates")
of `docs/RELEASING.md`, on a branch `release/<version>` from `main`. Before
editing anything, tell me the version you chose and why, and wait for my
approval. Ask before any heavy step if a match may be running on the
machine.

Don't merge into `main`, tag, push or publish: those are steps 3 and 4, and
they are mine. Finish with a short summary of what changed, the gates'
results and anything not checked.
