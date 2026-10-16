# Design

## Context

See proposal.md. The shared contract the operator settled between the
two plugin plans fixes the command, the document and the watch. What
exists: `State::branches` and `State::read` give every branch's
records as a snapshot (61); `Line::load` parses one; the log's lines
are `sluice/log@1`; `Checkout::session_places` and `Sluice::previews`
list what the place seam holds, with no write. `sluice previews`
releases unusable previews as it lists them, so it is no reader.

## Goals / Non-Goals

**Goals:**

- One document both plugins read, through the seams, with no lock and
  no write (rules core.11, core.14).
- A watch cheap enough to run for a session's life.
- A submission matched to its worktree by path.

**Non-Goals:**

- Live progress inside a run: a check's result is recorded when its
  run finishes (rule core.7), and the overview shows what is recorded.
- Other repositories on the host.

## Decisions

### D1. A pure branch view in the core

`overview::branch_view(BranchRead{branch, snapshot, rules, executor})`
turns one branch's records into `BranchView`. `Sluice::overview` reads
each branch through the records seam, takes `base` from its
`branch.json`, and reads the policy at `base` through the branch seam's
`read_file`, which never fetches. `at` is `base`. Reading the tip
would fetch a remote branch through the pushed-refs profile on every
read (ADR 0017), once a second under `--watch`.

### D2. Candidates from the records and the policy

A candidate's `started` is the `at` of the last `testing` line of the
log naming its revision. Its `limit`, `targets` and `executor` are the
policy's `timeout`, `required` and `[executor] kind` at `base`. The run
directory knows the affected subset and the executor that held the run,
but it is the check seam's, and reading it would add a seam method for
display alone. `ran`, `replayed` and `failed`, with each failed test's
kept log, are the candidate's record.

### D3. Recent events verbatim

`recent` is the last ten `landed`, `blamed` and `conflict` lines of the
branch's log, each parsed as JSON and kept as written. A reader keys off
`event` and the log's own fields, and no second shape of an event
exists.

### D4. The watch compares documents

`--watch` builds the whole document every pass, drops `at`, and prints
when the rest differs from the last printed. That covers every
branch's `version`, the candidates under test, and the worktrees and
previews, which have no version. `--watch` requires `--json`. It reads
every second (main gives the overview a one-second pause; a round's
stays three), and Ctrl-C ends it with exit 130, as it ends `sluice run`.

### D5. `from.path` from the driver

The worktrunk driver runs `git rev-parse --show-toplevel` in the
session's directory and sets `Submitter.path`. It is optional with
serde default, so older records read, and skipped when absent, so the
fakes' and pull requests' JSON is unchanged. Resubmission still matches
on `from.worktree` (67).

### D6. Host and repository

`exec::Host` gains `name()` (the check seam's `hostname`) and
`repository()` (the canonical common git directory). The fake host
answers fixed strings so tests read them.

## Risks / Trade-offs

- `targets` read as the gate's required tests may list more than a
  run affected: the board says what the gate requires, which is what
  a verdict is read against.
- A policy that does not parse at `base` fails the overview, as it
  fails `sluice status`.
