# Spec Delta

## MODIFIED Requirements

### Requirement: A branch's queue is measured

While telemetry is on, sluice SHALL send, each with `sluice.branch`, the
gauges `sluice.queue.depth` (submissions not yet out of the queue) and
`sluice.window` (combinations tested at once), and the counters
`sluice.verdicts` by `sluice.outcome`, `sluice.tasks.ran`,
`sluice.tasks.replayed`, `sluice.culprits` (submissions blamed),
`sluice.conflicts`, `sluice.landed` and `sluice.refused` (landings
refused because the branch moved).

#### Scenario: A landing's metrics

- **WHEN** a submission is tested, passes and lands
- **THEN** sluice reports `sluice.queue.depth`, `sluice.verdicts`,
  `sluice.tasks.ran` and `sluice.landed`
  (`a_landing_reports_its_round_verdict_and_landing` in
  `crates/sluice/tests/telemetry.rs`)

#### Scenario: A blamed submission is counted

- **WHEN** a submission's combination fails its gate
- **THEN** sluice reports `sluice.culprits` (`a_failure_reports_its_culprit`
  in `crates/sluice/tests/telemetry.rs`)
