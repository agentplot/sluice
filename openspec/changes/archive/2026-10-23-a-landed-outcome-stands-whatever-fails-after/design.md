# Design

## Context

A waiting `sluice submit` and `sluice wait` share one loop, `wait_on` in
`crates/sluice/src/lib.rs`: read the submission, run a round, read again,
until it is decided (ADR 0041). A waiting submit that follows a pull
request has its own loop in `submit_pull`. In both, a round that returns
an error ends the command with that error, which `main` turns into exit
3, "sluice itself failed".

A round (`Sluice::round`, `crates/sluice-core/src/step.rs`) performs, in
order: the landing, the releases, telling outcomes, the starts. The
landing moves the branch, then writes the submission `landed` with its
`landed_as` and queues its notification, in one records write. Anything
after it, a release command, a notification, a forge call, can fail and
return an error from the round; so can the records write that confirms
the landing, after the branch has moved. Each leaves records the next
round finishes from (24, 51; rule core.14): `perform_landing` finds the
branch at the intent's revision and confirms it, the release stays in
`release`, the notification stays in the outbox.

So the facts are already in the records and the branch when the round
fails; only the waiting command reads the round's error as the outcome.

## Goals / Non-Goals

**Goals:**
- The command waiting on a submission whose work is on its branch ends
  as landed, exit 0, whatever the round it ran failed on afterwards, and
  says what failed apart from the outcome (122).

**Non-Goals:**
- Changing what a round does after a landing, or how a failing release
  is retried or held (123, the change
  `a-failing-release-holds-its-place-not-its-branch`).
- Recording the after-failure in the submission's record: it is the
  round's, not the submission's, and the round's own records keep what
  is left to do.

## Decisions

### The waiting command decides, from the records and the branch

On a round error (and not Ctrl-C, which keeps its exit 130), the loop
asks the core whether the submission's work is on its branch, through a
new read beside `Sluice::state` in `crates/sluice-core/src/ops.rs`:

- the submission's record says `landed`: its `landed_as`;
- otherwise, the branch's record holds a landing intent naming the
  submission, and the branch read now is the intent's `to` or contains
  it, the same test `perform_landing` uses to confirm a landing: the
  intent's commit for that submission;
- otherwise: not landed.

When landed, the loop ends as `Waited::Decided` with the submission as
landed and the error carried beside it; `report` prints the outcome as
for any landing, and the error goes to stderr as
`sluice: after the landing: <cause>`, and to telemetry as `main` sends
an exit-3 error. When not landed, the error returns as now. The
pull-request loop in `submit_pull` does the same with the submission its
request was taken in as, when it has one; a request with no submission
yet has nothing landed, and its error returns as now.

Alternative: make the round itself never return an error after a
landing. That is the round's behavior, partly 123's change, and it would
still leave the records write that confirms the landing able to fail
after the move. The waiting command reading the facts covers every
failure after the move, whatever the round becomes.

Alternative: decide from the round's error kind. An error carries no
reliable mark of where in the round it came from; the branch and the
records do.

### Exit 0, the failure on stderr

The exit is the outcome's (C.5; the exits table of `submission-wait`),
and 122 says nothing after the move changes the outcome. So a landed
submission exits 0 however the round went; 3 stays sluice failing with
the outcome not landed, and the two are never confused. What failed is
on stderr, as every other failure sluice says is, behind its own words,
`after the landing:`, so a reader tells it from `sluice: <cause>`,
sluice failing itself. stdout keeps exactly what a landed ending prints:
`passed · landed as <rev>`, or under `--json` the record alone.

Alternative: a new exit for "landed, and something after failed". It
would make the after-failure change the outcome's exit, which 122 rules
out, and every reader that knows `0` as landed would read it as not
landed, the failure this change removes.

Alternative: an `after` field in the `--json` document. The document is
the submission's record as `sluice status <n> --json` prints it, and the
failure is not the submission's.

### A landing the round did not get to record

When the branch moved and the confirming write failed, the record still
says `landing`. The command reports it as the next round will write it:
`state` `landed`, `landed_as` the intent's commit for the submission,
and under `--json` that record. It writes nothing itself (rule core.4):
the next round, from whatever process runs it, confirms the landing
from the same intent.

## Risks / Trade-offs

- [A release that keeps failing fails every later round, and a later
  waiting command on the same branch then reports it beside its own
  outcome too] → That command's outcome still stands; the failure stops
  once 123's change holds the place instead of failing the round.
- [A submission still `queued` behind the landed one, waited on by
  another command, meets the same round error] → Its work is not on the
  branch, so it exits 3 as now; 123's change removes the cause.
- [`wt submit` after a landing whose record was not yet written keeps
  the worktree, since it removes one only once sluice's record says its
  head landed] → It keeps the worktree and says so, as it does today;
  the next round records the landing.
