---
number: 84
title: A required check its runner skipped for a task it depends on that failed is failed by that task
status: accepted
date: 2026-11-05
decision-makers:
- Chuck Swanberg
---

# A required check its runner skipped for a task it depends on that failed is failed by that task

Status: accepted by the user on 2026-11-06, in the words "go ahead into code", given on agentplot-planner's card at 22:42Z on the design of the change `a-required-check-moon-skipped-for-a-failed-dependency-is-failed-by-it`, this record and the statement's amendment, now on boomdrivedev/blueprints main at a4eeec3 (the vocabulary's "check", 16, 149, C.2's moon row, S74). Proposed on 2026-11-05 by that change's construct stage, on the brief the orchestrator gave it, which asks that such a check read "failed, by that dependency", with "which dependency, its key and its log" in the record, "never 'left without a result' where moon's report shows a dependency that ran and failed", and its failure "weighed under that dependency's key". The choices within it are the construct stage's own: how sluice finds a check's dependencies, which of them are recorded, how a key that reads passed is read, and the record's field. Each is cheap to change until a release carries it.

Deciders: the user, accepting on 2026-11-06; the construct stage of the change named above, proposing on the orchestrator's brief.
Sources: `boomdrivedev/blueprints:design/sluice/requirements.md` clauses 15, 16, 17, 18, 19, 20, 60, 66, 84, 108, 114, 115, 138, 139, C.2's moon row, as main words them at cd8fd73, and the vocabulary's "check", 16, 149, C.2's moon row and S74 as boomdrivedev/blueprints main words them at a4eeec3; records 0031, 0043, 0044, 0072, 0074; rules core.7, core.8, core.9, core.22, core.34, core.35; on sluice's main at 3f5aed5f, `crates/sluice/src/profiles/moon.rs` (`plan`, `read_report`), `crates/sluice/src/profiles/checks.rs` (`progress`, `logs`), `crates/sluice-core/src/verdict.rs` (`read`), `crates/sluice-core/src/step.rs` (`finish_run`, `after_run`) and `crates/sluice-core/src/records.rs` (`FailedTask`); afterthought/crew at f3c5abd4, `plugin/lib/plan.py` `landing_blame` and `verdict_of`; switchboard-kit's submission #1305 under sluice 0.66.0, 2026-11-05, and the misfilings #1176, #1252 and #1267 before it.

## Context and problem statement

switchboard-kit's main requires `root:integration`, a moon task whose command is `noop` and whose dependencies are the integration areas. On 2026-11-05 a combination's run failed two tests of one of them, `tests/integration:api`. moon went on with the rest of the run, as sluice runs it (`--on-failure continue`), and skipped `root:integration`: "221 completed, 1 failed, 2 skipped".

sluice reads only the required targets of moon's run report. `tests/integration:api` is no target, so its fail was read nowhere. `root:integration` had no hash and no result, so sluice named it left without a result: a `FailedTask` with an empty `hash`, read as no pass (15). The candidate failed, and #1305, its newest entry, was blamed. But the record named no task that failed, no key and no log of one, and gave `root:integration` the tail of the whole run's output as its log.

Every reader believed the record. sluice's own words called `root:integration` "not run". crew reads a failed check with no `hash` as nothing of the bolt's, so it recorded a host stop, with no take of main and no fix started. That was the fourth time, after #1176, #1252 and #1267. The culprit's session was told nothing it could fix, and a flake of `tests/integration:api` could not be weighed, since none of its results was recorded under its key.

The statement has no word for the case. A check ends in a pass, a fail, or no outcome (60, 114). A required check left without a result is no pass (15). Nothing says what a required check is when its runner skipped it for a task it depends on.

What is a required check its runner skipped because a task it depends on ran and failed, and what does the record hold of that task?

## Considered options

