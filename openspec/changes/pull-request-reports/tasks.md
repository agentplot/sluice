# Tasks

Read first: this change's proposal, specs and design; ADRs 0021, 0022,
0028, 0031 and 0032; `docs/architecture/core.md`; pull-requests-1 and
many-hosts-3 in agentplot/blueprints. Work inside `devenv shell`; run
suites through `sluice exec main -- <command>`.

## 1. Decisions and the statement

- [x] 1.1 Check by hand, on moon 2.6 in a scratch workspace, whether `moon exec --ci <t>` skips a `<t>` marked `runInCI: false` as `moon ci <t>` does; write the answer into design D4 and pick `exec --ci` or `ci` (`exec --ci`: `ci` refuses `--on-failure`); verify by the two commands' output, recorded in design.md.
- [ ] 1.2 Draft clauses 118 and 119, scenarios S50 and S51, and the C.2 and C.5 edits into `design/sluice/proposals/pull-requests-1.md` on a blueprints branch `pull-request-reports`; verify by showing it with `plannotator annotate`.
- [x] 1.3 Write ADR 0039, "A request shows its verdict from the records, the same for every task runner", and its `adrs.toml` entry; verify `openspec validate pull-request-reports --strict` and the ADR check task pass.

## 2. A runner's detail, recorded

- [x] 2.1 Add `TaskResult.took`/`attempts`, `RunReport.detail` (`RunDetail`) and `RunReport.kept_out` to `sluice-seams`; fill them as `None` or empty in the fakes; verify `cargo check --workspace`.
- [x] 2.2 Record `took_ms`/`attempts` on `ResultLine` and a `Detail` log line per run in `step.rs` `finish_run`; verify a core unit test that a run's lines carry them and that old lines still parse.
- [x] 2.3 Add a test that `verdict::read` gives the same verdict for results with and without detail, two attempts included; verify it passes.
- [x] 2.4 Carry `took_ms`/`attempts` in the signed statement's per-result fields; verify a signing round-trip test with and without them.

## 3. moon

- [x] 3.1 Run moon in CI mode as 1.1 decided; verify a moon e2e case (tools feature) in which a required task marked `runInCI: false` is not run and reads missing.
- [x] 3.2 Read `runInCI` from `moon query tasks` in `plan`, return the required targets kept out as `kept_out`, log them once per run, and name them in `sluice status` and a waiting `submit`; verify the conformance case `a_required_check_kept_out_of_ci_is_missing_and_named` (the fakes keep a task out) and the moon case of 3.1.
- [x] 3.3 Read each task's `duration` and `task-execution` count, and the run's duration, `comparisonEstimate` and `changedFiles`, in `read_report`; verify unit tests against a trimmed moon 2.6 `runReport.json` fixture from a real run.
- [x] 3.4 Set `took` on the command runner's results from the run's start and end; verify a checks unit test.

## 4. The report in the core

- [x] 4.1 Add `VerdictView` and its row and standing types beside `verdict_view` in the core, as the overview's views are; verify `cargo check --workspace`.
- [x] 4.2 Write a pure `verdict_view` in `sluice-core/src/verdict_view.rs` and `Sluice::verdict_of` in `ops.rs`, as design D1 and D2 say; verify unit tests for each verdict-report scenario: ran and replayed, two systems, could not run, flaky, set aside, waiting ahead, landed together, behind a blamed submission, read after the run's files are gone, a landed submission found among the finished (unit tests in `verdict_view.rs`, flows in `sluice-conformance/tests/reports.rs`).

## 5. Shared text and scrubbing

- [x] 5.1 Move `Scrub` and `end` from `telemetry.rs` to `crates/sluice/src/scrub.rs`; verify the telemetry tests unchanged and a scrub unit test.
- [x] 5.2 Move the flaky, could-not-run, set-aside and nothing-run wording out of `lib.rs` into `crates/sluice/src/words.rs`; verify the cli.rs cases of a waiting `submit` unchanged.
- [x] 5.3 Add `pull_requests.log_lines` to the policy's parser; verify a policy unit test with and without it, and a refusal of zero.

## 6. The pull request

- [x] 6.1 Add `Posting.text`, comment ids and `Forge::edit_comment` to `forge.rs`, `rest.rs` and `fake.rs`; verify the fake API in `tests/profiles/forge/github.rs` sees `output.text` and a PATCH of a comment.
- [x] 6.2 Write `github/render.rs`: the head line, the combination line, the totals, the rows in order, 20 then collapsed, the executor, changed files and output sections, one section per system, the size cut; verify snapshot unit tests for blamed, landed, testing, formed again, could not run, flaky, many checks and an oversized report.
- [ ] 6.3 Post the report comment in place and the `sluice` check's text from `verdict_of`, live rows from the overview while testing, posting only on change; link the blamed comment to the report; verify `tests/profiles/pull_requests.rs` cases for edited-not-repeated, queued-only, no repeated posts, landed and blamed.
- [ ] 6.4 Post `sluice / <check> (<system>)`, or `sluice / <check>`, per required check per system, read back before posting; verify `tests/profiles/pull_requests.rs` cases for a build on three systems, a result read by key and no systems.
- [ ] 6.5 Behind a blamed pull request, its checks go back to queued and its report shows no rows; verify the two-pull-request case for S50: #12's comment and checks name neither #10 nor `app:test`.
- [ ] 6.6 Post output tails only with `log_lines`, scrubbed; verify the not-asked and asked-with-a-secret cases.
- [ ] 6.7 Have `sluice forge check` require Checks and Pull requests write; verify `tests/profiles/forge.rs` for the no-permission-to-post-checks scenario.

## 7. Docs

- [ ] 7.1 Update `site/src/content/docs/guides/pull-requests.md` "What sluice posts" with the comment, the check's text, the checks per system and `log_lines`, plus "never require sluice's checks in a ruleset"; update `guides/github-app.md` for the permissions, and `reference/configuration.md` for `log_lines`; verify `site:prose` reports nothing and the site builds.
- [ ] 7.2 Add a "sluice and moon's CI" section to the moon guide or the executors guide, covering CI mode, `runInCI`, base and head, and why moon's own report is not posted; update `plugins/sluice/skills/using-sluice/references/verdicts.md`; verify `site:prose` and `plugin-sluice:validate`.

## 8. Integration

- [ ] 8.1 Run `sluice exec main` from this worktree, checking `docker info` and `sysctl vm.loadavg` first; verify every check passes.
- [ ] 8.2 After landing, open a pull request on sluice's own `main` and read its report comment and checks while testing and once landed; run `sluice forge check main`; record what was seen in the pull request's description. This is a by-hand check, because the gate cannot reach GitHub.

## Workflow follow-up

- Land with `sluice submit main` from this worktree.
- Archive `pull-request-reports` after 8.2.
- Bring the operator's ruling on 118 and 119 into `requirements.md` once given.
