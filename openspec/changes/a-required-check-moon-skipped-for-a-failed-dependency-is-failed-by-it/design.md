# Design

## Context

See proposal.md for why, and record 0084 for the decision and the
options weighed. The code reads as below at main 3f5aed5f, where this
change was cut.

- **The run.** `Moon::plan` (`crates/sluice/src/profiles/moon.rs`)
  queries every task with `moon query tasks`, narrows the required
  checks to what the combination affects, and runs `moon exec <targets>
  --ci --on-failure continue`. moon runs each target's dependencies
  first. When one fails, moon goes on with the rest and skips every
  task that depends on it. Its run report lists one action per task it
  ran or skipped, dependencies included, with each action's `status`,
  its `operations` (a `hash-generation` with the task's hash, each try's
  `task-execution` with its exit) and `allowFailure`. A skipped action
  has no hash, and the report names no edge of the graph.
- **The reading.** `read_report` (line 612) keeps only `RunTask(…)`
  actions of the targets. It passes over one with no `hash-generation`,
  and any status but `passed`, `failed`, `cached` and
  `cached-from-remote`. `Checks::progress` (`crates/sluice/src/profiles/checks.rs`,
  line 1641) then names every target with no result and not in
  `not_run` as `missing`, in a run its time limit did not end.
- **The verdict.** `verdict::read` (`crates/sluice-core/src/verdict.rs`)
  reads each of `report.tasks` under its key across every recorded
  result (19, 69). Then it gives every `missing` task a `FailedTask` with
  an empty `hash` (15), so the candidate fails and its newest entry is
  blamed (17). `finish_run` (`crates/sluice-core/src/step.rs`, line
  1464) records `report.tasks` only. It asks the check seam for the logs
  of `v.failed`, and `Checks::logs` gives a task moon kept no log for the
  tail of the whole run's output.
- **The record.** `FailedTask` (`crates/sluice-core/src/records.rs`,
  line 141) has `task`, `hash` ("Empty for a required check the run
  left without a result (15)"), `log`, `flaky`, `system` and `not_run`.
- **Who reads a run report.** `verdict::read`; `Step::after_run`
  (engine gone, 84); `session.rs` `passed` and `failed` (a session's
  last result); `exec.rs` `passed`, `summary` and `finish` (`sluice exec`
  and its `--json`); `preview.rs` (a preview's command); and every test
  that builds one.
- **Who reads the record.** sluice's `report` (`crates/sluice/src/lib.rs`,
  line 3019): a failed check with no hash is "not run". `verdict_view`'s
  rows: one with no result line is `Missing`. A pull request's check
  (`render.rs`). And crew's `landing_blame` (afterthought/crew,
  `plugin/lib/plan.py`, line 4279, at f3c5abd4): a failed submission is
  the bolt's when `result` is `timed out` or any failed check has a
  `hash`, and a host stop otherwise. Its `verdict_of` (line 4313) lists
  the checks with a hash as failed and those without as not run.

## Goals / Non-Goals

**Goals:**

- A required check moon skipped because a task it depends on ran and
  failed reads as failed by that task, everywhere the run or the record
  is read, and never as left without a result (149).
- The task's run recorded under its own key, its key and its log in the
  submission's record, so a reader sees a check that ran and failed (16,
  20; rule core.7).
- Its failure weighed under its own key: for a flake, under its grant,
  and through its signer (19, 138; rules core.22, core.34).
- A dependency that could not run told apart: no outcome, nobody
  blamed, tested again (114, 115).
- crew's decision right with no change of crew's.

**Non-Goals:**

- A required check moon skipped for any other reason, which stays
  missing (15).
- A run its time limit ended (66).
- The executor's refusal (139), which the open change
  `a-checks-key-holds-its-executor` reads.
- A dependency's passes, which are not recorded.
- crew's words.

## Decisions

### D1. Which tasks a required check depends on: moon's graph, read at the plan

The run report names no edges, so the plan keeps them. `Moon::tasks`
already reads `moon query tasks` in full. Each task there has `deps`, a
list whose items are a target or an object with `target` and `type`.
`Task` gains `deps: Vec<String>`, the targets of every item whose `type`
is not `cleanup`. moon runs a cleanup dependency after its task and
never skips the task for it.

`Plan` gains `deps: BTreeMap<String, Vec<String>>`: for each target, every
task it depends on at any depth, each once, sorted. The plan fills it
from the graph it read, before it decides whether anything runs.
`Meta` gains the same map, `serde(default)`, so the run's reading has
it after the plan. A run an older sluice started has none, and reads as
before.

The paths not taken:

- *The report alone, taking a failed action as the cause of every
  skipped target.* With two failures and two targets it names the wrong
  cause, and it names a cause for a target moon skipped for something
  else.
