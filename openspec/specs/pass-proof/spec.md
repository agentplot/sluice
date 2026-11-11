# pass-proof Specification

## Purpose
What a pass needs before it counts on a branch whose state is on its
remote: the task runner's own record that the test ran, kept where the
check profile reads it (C.6). A test that passed in a run that kept no
such record has no result. Nothing is recorded under its key, nobody is
blamed, and its combination is tested again within 115's bound. Why is
read from the missing record itself. The branch's log names each such
cause once per host and reason, with its end, and `branch.json` keeps it
while it stands (114, 115; record 0105).

## Requirements

### Requirement: A pass the run kept no proof of has no result

On a branch whose state is on its remote, where a test ran and passed
and the run kept nothing that proves it ran, sluice SHALL record no
result for it under its key, list it as not run with why, and blame no
submission (114, 16; record 0105; rule core.35).

#### Scenario: Every pass of a run has no proof

- **WHEN** `main`'s state is on `origin`, a run of #1's combination
  passes `a:test` and `b:test`, and the run kept no proof of either
- **THEN** `results.jsonl` gains no line for either test, the branch's
  log records the run with no result and each test not run with why,
  and #1 is not blamed
  (`a_pass_nothing_proves_has_no_outcome_and_runs_again` in
  `crates/sluice-conformance/tests/unkept_proof.rs`)

#### Scenario: A test that failed on its own beside it

- **WHEN** a run of the combination fails `a:test` and passes `b:test`,
  and the run kept no proof of `b:test`
- **THEN** the combination fails on `a:test` as before, and `b:test` is
  named as could not run, with why
  (`a_check_that_failed_beside_a_pass_nothing_proves_fails_as_before`
  in `crates/sluice-conformance/tests/unkept_proof.rs`)

### Requirement: A combination whose passes had no proof is tested again

A combination whose run kept no proof of its passes SHALL be tested
again, and the run SHALL count toward the runs in a row whose tests
could not run, so that the third such run reads the combination as not
passed and blames with why (114, 115).

#### Scenario: The next run keeps its proof

- **WHEN** the combination's next run passes both tests and keeps their
  proof
- **THEN** both passes are recorded under their keys and #1 lands
  (`a_pass_nothing_proves_has_no_outcome_and_runs_again`)

#### Scenario: Three runs in a row

- **WHEN** three runs in a row of the same combination each keep no
  proof of its passes
- **THEN** the third leaves the combination not passed, its newest
  submission blamed, and its session told each test could not run, with
  why (`three_runs_whose_passes_nothing_proves_blame_with_why` in
  `crates/sluice-conformance/tests/unkept_proof.rs`)

### Requirement: Why is read from the missing proof itself

The reason SHALL be read from the fact itself, the task runner's record
missing where the check profile reads it, and SHALL name where it was
looked for, never what the task runner printed (record 0105; rule
core.35). For moon, the record is moon's manifest of the test's key and
of each key it depends on (C.2 moon row), and the reason SHALL read
`it passed, but nothing the run kept proves it ran: moon's manifest of
it is not in <store>`.

#### Scenario: A manifest gone from the store

- **WHEN** a moon run passes `alpha:test` and `beta:test`, and
  `alpha:test`'s manifest is not in the store the profile reads for the
  run
- **THEN** `beta:test` is proven, and `alpha:test` is among the passes
  with no proof, saying
  `it passed, but nothing the run kept proves it ran: moon's manifest of
  it is not in <store>` with that store's path
  (`a_pass_whose_manifest_is_gone_is_unkept_naming_the_store` in
  `crates/sluice/tests/profiles/moon.rs`)

#### Scenario: A dependency's manifest missing

- **WHEN** a passing task's own manifest is in the store and the
  manifest of a task it depends on is not
- **THEN** the task's proof is read as not kept, not as a key the tree
  does not bear out
  (`a_missing_manifest_is_told_from_one_the_tree_does_not_bear_out` in
  `crates/sluice/src/profiles/moon.rs`)

### Requirement: Only a pass with no proof is read so

A manifest the combination's tree does not bear out, a test that
failed, a replay and a task a check was failed by SHALL be read as
before, whether their manifests were kept or not (108, 139, 149, C.6).

#### Scenario: A key the tree does not bear out

- **WHEN** a run on `main`, whose state is on `origin`, passes
  `a:test` under a key whose manifest names inputs the tree does not
  hold
