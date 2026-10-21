# Proposal

## Why

An agent whose tool calls end at ten minutes can't block on
`sluice submit`, since a verdict takes from one minute to forty-five.
One such agent submitted about forty times with `--no-wait` and polled
`sluice status main` every minute, grepping the listing for
`landed|failed`. A decided submission leaves that listing, so twenty of
its pollers never stopped. The listing says `testing` and the
combination, and nothing of how far the run has got, so the agent
couldn't tell a one-minute docs run from a forty-minute integration
run.

## What Changes

- `sluice wait <n|branch> [--timeout <duration>] [--json]` waits until
  a submission is decided, running rounds between reads exactly as a
  waiting `sluice submit` does (1; rule core.4; ADR 0011). A waiting
  submit and `wait` share one loop. With `--timeout`, it returns when
  the time runs out, with the submission as it stands, so an agent
  with a capped tool call loops on one command. `<branch>` waits for
  this worktree's last submission to that branch, found as
  `sluice submit <branch> --status` finds it.
- Exits, for a waiting submit and `wait` alike: `0` landed, `1`
  failed, `2` conflict, `4` withdrawn, `5` still waiting. Withdrawn
  exits `1` today.
- Under `--json`, every ending prints the submission's record, the one
  `sluice status <n> --json` prints, including a timeout and Ctrl-C.
  The record's `state` and its nine values are documented as stable
  under `sluice/submission@1`.
- The run's supervisor notes when the run's output grew, in
  `out.times` beside `out.log`. The moon runner reads a running task's
  start from it, so a running task carries `started`. Nothing else is
  recorded, and the round and the records are unchanged.
- `sluice status <branch>`, `sluice status <n>`, a waiting submit and
  `wait` say, for a combination under test, how many of its tasks have
  ended of how many, the tasks running and for how long, and the
  combination's time against its limit, read from the tasks the
  overview reads (ADR 0028). The overview's text says it the same way.
  No JSON form changes.
- The `plugins/sluice` skills, the session note, the commands
  reference and the guides tell an agent to submit with `--no-wait`
  and loop on `sluice wait <n> --timeout 9m --json`, keying on
  `state`.
- ADRs 0041 and 0042; rule core.4's and core.7's Details.

No clause changes: `wait` submits nothing (1), reads an entry's state
(26, 49, 51) and runs the rounds a waiting driver runs; the progress it
and `status` show is read on the host that runs the combination and
decides nothing (16; rules core.7, core.9, core.11).

Out of scope:

- A run's tasks read from another host. That waits on a shared records
  profile deciding whether tasks travel (ADR 0028); until then another
  host's views show the combination's time alone.
- A `--timeout` on `sluice submit`.
- Counting tests inside a task.

## Capabilities

### New Capabilities
- `submission-wait`: waiting on a submission's outcome, with a time
  limit an agent's tool call can live within, and the exits and
  document it ends with.

### Modified Capabilities
- `run-progress`: a running task says when it started, and the text
  views show a combination's progress.
- `plugin-usage`: an agent with a capped tool call waits with
  `sluice wait`.

## Impact

- **The core:** `Sluice::branch_view_of`, one branch's part of the
  overview, which the overview is built from.
- **The runs:** the supervisor writes `out.times`; `Runs::times` reads
  it.
- **The profiles:** `checks` passes the times to `moon`, which gives a
  running task its `started`.
- **The CLI:** `sluice wait`; one wait loop under it and a waiting
  submit; exits `4` and `5`; the progress line in `status`, `submit`,
  `wait` and `overview` text.
- **`plugins/sluice`:** the using-sluice skill and its references, the
  session note and its test.
- **Docs:** ADRs 0041 and 0042, `docs/architecture/core.md` and the
  site's rules page, `reference/commands.md`, `reference/state.md`,
  `guides/claude-code.md`, `guides/walkthrough.md`,
  `guides/pull-requests.md`, `README.md`.
