## ADDED Requirements

### Requirement: Every run records its executor's grant
Every run SHALL record, with what its executor enforced, the executor's
grant: for the sandbox, a digest of the profile sluice writes with every
path named by its role; for the open executor, one constant. Two places
or hosts given the same profile SHALL record the same grant, and the
version of sluice SHALL NOT be part of it (83, 138; rule core.34).

#### Scenario: Two places, one profile
- **WHEN** two candidates run under the sandbox in two places with the same network and caches
- **THEN** their result lines carry the same grant

#### Scenario: Two hosts' caches
- **WHEN** one host declares two cache directories and another three, with the same policy
- **THEN** their sandbox runs carry the same grant

#### Scenario: A profile that allows more
- **WHEN** one run's profile lets a command open a pseudo-terminal and another's does not
- **THEN** their grants differ

### Requirement: Results under two grants are never weighed together
The gate SHALL weigh a key's results for a flake only among results
under the run's grant, and SHALL prove a replay only by a trusted pass
under the run's grant, on a signed branch as on an unsigned one (19,
108, 138; rules core.9, core.34).

#### Scenario: A fail under a narrower profile
- **WHEN** a check's key holds a fail under one grant, and a run under another grant passes it, on a branch with `flaky = "fail"`
- **THEN** the run reads the check passed, and no flake is named

#### Scenario: A replay under another grant
- **WHEN** moon replays a check whose only recorded pass is under another grant
- **THEN** the replay is unproven, and the round runs the check again with no replay

#### Scenario: A signed branch
- **WHEN** a signed branch's key holds a signer's fail under one grant and a run under another grant passes
- **THEN** the run reads the check passed

### Requirement: A result with no grant is weighed only with others with none
A result line that carries no grant SHALL be read as run under a grant
unknown: weighed with other lines that carry none, and never with a
line or a run that carries one (138).

#### Scenario: An old fail
- **WHEN** a key holds a fail recorded with no grant, and a run that records a grant passes it
- **THEN** the run reads the check passed

#### Scenario: Two old lines
- **WHEN** a key holds a pass and a fail, both recorded with no grant, and an older sluice reads them
- **THEN** they read as the branch's `flaky` says, as before

### Requirement: A signed run's grant is read from its statement
A signed run's statement SHALL carry the grant with what the executor
enforced, and the grant of a signed result SHALL be read from that
statement alone, needing no allowance of the signer's (core.22).

#### Scenario: A grant beside the statement
- **WHEN** a signed record's statement carries one grant and an unsigned field beside it another
- **THEN** the result is read under the statement's grant
