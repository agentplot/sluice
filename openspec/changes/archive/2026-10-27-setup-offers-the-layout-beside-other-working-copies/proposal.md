# Proposal

## Why

Asked to set sluice up in a clone that already has worktrees beside it,
the setup skill today says nothing of the layout and goes on, leaving the
git directory inside one of the working copies. The statement now says
what it does there (126, blueprints 83865e8): every session can already
work from a copy of its own, so the setup offers the same move it offers
a plain clone, with its cost, and goes on as the repository stands. Only
a repository's one and only working copy waits for the operator's word.

## What Changes

- **A clone with other working copies gets the offer, and the setup goes
  on.** Where the git directory is `.git` inside a working copy and the
  repository has other working copies, the setup shows the move's plan,
  costs first, as it does for a plain clone: what holds the folder, the
  Claude Code history it would carry and what stays behind, and the
  layout the folder would take. It says it goes on as the repository
  stands and that the move is made whenever the operator asks, and goes
  straight on to the survey without waiting for an answer.
- **The offer names the right folder wherever the setup was asked.**
  Asked from a worktree beside the clone, the offer is about the folder
  that holds the git directory, not the worktree the session is in.
- **What stops the move now is part of the offer.** Where the move would
  be refused today, such as a worktree inside the folder, the offer says
  so, and the setup still goes on.
- **"Lay it out" later runs the same move.** When the operator answers
  the offer during the setup, the setup runs the conversion exactly as it
  does for a plain clone, and carries on where it was.
- **Only the one and only working copy waits.** sluice's own test
  checkouts under the git directory, and worktrees whose folder is gone,
  are no session's working copy: a clone with only those beside it is
  still its repository's only working copy, and the setup waits for the
  operator's word there as today. The commit that adds
  `.config/sluice.toml` records the operator's choice only in that case.
- **The conversion script tells how the repository stands.** A new
  `kind` form of the plugin's conversion script answers which of the
  three the repository is (its only working copy, one of several, or
  apart from its git directory), so the setup reads one answer and the
  gate tests it.
- **Docs**: the worktrees skill, and the site's Claude Code and worktrees
  guides, say what the setup does in a clone with other working copies.

Out of scope:
- sluice warning of the layout on later commands: the statement says it
  warns on none (126), and nothing outside the setup reads the layout
  today, so nothing changes.
- Converting a `.bare` directory, which the worktrees skill does on
  request, and a git directory kept apart by `--separate-git-dir`: both
  already stand apart from their working copies, and the setup makes no
  offer there.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `plugin-setup`: the only working copy is counted without sluice's own
  checkouts and gone worktrees; a git directory inside one of several
  working copies gets the offer and the setup goes on without an answer.
- `plugin-worktrees`: the conversion script tells how a repository
  stands.

## Impact

- `plugins/sluice/scripts/lay-out.sh` gains its `kind` form, and
  `plugins/sluice/tests/lay-out.sh` tests it; both run in the existing
  `plugin-sluice:lay-out` moon task.
- `plugins/sluice/skills/setting-up-sluice/SKILL.md`, step 2 and step 5;
  `plugins/sluice/skills/worktrees/SKILL.md`, "With sluice"; checked by
  `plugin-sluice:validate`.
- `site/src/content/docs/guides/claude-code.md` and
  `site/src/content/docs/guides/worktrees.md`; checked by `site:prose`.
- Nothing under `crates/` (rule core.1), and no change to what the
  conversion's `plan` and `go` do.

## Touches

```
plugins/sluice/scripts/lay-out.sh
plugins/sluice/tests/lay-out.sh
plugins/sluice/skills/setting-up-sluice/SKILL.md
plugins/sluice/skills/worktrees/SKILL.md
site/src/content/docs/guides/claude-code.md
site/src/content/docs/guides/worktrees.md
```
