## Why

On 2026-11-08, sluice's `main` gate failed `cli:tools` for submission
#33 (round `c27`), at sluice main 8520f5c: `handed_over_by_a_wait` in
`crates/sluice/tests/profiles/pull_requests.rs` panicked at line 5450
with `left: Null`, `right: {"pull_request": 1, "state": "open"}`. The
wait's document was its submission's record as it stood before the
wait did anything: `state` `testing`, no `carried_by`, the run never
withdrawn for the hand-over. Run alone on main's tip, it passed five
times of five.

What sluice does at 5b82a38:

- `sluice wait <id>` sets its deadline as the command starts.
  `wait_on` in `crates/sluice/src/lib.rs` reads its submission and the
  overview, then checks the deadline at the top of its loop, before its
  first pass. Where the start, the lookup of the submission and that
  first read of every branch, took the whole limit, the wait returns
  undecided with the record it read, and it has moved nothing: no intake,
  no round.
- A wait that follows a pull request, `submit_pull`'s loop, takes its
  intake and reads its request before it looks at its deadline, so its
  first pass is taken whatever the limit.
- `handed_over_by_a_wait` gives its wait two seconds, and the hand-over
  is withdrawn only by that wait's first intake. Under the gate's load,
  167 tests of the suite at once, the wait's start took the two seconds.

With `--timeout 0s`, the test fails this way on every run at 5b82a38.
An agent whose capped tool calls run `sluice wait <n> --timeout 9m` in
turn, on a host that busy, is told where its submission stood before
the wait, and no wait of its moves the queue.

## What Changes

- **A wait takes its first pass whatever its limit.** `sluice wait`
  looks at its deadline only after its first pass: its intake and round
  where it runs rounds, or a read of the driver's document where `sluice
  run` lives (135). Out of time, its record is as its last pass left it.
  The pull request's follow already does this, so both waits now begin
  the same way (1).
- **The test holds the guarantee.** `handed_over_by_a_wait` waits with
  `--timeout 0s`, its time up as it starts, so its first pass is the one
  that withdraws. `a_wait_out_of_time_at_its_start_takes_its_first_pass`
  holds the same over the fakes, for a queued submission whose first
  round starts its run.

Out of scope: how far a pass may run past the limit, which is as
before. No later change waits on this one.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `submission-wait`: a wait's limit ends it only after its first pass,
  and its record out of time is as that pass left it.

## Impact

- `crates/sluice/src/lib.rs`: `wait_on` checks its deadline, and pauses,
  from its second pass on.
- `crates/sluice/tests/cli.rs`:
  `a_wait_out_of_time_at_its_start_takes_its_first_pass`.
- `crates/sluice/tests/profiles/pull_requests.rs`:
  `handed_over_by_a_wait` at `--timeout 0s`.
- `site/src/content/docs/reference/commands.md`: the `sluice wait` row
  says it runs one round, or reads once what the driver read, before its
  limit ends it.
- No statement clause changes: clause 1 has a wait run rounds between
  reads, and request-intake-host's "A wait on a submission handed over
  follows its pull request" already has the wait's pass withdraw.
