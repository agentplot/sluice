# Design

## Context

See proposal.md for the incident. What the code does today:

- **The time limit.** `sluice supervise` (`crates/sluice/src/runs.rs`)
  is started in a session of its own by `Runs::start`, outlives the step
  that started it, runs the checks as one process group, ends the group
  at `request.limit` and writes `result.json` with `timed_out`. A
  supervisor that died is read as `RunState::Lost` and its group ended by
  `Runs::state`. `tests/runs.rs` holds both
  (`a_run_over_its_limit_is_ended_and_timed_out`,
  `a_run_whose_supervisor_died_is_lost_and_ended`). On deneb the run of
  `#405` ended by itself after 653 s (its `detail` line); the "35m of
  20m" was the view of a combination nothing had read since.
- **Which branches a driver steps.** `sluice run`'s pass calls
  `Sluice::in_play` (`sluice-core/src/ops.rs`) and runs a round only
  where a submission is in the queue. Once the hand-over withdrew `#405`
  and `#406`, `main` had none, so no round read the run or released its
  checkout.
- **The round's order.** `decide` (`sluice-core/src/step.rs`) calls
  `poll` before `arrange`. `poll` reads every finished run of every
  combination under test and writes its verdict; `arrange` then forms the
  chain and drops the combinations it no longer holds, listing their
  places for release. So a withdrawn submission's finished run is read
  (`result passed` for `9053c06` at 16:53:19) before its combination is
  dropped (`released sluice/try/main/193` at 16:53:20).
- **The hand-over.** `PullRequests::intake` in
  `profiles/github/pulls.rs`, finding another host's mark, calls
  `hand_over`, which withdraws each pull request submission through
  `Sluice::withdraw`. A wait then runs no round for the branch (137).
- **A host that does not keep the branch** (records on its remote)
  already releases, in `execute`, every place it made that no claim of
  its own holds (81).

## Goals / Non-Goals

**Goals:**

- One rule for every combination holding a withdrawn submission: dropped
  before any run is read, this host's run of it ended, its checkout
  released or held, nothing read or recorded from it.
- That rule applied at the host's next read of the branch: the driver's
  next pass, any round, and the hand-over's own pass.

**Non-Goals:**

- Changing how the supervisor holds the limit, or how the view shows a
  combination between reads.
- Ending another host's run (rule core.16). On a branch whose records
  are on its remote the lead drops the combination and each claiming
  host's `execute` releases its own place, as today.

## Decisions

### D1. Obsolete means holding a withdrawn submission

A combination under test is obsolete when any submission it holds is in
the state `withdrawn`. Blamed and conflicting submissions stay
`arrange`'s, which already re-forms the chain without them (21); a
landed submission never makes a combination obsolete, since a
combination whose entries ahead landed as tested stands (rule core.17).
Alternative: any submission `is_out()`. Rejected: `is_out()` holds
`landed`, and reading it so would drop combinations core.17 keeps.

### D2. Dropped before `poll`, in one place both callers use

A new core operation drops the obsolete combinations of a branch's
records in hand: for each, it unpublishes the combination as `arrange`
does, and lists the places of this host's runs of it in
`doc.release`. `decide` calls it after reading the records and before
`poll`; `perform_releases`, which runs before `perform_starts`, then
ends each run (the place seam's release ends its run first, through
`Runs::remove`) and releases or holds the checkout (123). That ordering
is what holds rule core.23: nothing is started or claimed while the
place is unreleased. `arrange`'s own drop stays for combinations the
chain re-forms.

Alternative: have `poll` skip obsolete combinations and leave the drop
to `arrange`. Rejected: two places would then decide what is obsolete,
and a round that returns before `arrange` (a branch put back, 121)
would leave the run going.

### D3. Nothing of the run is read, finished or not

The dropped combination's runs are not read: no check recorded, no
result line, no blame, no notice. This is what `arrange` already does
for a combination it drops, so a withdrawal is one more way to be
dropped rather than a new outcome. Rule core.7 holds: no verdict is
read from the run. Alternative: record the checks of a run that had
finished, since a key's result belongs to its key (69). Rejected for
this change: it adds a path that reads a run with no verdict after it,
for a cache gain on a head nobody is waiting on; the same head
submitted again runs again.

### D4. The driver steps a branch while anything of this host's is under way

`in_play` counts a branch in play when a submission is in the queue, a
combination is under test, or `doc.release` lists a place, all read
from the records alone, as today (129; rule core.31: no revision or
file read for the decision). A round then runs at the driver's next
pass; once the combination is dropped and the place released, the
branch is out of play again. Alternative: use `has_work`'s full test,
the landing and the outbox among it. Rejected as wider than this
change; a landing's submission is already in play.

### D5. The hand-over ends what it made obsolete in its own pass

After `hand_over` withdraws, it calls the same core operation under the
branch's lock, then the releases, so a wait whose pass hands over, and
which then runs no round (137), still ends the runs on this host. This
is no round: it advances no submission and takes nothing in. The driver
whose pass hands over gets the same result from its round (D4); doing
it in the hand-over too keeps the two takers alike.

### D6. The time limit gets a test, not a change

The supervisor already holds the limit apart from the step. A test with
the real check seam starts a run by `sluice step`, lets the step exit,
and shows the group gone after the limit with no sluice process
running, then the next step recording it timed out.

## Risks / Trade-offs

- [A round per pass while a run goes on with nothing queued] → the
  same cost a queued submission's round has today, and it ends at the
  round that drops or reads the combination.
- [The hand-over takes the branch's lock inside intake] → `withdraw`
  already takes it per submission; the drop takes it once more after
  them, never while holding another branch's.
- [A withdrawn head's finished passes are not kept] → submitting the
  same head again runs its checks again; accepted (D3).
