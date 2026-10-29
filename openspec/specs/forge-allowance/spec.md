# forge-allowance Specification

## Purpose
GitHub's word that an allowance of calls is spent, the pause it starts
for every process of sluice on the host, and how each kind of caller
meets it (134; rule core.29).

## Requirements

### Requirement: GitHub's word that an allowance is spent starts a pause
sluice SHALL read as a spent allowance a `429`, a `403` that carries
`x-ratelimit-remaining: 0` or `retry-after` or says `rate limit` in its
message, and a GraphQL answer whose errors say `RATE_LIMITED`, and SHALL
make no call under that allowance until the pause ends (134; rule
core.29).

#### Scenario: The primary allowance spent
- **WHEN** GitHub answers a read `403` with `x-ratelimit-remaining: 0`, `x-ratelimit-reset` twenty minutes ahead and `API rate limit exceeded`
- **THEN** the call is not made again before that time

#### Scenario: GraphQL's allowance spent
- **WHEN** GitHub answers a GraphQL query `200` with an error of type `RATE_LIMITED` and `x-ratelimit-remaining: 0`
- **THEN** no query is made before `x-ratelimit-reset`

### Requirement: The pause lasts as long as GitHub says
The pause SHALL last until the time `retry-after` names, else until
`x-ratelimit-reset` when no calls remain, else a minute, doubled for each
such answer in a row and thirty minutes at most (134).

#### Scenario: A secondary limit with retry-after
- **WHEN** GitHub answers `429` with `retry-after: 90`
- **THEN** the call is not made again for ninety seconds

#### Scenario: A secondary limit naming no time
- **WHEN** GitHub answers `403` with `You have exceeded a secondary rate limit` and neither header, twice in a row
- **THEN** the first pause lasts a minute and the second two

### Requirement: The last call of an allowance starts the pause
An answer that succeeds and leaves `x-ratelimit-remaining: 0` SHALL be
used, and SHALL start the pause until `x-ratelimit-reset` (134).

#### Scenario: The last call
- **WHEN** a read succeeds with `x-ratelimit-remaining: 0`
- **THEN** its answer is used, and the next call to GitHub is not made before `x-ratelimit-reset`

### Requirement: Every process on the host honours the pause
The pause SHALL be written in the host's state, for the app's
installation on the repository and for the allowance GitHub names
(`x-ratelimit-resource`: REST's apart from GraphQL's). Every process of
sluice on the host SHALL read it before it calls GitHub, SHALL make no
call under that installation and allowance while it stands, and SHALL
make none to find out whether it has ended. The pause SHALL hold no
token (134).

#### Scenario: One process meets it, another honours it
- **WHEN** `sluice run` is refused with the allowance spent until 14:20, and a `sluice submit main` starts in a worktree on the same host at 14:05
- **THEN** the submit makes no call to GitHub's REST API before 14:20

#### Scenario: GraphQL's allowance is its own
- **WHEN** the REST allowance is paused and the GraphQL one is not
- **THEN** a GraphQL query is made

#### Scenario: The pause outlives the process
- **WHEN** the process that met the pause is stopped and started again before the pause ends
- **THEN** the new process makes no call to GitHub before the pause ends

### Requirement: A command waits the pause out within its own limit
A waiting `sluice submit`, a `sluice wait` and a `--status` that
meet a pause SHALL wait until it ends, up to their `--timeout` where they
take one, saying once on stderr until when, with GitHub's words, and
SHALL then go on. Ctrl-C SHALL stop the wait at once. A command whose
limit ends first SHALL end as it ends when its time runs out (134).

#### Scenario: A waiting submit meets the pause
- **WHEN** a waiting `sluice submit main` meets GitHub's allowance spent until 14:20 at 14:05
- **THEN** stderr says once that GitHub's allowance of calls is spent until 14:20, the submit calls GitHub again after 14:20, and goes on to its outcome

#### Scenario: A wait whose timeout comes first
- **WHEN** `sluice wait 77 --timeout 5m` meets a pause twenty minutes long
- **THEN** after five minutes it exits `5` with the submission as it stands, having made no call to GitHub meanwhile

#### Scenario: Ctrl-C during the pause
- **WHEN** a waiting submit is waiting out a pause and the operator presses Ctrl-C
- **THEN** it stops at once, as a stopped wait does

### Requirement: A submit that does not wait fails at once on a pause
A `sluice submit --no-wait` that meets a pause SHALL NOT wait for it: it
SHALL fail at once with exit `3`, naming the time the pause ends and
GitHub's words, and SHALL make no call to GitHub under that allowance
(134; the user, 2026-10-28).

#### Scenario: A no-wait submit meets the pause
- **WHEN** `sluice submit main --no-wait` starts at 14:05 while the host's state says GitHub's allowance is spent until 14:20
- **THEN** it exits `3` at once, saying GitHub's allowance of calls is spent until 14:20, and no pull request is opened

### Requirement: A round passes the forge's part by
A round of `sluice run` or `sluice step` that meets a pause SHALL NOT
wait for it: it SHALL pass by intake of the branch's pull requests and
the posts on them, say so once for that pause, and go on with the rest
of its work, landing what has passed. When the pause ends, the next
round SHALL take the pull requests in again (133, 134).

#### Scenario: A landing during the pause
- **WHEN** a pause stands and a combination of main has passed
- **THEN** `sluice run` lands it, and says once that GitHub's allowance is spent until the time GitHub named

#### Scenario: Said once
- **WHEN** a pause stands across twenty passes of `sluice run`
- **THEN** stdout says it once
