# Design

## Context

The check seam reports a check as `Pass` or `Fail` (`sluice_seams::Outcome`).
The `command` runner turns a spawn error (`Ended.error`) into `Fail`. The
moon runner reads moon's run report, where a task that broke because the
host lacked something is `failed` like any other failure. The verdict
reads every trusted result for a key (69, 19). The statement already has
the ending such a run needs: no outcome, never read as a pass or a fail,
and the check run again (60). The core already handles a whole run with
no outcome, `Progress::Lost`.

## Decisions

### No outcome is a list beside the results, never a third outcome

`RunReport` gains `not_run: Vec<NotRun { task, reason }>`. A check in it is
neither in `tasks` nor in `missing`. `Outcome` stays `Pass | Fail`, and
`results.jsonl` holds only outcomes. An older reader sees nothing new, and
the verdict needs no change to keep such a check off its key.

### The three ways, all in the profile

The check profile works out `not_run` when a run finishes:

1. `Ended.error` is set, so the runner never started: every target is
   not run, with the error.
2. A check's exit is one `[tests] not_run` lists, with
   `not_run = [77]` or `not_run = 77`. For the command runner that is
   the command's exit. For moon it is the task's `exitCode` in its
   `task-execution` operation in `runReport.json`.
3. `[tests] ready` is set and something did not pass, a fail or a
   missing check. The profile runs `ready` once, and keeps the answer
   in the run's directory, since `progress` is read more than once. If
   `ready` fails, every fail and missing check becomes not run, with
   the command and the end of its output. Passes stand.

`[tests] ready` is a string run by `sh -c`, or a list with the command
first, the same shape as `release`. It runs in the place with the run's
environment, the repository's toolchain sourced, under the run's
executor (a Seatbelt profile of its own, as `release_request` writes),
and with a 60-second limit. In `start`, after the toolchain and before
the runner's plan, a failing `ready` is an error naming the command, so
the run is refused before anything of it runs (84).

### What the core does with it

- **The gate's poll.** It records `tasks` as now and reads the verdict.
  - If the verdict failed on a recorded check, the candidate fails as
    today. The checks that could not run are named among its failed
    tasks, with their reason.
  - Otherwise, if `not_run` is not empty, the candidate's new
    `not_run_runs` count goes up by one. Below three, the candidate is
    formed again: its place is released, and `Event::Lost` is logged
    with the new optional `not_run` list. At three, the candidate fails,
    with each check that could not run as a failed task carrying its
    reason. That is could-not-run-1 115, and it is attributed as any
    failure is (17).
- **A start the profile refuses.** Its error stops the step, as a
  toolchain that fails does today. `recover` forms the candidate again
  on the next round.
- **A session run.** `record_session` records only `tasks`, so nothing
  of a check that could not run is recorded. `finish_session` gives
  `RunOutcome::NotRun` when nothing failed and something could not run.
  `session_request` drops `ready` and `not_run` from a command run's
  settings.

`FailedTask` and `Event::Lost` gain optional fields with serde defaults,
so a branch's records written by this version still read in an older one.

### Set aside

`sluice set-aside <task> --at <time> [--hash <prefix>] --reason <text>`
finds exactly one result line by task and time (and hash prefix). It
appends a `sluice/set-aside@1` line to `set-aside.jsonl` beside
`results.jsonl`, under the records' lock: `{at, task, hash, run_at,
branch, combination, reason, by}`. It is a file of its own because
`parse_lines` is strict, and an unknown kind in `results.jsonl` would
break an older reader on another host (rule core.11).

`Sluice::results()` leaves out every line a set-aside names, so the
verdict, the duplicate check and `status` all read the same. The line
itself stays (rule core.7). The plugin's `guard.sh` refuses
`sluice set-aside` in an agent's Bash, since a session that could set
aside its own fails could land past any check (116).

### Not a probe

sluice names no tool. A container engine, a database or anything else a
repository's checks need is the repository's to declare in `ready`.
sluice's own policy declares `docker info`.

## Risks

- **Readiness is run-wide.** With the container engine down, a real
  failure in a check that never uses it is also not run, and runs again
  later, where it fails again. That costs one run.
- **The bound is fixed.** Three runs in a row can be wrong for a host
  that recovers slowly. A refusal before the run does not count toward
  it, so a host that stays down holds the queue without blaming anyone.
