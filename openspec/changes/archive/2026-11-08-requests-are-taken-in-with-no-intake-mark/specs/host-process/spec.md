## MODIFIED Requirements

### Requirement: sluice step is one round of the host process

`sluice step <branch>` SHALL ask the host process for one round of the
branch, starting it where none answers, and SHALL print that round's
outcome, its releases and its hold as it prints them today, as text or
under `--json`, with the same exits, and every warning the round's
intake met on stderr. The round SHALL be the one a pass of the host
process runs for the branch, so a step and a pass behave alike (rule
core.14).

#### Scenario: A step with a submission queued

- **WHEN** `main` has one submission queued and nothing under test, and
  `sluice step main --json` runs
- **THEN** it prints one document with `"outcome": "stepped"`, `main`'s
  records hold the submission's combination, and the round ran in the
  host process

#### Scenario: A step whose intake met a request it cannot read

- **WHEN** main takes pull requests, the host keeps it, GitHub serves no
  head for `#12`, and `sluice step main --json` runs
- **THEN** it prints `sluice: warning: taking in main's pull requests:
  pull/12:` and the failed fetch on stderr, and its document on stdout
