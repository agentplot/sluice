# Design

## Context

See proposal.md for why. This change builds on
`an-entry-records-its-submitting-session-and-the-round-pushes-its-outcome`
(the push change), which is ahead of it in the bolt. Once that lands:

- `Submitter` holds `agent`, `worktree` (the session's branch, or a pull
  request's `pull/<n>`), `path`, `head` (a pull request's branch), and the
  push change's `host` and `session` (the multiplexer's opaque name for
  the pane, `herdr:<pane>@<socket>`, formed by the worktrunk profile from
  its environment).
- `Submission` holds `told` (a time) and `told_to` (a session), both
  absent on an untold outcome.
- `perform_outbox` removes the item for a submission already told and
  pushes nothing; it removes an undelivered item too, leaving the
  submission untold. A landed submission with no outbox item moves to the
  finished records at the branch's next write (130), untold.
- The outbox items are named `<id>:landed`, `<id>:failed` and
  `<id>:conflict`; `Event::Told { id, notification }` names one.
- The push's words are one `[sluice]` line formed from the notification's
  `Told`, which the push change puts in the worktrunk profile and says
  the ask will share.

On main today, the things this change reuses:

- `Driver::session(worktree)` gives the asking worktree's `Submitter`;
  `last_submitted` in `crates/sluice/src/lib.rs` already finds "this
  worktree's last submission" by `from.worktree`.
- `report` in `crates/sluice/src/lib.rs` prints what `sluice wait`
  prints for a decided submission; `record_json` what `--json` prints.
- `Sluice::update` reads, changes and writes a branch's records, reading
  again when the write is refused (62); `State::wait_lock` takes a
  branch's lock for a write outside a round, as `withdraw` does.
- A finished submission is read by `finished_sub`, one record by name;
  the live index (`finished.jsonl`) keeps each finished one's id, state
  and worktree (130). `Line::changes` puts a finished record only when it
  moves one.
- Resubmitting from a worktree withdraws its earlier submissions that are
  not final (`admit`), so a superseded failure is `withdrawn`, not
  `failed`.

## Goals / Non-Goals

**Goals:**

- One command that tells the asking session every untold landing, blame
  and conflict of its submissions, in a wait's words, and marks each
  told to it.
- Told once across the push and the command, and at least once across a
  stop.

**Non-Goals:**

- A reporting `sluice wait` marking told, the board's carrying and its
  told list, and anything that runs the command at a session's start:
  other units (proposal).
- Telling a withdrawal.
- A command for another session's outcomes: the command tells its own
  worktree's alone.

## Decisions

### The command is `sluice untold`

`sluice untold [--worktree <path>] [--json]`. The name says what it
answers, the outcomes not yet told, in the word the state file already
uses (`told`). It is a one-shot read and write of the records, as `sluice
withdraw` is; it runs no round and follows no reader (core.25 allows a
one-shot read; core.4: it writes no branch's ref).

*Alternatives:* `sluice outcomes` reads as every outcome, told or not;
`sluice tell` reads as an order to tell someone else.

### Found by the submitter's work, not by the pane

A submission is the asking session's when its recorded `from.worktree`
is the asking worktree's branch, or its `from.head` is (a pull request
opened from that branch). This is how `sluice wait <branch>` and `submit
--status` already find a worktree's submissions, and it is the part of
the submitter that survives what the ask exists for: a restarted agent
comes back in a new pane, and a session on another host has another
server and path, but works on the same branch.

The recorded `session` is not a match key: a pane's ID is never reused,
so a session that restarted in a new pane would never find its own
outcomes by it.

A worktree on no branch (`Session::detached`) names no work and has
submitted nothing (3): the command prints nothing and exits 0.

### What is told

A submission whose state is `landed`, `failed` or `conflict` and whose
`told` is absent. Withdrawn, queued, waiting, testing, passed and landing
submissions are not told: the round tells only landings, blames and
conflicts, and a resubmission withdraws the failure it replaces, so a
superseded failure is never told.

### Where it reads: the live records and the session's own finished ones

For each branch with records (`State::branches`), the command reads the
live records, takes the live submissions that match, and from the
finished index the ids whose `worktree` is the asking worktree's branch
and whose state is `landed`, and reads each of those records by name. Clause 131
lets a status of one submission read that submission's own finished
record; the command reads its own submissions' and never the branch's
history. A failed or conflicting submission stays live (130), so only
landings are read from the finished records.

A pull request's submission is indexed by `pull/<n>`, not by its head,
and the pull request driver delivers its outcome on the request, so its
landing is told there (push change). The command finds a pull request's
live submissions by `from.head` and reads no finished record for them.

### Marking told: under each branch's lock, told before marked

