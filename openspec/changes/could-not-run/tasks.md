# Tasks

Read first: this change's proposal, design and specs; could-not-run-1 in
agentplot/blueprints; `docs/architecture/core.md`. Run suites through
`sluice exec main -- <cmd>`, the gate's checks with `sluice exec main`.

## 1. Reproduce

- [x] 1.1 Add red tests in `crates/sluice/tests/checks.rs`: a program that cannot start, an exit `not_run` names, readiness before and after a run. Add one in `tests/profiles/moon.rs`: a task exiting 77. Verify each fails today because the check is recorded as failed, or because `ready` is ignored.

## 2. Seams, core and fakes

- [ ] 2.1 `sluice-seams`: `NotRun`, `RunReport::not_run`, `RunOutcome::NotRun`, and `State::set_asides` and `State::set_aside`. Verify `cargo check --workspace --all-targets --all-features`.
- [ ] 2.2 `sluice-core`:
  - the poll forms a candidate again on not run, up to three runs in a row, and then fails it with each check that could not run and why;
  - `FailedTask::not_run` and `Event::Lost::not_run`;
  - `finish_session` gives `NotRun`;
  - `session_request` drops `ready` and `not_run` for a command run;
  - `results()` leaves out results set aside;
  - `policy.rs` accepts integers in a list.

  Verify the core's unit tests.
- [ ] 2.3 `sluice-fakes`: `cannot_run(task, reason)`, `not_ready(bool)` and set-asides kept in memory. Conformance tests in `tests/verdicts.rs` and `tests/scenarios.rs`:
  - a not-run check is recorded nowhere;
  - the next pass reads passed;
  - three runs in a row blame, with the reason;
  - a recorded fail beside a not-run check fails as before;
  - a refused start records nothing;
  - a session's not-run run;
  - a command run drops `ready`;
  - S43 and S44.

  Verify `sluice exec main -- cargo test -p sluice-conformance`.

## 3. Profiles and CLI

- [ ] 3.1 `checks.rs`: `ready` before a run (refuses) and after (fails and missing become not run, its answer kept in the run's directory); `not_run` exits; `Ended.error` gives not run. `moon.rs`: a task's `exitCode` from the report. Verify group 1's tests pass with `sluice exec main -- cargo test -p sluice --test checks -- --skip sandbox` and `sluice exec main -- cargo test -p sluice --features tools --test moon`.
- [ ] 3.2 `local_files.rs`: `set-aside.jsonl` under the records' lock. Verify a unit test.
- [ ] 3.3 CLI:
  - `sluice set-aside`;
  - `sluice exec` prints each check that could not run, and exits 1;
  - `status` and `submit` name a check that could not run, and why.

  Verify with cli.rs cases over the fakes and `tests/exec.rs`.

## 4. Plugin, policy and inputs

- [ ] 4.1 `plugins/sluice/scripts/guard.sh` refuses `sluice set-aside`. Verify a case in `plugins/sluice/tests/hooks.sh` (`plugin-sluice:hooks`).
- [ ] 4.2 `.config/sluice.toml` gets `ready = ["docker", "info"]` under `[tests]`. `cli:e2e` and `root:screenshots-scene` get explicit inputs. Verify `moon query tasks` shows the inputs, and `sluice exec main` passes.

## 5. Decisions, rules and docs

- [ ] 5.1 Add:
  - ADR 0031, a run that did not judge its check has no outcome;
  - core.7's Details "Not ruled out";
  - the site's results and policy pages;
  - `using-sluice` and `setting-up-sluice` references;
  - `CLAUDE.md` "Running checks".

  Verify Vale reports nothing and `openspec validate could-not-run --strict`.

## 6. Land

- [ ] 6.1 Commit, run `sluice exec main` from the worktree, read every failure to its cause, then `sluice submit main`.
