## ADDED Requirements

### Requirement: A group takes one subject per worktree or none
`sluice submit <branch>` given `--worktree` more than once SHALL take
`--subject` once for each worktree, the first subject the first
worktree's and so on in their order, or not at all. Any other count of
subjects SHALL be refused, exit 64, before anything is pushed, opened
or queued, saying to give one `--subject` per `--worktree`, in their
order, or none (125).

#### Scenario: One subject for two worktrees
- **WHEN** an agent runs `sluice submit main --worktree ../a --worktree ../b --subject "Both"`
- **THEN** it exits 64, saying to give one `--subject` per `--worktree`, in their order, or none, and nothing is pushed, opened or queued

#### Scenario: A subject for each
- **WHEN** an agent runs `sluice submit bolt/x --no-wait --worktree ../a --worktree ../b --subject "Batch review cards" --subject "Card filters"` and `bolt/x` has no `pull_requests`
- **THEN** two submissions are queued in that order, the first keeping the subject `Batch review cards` and the second `Card filters`

#### Scenario: No subject for any
- **WHEN** an agent runs `sluice submit bolt/x --no-wait --worktree ../a --worktree ../b`
- **THEN** two submissions are queued with no subject, each named by its branch when it lands

### Requirement: Under pull requests each request is titled by its own subject
To a branch that takes pull requests, each pull request the act opens
SHALL be titled by its worktree's subject, as written, or by that
worktree's branch where the act gives no subjects; each it updates that
sluice opened SHALL take its worktree's subject as its title when the
act gives subjects; one a person opened SHALL keep its title (118, 120,
125).

#### Scenario: Two requests, two subjects
- **WHEN** `main` takes pull requests with no conditions and `sluice submit main --no-wait --worktree ../a --worktree ../b --subject "Batch review cards" --subject "Card filters"` runs, `../a` on `unit/a` and `../b` on `unit/b`
- **THEN** `unit/a`'s pull request is titled `Batch review cards` and `unit/b`'s `Card filters`

#### Scenario: Two requests, no subjects
- **WHEN** the same act runs with no `--subject`
- **THEN** `unit/a`'s pull request is titled `unit/a` and `unit/b`'s `unit/b`