For each branch holding at least one of the session's untold outcomes,
in the branches' sorted order, the command takes the branch's lock
(`wait_lock`) and reads it again; what is still untold under the lock is
what it tells. It holds every such lock until it has written what it
tells and marked it, then lets them go:

1. Find, without a lock, the branches with candidates.
2. Take each one's lock, in sorted order, and read its candidates again.
3. Write the output (text or the JSON document) and flush it.
4. For each branch, in one `update`: set `told` and `told_to` on each
   told submission, live or finished; remove the outbox item for it if
   one stands; log `Event::Told { id, notification }` with the item's id,
   or `<id>:<landed|failed|conflict>` where none stands, the id the round
   would have used (51: named so a repeat is known as one).
5. Let the locks go.

Holding the lock across telling and marking is what makes one outcome
told once: the round's outbox runs under the same lock, so it either
pushed and marked before the command read, which then sees it told, or
runs after and finds it told and pushes nothing (push change). Telling
before marking is what makes it told at least once: a command stopped
between 3 and 4 marks nothing, and the next ask, push or wait tells it
again under the same notification. Taking the locks in one order means
two askers never wait on each other; a round takes its lock without
waiting and passes the branch by as busy, and a submit or a withdrawal
holds one lock only.

A write refused because another host wrote the branch's remote records
is read again by `update` (62); one already told there is not marked
again, and its telling was a repeat named as one.

*Alternatives:* marking before telling loses an outcome when the command
is stopped between them, which clause 51's at-least-once forbids;
telling and marking with no lock lets the round push an outcome the
command just told.

### A finished record is rewritten in the branch's write

`Line` gains a set of finished submissions changed in this write, which
`changes` puts at `finished_file(id)`. The record keeps its form (130);
only `told` and `told_to` change. The index line is unchanged, since it
carries neither (130). Both records profiles already put
`.finished/<id>.json` when a submission moves; a put over an existing
one replaces it the same way, which a test of each profile shows.

### told_to: the asking session as its driver names it

The `Submitter` the driver gives the asking worktree carries the
multiplexer's `session` where the command runs in one (push change).
`told_to` is that; where there is none, the worktree's path (`from.path`),
and where that too is absent, its branch. The core stores the string and
never parses it (core.1).

### The words

Text, for each outcome, in branch then id order, with a blank line
between outcomes:

```
[sluice] #12 landed on main as abc1234.
passed · landed as abc1234
```

The first line is the push's line, the same function, so a session is
told an outcome in the same words whether it was pushed or asked for.
The rest is `report` as `sluice wait 12` prints it. The push change puts
that function in the worktrunk profile; this change moves it to
`crates/sluice/src/words.rs`, which both the profile and the command
call, since code outside a profile names no profile (core.1). The line
is formed from a `Told`, which `Submission::told_outcome` derives from
the record where no outbox item stands: `landed_as`; the failed checks,
the tested combination, its submissions and whether the time limit ended
it; the conflict's paths and submissions.

`--json`: `{"outcomes":[<record>...]}`, each the submission's record as
`record_json` prints it, with `told` and `told_to` as the command marks
them; `"unread"` is added only when a branch could not be read.

Exit 0 whether it told anything or not; 3 when a branch could not be
read, after telling what it could; 3 when sluice itself failed, with the
error document under `--json`.

### A branch it cannot read

A read of one branch's records that fails is said on stderr as
`sluice: could not read <branch>'s records: <the reader's words>` and,
under `--json`, kept in `unread` as `{"branch","error"}`; the command
goes on with the other branches and exits 3 (core.11: no read refused
because another branch could not be read).

## Risks / Trade-offs

- [Two sessions in one worktree, one after the other, are one session to
  the command] → A worktree holds one branch and one agent's work; the
  first to ask is told.
- [A session asks while a remote host's round pushes] → The lock is the
  host's; on a branch whose records are on its remote, the other host's
  round holds its own. The write is refused and read again (62), the
  submission is found told and not marked twice, and the session was
  told a repeat, named by its notification.
- [A long-lived worktree with many landings reads one finished record per
  landing on every ask] → Bounded by its own submissions; a worktree
  landing hundreds of times is unusual. Reading from the newest and
  stopping at the first told was rejected because a landing pushed and
  one asked for are not told in id order.
- [Until a reporting wait marks told, an outcome a wait already reported
  is told again by the command] → The words name the submission and
  outcome, so the repeat is known as one; the wait's unit closes it.
- [The push change does not land as described] → The command needs
  `told_to`, the asking session's name and the push's line; the tasks
  name each, and the coder reads the push change as built before
  starting.

## Migration Plan

Nothing to migrate. The command writes only fields the push change
added, and a `told` log line as the round always has. Records written
before the push change carry `told` from an outbox that marked every
outcome, so the first ask after the upgrade tells only what was decided
since, and any blame or conflict that has stood untold. Rollback is a
revert.
