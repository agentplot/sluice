## MODIFIED Requirements

### Requirement: What the remote never judged is tried again
A call SHALL be tried again when the remote never judged it: the
connection was not made, the server could not answer (a 5xx) before a
push was sent or to a forge call that makes nothing, or the remote
refused the call's credential before it negotiated any ref, saying not
found, permission denied or unauthenticated (134). A `429`, and a `403`
that says an allowance is spent, SHALL be neither: each is the
allowance's pause, which the policy waits out or stops on and never
backs off (134 as amended 2026-10-28; rule core.29).

#### Scenario: A token refused on a fetch
- **WHEN** a round fetches the branch, GitHub answers `Repository not found.` on the token held, and serves the branch on a fresh one
- **THEN** the round reads the branch and goes on

#### Scenario: A 429 is no backoff
- **WHEN** GitHub answers a read `429` with `retry-after: 120`
- **THEN** the read is not made again for two minutes, whatever the backoff's pauses would be

### Requirement: What the remote answered is not tried again
A call the remote answered SHALL NOT be tried again: a ref rejected
under its lease, a branch protected, any other ref rejected, and any
4xx answer of the forge's API but a refused credential (401), a `429`
and a `403` that says an allowance is spent, a pull request not found
among them (24, 45, 134).

#### Scenario: A protected branch is answered once
- **WHEN** a push to `main` is answered `[remote rejected] (protected branch hook declined)`
- **THEN** one push was made, and the move is reported refused in the remote's words

#### Scenario: A pull request not found is answered once
- **WHEN** sluice reads a pull request and GitHub answers `404 Not Found` on a token it has accepted before
- **THEN** one read was made, and the pull request reads as absent

#### Scenario: A spent allowance is no answer
- **WHEN** a waiting submit's read is answered `403` with `x-ratelimit-remaining: 0` and `API rate limit exceeded`
- **THEN** the submit does not fail: it waits until `x-ratelimit-reset` and reads again
