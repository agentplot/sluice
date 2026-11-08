## MODIFIED Requirements

### Requirement: A waiting submit follows its pull request
A `wt submit` or `sluice submit` that waits SHALL report each state its
pull request reaches, as GitHub and sluice's check on its head show
them, run no round and take nothing in, and exit with the outcome:
landed, failed or conflict (137).

#### Scenario: Waits through review
- **WHEN** an agent's waiting submit opens a pull request that needs a review, and a reviewer approves it, and a round job takes it in and lands it
- **THEN** the submit prints waiting, then queued or testing, then landed, and exits 0, having run no round

### Requirement: A submit that does not wait answers with what sluice read
A `wt submit` or `sluice submit` with `--no-wait` SHALL return once the
pull request is open and marked ready at the head it pushed and its
round job is woken, and SHALL answer with the pull request and its
state as sluice's check on that head says, or `waiting to be taken in`
where no check stands there. It SHALL write no submission. Either
answer SHALL exit 0 (137, C.5).

#### Scenario: No wait
- **WHEN** an agent runs `wt submit main --no-wait`
- **THEN** it returns once the pull request is open, and `wt submit --status` later reports its state

#### Scenario: No wait, in words
- **WHEN** `main` takes pull requests with no conditions, and an agent runs `wt submit main --no-wait`
- **THEN** after the line `pushed unit/<name> · opened #<n>: <title>` it prints `#<n> waits to be taken in`, and exits 0

### Requirement: A submit that does not wait answers in one document
Under `--json`, a `--no-wait` submit's answer SHALL be `pull_request`,
`branch`, `state` and `queue`, the state and the block of sluice's
check on the head pushed, `queue` `null` where no check stands there
(137).

#### Scenario: No wait, its conditions hold
- **WHEN** `main` takes pull requests with no conditions, and an agent runs `wt submit main --no-wait --json` from a worktree on `unit/ready`
- **THEN** it exits 0 and prints `{"pull_request": <n>, "branch": "main", "state": "waiting to be taken in", "queue": null}`, with no `id`; and once a round job has taken it in, `sluice wait main --json` follows it to `landed`

#### Scenario: No wait, waiting for a review
- **WHEN** `main` takes pull requests with `reviews = 1`, a round job has posted `#<n>`'s check, and the agent runs `wt submit main --no-wait --json` with nothing committed since
- **THEN** it exits 0 and prints `pull_request` `<n>`, `branch` `main`, `state` `waiting: needs 1 review` and a `queue` whose `waits_for` is `["1 review"]`

## ADDED Requirements

### Requirement: A pull request's status names sluice's check

`sluice submit <branch> --status --json` and `wt submit <branch> --status --json` SHALL print a pull request landed or sent back with `state` `landed`, `failed` or `conflict`, the `title` and `summary` of sluice's check on its head, and its `pull_request`, `branch`, `head`, `draft` and `sends`, and SHALL exit 0, as `--status` does wherever it finds the worktree's pull request (137).

#### Scenario: Sent back by the round job

- **WHEN** a round job blamed an agent's pull request `<n>` at `<rev>` and marked it a draft, and `wt submit main --status --json` runs in the agent's worktree on host `a`
- **THEN** it exits 0 and prints `pull_request` `<n>`, `branch` `main`, `state` `failed`, the `title` and `summary` of sluice's check on `<rev>`, `head` `<rev>`, `draft` `true` and `sends` `<rev>`

### Requirement: A submit takes again a pull request the round job sent back

`sluice submit <branch>` and `wt submit <branch>` SHALL mark the worktree's pull request ready for review at the head pushed when sluice's check on its head before the push says it was sent back, failed or in conflict, or held for the next submit, draft or not (120, C.5). They SHALL NOT mark a draft, even for a moment, a pull request whose check says anything else.

#### Scenario: Sent back as a draft

- **WHEN** a round job blamed an agent's pull request `<n>` and marked it a draft, and the agent runs `sluice submit main --no-wait` on host `a` with nothing committed since
- **THEN** `<n>` is ready for review at that head, and the next round job takes it in again

#### Scenario: Sent back, not a draft

- **WHEN** a round job blamed pull request `<n>`, which a person opened from `unit/review` and which it therefore did not mark a draft, and an agent runs `sluice submit main --no-wait` on host `a` from a worktree on `unit/review` with nothing committed since
- **THEN** `<n>` is marked a draft and then ready for review at that head, and the next round job takes it in again

#### Scenario: Still queued

- **WHEN** the round job has pull request `<n>` queued, its `sluice` check saying `queued: next on main`, and the agent runs `sluice submit main --no-wait` on host `a` with nothing committed since
- **THEN** `<n>` is never marked a draft, and its submission stays queued

### Requirement: A pull request taken again waits to be taken in

A pull request a submit on a host took again at the head it stood at SHALL read as waiting to be taken in, to that submit, a wait and `--status` on the host, while sluice's check on that head still says what sent it back or held it; once the check says anything else, it SHALL read as the check says (120, C.5; record 0096).

#### Scenario: Taken again, its check still failed

- **WHEN** a round job blamed pull request `<n>`, which a person opened from `unit/review`, and an agent's `sluice submit main --no-wait` on host `a` takes it again at that head, its check still saying `failed: …`
- **THEN** the submit prints `#<n> waits to be taken in` and exits 0, `sluice submit main --status --json` gives `state` `waiting to be taken in`, and `wt submit main --no-wait` run again marks nothing

## REMOVED Requirements

### Requirement: A pull request's status on another host names sluice's check
**Reason**: Every host is one that does not take the branch's requests in (137; record 0093).
**Migration**: "A pull request's status names sluice's check", which holds on every host.

### Requirement: A submit takes again a pull request another host sent back
**Reason**: The round job, not another host, sends a request back (137; record 0093).
**Migration**: "A submit takes again a pull request the round job sent back".
