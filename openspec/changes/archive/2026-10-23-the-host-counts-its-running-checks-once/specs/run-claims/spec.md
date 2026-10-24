# Spec Delta

## MODIFIED Requirements

### Requirement: A run is taken only by a recorded claim
The lead SHALL open combinations within the branch's `parallel` window
and assign them to no host. A host SHALL run a combination's checks only
after its claim on that run is written to the branch's state, and only
while the host's bound has room, counting every check running on the
host, whatever repository it serves (71, 75).

#### Scenario: Two hosts reach for one run
- **WHEN** hosts A and B both try to claim the same run from the same version of `main`'s state
- **THEN** exactly one claim is written, and only that host runs the checks

#### Scenario: A full host claims nothing
- **WHEN** host A's bound is full
- **THEN** host A claims no run, whatever the window allows

#### Scenario: A host full with another repository's runs claims nothing
- **WHEN** host A's bound is 2 and two runs of another repository are running on it
- **THEN** host A claims no run of `main`, whatever the window allows

### Requirement: Each host shows it is alive and how much room it has
Each host running sluice for a repository SHALL rewrite
`refs/sluice/hosts/<host>` on its remote at the cadence of its
sign-of-life bound, with the checks running on the host, every
repository's, and the room the host's bound has left. Only that host
SHALL write its ref.

#### Scenario: A host's sign of life
- **WHEN** `sluice run` is running on host A with a bound of 4 and one run under test
- **THEN** `refs/sluice/hosts/A` is rewritten at least once per sign-of-life bound and says one run held and room for three

#### Scenario: Another repository's runs leave less room
- **WHEN** host A's bound is 4, one run of `main` is under test, and one run of another repository on host A is running
- **THEN** `refs/sluice/hosts/A` says two runs held and room for two