- *`moon query tasks` again after the run.* The place still holds the
  candidate, but it costs a second moon process per finished run, and
  the plan has already read the same graph.

### D2. What the report reads of a required check with no result

`read_report` takes the map. For each target whose action the report
names `skipped`, with no result of its own, neither passed, failed nor
cached, and not in `not_run`, it reads its dependencies' actions,
`RunTask(…)` and `RunPersistentTask(…)` alike, since a helper the check
waits on is a task it depends on:

1. **Failed by.** The dependencies whose action is `failed`, not
   `allowFailure`, with a hash, and whose last try's exit is not one
   `not_run` names. moon skips no dependent for a failure it allows. The
   target is failed by them: `FailedBy { task, by }`, `by` sorted. Each
   such dependency that is not itself a target gives its result to
   `dependencies`, read as a target's is: `Outcome::Fail`, `Source::Ran`,
   its hash, its time and its tries (119).
2. **Not run.** Otherwise, the dependencies whose last try's exit is
   one `not_run` names. The target is not run, with "its dependency
   `<task>` could not run: `<reason>`", the reason `not_run_reason`
   gives and the dependency named first where there are several (114).
   Its dependencies give nothing.
3. **Missing.** Otherwise, the target stays missing, as today (15).

A target the report does not name as skipped is read through nothing
and stays missing: moon skipped it for no task it depends on. One moon
keeps out of its CI mode is absent from the report (ADR 0044), and may
share a dependency that failed another target. The path not taken:
*every target with no result read through its dependencies*, which
reads such a target as failed by a task moon never skipped it for.

A failure is a failure whichever kind of dependency it is in, so one
that failed outweighs another that could not run. A target's failed
dependency is the task that ran: in a chain `root:all` → `api:build` →
`api:test` where `api:test` failed and moon skipped `api:build`, both
`root:all` and, where required, `api:build` are failed by `api:test`.
`dependencies` holds each task once, however many targets it failed.

In a run its time limit ended, nothing is read through a dependency. A
dependency the limit ended reports the exit it took on the signal,
which is no fail (66). `progress` names what the limit cut off, as
today.

`Moon::read` returns a `Read { tasks, not_run, dependencies, failed_by,
detail }` in place of its tuple.

### D3. The seam

`sluice-seams` gains:

- `FailedBy { task: String, by: Vec<String> }`, `Clone, Debug,
  PartialEq, Eq, Serialize, Deserialize`: "149: a required check the
  runner skipped because the tasks `by` it depends on ran and failed;
  its outcome is theirs."
- `RunReport::dependencies: Vec<TaskResult>`: "149, 16: the result of
  each task a check of `failed_by` was failed by that is no check of the
  run, recorded under its own key as any result is; never a replay,
  since a runner answers no failed task from its cache."
- `RunReport::failed_by: Vec<FailedBy>`, in neither `tasks`, `missing`
  nor `not_run`.
- `RunReport::failed(&self) -> bool`: a check failed on its own result,
  was failed by a dependency, or was left without a result (15, 149).
  `session.rs` `failed`, `exec.rs` `passed` and `preview.rs` read it, so
  no reader of a run counts a check failed by a dependency as passed.

`missing`'s doc comment becomes: "15: required checks the run was to run
that it left without a result, and no task they depend on failed or
could not run: skipped for another reason, never reached, or, in a run
its time limit ended, cut off by it (time-limit-1 66). None of them
passes."

`Checks::progress` puts the two lists in the report, and names as
missing only a target in none of `tasks`, `not_run` and `failed_by`.
The command runner gives neither list.

### D4. Recording: under the dependency's own key, a fail only

`finish_run` records `report.tasks` and then `report.dependencies` the
same way: `Event::Ran` in the log, and a `ResultLine::of` the result
with the run's executor, branch, combination and system (16; rule
core.7). On a branch whose records are on its remote, `Tests::proven`
is asked of both, so a dependency's line is signed only where moon's
manifest of its key bears it out, as a check's is today (C.6; rule
core.9). A dependency whose key the tree does not bear out is no pass,
as a check's is not. `record_session` records a session's `dependencies`
beside its `tasks`, under the same rule (92; rule core.18).

A dependency's line is a ran result like any other. Its grant is the
run's executor's, once `a-checks-key-holds-its-executor` records one
(138), so it is weighed only with results under the same grant (rule
core.34).

Only a dependency that failed a required check is recorded. The paths
not taken:

- *Every task of the run, passes included.* A run of switchboard-kit's
  checks runs 221 tasks. Every one of its runs would add some 200 lines to the
  results, which every verdict reads whole, and as many names to a
  signed run's statement. That is a cost to weigh in a change of its
  own (proposal, "Out of scope").
- *A dependency's later results only once its key holds a fail.*
  Whether a pass is recorded would then turn on which run came first.
  19 rules that out: which result came first decides nothing.
