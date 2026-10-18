# Design

## Context

The overview (`openspec/changes/overview`) shows a combination under
test with its start and limit only. That design (D2, Non-Goals) left
live progress out, because a check's result is recorded only when its
run finishes (rule core.7). Item 11 of the overview contract asks for
each task's state while the run goes, counted by task, never by test.
The question is where that state lives and who writes it, given these:

- The run's directory, `<common>/sluice/.runs/<place>/`, is the check
  seam's, and the records seam doesn't reach it. A profile never calls
  another seam (rule core.2).
- The branch's records are versioned, written all or nothing from the
  version read (61 to 63). Once they are shared under
  `refs/sluice/state/<branch>`, each write is a push with a lease,
  which `design/sluice/models/many-hosts-1.md` §1.2 costs at 0.6 to
  1.5 s, budgeted at about 8 writes per entry.
- The family's storage model (flywheel-next
  `design/storage/storage.md`, rule 1 and decision 2) keeps a piece of
  state in the lowest layer that holds every reader it has, and puts
  run status with the host. Its §2.1 already keeps sluice's runs in
  flight in the clone's unshared part.

## Goals / Non-Goals

**Goals:**

- Each task's state while the run goes, in `sluice/overview@1`, read
  through the records seam with no sluice process running.
- Nothing a verdict, gate, blame or result reads changes.

**Non-Goals:**

- Sending tasks to other hosts.
- Tests inside a task.
- A session run's tasks (`sluice exec` prints its own).

## Decisions

### D1. Kept beside the branch's records

A run's tasks are a record of their own, through the records seam:
`State::put_progress(branch, candidate, Option<&[TaskProgress]>)`,
where `None` removes, and `State::progress(branch)`, every candidate's
tasks. The local files profile keeps them at
`<common>/sluice/.progress/<branch>/<candidate>.json`, schema
`sluice/progress@1`, each file replaced whole by a rename, as it keeps
a failed candidate's logs under `.logs/`. A dot directory is never a
branch, so no `read` of a branch sees them.

They are not in `branch.json`, for three reasons:

- A write at each task's end would bump the branch's version and, once
  the records are shared, cost a push with a lease each time.
- It would publish the tasks to every host (63) before anyone has
  decided they should go there.
- The records a verdict comes from would hold something that is not a
  result.

A shared profile can later carry them as each host's own side record,
the way many-hosts-1 handles `refs/sluice/hosts/<host>`.

Rule core.11 binds what a reader may need from every host. The tasks
decide nothing, and are readable with no process running on the host
that runs them. Its Details gain a "Not ruled out" line, and ADR 0028
records the decision. The statement is unchanged: the tasks are not an
entry's state or a verdict (26), nor a run (16), and B.2's `run` (37,
38) still gives outcomes only when the run finishes.

### D2. The round writes them

The round already holds the branch's lock and asks the check seam after
each run under test. Just before it writes, it calls `Tests::tasks(run)`
for each candidate still under test, and keeps exactly what comes back
when that differs from what is kept. An empty answer removes what was
kept, so a lost run started again on the same candidate never shows the
tasks of the run before. The round also removes the tasks of every
candidate no longer under test: finished, lost or dropped alike. It is
the core that writes, so no profile calls the records seam (rule
core.2), and `sluice run` and `sluice step` behave alike (rule
core.14).

`tasks` is its own method, not a field of `Progress::Running`. That way
previews and `sluice exec`, which ask after their runs often, never pay
for reading it. An error from `tasks` is logged and leaves the kept
tasks as they were. It never fails the round.

The tasks are only as fresh as the rounds. A waiting `wt submit` or
`sluice run` runs one every 3 s. With no round running, the verdict
waits too, so the tasks shown are never newer than the combination's
state.

A progress write sits outside the branch's versioned write. If that
write is then refused (62), the tasks stay as written, which is
harmless, since nothing decides from them.

### D3. What moon says while it runs

This was checked against moon 2.6.1, the repository's pin, on
2026-10-18, with `moon exec … --on-failure continue` writing to a file
as sluice's supervisor has it:

- **The run's output** carries moon's checkpoints, written as they
  happen:
  - `▮▮▮▮ a:test (f8bf095b)` when a task starts;
  - `▮▮▮▮ a:test (1s 65ms, f8bf095b)` when it ends;
  - `▮▮▮▮ b:test (cached, b7d5ce19)` for a cache hit, with no start
    line.
  
  A failed task's end line differs only in colour. A task's own output
  is prefixed `a:test | `.
- **`.moon/cache/states/<project>/<task>/lastRun.json`** in the
  checkout is written as each task ends, before `runReport.json`:
  `{"exitCode":3,"lastRunTime":1792341195651,"target":"c:test",…}`. A
  cache hit rewrites it with the same exit code.
- **moon's `--log`** goes to stderr even with `--log-file`, so it
  would flood the run's output. It is not used.

`Moon::progress` takes the planned targets from the run's request, and
reads the run's output and each target's `lastRun.json`:

| Seen | State | Times |
|---|---|---|
| an end checkpoint marked `cached` | `replayed` | `ended` from `lastRun.json` |
| an end checkpoint, or `lastRunTime` at or after the run's start | `passed` when `exitCode` is 0, else `failed`; `exit` set | `ended` = `lastRunTime`; `started` = `ended` less the checkpoint's time |
| a start checkpoint, no end | `running` | none: checkpoints carry no clock time |
| nothing | `waiting` | none |

moon writes a task's run record a moment before its end checkpoint, so
an ended task can lack `started` until the next round. Only a line that
starts with the checkpoint mark and names a planned target counts, after colour codes are stripped. The run's start is
recorded in the run's request (`Meta.started`). The command runner
returns no tasks, since its one task's state is the run's own.

### D4. The document and the plugins

`CandidateView.tasks: Option<Vec<TaskProgress>>` is set for a candidate
`starting` or `testing` whose tasks are kept, and skipped when absent.
`branch_view` stays pure: `Sluice::overview` reads `State::progress`
and passes it in. Times are RFC 3339 like every other time in the
document.

The board counts a task as done once it has ended (passed, failed or
replayed). Each view keeps its own time form (`2m10s/15m` in the band,
`2m10s / 15m` in the pane) and adds the count and bar before it. The
bar's ten cells are rounded down, so a run shows full only when every
task has ended. A redraw needs no version change: `--watch` prints each
document that differs from the one before.

## Risks

- **moon's checkpoints are console output, not an interface.** A later
  moon could change them. The moon tools test reads them from a real
  moon, so a change shows there. Ended tasks would still come from
  `lastRun.json`, which C.2 names.
- **The run's output is read whole at each round.** That is about a
  scan of a few megabytes every 3 s for a long run.
