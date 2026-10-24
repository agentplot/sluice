# Design

## Context

`submit_pull` in `crates/sluice/src/lib.rs` serves `sluice submit` and
`wt submit` to a branch with pull requests, `--status`, and `sluice wait
<branch>`. In `PullMode::Open` it calls `PullRequests::open`, which
pushes the branch, opens or updates the request, withdraws a queued
submission on an older head (67, 120) and marks the request ready. Its
loop then runs `intake` for the branch, which submits each ready request
whose conditions hold, from the place `pull/<n>`, and asks
`PullRequests::follow` where request `<n>` stands.

`follow` (`profiles/github/pulls.rs`) reads the request's latest entry
with `latest_of(branch, [place(n)])`. It returns `Followed::Done` only
for an entry landed, failed or in conflict on the request's current
head; for everything else, a queued or testing entry included, it
returns `Followed::Open { title, lines }`, the words intake posted. So a
no-wait submit whose intake took the request still prints only
`{"pull_request","branch","state"}`, and so does `sluice wait <branch>
--json` out of time while the request's submission is under test. The
entry is in the records; nothing hands it to the CLI.

## Goals / Non-Goals

**Goals:**

- The no-wait answer names the submission intake took, when it took one
  on the head the submit pushed.
- `sluice wait <branch>` and a stopped waiting submit end, once the
  request has a submission, with that submission's record, as `sluice
  wait <id>` does.

**Non-Goals:**

- A new way to name a request to `sluice wait`.
- Any change to when intake takes a request in (C.5, 120, 128).

## Decisions

### `Followed::Open` carries the request's submission

`follow` already holds the request's latest entry. When that entry is
not out (`State::is_out` false) and its head is the request's current
head, `follow` returns it in a new field, `Followed::Open { title,
lines, submission: Option<Submission> }`. The CLI reads it there and
reads no records of its own for it.

Alternatives: the CLI reading `taken_in` (`lib.rs`) on the worktree's
head, as the 122 path does after a failed round. That finds the newest
entry with the head on the whole branch, which could be a session's
rather than the request's, and puts the pull-request profile's knowledge
of `pull/<n>` back into the CLI. A separate `PullRequests` method would
read the same entry a second time in the same step.

### The no-wait answer is the no-wait document of any branch

When `Followed::Open` carries a submission, the no-wait answer under
`--json` is `{"id","branch","ahead","pull_request"}`: the document
`submit` prints for `--no-wait` on any branch, plus the request's
number. `ahead` is the ids of the submissions ahead of it in the
branch's queue, in the order `sluice status <branch>` lists them. A
script then does what it does on any branch: hand `id` to `sluice wait`.
crew's SUBMIT and the board hook (`plugins/sluice-board/hooks/board.ts`
`submitted`) already read `"id"` first; with `pull_request` beside it
they also know the request.

Without `--json`, the submit prints, after `open`'s `pushed … ·
opened #<n>` line, `submission <id> to <branch> from #<n> · <ahead>`,
the line a no-wait submit prints with `from #<n>` added and `ahead` in
its words (`nothing queued ahead`, `2 queued ahead (#38, #40)`, with the
pull requests' names as `follow` gives them).

Alternatives: printing the submission's full record, as `sluice status
<id> --json` does. That is a second shape for one command's no-wait
answer, and a script would have to tell them apart.

When `Followed::Open` carries none, the answer is today's: the
request's document under `--json`, its title and lines without.

### The wait's undecided ending follows the submission

`submit_pull`'s `still` closure, used for `--timeout` and Ctrl-C, prints
the last `Followed::Open`'s submission when it had one: under `--json`
its record as `serde_json::to_string_pretty` prints it in
`await_submission`, and in words `still waiting: sluice wait <id>`.
Otherwise it prints the request's document as today. The record is read
again with `Sluice::state(id)` at that moment, so it is the record as it
stands, not as the last step saw it. Exits do not change: `5` for the
time, `130` for Ctrl-C.

### What does not change

The decided ending, `Followed::Done`, already reports the submission's
record and exit. `--status` keeps its documents. `open` and `intake` are
untouched, so the submit still writes no ref and submits nothing itself
(rule core.4; 1, 120).

## Risks / Trade-offs

- [The request's latest entry is on an older head, still queued] →
  `open` withdraws a queued entry on an older head before the loop
  reads, and `follow` names a submission only on the current head; a
  landing entry on an older head (the one `open` leaves to land) is
  never named, so the answer is the request's and the next submit or
  wait finds the new head's.
- [Intake fails for the branch, 128] → `follow` reads what the records
  hold; with no entry on the head the answer is the request's, exit 0,
  and the warning is on stderr as today.
- [A script reads `pull_request` before `id`] → the document holds
  both; a script that already handled the request answer still works,
  and one that reads `id` gets the submission.
