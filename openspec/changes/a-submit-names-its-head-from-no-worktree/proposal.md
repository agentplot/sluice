## Why

The user approved Flume's settles design on 2026-11-06. In it a branch
that work lands on, and that itself lands on another, is a *base
place*: a branch and Flume's record, with no worktree ("No worktree: a
branch and a record", the user, 2026-11-06). Flume submits it by naming
its branch from the repository and reads its outcome from sluice's
records (flume 133, 202, 203; C.6's row for a base place's submit;
Flume's design, decision 8 and follow-up S1).

sluice 124, as amended on agentplot/blueprints main at 51b6316, takes
that submission: a submission that names a branch sluice lands on as
its head, run anywhere in the repository, from a worktree or from none,
is of that branch as its policy holds it, records no worktree and no
session as its submitter, and has its outcome pending in no inbox, read
from the branch's records with the words an inbox would give.

sluice cannot do that today. `--head` is taken only with `--check`
(`crates/sluice/src/lib.rs`, `Command::Submit`). A submit reads the
worktree it runs in as its submitter, through the driver, and records
its path and pane. Its outcome is matched to whichever worktree is on
the branch it names, by `from.worktree` or `from.head`
(`crates/sluice-core/src/ops.rs`, `pending_on`). And `sluice status
--json` gives a decided submission without the `notice` the inbox gives
it. crew meanwhile submits a bolt by making a `places/land-<bolt>`
worktree for it and running `wt submit main` there, with a `sluice wait`
in the background. The runner reads `sluice status <id>` for each
submission Flume names.

## What Changes

- **`sluice submit <target> --head <branch>` without `--check` submits
  the branch by name.** It submits a branch sluice lands on as its policy
  holds it, the remote's revision where the policy names a remote, run
  anywhere in the repository, a bare repository's git directory among
  it. It reads no worktree and no session, and runs no `git status`
  (core.48). `--check --head` forecasts the same head as before, so the
  two read together: the forecast says what the submit by name would
  meet. A branch sluice lands on nowhere, the target itself and a missing
  branch are refused, naming why; `--head` with `--worktree` or
  `--status` is a usage error.
- **The record of a submission by name.** `from.worktree` and `from.head`
  are both the branch; no `path` and no `session`; the host it ran on;
  the agent the harness names, or the branch. A submission by name and
  one from a worktree on that branch are one work: each withdraws the
  other's earlier entry on the same target, and the attempts are the
  branch's (54; 67 and 120 as amended on a blueprints branch).
- **Its outcome is delivered by nothing.** The round queues no notice
  for it, pushes nothing, logs no `pending` line and never marks it
  delivered. `sluice inbox` anywhere answers nothing for it, and a wait
  that reports it marks nothing.
- **A worktree takes as its own only what it submitted.** The inbox, a
  wait's delivery and the home view take a submission as a worktree's
  where the worktree submitted it, or where it is a pull request's from
  the worktree's branch and that branch is one sluice lands on nowhere;
  never a submission by name.
- **On a target that takes pull requests**, the submit by name opens or
  updates the request from the branch and pushes nothing (124, C.5). The
  entry is the request's, `from.head` the branch, its outcome posted on
  the request.
- **`sluice status --json` gives an inbox's words.** `sluice status
  <branch> [--all] --json` and `sluice status <id> --json` give each
  landed, failed or conflicting submission's `notice`, beside the
  record's `failed`, `conflict.paths` and `tested.on` they give already.
- **A named submission's way on** names a fix landed on its branch and
  `sluice submit <target> --head <branch>` again, never an agent working
  in the branch.

## Out of scope

- Flume's base place, its pin and its profile's submit by name: Flume's
  task 5.2 moves its pin to the release that carries this change.
- crew's retirement of its `places/land-<bolt>` path: crew's follow-up
  C1, once Flume's release is out. crew's path keeps working until then.
  A bolt must not be submitted both by crew's worktree and by name at
  once, since each withdraws the other's entry (67).
- The runner's watch and its bolts as launches: Boomdrive's follow-up
  B1. The runner's read of `sluice status <id>` keeps working unchanged
  and gains `notice`.
- Several branches by name as one act (125). `--head` is given once.
- `sluice submit --status --head`: `sluice status <target> --all --json`
  answers where a branch's submissions stand.

## Capabilities

### New Capabilities

- `submit-by-name`: a submit that names a branch sluice lands on as its
  head, from anywhere in the repository, recording no worktree and no
  session, its outcome delivered by nothing and read from the branch's
  records in an inbox's words.

### Modified Capabilities

- `inbox`: a worktree collects only the outcomes of what it submitted,
  and of pull requests from its branch where that branch is one sluice
  lands on nowhere; never a submission by name.

## Impact

- `crates/sluice-seams`: `Driver::named`, the submitter of a branch
  named; `Submitter::by_name`.
- `crates/sluice-core`: `SubmitRequest` and `GroupRequest` name their
  head as a worktree or a branch; `SubmitRefusal::NotLandedOn` and
  `SubmitRefusal::ToItself`; the session of a branch named; the outbox
  skipping a submission by name; one reading of whose a submission is,
  for `pending_on` and the home view's `Here::made`.
- `crates/sluice`: `--head` without `--check`; the worktrunk driver's
  `named`; `PullRequests::open` and `opening` taking a branch named; the
  delivery read's words; `notice` in the status documents; the way on of
  a named submission.
- `crates/sluice-fakes`: the fake driver's `named`.
- Records: no new field. A submission by name is told by its record's
  form, `from.head` equal to `from.worktree`.
- The site's commands and state references and the `using-sluice` skill
  give `--head` without `--check` and the status documents' `notice`.
- Statement: 124 as amended at agentplot/blueprints 51b6316; 67 and 120
  amended on the branch
  `design/sluice-a-submit-names-its-head-from-no-worktree` (97c76e8),
  cut from main and unpushed. Record 0090 and rule core.50 hold the
  decisions.
- Release: the minor release CI tags at this change's landing (git-cliff,
  a `feat`): v0.71.0 where it lands next, otherwise the next minor after
  the latest tag then. Flume's task 5.2 moves its pin to that tag.