1. Leave it missing, and name the failed task beside it in the record's words alone.
2. The check is failed by that task. The task's run is recorded under its own key and listed in the record as a check that ran and failed, and the check names it.
3. The check's fail is recorded under the check's own key, read from moon's hash of it where moon gives one.
4. As option 2, and every task the run runs is recorded under its key, its passes as its fails.

## Decision outcome

Option 2.

**What the check is.** Where a task a required check depends on, at any depth of the runner's own graph, ran and failed, and the runner skipped the check for it, the check is failed by that task (149). It is never read as left without a result. A task whose failure the runner allows, or which the runner runs after the check to clean up, is no cause of the skip and fails no check.

**How sluice knows.** moon's run report names no edge between its actions. The plan already reads every task with `moon query tasks`, and keeps each target's dependencies, at any depth, its `cleanup` ones left out, for the run's reading (C.2's moon row).

**What is recorded.** The task's run is recorded under the task's own key before the gate's result is read, as any check's run is: `ran` in the log, and an entry in the results with its task, hash, outcome, executor, branch, combination and system, signed with the run where the branch's state is on its remote (16; rule core.7). Only a task that failed a required check is recorded so. Its passes are recorded only where it is itself a required check of some branch.

**How it is read.** The task's key is read as a required check's key is: across the trusted results under it, held to the run's limits, under the run's grant once records carry it, on the run's system, and through its signer (19, 28, 69, 83, 138; rules core.22, core.34). Where it reads failed, flaky read as failed among them, the check is failed by it. Where every task the check was failed by reads passed, as `flaky = "pass"` reads a key holding a pass and a fail, nothing judged the check: it has no result, and the combination is tested again (114, 115). The check's own key is never read or written for it, since no run gave it one (38).

**What is told.** The submission's `failed` lists the task as a failed check, with its `hash`, its `log` and its `flaky`, and the check with `failed_by` naming the task and no hash or log of its own. The culprit's session is told the task, its key and what it printed (20). sluice's words say `failed: <task>` and `<check> failed by <task>, a task it depends on`.

**What it is not.** A required check skipped because a task it depends on exited as the policy's `not_run` names has no outcome, for that task's cause (114), and nobody is blamed. One skipped in a run whose executor refused it something (139), or after which a requirement the policy asks has left the host (84), has none either, and neither has the task. One skipped for any other reason is left without a result and is no pass (15), as today. A run its time limit ended reads as 66 has it.

Option 1 keeps a record whose top level names a failed check with no key, which is what crew and sluice's own words read as "not run", so every reader would still need to learn a second place to look. It also leaves the task's fail unrecorded, so no flake of it is ever weighed. Option 3 reads under a key no run gave: moon hashes a task only when it runs it, so a skipped one has no hash, and a hash taken from elsewhere would make a key nothing observed (38). Option 4 records every one of the 221 tasks a switchboard-kit run runs, about 200 more lines a run in results every verdict reads whole, and as many names in every signed statement. A cheaper middle, recording a task's later results only once its key holds a fail, would make whether a pass is recorded turn on which run came first, which 19 rules out.

### Consequences

- crew's decision is right with no change of crew's: a failed submission's `failed` holds the task with its `hash`, so `landing_blame` reads the bolt's, and its team's fix answers it. crew's `verdict_of` still calls the check "not run", since it has no hash. Reading `failed_by` there is a change of crew's words alone.
- A task that is no required check holds only fails under its key, so a flake of it reads as a fail under either `flaky` setting. A branch that wants it weighed requires it.
- The check seam's run report gains the failed tasks' results and the checks they failed. Every reader of a run, the gate, `sluice exec`, a session's last result and a preview, reads such a check as failed.
- The record gains `failed_by` under `sluice/submission@1`, and a record without it reads as before.
- Rule core.7's details rule out the check read as left without a result and the task's run left unrecorded. Rule core.34's rule out the failure weighed under the check's key.
- The change `a-check-its-executor-refused-has-no-outcome` reads the executor's refusal (139) over the failed tasks and the checks they failed, as this change's reading of a gone engine does.
