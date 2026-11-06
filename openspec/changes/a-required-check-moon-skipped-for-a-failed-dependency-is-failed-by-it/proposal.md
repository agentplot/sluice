## Why

On 2026-11-05, under sluice 0.66.0, switchboard-kit's submission #1305
(bolt every-class-difference-is-a-declared-input) failed. Its record
names one failed check, `root:integration`, with no `hash` and no
`not_run`. That check is a moon task whose command is `noop` and whose
dependencies are the integration areas. One of them,
`tests/integration:api`, ran and failed two tests. moon then skipped
`root:integration`: its output says "root:integration (skipped)" and
"221 completed, 1 failed, 2 skipped".

sluice's moon runner reads only the required targets from moon's run
report. It passes over an action with no hash, and any status but
passed, failed or cached (`crates/sluice/src/profiles/moon.rs`,
`read_report`). So `root:integration` is named as left without a
result, the reading `FailedTask`'s doc gives an empty `hash` ("Empty for
a required check the run left without a result (15)"). The task that
failed, its key and its log appear nowhere in the record. The log the
record names for `root:integration` is the tail of the whole run's
output, since moon kept none for a task it never ran.

Its readers believe the record. crew's `landing_blame` reads a failed
check with no `hash` as no outcome of the bolt's. So it records a host
stop: no take-main and no fix start. That is the fourth such misfiling,
after #1176, #1252 and #1267. sluice's own words call the check "not
run", and its report calls it missing. The task the culprit's session
has to fix is named nowhere, and no flake of it can be weighed under
its key (19).

The statement has no word for this case. A check ends in a pass, a
fail, or no outcome (60, 114), and a check left without a result is no
pass (15), but nothing says what a required check is when its runner
skipped it for a task it depends on.

## What Changes

- **A required check moon skipped because a task it depends on ran and
  failed is failed by that task.** sluice reads each required target's
  dependencies, at any depth, from moon's task graph, which the plan
  already queries. A target with no result of its own whose
  dependencies include a task that ran and failed is failed by that
  task (149, proposed).
- **The task's run is recorded under its own key.** The failed
  dependency's result is recorded as any check's run is: `ran` in the
  log, a line in the results under its task and hash, with the run's
  executor and system, signed with the run where the records are on
  the remote (16; rule core.7). It is weighed for a flake under its own
  key, with its own grant once the grant is recorded (19, 138; rule
  core.34). A dependency's pass is never recorded, since no reading is
  taken from it.
- **The record names both.** The submission's `failed` lists the
  dependency as a check that ran and failed, with its `hash`, its `log`
  and, where its key holds both, its `flaky`. The required check is
  listed with `failed_by` naming the dependency, and no hash or log of
  its own, since it never ran.
- **A dependency that could not run leaves the check with no
  outcome.** Where the only failed dependencies exited as the policy's
  `not_run` names, the required check is not run, "its dependency
  `<task>` could not run: `<why>`". Nobody is blamed, and the
  combination is tested again (114, 115). A run after which the
  container engine has gone leaves no outcome to the dependency or to
  the check (84). A required check moon skipped for any other reason is
  still missing (15).
- **Every reader of a run reads it.** A combination's verdict, a
  session's `sluice exec`, its recorded results and its last result,
  and a preview read a check failed by a dependency as failed. The
  check seam's run report carries `dependencies` and `failed_by`, and
  one `RunReport::failed` reads them with what failed on its own.
- **The words.** `sluice wait`, `sluice status` and `sluice exec` say
  `failed: <dependency>` and `<check> failed by <dependency>, a task it
  depends on`, and give the dependency's log. `--json` gives
  `failed_by`, and `sluice exec --json` gives `dependencies` and
  `failed_by`. A submission's report, and the pull request check drawn
  from it, list the dependency failed with its key, and the check
  failed by it.
- **The statement.** Clause 149, the vocabulary's "check", clause 16
  and C.2's moon row are amended, with S74. They are committed on the
  agentplot/blueprints branch
  `design/sluice-a-required-check-skipped-for-a-failed-dependency`, for
  the orchestrator to push on the user's word before the code lands
  (rule core.12). Record 0084 sets the decision down as proposed. The
  details of rules core.7 and core.34 say where it sits.

## Capabilities

### New Capabilities

- `failed-dependency`: what a required check is when its task runner
  skipped it for a task it depends on, how that task's run is recorded
  and weighed, and how a dependency that could not run differs from one
  that failed.

### Modified Capabilities

- `verdict-report`: a required check's results gain "failed by a task
  it depends on", naming it, and the dependency is listed with its key.
- `host-requirements`: an engine gone after a run leaves a check failed
  by a dependency, and the dependency, with no result.

## Out of scope

- **crew's words.** crew's decision needs no change: it reads the
  dependency's `hash` and gives the bolt its fix (design, "What crew
  reads"). Its `verdict_of` would call the skipped check "not run",
  since it has no hash. Reading `failed_by` there is crew's change, to
  make in afterthought/crew when this lands, and it decides nothing.
- **Flume's and Boomdrive's readings.** Flume's sluice profile names
  every failed row's task in a failed submission's words, so it says
  both tasks, which decides nothing. Boomdrive's runner carries each
  failed row to the hosted control plane with its task, hash, flaky and
  system alone, so the skipped check reaches it with no hash and no
  `failed_by`, read as not run. No decision changes. Carrying
  `failed_by` is Boomdrive's change to make when this lands, for the
  orchestrator to give it (design, "What crew reads").
