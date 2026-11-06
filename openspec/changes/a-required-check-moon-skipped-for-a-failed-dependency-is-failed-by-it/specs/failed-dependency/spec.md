## ADDED Requirements

### Requirement: A required check skipped for a task it depends on that failed is failed by that task

When the task runner skipped a required check because a task the check
depends on, at any depth of the runner's own graph, ran and failed,
sluice SHALL read the check as failed by that task, and SHALL NOT read
it as left without a result (149).

#### Scenario: A dependency two steps down

- **WHEN** `root:all` depends on `api:build`, which depends on `api:test`, `api:test` fails and moon skips both
- **THEN** `root:all` is failed by `api:test`, and `api:build`, which is no required check, is named nowhere

#### Scenario: Two dependencies failed

- **WHEN** `root:integration` depends on `api:integration` and `ui:integration`, and both fail
- **THEN** `root:integration` is failed by both, and each is listed once with its key

#### Scenario: A dependency that is itself required

- **WHEN** `main` requires `#check-fast`, `beta:test` depends on `alpha:test`, both carry the tag, and `alpha:test` fails
- **THEN** `alpha:test` is listed failed with its key once, and `beta:test` is failed by it

#### Scenario: The run report from moon

- **WHEN** moon 2.6 runs a workspace whose required `root:all` is a `noop` task depending on `alpha:test`, and `alpha:test` fails
- **THEN** the run's report names `root:all` failed by `alpha:test`, and gives `alpha:test`'s result with its 64-character hash and its log as moon kept it (`a_noop_check_is_failed_by_the_task_it_depends_on` in `crates/sluice/tests/profiles/moon.rs`)

### Requirement: The record names the task and the check it failed

The submission's record SHALL list a task a required check was failed
by as a failed check, with its key and its log, and the check with
`failed_by` naming the task, with no key or log of its own (149).

#### Scenario: A check that runs nothing of its own

- **WHEN** `main` requires `root:integration`, a task whose command is `noop` and which depends on `api:integration`, and a combination's run fails `api:integration`, so moon skips `root:integration`
- **THEN** the newest submission of the combination is blamed, and its record's `failed` lists `api:integration` with its `hash` and `log`, and `root:integration` with `failed_by` `["api:integration"]` and no `hash` (`a_check_skipped_for_its_dependency_is_failed_by_it` in `crates/sluice-conformance/tests/failed_dependency.rs`)

### Requirement: A failure the runner allows fails no check

A task whose failure the task runner allows, and a task the runner runs
after the check to clean up, SHALL NOT fail a check that depends on it.

#### Scenario: A failure the runner allows

- **WHEN** `root:integration` depends on `api:lint`, whose `allowFailure` is set, `api:lint` fails and moon skips `root:integration` for another reason
- **THEN** `root:integration` is not failed by `api:lint`, and reads missing

#### Scenario: A task that cleans up after the check

- **WHEN** `root:integration` names `api:stop` as a `cleanup` dependency, `api:stop` fails and moon skips `root:integration` for another reason
- **THEN** `root:integration` is not failed by `api:stop`, and reads missing

### Requirement: The task's run is recorded under its own key

sluice SHALL record the run of a task a required check is failed by
under the task's own key, before the verdict is read, as any check's run
is: `ran` in the branch's log and a line in the results with its task,
hash, outcome, executor, branch, combination and system. On a branch
whose records are on its remote, the line SHALL be signed with the run's
only where the task runner's own manifest bears its key out (16, 149;
rule core.7). sluice SHALL NOT record a dependency's pass.

#### Scenario: The fail in the results

- **WHEN** `root:integration` is failed by `api:integration` on combination `c41`
- **THEN** `results.jsonl` gains a fail of `api:integration` under its hash, on `c41`, with the run's executor and host, and no line for `root:integration`

#### Scenario: A pass of the dependency

- **WHEN** a later run passes `api:integration` and `root:integration`
- **THEN** `results.jsonl` gains a pass of `root:integration` and no line for `api:integration`

#### Scenario: A signed branch

- **WHEN** `main`'s records are on its remote, its policy names this host's signer, and `root:integration` is failed by `api:integration`
- **THEN** the run's signed record holds `api:integration`'s fail under its key, and the gate counts it through that signer

### Requirement: A failure through a dependency is weighed under the dependency's key

sluice SHALL read the key of a task a required check is failed by as it
reads a required check's key: across every recorded result under it the
gate trusts, held to the run's limits and under the run's grant, on the
run's system, as the branch's `flaky` reads a key with a pass and a fail
(19, 138; rule core.34). Where that key reads failed, the check is
failed by the task.

#### Scenario: A flaky dependency read as failed

- **WHEN** `api:integration`'s key holds a pass a trusted run recorded, `main` says `flaky = "fail"`, and a run fails it and skips `root:integration`
- **THEN** `api:integration` is listed flaky, naming the pass, and `root:integration` is failed by it

#### Scenario: A pass under other limits

