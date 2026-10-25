# Proposal

## Why

A newcomer reading over Claude's shoulder took sluice's way on as
instructions to themselves: their session, in the repository's main
checkout, had started two background agents in worktrees, and when they
ended it told the newcomer "Run this from inside each worktree: `sluice
submit dev/ai-spike`". A person has nothing to run there. The statement's actors say who
does: a session that built the work submits it (statement preamble; §2
Actors, a session), and the agent working in a worktree is that session.
sluice's own lines say "fix it in your worktree and submit again", which
names no one, so whoever reads them, a person or a session that is not
in that worktree, takes "you" as themselves. The plugin tells Claude to
submit "from your worktree" but never says what to do about worktrees it
handed to background agents, so a coordinating session passes the
command to the person (signal
`2026-10-23-madswan-operator-bea2f287/04-submit-hints-speak-to-agents`;
the user's call "Sluce Work Tree Issues", 2026-10-20).

## What Changes

- Every line in which sluice says how a submission moves on, after a
  failure, a conflict, a time limit, a check the host could not run, or
  a refusal for uncommitted changes, a detached head or a check kept
  out of CI, names who does it: the agent working in the worktree the
  submission came from, by that worktree. "fix it in your worktree and
  submit again" becomes "the agent working in unit/x fixes it there and
  submits again". No such line asks its reader to go into a worktree, to
  run a command from inside one, or to hand a command to a person. A step
  that is the operator's, setting a result aside, still names the
  operator.
- `sluice` with no command, run outside a worktree it can submit from,
  says that work is submitted by the agent working in its worktree.
- The `next` commands and the JSON documents' fields are unchanged; a
  JSON error's `message` carries the new words.
- The using-sluice skill tells Claude: a background agent it starts to
  work in a worktree is told, in the prompt Claude gives it, to commit
  and submit its own work through sluice from that worktree and to
  report the outcome; work left in a worktree whose agent has ended is
  submitted by an agent Claude starts in that worktree; Claude never asks
  the user to run `sluice submit`.
- The session note the plugin adds at session start says the same in
  one sentence.
- The site's Claude Code guide, the pages and skill references that
  quote sluice's way-on lines, the README and `docs/examples` are
  brought to the new words.

Out of scope:
- The comments sluice posts on a pull request (`profiles/github/pulls.rs`):
  they are read on the forge by whoever opened the request, often a
  person, and `pull-request-status` owns their words.
- The git hook's refusal (`protect.rs`): it speaks to whoever ran git in
  that worktree, which is its session.
- The `sluice-board` plugin's wake turns: they are already a turn in the
  agent's own session, telling it what to do.
- The README's first-time setup over an existing clone, from the same
  call: another signal, another unit.

## Capabilities

### New Capabilities
- `submit-hints`: the words in which sluice says how a submission moves
  on, and to whom they are addressed: the agent working in the worktree
  the submission came from.

### Modified Capabilities
- `plugin-usage`: an agent that starts background agents in worktrees
  has each submit its own work through sluice, and never hands the
  submit to the user.
- `plugin-guard`: the session-start note says agents started in
  worktrees submit their own work.

## Impact

- `crates/sluice/src/lib.rs` (the verdict report's way-on lines, the
  submit refusals), `crates/sluice/src/words.rs` (the time-limit and
  kept-out wording), `crates/sluice/src/home.rs` (outside a worktree),
  and the tests that quote them in `crates/sluice/tests/`.
- `plugins/sluice/skills/using-sluice/` and its references,
  `plugins/sluice/scripts/session-context.sh`,
  `plugins/sluice/tests/hooks.sh`.
- Site pages quoting the lines, the Claude Code guide, `README.md`,
  `docs/examples/README.md`.
- No rule of `docs/architecture/` is departed from; no clause changes.

## Touches

```
crates/sluice/src/lib.rs
crates/sluice/src/words.rs
crates/sluice/src/home.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/profiles/pull_requests.rs
plugins/sluice/skills/using-sluice/**
plugins/sluice/scripts/session-context.sh
plugins/sluice/tests/hooks.sh
site/src/content/docs/guides/claude-code.md
site/src/content/docs/guides/walkthrough.md
site/src/content/docs/index.mdx
site/src/content/docs/reference/commands.md
README.md
docs/examples/README.md
```
