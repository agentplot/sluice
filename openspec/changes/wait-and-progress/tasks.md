# Tasks

Read first: this change's proposal, specs and design; ADRs 0011, 0023,
0028, 0041 and 0042; `docs/architecture/core.md`. Run suites through
`sluice exec main -- <command>`; before submitting, `sluice exec main`.

## 1. Decisions

- [x] 1.1 Write ADRs 0041 and 0042; name `sluice wait` and a waiting `sluice submit` in core.4's Details and ADR 0042 in core.7's, in `docs/architecture/core.md` and `site/src/content/docs/architecture/rules.md`; verify `openspec validate wait-and-progress --strict` (by hand: no gate task runs openspec) and `site:prose`.

## 2. The core

- [x] 2.1 Move the overview's per-branch read into `Sluice::branch_view_of` and build `Sluice::overview` from it; verify the overview tests over the fakes, and a test that one branch's view is the overview's entry for it.

## 3. A running task's start

- [x] 3.1 The supervisor notes `out.log`'s length and the time in `out.times` at most once a second while it grows and once at the end; `Runs::times` reads them; the run directory's layout in `runs.rs` names the file; verify a `tests/runs.rs` case where a command prints, sleeps and prints again and leaves two notes.
- [x] 3.2 `checkpoints()` keeps where each start checkpoint ends; `Moon::progress` gives a running task the time of the first note reaching it; `Checks::tasks` passes the notes; verify unit tests in `moon.rs` for a running task with notes, without notes, and an ended task unchanged.
- [x] 3.3 Extend `a_run_shows_each_task_as_it_ends` in `tests/profiles/moon.rs`: the running task has a `started`; verify `cargo test -p sluice --features tools --test moon`.

## 4. Progress in the text views

- [x] 4.1 One formatter for the progress line and the durations; verify unit tests with fixed times: a count with running tasks, more than three running, a running task with no start, no tasks, no limit.
- [x] 4.2 `status <branch>` prints a line per combination under test, `status <n>` its combination's line, and the overview's text uses the same line; verify cli.rs cases over the fakes after `says_tasks` and a step, with `status_lists_the_queue` unchanged.

## 5. sluice wait

- [x] 5.1 `wait_on`, the loop under a waiting submit and `wait`: reads first, runs a round, pauses, stops at a deadline; a waiting text command prints the progress line when it changes; exits `4` withdrawn and `5` still waiting; under `--json` one record however it ends; verify cli.rs cases for landed, blamed, conflict, withdrawn (wait and submit), a timeout with runs held, already decided with no round, Ctrl-C under `--json`, not found, and the progress line printed once per change.
- [x] 5.2 `sluice wait <n|branch> [--timeout] [--json]`: the branch form through the lookup `submit --status` uses, intake on a branch that takes pull requests, `Cli::json`, protection installed as for submit; help with its exits, and submit's exits with `4`; verify cli.rs for the branch form, `tests/usage.rs` for the help and `--timeout soon` exiting 64, and `tests/profiles/pull_requests.rs` for `wait <n>` landing through intake.

## 6. Docs and the plugin

- [x] 6.1 `reference/commands.md`: a `sluice wait` row and section, exits `4` and `5`, the progress line in the overview and `wt submit` sections; `reference/state.md`: `state` stable, `out.times`; `README.md`'s rows; verify `site:prose`.
- [x] 6.2 `guides/claude-code.md`, `guides/walkthrough.md` and `guides/pull-requests.md`: submit with `--no-wait`, then `sluice wait` in a loop on exit 5, keyed on `state`; verify `site:prose`.
- [x] 6.3 The using-sluice skill and its `submit.md` and `verdicts.md`, and the session note in `scripts/session-context.sh` with `tests/hooks.sh`; verify `plugin_docs.rs`, `plugins/sluice:validate` and `plugins/sluice:hooks`.

## 7. Gate

- [x] 7.1 Verify `sluice exec main` passes with everything committed.
- [x] 7.2 By hand, since it needs real moon and real time: in a scratch repository with a moon gate and this build, `submit --no-wait --json`, `wait <n> --timeout 20s --json` exits 5 with `state` `testing`, `sluice status main` shows the running task's time, and `wait <n>` exits 0 once it lands; keep the transcript here.

  Done 2026-10-21 with this build and moon 2.6.1, a quick task and one
  that sleeps 40 s:

  ```console
  $ sluice submit bolt/x --no-wait --json
  {"ahead":[],"branch":"bolt/x","id":1}
  $ sluice wait 1 --timeout 20s --json      # the record, "state": "testing"
  exit 5
  $ sluice status bolt/x
  bolt/x at de551b4 · 1 in the queue · testing 1 at once (up to 2)
    testing bolt/x + #1 · 1/2 tasks · running slow:test 22s · 22s of 10m
    #1  testing   live-a  unit/a   bolt/x + #1
  $ sluice wait 1 --timeout 5m
  testing bolt/x + #1 · 1/2 tasks · running slow:test 22s · 22s of 10m
  testing bolt/x + #1 · 2 tasks to run, 0 replayed
  passed · landed as bd8027e
  exit 0
  $ sluice wait 1 --json | jq -c '{id, state}'
  {"id":1,"state":"landed"}
  exit 0
  ```