- **WHEN** `api:integration`'s key holds a pass from a run with `docker = false`, and a run with `docker = true` fails it
- **THEN** the pass is not weighed, and `root:integration` is failed by `api:integration`

#### Scenario: Never under the skipped check's key

- **WHEN** `root:integration`'s key holds a pass, and a later run is failed by `api:integration`
- **THEN** no result is recorded or read under `root:integration`'s key, and `root:integration` does not read flaky

### Requirement: A check whose failed dependencies read passed has no result

Where every task a required check was failed by reads passed under its
key, sluice SHALL record no result for the check, list it as not run
with why, blame nobody, and test the combination again (114, 115).

#### Scenario: A flaky dependency read as passed

- **WHEN** `api:integration`'s key holds a pass a trusted run recorded, `main` says `flaky = "pass"`, and a run fails it and skips `root:integration`
- **THEN** `root:integration` is listed could not run, saying `api:integration` failed and its key reads passed, no submission is blamed, and the combination is tested again

### Requirement: A check whose dependency could not run has no result

When the task runner skipped a required check because a task it depends
on exited with a code the branch's `[tests] not_run` lists, and no task
it depends on ran and failed, sluice SHALL record no result for the
check or for the task, list the check as not run, saying which task
could not run and why, blame nobody, and test the combination again
(114, 115). A required check the runner skipped for any other reason
SHALL read as missing, as before (15).

#### Scenario: The dependency's exit is a not-run exit

- **WHEN** `[tests] not_run = [77]`, `api:integration` exits 77 and moon skips `root:integration`
- **THEN** `results.jsonl` gains no line for either, `root:integration` is listed could not run with "its dependency api:integration could not run: exited 77", and no submission is blamed (`a_check_whose_dependency_could_not_run_is_tested_again` in `crates/sluice-conformance/tests/failed_dependency.rs`)

#### Scenario: Three runs in a row

- **WHEN** `api:integration` exits 77 in three runs in a row of the same combination
- **THEN** the combination's newest submission is blamed with `root:integration` could not run, and why

#### Scenario: One failed and one could not run

- **WHEN** `root:integration` depends on `api:integration`, which fails, and `ui:integration`, which exits 77
- **THEN** `root:integration` is failed by `api:integration`

#### Scenario: Skipped for no dependency

- **WHEN** moon skips a required check and no task it depends on failed or could not run
- **THEN** the check reads missing, as before

#### Scenario: The time limit ended the run

- **WHEN** the time limit ended a run while `api:integration` ran, and moon reports it failed and `root:integration` skipped
- **THEN** the run reads as timed out, and nothing is read through `api:integration`

### Requirement: Every reader of a run reads a check failed by a dependency as failed

`sluice exec`, a session's last result, a preview and the gate SHALL
each read a run in which a required check was failed by a task it
depends on as one in which a check failed. `sluice exec` SHALL record
the task's fail under its key where it records the run's results, say
`failed: <task>` and `<check> failed by <task>, a task it depends on`,
and exit 1. Its `--json` document SHALL give `dependencies` and
`failed_by`, and SHALL NOT list the check in `missing`.

#### Scenario: A session's run

- **WHEN** an agent runs `sluice exec main` in a clean worktree at its head and `root:integration` is failed by `api:integration`
- **THEN** it prints `failed: api:integration` and `root:integration failed by api:integration, a task it depends on`, exits 1, records `api:integration`'s fail under its key, and the worktree's last result is failed

#### Scenario: The session's document

- **WHEN** the same run is `sluice exec main --json`
- **THEN** its document's `failed_by` is `[{"task": "root:integration", "by": ["api:integration"]}]`, `dependencies` gives `api:integration` with its hash, `"outcome": "fail"` and `"recorded": true`, and `missing` is empty

### Requirement: A submission failed by a dependency says which task to fix

`sluice wait` and `sluice status` for a submission blamed for a check
failed by a dependency SHALL name the dependency failed, say which check
it failed and that the check depends on it, and give the dependency's
log. They SHALL NOT name the check as not run. Their `--json` SHALL give
the record's `failed` as it is, `failed_by` included.

#### Scenario: The words of a wait

- **WHEN** submission 1305 is blamed with `root:integration` failed by `api:integration`
- **THEN** `sluice wait 1305` prints `failed: api:integration`, then `root:integration failed by api:integration, a task it depends on`, and `log of api:integration:` with where its log is kept, and no `not run:` line

#### Scenario: What a tool reads

- **WHEN** a tool reads `sluice wait 1305 --json`
- **THEN** `failed` holds an entry for `api:integration` with a non-empty `hash`, so the record names a check that ran and failed, and an entry for `root:integration` with `failed_by`

### Requirement: An older record reads as before

A submission's record written before `failed_by` SHALL read as it did,
and a run started by an older sluice, whose run names no dependencies,
SHALL read a skipped required check as missing.

#### Scenario: A record with no failed_by

- **WHEN** `sluice status 1176` reads a record whose `failed` names `root:integration` with no `hash`, written by sluice 0.66.0
- **THEN** it lists `root:integration` as not run, as before