- *The skipped check's fail under its own key.* moon never hashed it,
  so it has no key in this run (38), and a fail under an empty or a
  borrowed hash would be read under a key no run gave it.

So a dependency that is no required check of any branch holds only
fails under its key, and reads failed under either `flaky` setting.
Where a branch requires it, its passes are recorded as that branch's
runs, and its key reads as 19 has it.

### D5. Reading: a dependency's key as a check's, and the check through it

`verdict::read` reads each of `report.dependencies` exactly as it reads
a check of `report.tasks`: its recorded results under its key, trusted,
held to the run's limits, on the run's system, through its signer where
the records are on the remote (19, 28, 69, 78, 83, 85). It counts toward
`ran`. A dependency that reads failed, or flaky read as failed, gets a
`FailedTask` with its hash and `flaky` as a check does. One whose signed
result the branch does not count is `missing` (87, S18).

Then each `FailedBy`:

- **Failed.** Where any of `by` is in `failed`, the check gets a
  `FailedTask { task, hash: "", failed_by: <those of by in failed>,
  system }`, with no log. The candidate is not passed.
- **Missing.** Otherwise, where any of `by` is `missing`, nothing more.
  The uncounted run is open again for a host whose results count, as
  today, and the check is run again with it.
- **Unjudged.** Otherwise every one of `by` read passed, its key
  holding a pass and a fail where the branch's `flaky = "pass"`. The
  check was never run, and nothing judged it. `Verdict` gains `unjudged:
  Vec<NotRun>`, the check with "its dependency `<task>` failed, and its
  key reads passed, as `flaky = "pass"` reads a key with a pass and a
  fail". `finish_run` reads it as it reads `report.not_run`: nobody is
  blamed and the combination is tested again, until 115's runs in a row
  read it not passed (114, 115).

`finish_run` asks `Tests::logs` for every failed task but those with
`failed_by`. The dependency's log is moon's own for it, kept as any
failed check's is (20, 119), and a check that never ran has none of its
own.

The path not taken: *the check's reading as its dependency's, with no
`FailedTask` of the dependency.* The record would then name a failed
check with no key at its top level, which is what crew and sluice's own
words read as "not run", and the dependency's `flaky` would have no
home.

### D6. The engine gone after a run

`Step::after_run` (84) counts a check failed by a dependency as one that
did not pass. Where the engine has gone after the run, each check of
`failed_by` joins `not_run` with the reason, and `dependencies` is
emptied. The dependency has no outcome and is recorded nowhere,
and a check that passed keeps its pass (114, 139). When
`a-checks-key-holds-its-executor` lands, its refusal reading does the
same with what the executor refused.

### D7. The record

`FailedTask` gains `failed_by: Vec<String>`, `serde(default,
skip_serializing_if = "Vec::is_empty")`: "149: the tasks this required
check depends on that ran and failed, which its runner skipped it for;
each is in the same list with its key. Empty for a check that failed,
or was left without a result, on its own." Its `hash` doc comment
becomes: "Empty for a required check no run gave a key: left without a
result (15), or failed by a task it depends on (149)."

#1305's record would read, its hash and paths shortened:

```json
"failed": [
  { "task": "tests/integration:api", "hash": "8c41…",
    "log": ".git/sluice/.logs/main/c41/tests%2Fintegration:api.log" },
  { "task": "root:integration", "failed_by": ["tests/integration:api"] }
]
```

A record an older sluice wrote reads as before, with no `failed_by`.
`sluice/submission@1` stays: the field is added, and no field changes
its meaning for a check that failed or was left without a result on
its own.

### D8. The words

`report` (`crates/sluice/src/lib.rs`) for a failed submission, as #1305's would read:

```text
failed: tests/integration:api
  root:integration failed by tests/integration:api, a task it depends on
  on main + #1305 (8f2c1d0)
  log of tests/integration:api: .git/sluice/.logs/main/c41/tests%2Fintegration:api.log
the agent working in every-class-difference-is-a-declared-input fixes it there and submits again
```

A check with `failed_by` is never in `not run:`. Its line comes after
`failed:`, through `words::failed_by(check, by)`, the dependencies
joined with `, ` and the last with ` and `.

`sluice exec`'s summary says `failed: <dependency>` with the checks that
failed on their own, and a line per check failed by one. Its `--json`
document gains `dependencies`, each `{task, hash, outcome, recorded}` as
`tasks` has them, and `failed_by`, each `{task, by}`. `missing` no
longer lists a check failed by a dependency, and `exit` is 1, as for a
check that failed.

A verdict's rows: `Row` gains `failed_by: Vec<String>`. A check named
in the record with `failed_by` and no result line is `RowResult::Failed`,
its key `None`. The dependency's row comes from its result line, as any
check's does, failed with its key and its log. A pull request's check
shows the check failed, its info "failed by `tests/integration:api`".

