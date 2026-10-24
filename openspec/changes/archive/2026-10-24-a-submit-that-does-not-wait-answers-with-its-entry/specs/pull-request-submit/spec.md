## MODIFIED Requirements

### Requirement: A waiting submit follows its pull request
A `wt submit` or `sluice submit` that waits SHALL report each state its
pull request reaches, run sluice's rounds while it waits as it does
today, and exit with the outcome: landed, failed or conflict. With
`--no-wait` it SHALL return once the pull request is open and sluice has
read it once, and SHALL answer with the submission sluice took for the
pull request at the head it pushed, when sluice took it then because its
conditions held; otherwise with the pull request and what it waits for.
Either answer SHALL exit 0. Under `--json`, the submission's answer SHALL
be the document a `--no-wait` submit prints on any branch, `id`,
`branch` and `ahead`, with `pull_request`, the pull request's number,
beside them; the pull request's answer SHALL be `pull_request`, `branch`
and `state`, the state sluice posts on it.

#### Scenario: Waits through review
- **WHEN** an agent's waiting submit opens a pull request that needs a review, and a reviewer approves it, and it lands
- **THEN** the submit prints waiting, then queued or testing, then landed, and exits 0

#### Scenario: No wait
- **WHEN** an agent runs `wt submit main --no-wait`
- **THEN** it returns once the pull request is open, and `wt submit --status` later reports its state

#### Scenario: No wait, its conditions hold
- **WHEN** `main` takes pull requests with no conditions, and an agent runs `wt submit main --no-wait --json` from a worktree on `unit/ready`
- **THEN** it exits 0 and prints one document with the `id` of a submission from the pull request's checkout `pull/<n>` at the head pushed, `branch` `main`, `ahead`, and `pull_request` `<n>`; and `sluice wait <id> --json` follows that submission to `landed`

#### Scenario: No wait, waiting for a review
- **WHEN** `main` takes pull requests with `reviews = 1`, and an agent runs `wt submit main --no-wait --json`
- **THEN** it exits 0 and prints `{"pull_request": <n>, "branch": "main", "state": "waiting: needs 1 review"}`, with no `id`, and no submission of the pull request is queued

#### Scenario: No wait, in words
- **WHEN** `main` takes pull requests with no conditions, and an agent runs `wt submit main --no-wait`
- **THEN** after the line `pushed unit/<name> · opened #<n>: <title>` it prints `submission <id> to main from #<n> · nothing queued ahead`, and exits 0
