## ADDED Requirements

### Requirement: A removed branch is named gone

`sluice overview --json` SHALL keep every branch with records in
`branches[]`, and SHALL give `"gone": true` to a branch whose records it
read without refusal and of which the host holds no revision, leaving
`gone` out otherwise (26, 105, 129; record 0091).

#### Scenario: A branch made and removed
- **WHEN** `bolt/x` was made here and removed, every submission to it made in another copy of the repository
- **THEN** `sluice overview --json` names `bolt/x` in `branches` with `"gone": true`

#### Scenario: A branch that is there
- **WHEN** `main` has records and is there
- **THEN** `sluice overview --json` gives `main` no `gone` field

### Requirement: The text shows a gone branch with nothing to watch apart

`sluice overview` SHALL name each gone branch with nothing queued, under
test or recent, no hold, no held checkout, no refused write and no read
refused on one line after the branches, `gone, nothing queued or recent:
<branch>, ...`, and give it no line of its own (129; record 0091).

#### Scenario: Nothing to watch
- **WHEN** `bolt/x` was made here and removed with nothing submitted to it here
- **THEN** `sluice overview` prints `gone, nothing queued or recent: bolt/x` and no line `bolt/x at`

### Requirement: The text heads a gone branch with something to watch as gone

`sluice overview` SHALL show a gone branch with something to watch as
any branch, its first line `<branch> is gone`, followed by
`, last at <revision>` where its records saw one, in place of the
queue's counts (record 0091).

#### Scenario: A removed branch with its landings
- **WHEN** `bolt/x` landed `#1` at `R` and was removed into `main`
- **THEN** `sluice overview` shows `bolt/x is gone, last at R` with `#1`'s landing under it
