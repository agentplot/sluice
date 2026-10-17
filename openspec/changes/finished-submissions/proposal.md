# Proposal

Status: approved by the operator 2026-10-16. Part 1, finished
submissions, is built; part 2, the log in parts, is not. The statement
change is `agentplot/blueprints:design/sluice/proposals/finished-records-1.md`.

## Why

Every read of a branch's records takes the whole of them: the records
seam's `read` (61) returns `branch.json`, every `submissions/<id>.json`
and `log.jsonl`, and the core parses every submission to answer any
question about the branch, one submission's state included. Nothing ever
leaves that set. This repository's `main`, one day after it took its
first submission, holds 86 submission files (344 KB) and a log of 1,058
lines (200 KB); `sluice overview --json` and `sluice status <id>` read
it in about 0.15 s. The cost grows with every submission the branch has
ever had, not with its queue: at the first day's rate a month leaves
about 2,500 files and 6 MB of log, so every status line, every round
and every `sluice` with no command pays for the whole history.

## What Changes

- A submission that has landed or been withdrawn moves, in the write
  that tells its outcome and once no combination or landing names it,
  from `submissions/<id>.json` to `.finished/<id>.json`, unchanged in
  form (53). A blamed or conflicting one stays live, since a withdrawal
  or a resubmission still changes it. `finished.jsonl` keeps its id,
  state and worktree among the live records, for ordering an `after`
  and counting attempts.
- The log is kept in segments: `log.jsonl` is the current one, and a
  write that would take it past a bound (1,000 lines) starts a new one,
  moving the old to `log/<n>.jsonl`. Its lines do not change.
- The records seam's `read` returns the branch's live records:
  `branch.json`, `submissions/`, `finished.jsonl` and, until part 2, the
  whole log. `read_finished` reads one finished submission and
  `read_all_finished` every one, for a reader that asks for history.
- `Changes` gains a removal, so a write can move a file all or nothing
  (63).
- The core's `find(id)` finds a submission by the live records and the
  index, and `state(id)` reads a finished one's single file.
- `sluice status <id>` of a finished submission and `sluice status
  <branch> --all`, as text or `--json`, read the finished records;
  everything else reads only the live ones. `status <branch> --json` lists only the queue
  unless `--all` is given, as its text already does.
- The next write to a branch moves its finished submissions written
  before this change, so no migration step is run by hand.

## What it needs from the statement

Clause 61 reads "given a branch, its records as last written". A read
that leaves out finished entries changes it, so the statement changes
first (rule core.12): `finished-records-1.md` in agentplot/blueprints
rewrites 61 as the branch's live records, and proposes 104 (the move),
105 (reading a finished entry) and 106 (the log in parts). 26 and 49
hold unchanged: every state and verdict stays readable by every host
with sluice stopped, now by 105. 53 holds: the files keep their form,
only their place changes.

## Not in this change

- Deleting history. Every record stays (26).
- A database or an index service: records stay files any host reads
  with no sluice process running (rule core.11).
- Changing the overview's or the home view's documents: they already
  show only the live queue and the recent lines.
