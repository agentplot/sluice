## Context

`sluice wait` runs one of two loops. A wait on a submission runs
`wait_on`; a wait on a branch that takes pull requests, and a wait that
follows the pull request a hand-over left its work on, run
`submit_pull`'s. Clause 1 has both run rounds between reads, and
submission-wait has a waiting submit and `sluice wait` wait through one
loop. The deadline is set as the command starts, so it counts the
lookup of the submission and the first read of the overview, which on a
busy host take seconds.

`submit_pull`'s loop takes its intake and reads its request, then looks
at its deadline. `wait_on` looked at its deadline first. The two loops
began differently, and only `wait_on` could end having moved nothing.

## Goals / Non-Goals

**Goals:** a wait moves its queue once whatever its limit, so an agent's
waits in turn on a busy host still move it; `handed_over_by_a_wait`
holds at the cause, whatever the host's load.

**Non-Goals:** how long a pass may take past the limit, which is as
before: a round runs to its end, and the limit is looked at between
passes (134 bounds a remote's calls by the wait's own limit).

## Decisions

### D1. The first pass is taken before the deadline is looked at

`wait_on` looks at its deadline, and pauses, from its second pass on.
Its first pass is its intake and round where no driver lives, or a read
of the driver's document where one does (135). A submission already
decided is still reported before any pass, as submission-wait has it.
Out of time, its record is the one its last pass read.

Alternatives considered:

- **The limit counted from the wait's first read.** A wait at `0s`
  would still return before any pass, and the two loops would still
  begin differently.
- **A longer limit in the test.** The race stays: a host busier than
  the limit's margin fails it again, and agents' waits on such a host
  still move nothing.
- **The test running a step before its wait.** The step would withdraw
  for the hand-over, and the test would no longer show what a wait does
  alone, which request-intake-host's scenario asks.

### D2. The tests hold it with no time to spare

`handed_over_by_a_wait` waits with `--timeout 0s`, so a wait that
skipped its first pass fails it on every run and never by the host's
load. `a_wait_out_of_time_at_its_start_takes_its_first_pass` holds the
same over the fakes, where a queued submission's first round starts its
run.

## Risks / Trade-offs

- A wait given `0s` now runs a round before it answers. A caller that
  wants where a submission stands with nothing moved has `sluice status
  <n>`, which runs no round.