- **A required check moon skipped for any other reason.** It stays
  missing and no pass (15), as today, and blames as today. Whether a
  skip that is the host's should blame nobody waits on a change that
  finds such a skip in moon's report.
- **A run its time limit ended.** It reads as 66 has it, with nothing
  read through a dependency, since a dependency the limit ended
  reports a fail that is no fail.
- **The executor's refusal (139).** The open change
  `a-checks-key-holds-its-executor` reads it. When it lands after this
  one, its reading treats `dependencies` and `failed_by` as checks that
  did not pass, as this change's engine reading does. When it lands
  first, this change's group 4 does so.
- **A dependency's passes.** They are never recorded, so a dependency
  that is no required check holds only fails under its key, and a
  flake of it is read as one only where a pass of it is recorded, as a
  required check of some branch. Recording every task a run runs waits
  on a change that weighs what that costs the records.
- **The command runner.** It runs one command, with no dependencies,
  and gives neither list.

## Impact

- `crates/sluice-seams/src/lib.rs`: `FailedBy`; `RunReport::dependencies`,
  `RunReport::failed_by` and `RunReport::failed`; `Row::failed_by`.
- `crates/sluice/src/profiles/moon.rs`: each task's dependencies from
  `moon query tasks`; `Plan`'s dependency map; `read_report` reading a
  skipped target through its dependencies; the module's doc comment.
- `crates/sluice/src/profiles/checks.rs`: `Plan::deps`, `Meta::deps`,
  `progress` carrying the two lists.
- `crates/sluice-core/src/records.rs`: `FailedTask::failed_by`.
- `crates/sluice-core/src/verdict.rs`: a dependency's key read as a
  check's, a check failed by it, one left unjudged by a key that reads
  passed.
- `crates/sluice-core/src/step.rs`: `finish_run` recording dependencies
  and asking their logs; `after_run`.
- `crates/sluice-core/src/session.rs`, `crates/sluice-core/src/preview.rs`,
  `crates/sluice/src/exec.rs`: `RunReport::failed`; a session's run
  recording a dependency; `sluice exec`'s words and document.
- `crates/sluice-core/src/verdict_view.rs`,
  `crates/sluice/src/profiles/github/render.rs`: the rows.
- `crates/sluice/src/lib.rs`, `crates/sluice/src/words.rs`: a failed
  submission's words.
- `crates/sluice-fakes/src/lib.rs`: `FakeTests::depends_on`.
- Every `RunReport` built: `crates/sluice-core/src/step.rs`
  (`unstarted`), `crates/sluice-core/src/verdict.rs`'s tests,
  `crates/sluice-conformance/tests/verdicts.rs`,
  `crates/sluice/tests/checks.rs`, `crates/sluice/tests/profiles/moon.rs`.
- Tests: `crates/sluice/src/profiles/moon.rs`'s `tests` module, whose
  fixtures are moon run reports; `crates/sluice/tests/profiles/moon.rs`;
  `crates/sluice-core/src/verdict.rs`'s tests;
  `crates/sluice-conformance/tests/failed_dependency.rs` (new),
  `scenarios.rs`, `reports.rs`, `engine.rs`; `crates/sluice/tests/cli.rs`.
- Docs: `site/src/content/docs/reference/state.md`,
  `site/src/content/docs/reference/commands.md`,
  `plugins/sluice/skills/using-sluice/references/verdicts.md`,
  `plugins/sluice/skills/using-sluice/references/exec.md`,
  `plugins/sluice/skills/setting-up-sluice/references/profiles/moon.md`.
- Records and rules: record 0084; the details of core.7 and core.34,
  which the site's rules page does not carry, so it is unchanged.
- Sources: clauses 15, 16, 17, 18, 19, 20, 60, 66, 84, 108, 114, 115,
  138, 139, C.2's moon row; 149 and S74 as proposed; records 0031, 0043,
  0044, 0072, 0074, 0084; rules core.7, core.8, core.9, core.22, core.34,
  core.35.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice/src/profiles/moon.rs
crates/sluice/src/profiles/checks.rs
crates/sluice/src/profiles/github/render.rs
crates/sluice/src/exec.rs
crates/sluice/src/lib.rs
crates/sluice/src/words.rs
crates/sluice-core/src/records.rs
crates/sluice-core/src/verdict.rs
crates/sluice-core/src/verdict_view.rs
crates/sluice-core/src/step.rs
crates/sluice-core/src/session.rs
crates/sluice-core/src/preview.rs
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/tests/**
crates/sluice/tests/profiles/moon.rs
crates/sluice/tests/checks.rs
crates/sluice/tests/cli.rs
site/src/content/docs/reference/state.md
site/src/content/docs/reference/commands.md
plugins/sluice/skills/using-sluice/references/verdicts.md
plugins/sluice/skills/using-sluice/references/exec.md
plugins/sluice/skills/setting-up-sluice/references/profiles/moon.md
docs/adr/0084-a-required-check-its-runner-skipped-for-a-task-it-depends-on-that-failed-is-failed-by-that-task.md
docs/architecture/core.md
openspec/changes/a-required-check-moon-skipped-for-a-failed-dependency-is-failed-by-it/**
```
