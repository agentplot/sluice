## MODIFIED Requirements

### Requirement: Release at the caller's asking

`sluice release <preview>` SHALL run the release command of the
previewed revision's policy in the checkout under the host's executor,
with the preview's environment, names and ports; end everything started
in it; remove the checkout; and let its ports and names go (101, 34).
Releasing a preview that does not exist SHALL exit 0 saying there is
nothing to release. A preview held because its release failed on every
try its branch gives it SHALL have its release run again only by
`sluice release <preview> --again`, as any held place's is (123).

#### Scenario: The next landing replaces a preview

- **WHEN** a caller asks for R2's preview and then releases R1's
- **THEN** R1's release command removes its containers and routes, its
  checkout is gone, and its ports and names are free for another
  checkout (S31)

#### Scenario: Release twice

- **WHEN** a caller releases a preview that was already released
- **THEN** sluice says there is nothing to release and exits 0

#### Scenario: A held preview released by its caller

- **WHEN** a preview's release failed on every try its branch gives it,
  and its caller runs `sluice release <preview>`, then `sluice release
  <preview> --again`
- **THEN** the first runs nothing, says the preview is held and exits
  1, and the second runs its release command
  (`a_held_previews_release_runs_again_only_when_asked_again` in
  `crates/sluice/tests/previews.rs`)
