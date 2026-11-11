## ADDED Requirements

### Requirement: Every release runs with the scratch its record names

Before a release command runs for a worktree, a combination's checkout
or a preview, sluice SHALL make the scratch directory the checkout's
record names, where it is gone, at the same path, as it does before a
run. The release's `TMPDIR` and `SLUICE_TMP` SHALL name that directory
on every try (35 as amended; rule core.30).

#### Scenario: Emptied between two tries

- **WHEN** a worktree's, a combination's checkout's and a preview's
  scratch directories are removed, and each one's release command runs
  `mktemp` under `$TMPDIR`
- **THEN** each release passes, and its scratch was at the path its
  record names
  (`every_release_runs_with_the_scratch_its_record_names` in
  `crates/sluice/tests/profiles/places.rs`)