- **THEN** `a:test` is read as no pass under its key, as before, and
  #1 is blamed for it
  (`a_key_the_tree_does_not_bear_out_is_still_failed` in
  `crates/sluice-conformance/tests/unkept_proof.rs`)

#### Scenario: A failed test whose manifest is gone

- **WHEN** a moon run fails `gamma:test` and its manifest is not in the
  store
- **THEN** the check profile names `gamma:test` neither proven nor among
  the passes with no proof
  (`a_failed_task_whose_manifest_is_gone_is_not_unkept` in
  `crates/sluice/tests/profiles/moon.rs`)

### Requirement: A branch in one host's own files reads no proof

A branch whose state is in one host's own files SHALL read no proof of
its runs, and SHALL be read as before (78).

#### Scenario: Its runs keep no manifest

- **WHEN** `main`'s state is in the host's own files and a run of its
  combination keeps no proof of its passes
- **THEN** the combination passes and lands as before, and the log
  holds no `proof-missing` line
  (`a_branch_in_one_hosts_files_reads_no_proof` in
  `crates/sluice-conformance/tests/unkept_proof.rs`)

### Requirement: A missing proof is logged once per cause and host

When a run keeps no proof of a pass, the branch's log SHALL gain a
`proof-missing` line naming the host that read the run and why, once
for each distinct reason while it stands on that host, never once per
run. Each host's causes SHALL be logged apart (record 0105; rule
core.28).

#### Scenario: Two combinations on one host

- **WHEN** runs of two combinations on one host, three runs in all,
  each keep no proof of their passes in the same store
- **THEN** the log holds one `proof-missing` line for them
  (`the_cause_is_logged_once_across_runs_and_combinations` in
  `crates/sluice-conformance/tests/unkept_proof.rs`)

#### Scenario: Two hosts, and words that change

- **WHEN** runs on `build-1` and on `build-2` each keep no proof, and
  `build-1`'s later runs keep none in another store
- **THEN** the log holds a `proof-missing` line for `build-2` and one
  for each of `build-1`'s two stores
  (`a_cause_is_noted_once_per_host_and_reason` in
  `crates/sluice-core/src/proof.rs`)

### Requirement: A standing cause is kept in the branch's record

While a cause stands, `branch.json` SHALL keep it under
`proof_missing`, with its host, why, since when and how many of that
host's runs it touched. Nothing a round, gate, landing or submission
decides SHALL read it (rule core.14).

#### Scenario: Three runs touched

- **WHEN** three runs on `fake-host` kept no proof in the same store
- **THEN** `proof_missing` names `fake-host`, the reason and three runs
  (`the_cause_is_logged_once_across_runs_and_combinations`)

#### Scenario: The lines and the record read back

- **WHEN** the log holds a `proof-missing` and a `proof-kept` line
- **THEN** each reads back as written, and a `branch.json` written
  before `proof_missing` existed reads with none
  (`a_missing_proof_and_its_end_are_log_lines` in
  `crates/sluice-core/src/records.rs`)

### Requirement: The end of a cause is logged with how long and how many runs

When a run on a host whose cause stands proves at least one pass and
leaves none without proof, the branch's log SHALL gain a `proof-kept`
line for each of that host's causes, naming the host, why, when it
began, how many seconds it stood and how many runs it touched, and
`branch.json` SHALL keep it no longer. A run that passed nothing SHALL
end nothing (record 0105; rule core.28).

#### Scenario: The host keeps proof again

- **WHEN** `build-1`'s cause began at 09:16:40 and touched three runs,
  and its run at 09:46:40 proves its passes
- **THEN** `main`'s log gains
  `{"event":"proof-kept","host":"build-1","why":"<why>","since":"2026-11-09T09:16:40Z","stood":1800,"runs":3}`
  with that run's time, and `proof_missing` no longer names it
  (`a_cause_ends_with_how_long_and_how_many_runs` in
  `crates/sluice-core/src/proof.rs`)

#### Scenario: A run that passed nothing

- **WHEN** a run on `build-1` whose cause stands fails every test
- **THEN** the cause stands and nothing is logged
  (`a_run_that_proved_no_pass_ends_nothing` in
  `crates/sluice-core/src/proof.rs`)

#### Scenario: A cause that comes back

- **WHEN** a cause ended and a later run on the same host keeps no
  proof again
- **THEN** the log gains a new `proof-missing` line
  (`a_cause_ends_with_how_long_and_how_many_runs`)
