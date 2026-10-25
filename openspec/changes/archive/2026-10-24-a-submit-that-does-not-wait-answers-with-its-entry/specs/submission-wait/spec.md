## MODIFIED Requirements

### Requirement: A branch names this worktree's last submission

`sluice wait <branch>` SHALL wait for this worktree's last submission
to `<branch>`, found as `sluice submit <branch> --status` finds it,
and SHALL say `nothing submitted from <worktree> to <branch>` and exit
1 when there is none. On a branch that takes pull requests, it SHALL
follow this worktree's pull request as a waiting submit does, and once
sluice has taken the pull request in at its current head, it SHALL
follow that submission as `sluice wait <id>` follows one: out of time
or stopped with Ctrl-C, under `--json` it SHALL print the submission's
record, and without `--json` it SHALL say `still waiting: sluice wait
<id>`. Before the pull request is taken in, it SHALL answer with the
pull request and what it waits for, `pull_request`, `branch` and
`state` under `--json`. A waiting `sluice submit` that follows a pull
request and is stopped with Ctrl-C SHALL end the same way.

#### Scenario: The worktree's own

- **WHEN** a worktree submitted #2 and then #4 to `main`, and another
  worktree submitted #3
- **THEN** `sluice wait main` in the first waits for #4

#### Scenario: A pull request taken in, out of time

- **WHEN** `main` takes pull requests with no conditions, an agent's
  `wt submit main --no-wait` opened pull request `<n>` and sluice took
  it in as submission `<id>`, whose combination is still under test,
  and `sluice wait main --timeout 2s --json` runs in the agent's
  worktree
- **THEN** it exits 5 and prints the record of submission `<id>`, with
  `state` `queued` or `testing`, as `sluice wait <id> --timeout 2s
  --json` would

#### Scenario: A pull request waiting for a review, out of time

- **WHEN** an agent's pull request waits for a review and `sluice wait
  main --timeout 2s --json` runs in the agent's worktree
- **THEN** it exits 5 and prints `pull_request`, `branch` and `state`
  `waiting: needs 1 review`
