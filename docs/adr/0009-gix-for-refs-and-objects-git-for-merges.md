---
number: 9
title: gix for refs and objects, the git command for merges, and the checkout method as the only git setting
status: accepted
date: 2026-10-11
decision-makers:
- Chuck Swanberg
---

# gix for refs and objects, the git command for merges, and the checkout method as the only git setting

Deciders: the user, in the setup walkthrough review of 2026-10-11.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 10, 12, 23, 24, 42 to 46, C.1, C.4; flywheel-next's `crates/flywheel-store-git` and `crates/flywheel-world-host` (gix 0.87, and twelve calls to the `git` command beside it); worktrunk's custom subcommands (worktrunk.dev/extending).

## Context and problem statement

sluice does git work of two kinds. On the branch it reads the branch, forms each combination as a three-way merge, writes one merge commit per landing, and moves the branch only from the revision it expects (42 to 46). For tests it makes a clean checkout of each combination (33, 34). Each can be done in-process with gix, by the `git` command, or, for checkouts, through worktrunk. Should the user choose among these, and which does sluice use?

## Considered options

1. One user setting with three values, gix, the git command, or worktrunk, governing all git work.
2. The branch operations fixed inside sluice, gix for reads, objects and the ref update, the git command for merges; the checkout method the only user setting, with worktrunk, plain git worktree and archive as its values.
3. The git command for everything, with no setting.

## Decision outcome

Option 2. The branch operations must behave identically for every user, so they are not a setting. Reading refs, writing commits and moving a branch from an expected revision run in-process through gix, as flywheel-next does; gix's ref transactions carry the expected old value, which clause 45 requires. Forming a combination runs `git merge-tree --write-tree`: whether a combination conflicts must match exactly what `git merge` would report, git's merge is the reference for that, and a disagreement would blame or land the wrong submission (12, 17). Both sit behind one internal interface, so moving merges to gix later changes code, not configuration.

The checkout is a user setting because the methods differ in what the checkout gets. `worktrunk` makes it with `wt switch --create` and removes it with `wt remove`, so it receives the repository's worktrunk hooks like any agent's worktree; `git-worktree` runs `git worktree add --detach` with nothing else; `archive` exports the tree with no git metadata. A gix checkout would only be a slower archive, since gix does not create linked worktrees.

Submitting is separate again: sluice installs `wt-submit`, which worktrunk runs as `wt submit`, and `sluice submit` works without worktrunk.

Option 1 offers a choice whose only visible effect would be differences in behaviour. Option 3 forgoes in-process ref and object work the core does on every step.

### Consequences

- sluice requires the `git` command, 2.38 or later for `merge-tree --write-tree`, whatever the checkout method.
- C.1 of the statement names three checkout profiles: worktrunk, git worktree and archive.
- core.1 still holds: `wt` is named only in the worktrunk checkout and driver profiles.
