# Proposal

## Why

On 2026-10-21 a host under heavy I/O ran a bolt's tests past the
branch's one-hour time limit. sluice ended the run while `scripts/ci:test`
was under two minutes in. moon reported that test failed, and sluice
recorded the fail under its key. Half an hour later the same head ran in
a minute and the test passed under the same key. The key then held a
pass and a fail (19), the bolt reads that as failed, and the head could
not land again. The agent's guide sent it toward changing the test's
inputs to get a new key, which its operator rightly refused.

A time limit says how long a run may take, not whether a test it ended
would have passed. The statement's 66 reads such a run as failed but says
nothing of the tests it cut off, so this change rests on the unratified
proposal `design/sluice/proposals/time-limit-1.md` in agentplot/blueprints,
which amends 66: a check that ended before the limit keeps its outcome,
and a check the limit ended has none and is recorded under no key.

## What Changes

- On a run its time limit ended, the moon runner keeps the results of
  the tests that ended before the limit, a fail as much as a pass. A test
  whose end moon did not record before the limit has no result, whatever
  moon reported of it. moon reads such a test as failed when its process
  caught the signal and exited first, and as skipped when moon stopped
  first. sluice names it as left without a result, keeps its log, and
  records nothing under its key (time-limit-1 66).
- `sluice submit` and `sluice status` on a timed-out submission name the
  tests the limit ended, each with its log. They say nothing was recorded
  for those tests, so the same head may be submitted again once the host
  keeps up, and a test that never ends needs a fix.
- A flaky failure stops telling the agent to change what the test reads.
  It says the same head lands once the operator sets the fail aside, or
  once a real fix changes what the test reads.
- The plugin's guide gives an agent one move when a flaky failure's other
  result looks like the host's doing: stop, and tell the user what it
  found and the runs' times. The reference pages keep `sluice set-aside`
  in full for the operator, and say which failures sluice tells apart on
  its own and so never need setting aside.

## Capabilities

### Modified Capabilities

- `run-outcome`: a test a run's time limit ended has no result.

## Impact

- Code: `crates/sluice/src/profiles/checks.rs` and `profiles/moon.rs`,
  the CLI's words, `crates/sluice-fakes` (a hung test is cut off).
- Docs: `plugins/sluice/skills/using-sluice/references/verdicts.md`; the
  site's commands, configuration and walkthrough pages;
  `docs/architecture/core.md` (core.7's details); ADR 0041.

## Out of scope

- Bounding the time sluice spends preparing a run (the checkout, the
  toolchain, moon's plan) or waiting for room under the host's bound.
  The limit times only the checks' command (66), and on 2026-10-21 the
  preparation took over an hour on the loaded host. An event when a
  run's checks start would make such a wait readable; it waits on a
  change of its own.
- A way to list the results the operator has set aside, in `sluice status
  --json` or the overview, so each can be reviewed for whether sluice
  could have told the failure apart itself. Each set-aside is a sign
  sluice recorded something it should not have. This waits on a change
  of its own.
- The command runner and a session's run, which already record nothing
  for a run its time limit ended.
