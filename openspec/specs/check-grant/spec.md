# check-grant Specification

## Purpose
What a run's executor let it reach, its grant, recorded with every
result and read as part of the result's key. Results under two grants
are never weighed together for a flake, nor does a pass under one prove
a replay under another, on every branch, signed or not. A result
recorded before the grant is weighed only with others recorded before
it (138; record 0072; rule core.34).

## Requirements

### Requirement: Every result records its executor's grant

Every run SHALL record, with what its executor enforced, the executor's
grant: for the sandbox, a digest of the profile sluice writes with every
path named by its role; for the open executor, one constant. Places and
hosts given the same profile SHALL record the same grant. sluice's
version SHALL be no part of it (83, 138; rule core.34).

#### Scenario: The grant is the profile the run writes

- **WHEN** a sandboxed run's profile is written for a checkout, its
  scratch, two caches and the container engine's socket
- **THEN** the text its grant digests gives the written profile's rules
  once each path is put back for its role
  (`the_grant_renders_the_profile_the_run_writes` in
  `crates/sluice/src/profiles/checks.rs`)

#### Scenario: Two places, two hosts, one profile

- **WHEN** two runs under the same policy are given two checkouts, two
  scratch directories, two and three cache directories, the runner's
  store under two homes and two temporary directories
- **THEN** both record the same grant, and a run whose runner's store is
  in its checkout records the grant of one given none
  (`one_profile_gives_one_grant_wherever_it_runs`)

#### Scenario: A profile that reaches more

- **WHEN** two runs differ in their network, in whether they may reach
  the container engine, or in whether they are given any cache
- **THEN** their grants differ
  (`a_profile_that_reaches_more_gives_another_grant`)

#### Scenario: The open executor

- **WHEN** `sluice exec` runs a branch's tests with the open executor
  in a clean worktree
- **THEN** each result it records carries the grant `open:1`
  (`exec_records_the_open_executors_grant` in
  `crates/sluice/tests/exec.rs`)

#### Scenario: A sandboxed run

- **WHEN** a sandboxed run finishes
- **THEN** its report carries the grant of the profile it ran under,
  beginning `sb1:` (`a_sandbox_run_records_the_grant_of_its_profile` in
  `crates/sluice/tests/checks.rs`)

### Requirement: Results under two grants are never weighed together

The gate SHALL weigh a key's results for a flake only among results
under the run's grant, and SHALL prove a replay only by a counted pass
under the run's grant, on a branch whose state is on its remote as on
one in a host's own files (19, 108, 138; rules core.9, core.34).

#### Scenario: A fail under a narrower profile

- **WHEN** a test's key holds a fail under one grant, and a run under
  another grant passes it, on a branch with `flaky = "fail"`
- **THEN** the run reads the test passed, and names no flake
  (`a_key_is_flaky_only_under_the_same_grant` in
  `crates/sluice-core/src/verdict.rs`)

#### Scenario: A fail under the same grant

- **WHEN** a test's key holds a fail under a grant, and a run under the
  same grant passes it, on a branch with `flaky = "fail"`
- **THEN** the run reads the test failed as flaky, as before
  (`a_key_is_flaky_only_under_the_same_grant`)

#### Scenario: A branch whose state is on its remote

- **WHEN** a signer the branch counts signed a fail of a key under one
  grant, and a run under another grant passes it
- **THEN** the run reads the test passed
  (`a_key_is_flaky_only_under_the_same_grant`)

#### Scenario: A replay under another grant

- **WHEN** a run replays a test whose only counted pass is under another
  grant
- **THEN** the replay is unproven, and the test runs again with no
  replay (`a_replay_is_proven_only_by_a_pass_under_its_grant`)

#### Scenario: Through the queue

- **WHEN** a submission is blamed for a fail of `a:test` under one
  grant, and the same work is submitted again from a host whose runs
  are under another grant, where it passes
- **THEN** it lands, and no flake is named
  (`a_fail_under_another_grant_is_never_weighed_against_a_pass` in
  `crates/sluice-conformance/tests/verdicts.rs`)

### Requirement: A result with no grant is weighed only with others with none

A result recorded with no grant SHALL be read as run under a grant
unknown: weighed with others recorded with none, and never with a
result or a run that carries one. Such a line SHALL read back with no
grant and be written again without one (138).

#### Scenario: An old fail

- **WHEN** a key holds a fail recorded with no grant, and a run that
  records a grant passes it
- **THEN** the run reads the test passed
  (`a_key_is_flaky_only_under_the_same_grant`)

#### Scenario: Two old lines

- **WHEN** a key holds a pass and a fail, both recorded with no grant,
  and a run recorded with no grant reads them
- **THEN** they read as the branch's `flaky` says, as before
  (`a_key_is_flaky_only_under_the_same_grant`)

#### Scenario: An old line read back

- **WHEN** `results.jsonl` holds a line whose executor names no grant
- **THEN** it reads with none and is written again without one
  (`a_result_line_with_no_grant_reads_back` in
  `crates/sluice-core/src/records.rs`)

### Requirement: A signed run's limits and grant are read from its statement

A signed run's statement SHALL carry the limits its executor held it to
and its grant, and a signed result's SHALL be read from that statement
alone, needing no allowance of its signer's. One statement SHALL hold
one executor's limits and grant. A statement with neither SHALL read
with none (19, 138; rule core.22).

#### Scenario: Two grants in one combination

- **WHEN** a host signs results of one combination run under two grants,
  held to `network = "localhost"` and the container engine
- **THEN** they are signed as two statements, and each result reads back
  with those limits and the grant it was run under
  (`a_signed_runs_grant_is_read_from_its_statement` in
  `crates/sluice/tests/remote_refs.rs`)

#### Scenario: An older statement

- **WHEN** a signer's ref holds a statement signed with no limits and no
  grant
- **THEN** its results read with none
  (`a_statement_with_no_grant_reads_with_none` in
  `crates/sluice/src/signing/statement.rs`)

### Requirement: A signed result is weighed only under the run's limits

On a branch whose state is on its remote, a key's results SHALL be
weighed for a flake, and a replay proven, only among signed results
held to the run's limits, as on a branch in a host's own files (19, 83;
record 0072).

#### Scenario: Another network

- **WHEN** a counted signer's fail of a key was held to no network, and
  a run held to `network = "localhost"` passes it, on a branch with
  `flaky = "fail"`
- **THEN** the run reads the test passed
  (`a_signed_key_is_weighed_only_under_the_runs_limits` in
  `crates/sluice-core/src/verdict.rs`)

### Requirement: A replay another grant's pass does not prove says so

Where a counted result of a replayed key is held to the run's limits but
for its grant, and none is under the run's grant, the `unproven` line
SHALL give the replay's why as `no result of its key was run under this
run's grant`, after the limits' why, on every branch (108, 138).

#### Scenario: Another grant

- **WHEN** a replay's key holds a trusted pass held to the run's limits
  under another grant, on a branch in a host's own files, and a counted
  signer's pass so held under another grant, on one whose state is on
  its remote
- **THEN** each replay's why is `no result of its key was run under this
  run's grant` (`an_unproven_replay_names_another_grant` in
  `crates/sluice-core/src/verdict.rs`)

#### Scenario: Other limits on a signed branch

- **WHEN** a replay's key holds only a counted signer's pass held to
  other limits, on a branch whose state is on its remote
- **THEN** its why is `no result of its key was held to this run's
  limits` (`an_unproven_replay_names_another_grant`)
