## ADDED Requirements

### Requirement: A worktree held with no ports is given them at its next run

When `sluice exec` runs in a worktree whose place sluice holds with no
ports, as a failed release of a worktree it held nothing for leaves it,
and the policy names ports, sluice SHALL give the place a block of
ports, as it gives a worktree at its first run, and SHALL keep what
failed of its release (session-runs-1 clause 90; 123; record 0112 as
amended).

#### Scenario: A run after a failed release

- **WHEN** a worktree's place is held with no ports and `sluice exec`
  runs there under a policy that names the port `web`
- **THEN** the run is given `SLUICE_PORT_WEB`, and the place's record
  names the block it holds
  (`a_worktrees_place_without_ports_is_given_them_at_its_next_run` in
  `crates/sluice/tests/profiles/places.rs`)
