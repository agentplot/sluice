# Design

## Context

The moon runner runs `moon exec --ci` (ADR 0044). `Moon::plan` reads
every task with `moon query tasks`, expands each required tag, narrows
to what the combination affects, and names as `kept_out` each target
whose `options.runInCI` is `false` or `"skip"`. Those read missing, so
the candidate fails, and the run's detail carries `kept_out` into the
branch's log, from which `sluice status` and a waiting submit warn
(`ops::kept_out`, `kept_out_warning`). Nothing looks before the first
run.

`Sluice::submit` (`crates/sluice-core/src/ops.rs`) already refuses
before writing anything: a detached or dirty worktree (2, 3), a missing
branch, a branch no `[branches]` table names, an unknown `after`. The
pull-request submit (`profiles/github/pulls.rs`, `open`) repeats the
first of these before it pushes. Statement clause 27 has sluice admit
every submission a driver makes, whether work may be submitted at all
being decided before it reaches sluice; clause 2 has the driver refuse
a worktree holding more than its commits. This refusal is the driver's
in the same way: it is made before the head is published or an id
taken.

## Goals / Non-Goals

**Goals:**

- A submission to a branch whose gate can never pass for want of a
  check the runner keeps out of CI is refused at submit, naming the task
  and the policy line.
- The submission that fixes it, by letting the task run in CI, is taken.

**Non-Goals:**

- Changing what a run does with a kept-out task: it still reads missing
  (moon-runner, "moon runs a combination's checks in its CI mode").
- Any check of a request opened by hand, a forecast, or a group submit
  (proposal, Out of scope).

## Decisions

### Ask the runner in the submitting worktree

The refusal reads `runInCI` from the submitting worktree, which holds
exactly the submission's head (2), by asking the runner there. The
required checks and their lines are read from the branch's current
revision, as every gate is (56, core.15).

The runner's settings are the tree's, and the combination carries the
submission's tree. Reading them from the branch's revision would refuse
the one submission that repairs the branch by letting the task run in
CI, and a change that only drops the requirement from the policy is
judged by the policy it changes (56), so it would be refused too: the
branch could never be repaired through sluice.

Alternatives:

- The records: refuse when the branch's latest run said a required task
  was kept out (`ops::kept_out`). It says nothing before the first run,
  and refuses the repairing submission, since the latest run predates
  it.
- A checkout of the branch's revision made for the purpose: slow on
  every submit, and refuses the repair for the same reason.
- Reading `moon.yml` from the head with git, without moon: task
  inheritance, tags and `runInCI`'s forms are moon's to resolve, and a
  second reading would drift from the one `Moon::plan` uses.

### One operation on the check seam

`Tests` gains `kept_out(worktree, tests, required)`: given the
worktree's path, the policy's `[tests]` and the branch's required
entries, the tasks the runner keeps out of CI, each with the required
entry that names it. Its default answers none, so the command runner,
the fakes' other users and `RecordedChecks` need nothing new.

`Checks` answers it for `runner = "moon"`. It sets up as a run does:
the run's environment, `[tests] toolchain` run in the worktree into a
script in a scratch directory of sluice's state, then `Moon::install`,
`check_version`, and `Moon::tasks` with no range. It expands the
required tags as `Moon::plan` does, and keeps the targets whose task is
not `in_ci`. Its executor is none: it is a query, run outside the
executor as `plan`'s own `moon query` is. `plan` and this operation
share one function from `tasks` and the required entries to the
kept-out list, so the two readings cannot differ. Rules core.1 and
core.2 hold: only the moon profile names moon.

### Not narrowed to what the submission affects

The refusal holds for every submission while the policy requires the
task, whether or not the submission affects it. The policy is what is
wrong, and the existing warning already says the branch cannot pass
while it is required. A refusal that came and went with the files a
submission touches would let some submissions through and blame others
for the same mistake.

### The policy keeps each required entry's line

`RulesForm::required` becomes a list of `toml::Spanned<String>`, as
`branches` and `pools` already are, and `Rules` keeps the line of each
entry beside `required`. The line is counted from the span's start in
the text `Policy::parse` was given. Nothing else reads it.

### The refusal and its words

`SubmitRefusal` gains `KeptOut(Vec<KeptOut>)`, each with the task, the
required entry and its line. The core asks the seam after the checks it
already makes and before `publish_head`. The CLI's words, in the user
words and naming no runner outside the profile (core.1), say for each:

```
refused: main requires root:integration (.config/sluice.toml line 59),
which the task runner in this worktree keeps out of CI, so it is never
run and main cannot pass while it is required
let it run in CI in this worktree, or drop it from main's policy in the
same submission that lets it run, and submit again
```

A task required through a tag names the tag: `requires app:serve
through #check-fast (.config/sluice.toml line 39)`. `refusal_next` gives
`sluice status <branch>`. Under `--json` it is the usual refusal
document. The pull-request submit refuses with the same refusal before
`push_branch`, and its `refusal_word` gains the case.

### A runner that cannot be asked takes the submission

When the toolchain, the install or the query fails, `kept_out` returns
the error, and the core takes the submission and returns a warning with
it, which the CLI prints (and `--json` carries as `warnings`). The run
then fails or reads the task missing, as today. Refusing instead would
make every submit hinge on the toolchain working in the session's
worktree, for a check that exists to name a mistake early.

### The existing warning stays

`sluice status` and a waiting submit still warn from the branch's log.
A submission can pass the refusal and still meet a kept-out task when
an entry ahead of it changes `moon.yml`; the warning covers that.

## Risks / Trade-offs

- [Every submit on a moon branch now runs the toolchain and `moon query
  tasks` in the worktree, seconds where the toolchain is already built]
  → it is what `sluice exec` already runs there; a command runner asks
  nothing.
- [The query writes in the session's worktree: moon's cache, the
  toolchain's own files] → only what the session's own `moon` and shell
  write, all ignored by git; rule core.20 rules out moving or rewriting
  a working copy, which this does not. The real-moon test asserts the
  worktree is clean after the query, since an untracked file would get
  the next submit refused as dirty (2).
- [A kept-out task with no tag and not required, as this repository's
  `root:screenshots`] → not required, so never asked about.
- [Rule core.15 says a gate is never read from a place] → the gate is
  still read from the branch's revision; the worktree gives only the
  runner's own setting, which decides nothing the gate counts. ADR 0047
  records it, and core.15's details gain it under "Not ruled out".

## Migration Plan

None. A branch already failing for a kept-out task starts refusing its
submissions with the reason; a branch without one sees no change.