### D9. What crew reads

crew's `landing_blame` takes a failed submission as the bolt's when any
failed check has a `hash`. Its type check reads `hash` and `not_run` of
each entry, as strings where present, and nothing else. Once this
lands, #1305's record holds `tests/integration:api` with its hash, so
crew reads `("bolt", None)`: the bolt's team takes main in and starts
its fix, with no change of crew's. A record whose every
failed check is still missing or not run reads as a host stop, as
today.

crew's `verdict_of` lists the checks with a hash as failed and those
without as not run, so it says `failed: tests/integration:api; not run:
root:integration`. Those words call the skipped check "not run", which
it is not. Reading `failed_by` there is a change of crew's words alone,
for the orchestrator to give crew when this lands (proposal, "Out of
scope").

Two more of the family read the failed list, and neither decides
anything by it. Flume's sluice profile (`said`, in
`crates/flume/src/profiles/merge_queue/sluice/mod.rs`) names every
failed row's task in a failed submission's words, so it says both
`tests/integration:api` and `root:integration`. Boomdrive's runner
(`carried`, in `crates/boomdrive/src/sluice.rs`, its Decision 13)
carries each failed row to the hosted control plane with `task`,
`hash`, `flaky` and `system` alone, so `root:integration` reaches it
with no hash and no `failed_by`, read there as not run, while the
dependency reaches it with its hash. Carrying `failed_by` is
Boomdrive's change to make when this lands, for the orchestrator to
give it, as crew's words are (proposal, "Out of scope"). Timber, weir
and boom read no failed check of sluice's record.

### D10. The statement

The amendment is on agentplot/blueprints main at a4eeec3 (record
0084):

- **Vocabulary, "check":** "A check its runner skipped for a task it
  depends on that failed ends in that task's fail (149)."
- **16:** "So is the run of a task that failed a required check (149)."
- **149**, after 139: where a task a required check depends on, at any
  depth of the runner's own graph, ran and failed, and the runner
  skipped the check for it, that task fails the check, and the check is
  never read as left without a result. The task's run is recorded under
  its own key and read there alone, for a flake, under its grant and
  through its signer. Where every such task reads passed, nothing judged
  the check and it is run again. A task allowed to fail, or run after
  the check to clean up, fails no check. One that could not run leaves
  the check no outcome, and so do the executor's refusal and a
  requirement gone from the host. A check skipped for any other reason
  is missing. A task's passes are recorded only where it is itself a
  required check.
- **C.2's moon row:** the dependencies read from `moon query tasks` as
  the run is planned, `cleanup` ones left out, since the run report
  names no edge, and a skipped required task read through them (149).
- **S74**, the scenario.

15, 17, 18 and 60 are unchanged. 15 reads a verdict over the checks the
candidate affects, and a check failed by a dependency is one of them,
failed. 17 and 18 attribute it as any failure, after every candidate
ahead has a verdict. 60 is a run whose executor stopped before
reporting, and this run reported.

### Tests, and where each scenario is held

The moon runner's reading is tested on moon run reports as fixtures, in
`moon.rs`'s `tests` module, as `read_report` is today, with the graph
given as `moon query tasks` prints it. The tools tests in
`crates/sluice/tests/profiles/moon.rs` run moon 2.6 itself on a scratch
workspace, a `noop` task depending on a failing one, so the shape of
both documents is moon's own. The core's reading is tested over the
fakes, in the conformance suite, with a new `FakeTests::depends_on(task,
dependency)`: the dependency runs as the task's own does, under its own
hash and fail rules, and where it fails, or cannot run, the task is
skipped and reported as moon's runner reports it. Every test works in
its own temporary folder, never the host's state.

## Risks / Trade-offs

- **[A dependency's flake reads as a fail]** A dependency that is no
  required check holds no pass under its key, so `flaky = "pass"` never
  passes it. → Today such a run blames the same entry with no key at
  all. A branch that wants the flake weighed requires the task.
- **[moon changes its query's shape]** `deps` items are a target or an
  object with `target` and `type` in moon 2.6. → The tools test reads
  moon's own output. An item of another shape gives no dependency, so
  the check stays missing, as today.
- **[A larger record]** Each failed dependency adds a line to the
  results and an entry to the record. → Only one that failed a required
  check, once per run.
- **[A run started before the upgrade]** Its meta has no map. → It reads
  as before, missing, for that run alone.

## Migration Plan

Nothing to migrate. Records gain `failed_by` and results gain
dependencies' lines as runs finish. A record without them reads as
today. The release CI makes from the landing is a new minor version.
The orchestrator gives crew the words change of D9.

## Open Questions

None. The user accepted this design, record 0084 and the statement's
amendment on 2026-11-06, and the amendment is on agentplot/blueprints
main at a4eeec3.
