# Spec Delta

## MODIFIED Requirements

### Requirement: A wait runs to the submission's outcome

`sluice wait <n>` SHALL wait until submission `<n>` is decided, landed,
failed, conflict or withdrawn, running sluice's rounds between reads of
its state exactly as a waiting `sluice submit` does (1; rule core.4)
where no driver of its repository is live on the host. Where one is
live, it SHALL run no round and follow the driver's document to the
outcome (135; `host-driver`). It SHALL then report the outcome as a
waiting submit does. A waiting submit and `sluice wait` SHALL wait
through one loop. A submission already decided SHALL be reported at
once, with no round run (51).

#### Scenario: It lands

- **WHEN** an agent submits with `--no-wait` and runs `sluice wait <n>`
  on a host with no `sluice run` live
- **THEN** the command runs rounds until the submission lands, prints
  `passed · landed as <rev>` and exits 0

#### Scenario: It lands under a driver

- **WHEN** an agent submits with `--no-wait` and runs `sluice wait <n>`
  while `sluice run` drives the repository on the host
- **THEN** the command runs no round, and prints `passed · landed as
  <rev>` and exits 0 once the driver lands it

#### Scenario: Already decided

- **WHEN** submission `<n>` landed before `sluice wait <n>` runs
- **THEN** it exits 0 at once, and no round ran
