# Design

## Context

The setup skill's step 2 (`plugins/sluice/skills/setting-up-sluice/SKILL.md`)
tells the plain clone from everything else with four git commands typed
into the skill: not bare, the folder is the top, the common directory is
`<folder>/.git`, and `git worktree list` has one entry. Anything else
goes straight to the survey, and a clone that has worktrees of its own is
left to the `worktrees` skill "when the user asks". The fourth check
also counts sluice's own checkouts, which sluice adds under
`<git-dir>/sluice/`, and worktrees whose folder is gone, so a plain clone
sluice has already run in can read as "several" and skip the offer.

The conversion, `plugins/sluice/scripts/lay-out.sh`, already lays out a
clone with worktrees beside it: `plan` lists each other worktree and what
becomes of it (outside the folder: repaired in place; under `.git`:
moved with it; anywhere else in the folder: stops the move), and `go`
repairs them. Its tests (`plugins/sluice/tests/lay-out.sh`, the
`plugin-sluice:lay-out` task) already convert a clone with a worktree
outside it. What is missing is the setup making the offer there, and not
waiting on it (126).

## Goals / Non-Goals

**Goals:**
- One answer to "how does this repository stand", computed by the
  plugin's script and tested by the gate, read by the setup skill.
- The offer beside other working copies is the plain clone's offer,
  word for word in its costs, made without a question that waits.

**Non-Goals:**
- Any change to `plan` or `go`: what they move, refuse on and carry
  stays as it is.
- Any check of the layout outside the setup (126: sluice warns of it on
  no later command).

## Decisions

### D1. The script tells how the repository stands: `lay-out.sh kind <path>`

A third form, `kind`, prints exactly one line and changes nothing:

| line | when |
|---|---|
| `only <folder>` | the git common directory is a directory named `.git`, not bare, whose parent `<folder>` is the top of a working copy, and that working copy is the only one counted |
| `several <folder>` | the same, with more than one counted |
| `apart` | anything else in a git repository: a bare git directory beside its worktrees, a `.bare` directory, a `--separate-git-dir` clone |

It exits 0 for each, and 1 with `refused: <path> is not a working copy
of a git repository` otherwise. `<folder>` is the physical path (`pwd
-P`), as `plan` and `go` resolve it. The working copies counted are the
blocks of `git --git-dir=<common> worktree list --porcelain` that are not
`bare`, not `prunable`, and whose path is not the common directory or
under it (sluice's checkouts sit under `<git-dir>/sluice/`, the root
`crates/sluice/src/profiles/places.rs` gives them). Asked from a linked worktree, `kind` still names the
folder that holds `.git`, so the offer is about the move that would be
made, not the folder the session is in.

Rejected: keeping the four commands in the skill and adding a fifth.
The gate cannot run a skill's prose, and the miscount above is exactly
the kind of slip a test catches. `kind` lives in `lay-out.sh` rather
than a new script because it answers the question `plan` refuses on
("is not a plain clone"), and the existing task and its inputs cover it.

### D2. The setup reads `kind` and takes one of three paths

Step 2 runs `bash "${CLAUDE_PLUGIN_ROOT}/scripts/lay-out.sh" kind .`
from where it was asked:

- `apart`: no offer; go on to step 3.
- `only <folder>`: today's offer, unchanged, and the wait for the
  operator's choice (126, S61).
- `several <folder>`: the offer of D3, then step 3 in the same turn.

### D3. Beside other working copies, the offer is the plan, then the setup goes on

The skill runs `plan <folder> --session-pid "$PPID"` and shows it as it
shows a plain clone's (the costs first: each process holding the folder,
the editors to reopen at `main/`, the history to be carried and where,
what stays under the old path; then the layout). Where `plan` exits 1,
its "go refuses now" lines are shown as what stops the move today. The
skill then says, in its own words, that every session can already work
from a copy of its own, so the setup goes on with the repository as it
stands, and the folder is laid out whenever the operator asks. It asks
no question: no AskUserQuestion, no "lay it out or keep it?", nothing
that ends the turn waiting. It goes on to step 3 in the working copy it
was asked in, and writes and commits the policy there as for any
repository.

Rejected: asking the question and going on if no answer comes. A
question in Claude Code ends the turn, which is waiting on an answer,
which 126 rules out here.

### D4. "Lay it out" during the setup runs `go` as for a plain clone

If the operator says to lay the folder out at any point in the setup,
the skill runs step 2's `go` (with `--session` and `--session-pid`),
with the same handling of exits 1, 2 and 0. On exit 0 it asks for
`/cd <folder>/main` only when it was asked in `<folder>` itself; asked
in a worktree outside the folder, its working copy was repaired in
place and it goes on there. A policy already written but not committed
in `<folder>` moves into `main/` with the working copy's other changes,
as `plan` says of every changed path. The skill then carries on with
the step it was on.

### D5. The setup's record says a choice only where one was asked for

The commit line "The operator chose to keep this plain clone ..." stays
step 5's for the `only` path, where the operator kept the plain clone.
Beside other working copies nothing was chosen, so the commit says
nothing of the layout, and step 5's "write nothing until the user has
answered" binds only the `only` path.

## Risks / Trade-offs

- [The operator reads past the offer, since nothing waits on it] → It
  is the first thing the setup shows after the binary, costs first, and
  the `worktrees` skill makes the same move whenever asked later.
- [A worktree whose folder is on an unmounted volume shows as
  `prunable` and goes uncounted, so a clone may read as `only`] → The
  setup then waits for an answer it did not strictly need: the safe
  side.
- [`kind` resolves paths with `pwd -P` while `git worktree list` may
  print them as recorded (relative paths under
  `worktree.useRelativePaths`, or through a symbolic link)] → Compare
  each listed path after resolving it the same way; the tests cover a
  scratch directory reached through a symbolic link, as macOS's
  `/var` → `/private/var` gives.
