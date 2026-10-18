# Proposal

## Why

While a combination is under test, the board and the herdr popup show
only its elapsed time against its limit, so a person can't tell a run
that is nearly done from one that has hardly started. moon already
writes a record for each task as that task ends. This change keeps that
progress on the host and shows how many of the combination's tasks have
finished, and in a detail view, each task's state. It is item 11 of the
shared overview contract.

## What Changes

- While a combination is under test, each round that finds its run
  still going records the run's tasks, each one waiting, running,
  passed, failed or replayed, with when it started and ended and its
  exit code. The tasks are kept beside the branch's records, never
  among them. They change no branch's version, and no result, gate or
  verdict reads them (16, 37, 38).
- The moon runner reads these tasks from moon's own output while the
  run goes: its start and end checkpoints in the run's output, and the
  run record (C.2) moon writes in the checkout as each task ends. The
  command runner records none.
- `sluice overview --json` gives each combination under test a `tasks`
  list, `{target, state, started?, ended?, exit?}`. It is an added
  field of `sluice/overview@1` and is absent when nothing was recorded.
  The text form adds `3/5` to a combination under test.
- The Claude Code board shows `3/5` and a ten-cell bar wherever it shows
  a combination under test, and lists each task with its state and time
  in its pane.
- ADR 0028, and Details lines for rules core.7 and core.11.
- Docs: the `sluice overview` reference and the Claude Code guide.

Out of scope:

- Showing the tasks in herdr. That waits on the `herdr-plugin` change
  landing, and comes as the last group of this change, rebased onto it.
- Sending a run's tasks to other hosts. Once the records are shared
  through the remote (ADR 0008), whether a run's tasks travel with them
  is for that change to decide. Until then a host shows only the tasks
  of runs on that host.
- Counting tests inside a task.

## Capabilities

### New Capabilities
- `run-progress`: the tasks of a combination under test, recorded while
  its run goes and shown in the overview.

### Modified Capabilities
- `plugin-board`: the band and the pane show how many tasks have
  finished, and the pane lists each task.

## Impact

- **The seams:** `TaskProgress` and `TaskState`, `Tests::tasks`, and
  `State::progress` and `State::put_progress`.
- **The core:** the round's poll records and sweeps the tasks;
  `overview::branch_view` reads them into `CandidateView.tasks`.
- **The profiles:**
  - `local_files` keeps tasks at `<common>/sluice/.progress/<branch>/<id>.json`;
  - `checks` and `moon` read them from the run;
  - the fakes script them.
- **The CLI:** `sluice overview`'s text and document.
- **`plugins/sluice-board`:** `board.ts`, `register.tsx`, the types and
  the tests.
- **Docs:** ADR 0028, `docs/architecture/core.md` and the site's rules
  page, `reference/commands.md`, `guides/claude-code.md`.
