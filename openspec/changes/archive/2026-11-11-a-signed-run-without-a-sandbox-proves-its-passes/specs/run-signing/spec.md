## ADDED Requirements

### Requirement: A replay no counted pass proves is logged with its key and why
Where a run replays checks that no counted pass proves, the branch's
log SHALL record, beside the combination and its tasks, each replay's
task, key and why no counted pass proves it, as `keys` on the
`unproven` line. A line written before `keys` SHALL read with none
(108, 19, 78, 79, 83; rule core.41).

#### Scenario: Each why
- **WHEN** a run's replays are read against results that hold no run of
  the first key, a run of the second on another system alone, a run of
  the third by an executor or signer the branch does not count, a run
  of the fourth by a trusted executor held to other limits, and a
  counted fail of the fifth
- **THEN** each replay is unproven, its why in turn `no run of its key
  is recorded`, `its key ran only on another system`, `no result of its
  key is signed by a signer the branch counts` on a branch whose
  records are on its remote or `no result of its key is by a run the
  branch trusts` on one in a host's own files, `no result of its key
  was held to this run's limits`, and `no result the branch counts for
  its key is a pass`
  (`an_unproven_replay_says_why_by_its_key` in
  `crates/sluice-core/src/verdict.rs`)

#### Scenario: The unproven line
- **WHEN** a run on a branch held to a container engine replays three
  checks whose only passes were recorded by runs held to none
- **THEN** the log's `unproven` line names the three tasks and, under
  `keys`, each task with its key and `no result of its key was held to
  this run's limits`
  (`a_replay_no_trusted_pass_proves_runs_again_with_no_replay` in
  `crates/sluice-conformance/tests/verdicts.rs`)

#### Scenario: An older line
- **WHEN** the log holds an `unproven` line written with no `keys`
- **THEN** it reads with none, and is written again without them
  (`an_unproven_line_and_an_unsigned_check_read_back` in
  `crates/sluice-core/src/records.rs`)
