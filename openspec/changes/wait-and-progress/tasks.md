# Tasks

Read first: this change's proposal, specs and design; ADRs 0011, 0023,
0028, 0041 and 0042; `docs/architecture/core.md`. Run suites through
`sluice exec main -- <command>`; before submitting, `sluice exec main`.

## 1. Decisions

- [ ] 1.1 Write ADRs 0041 and 0042; name `sluice wait` and a waiting `sluice submit` in core.4's Details and ADR 0042 in core.7's, in `docs/architecture/core.md` and `site/src/content/docs/architecture/rules.md`; verify `openspec validate wait-and-progress --strict` (by hand: no gate task runs openspec) and `site:prose`.

## 2. The core

- [ ] 2.1 Move the overview's per-branch read into `Sluice::branch_view_of` and build `Sluice::overview` from it; verify the overview tests over the fakes, and a test that one branch's view is the overview's entry for it.

## 3. A running task's start

- [ ] 3.1 The supervisor notes `out.log`'s length and the time in `out.times` at most once a second while it grows and once at the end; `Runs::times` reads them; the run directory's layout in `runs.rs` names the file; verify a `tests/runs.rs` case where a command prints, sleeps and prints again and leaves two notes.
- [ ] 3.2 `checkpoints()` keeps where each start checkpoint ends; `Moon::progress` gives a running task the time of the first note reaching it; `Checks::tasks` passes the notes; verify unit tests in `moon.rs` for a running task with notes, without notes, and an ended task unchanged.
- [ ] 3.3 Extend `a_run_shows_each_task_as_it_ends` in `tests/profiles/moon.rs`: the running task has a `started`; verify `cargo test -p sluice --features tools --test moon`.

## 4. Progress in the text views

- [ ] 4.1 One formatter for the progress line and the durations; verify unit tests with fixed times: a count with running tasks, more than three running, a running task with no start, no tasks, no limit.
- [ ] 4.2 `status <branch>` prints a line per combination under test, `status <n>` its combination's line, and the overview's text uses the same line; verify cli.rs cases over the fakes after `says_tasks` and a step, with `status_lists_the_queue` unchanged.

## 5. sluice wait

- [ ] 5.1 `wait_on`, the loop under a waiting submit and `wait`: reads first, runs a round, pauses, stops at a deadline; a waiting text command prints the progress line when it changes; exits `4` withdrawn and `5` still waiting; under `--json` one record however it ends; verify cli.rs cases for landed, blamed, conflict, withdrawn (wait and submit), a timeout with runs held, already decided with no round, Ctrl-C under `--json`, not found, and the progress line printed once per change.
- [ ] 5.2 `sluice wait <n|branch> [--timeout] [--json]`: the branch form through the lookup `submit --status` uses, intake on a branch that takes pull requests, `Cli::json`, protection installed as for submit; help with its exits, and submit's exits with `4`; verify cli.rs for the branch form, `tests/usage.rs` for the help and `--timeout soon` exiting 64, and `tests/profiles/pull_requests.rs` for `wait <n>` landing through intake.

## 6. Docs and the plugin

- [ ] 6.1 `reference/commands.md`: a `sluice wait` row and section, exits `4` and `5`, the progress line in the overview and `wt submit` sections; `reference/state.md`: `state` stable, `out.times`; `README.md`'s rows; verify `site:prose`.
- [ ] 6.2 `guides/claude-code.md`, `guides/walkthrough.md` and `guides/pull-requests.md`: submit with `--no-wait`, then `sluice wait` in a loop on exit 5, keyed on `state`; verify `site:prose`.
- [ ] 6.3 The using-sluice skill and its `submit.md` and `verdicts.md`, and the session note in `scripts/session-context.sh` with `tests/hooks.sh`; verify `plugin_docs.rs`, `plugins/sluice:validate` and `plugins/sluice:hooks`.

## 7. Gate

- [ ] 7.1 Verify `sluice exec main` passes with everything committed.
- [ ] 7.2 By hand, since it needs real moon and real time: in a scratch repository with a moon gate and this build, `submit --no-wait --json`, `wait <n> --timeout 20s --json` exits 5 with `state` `testing`, `sluice status main` shows the running task's time, and `wait <n>` exits 0 once it lands; keep the transcript here.
