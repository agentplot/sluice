# Design

## Context

A waiting `sluice submit` (`crates/sluice/src/lib.rs`, `submit`) loops:
it runs a round, reads the submission, stops on a decided state, and
pauses three seconds. It has no time limit, and an agent's tool call
does. `sluice status <n> --json` answers for a finished submission too
(the finished records, #92), but nothing loops on it for the agent.

While a combination is under test, a round on the host running it keeps
the run's tasks beside the branch's records (ADR 0028), and the
overview reads them into `candidates[].tasks[]`. The moon runner reads
them from moon's checkpoints in the run's output and from each task's
`lastRun.json`. A checkpoint carries no clock time, so a running task
has no `started`. `sluice status` reads no candidate and no tasks.

## Goals / Non-Goals

**Goals:**

- One command an agent loops on within a capped tool call, whose exit
  and document say landed, failed, conflict, withdrawn or still
  waiting.
- The text views say how far a combination under test has got, and how
  long its running tasks have run.

**Non-Goals:**

- A run's tasks read from another host (ADR 0028).
- Any change to the records, the round, or a JSON form.

## Decisions

### D1. One wait loop

`wait_on` holds the loop a waiting submit has today, with a deadline.
Each pass reads the submission. A decided one (`State::is_out`) is
reported. Past the deadline, it returns as still waiting. Otherwise it
runs one round (`Sluice::round`) and pauses. It reads before its first
round, so a submission already decided returns at once and runs
nothing. The deadline is measured with the command's `Clock`. A waiting
submit calls `wait_on` with no deadline, and `wait` with its
`--timeout`. A round takes no longer than it does under a waiting submit,
and the pause is three seconds, so `wait` returns within a few seconds of
its timeout.

On a branch that takes pull requests, `wait <n>` runs intake before
each round, as a waiting pull-request submit does, and `wait <branch>`
follows this worktree's pull request in that submit's loop, finding it
as `submit --status` does. A pull request closed without landing exits
`1`, as it does for submit.

### D2. Exits and the document

`0` landed, `1` failed (or a pull request closed), `2` conflict or
refusal, `4` withdrawn, `5` still waiting, `130` stopped with Ctrl-C;
`3` and `64` as every command. A waiting submit shares them, so its
withdrawn exit moves from `1` to `4`. A submission that does not exist
exits `1` with the not-found error document, as `status <n>` does. An
agent's loop goes on only on `5`, so it stops.

Under `--json`, each ending prints one document, the submission's
record as `status <n> --json` prints it. A timeout and Ctrl-C print it
too, in place of today's `still queued` text, which reached stdout even
under `--json`. The record's `state` is documented as stable: under
`sluice/submission@1` none of its nine values is renamed, removed or
added.

### D3. A running task's start, from the run's output

`sluice supervise` already wakes every 100 ms while its command runs.
When `out.log` has grown and a second has passed since its last note,
it appends `<length> <time>` to `out.times`, and once more as the
command ends. `runs.rs` names no tool: it notes when bytes arrived, not
what they say. `Runs::times(run)` reads the notes.

`checkpoints()` in the moon profile keeps where each start checkpoint's
line ends. `Moon::progress` gives a running task the time of the first
note at or past that point: when its start was in the output, to within
a second. A finished task keeps its time from `lastRun.json`. A run
whose `out.times` is missing, as one started by an older sluice is,
gives running tasks no `started`, as today.

The round, the records and `sluice/progress@1` are unchanged:
`started` is already in the schema. `out.times` sits with the run's
own files and goes with its directory. The rejected alternative, the
round noting when it first saw a task running, is late by however long
no round ran.

### D4. The progress line

One formatter in the CLI:
`7/12 tasks · running sluice:e2e 25m03s, site:prose 40s · 26m10s of 45m`.
At most three running tasks are named, longest first, then `+N more`. A
running task with no `started` is named with no time. With no tasks
kept (the command runner, another host, a runner that has said nothing
yet), the line is the combination's time alone. The combination's time
runs from its testing line in the branch's log, against the limit its
policy gives; durations read like the board's (`45s`, `2m10s`,
`1h03m`).

- `status <branch>` prints one `testing <combination> · <progress>`
  line per combination under test, under its header, as the overview's
  text does; its rows are unchanged.
- `status <n>`, a waiting submit and `wait` print the line for the
  submission's own combination: the one under test whose last
  submission it is, else any under test that holds it. A waiting
  command prints it when the state, the count ended or the tasks
  running change, never on each pause.
- The overview's text uses the same line in place of `3/5 since <t>`.

`status` reads one branch through `Sluice::branch_view_of`, the
overview's per-branch read moved into a function of its own. ADR 0023
kept `status`'s forms unchanged; its JSON forms stay so, and only its
text gains the line.

## Risks / Trade-offs

- **A note per second while output grows.** A forty-five-minute run
  that prints all the time writes about 2,700 short lines, removed with
  its run.
- **A start is known to a second, and only on the host that runs it.**
  Another host shows the combination's time alone.
- **Withdrawn changes a waiting submit's exit.** It was `1`,
  undocumented; the reference, the help and the skills say `4`.
