# Spec Delta

## MODIFIED Requirements

### Requirement: The exits tell the outcomes apart

A waiting `sluice submit` and `sluice wait` SHALL exit 0 when the
submission landed, 1 when it failed or its pull request was closed, 2
on a conflict, 4 when it was withdrawn, 5 when `--timeout` ran out, and
130 when stopped with Ctrl-C. A submission that does not exist SHALL
exit 1 with the not-found error document under `--json`. A number found
on no branch sluice could read, while some branch could not be read,
SHALL exit 3 with the internal error document under `--json`, naming
each branch that could not be read, since whether the submission exists
is not known (26).

#### Scenario: Withdrawn while waited on

- **WHEN** submission `<n>` is withdrawn while `sluice wait <n>` waits
- **THEN** it exits 4, and a waiting `sluice submit` withdrawn the same
  way exits 4

#### Scenario: Blamed

- **WHEN** submission `<n>` is blamed while `sluice wait <n>` waits
- **THEN** it exits 1 and names the failed tasks as a waiting submit
  does

#### Scenario: No such submission

- **WHEN** `sluice wait 9999 --json` runs and there is no submission
  9999
- **THEN** it prints `{"error":{"kind":"not_found",...}}` and exits 1

#### Scenario: Not found among the branches that could be read

- **WHEN** `sluice wait 9999 --json` runs, no branch sluice could read
  holds submission 9999, and `bolt/b`'s state could not be read
- **THEN** it prints `{"error":{"kind":"internal",...}}` whose message
  names `bolt/b` and the words it was refused with, and exits 3
