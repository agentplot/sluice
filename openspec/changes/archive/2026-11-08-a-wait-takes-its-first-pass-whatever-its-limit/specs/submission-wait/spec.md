## MODIFIED Requirements

### Requirement: A timeout returns the submission as it stands

With `--timeout <duration>` (`90s`, `9m`, `1h30m`), `sluice wait` SHALL
take its first pass whatever that time, a round where it runs rounds or
a read of the driver's document where a driver lives (1, 135), and SHALL
return once that time has passed with the submission undecided, exit 5,
and leave the submission queued (5). Its record SHALL be as its last pass
left it. A duration it cannot read SHALL exit 64.

#### Scenario: Still testing

- **WHEN** a submission's combination is still under test and
  `sluice wait <n> --timeout 10s --json` runs
- **THEN** it exits 5 within a few seconds of the ten, printing the
  submission's record with `state` `testing`, and the submission stays
  in the queue

#### Scenario: Out of time as it starts

- **WHEN** a submission is queued, no `sluice run` lives on the host,
  and `sluice wait <n> --timeout 0s --json` runs
- **THEN** its round starts the submission's combination, and it exits
  5, printing the submission's record with `state` `testing`

#### Scenario: A duration it cannot read

- **WHEN** `sluice wait 3 --timeout soon` runs
- **THEN** it exits 64
