## MODIFIED Requirements

### Requirement: The ahead token forecasts the merge

For a worktree with commits ahead of its branch and nothing else to show,
the token SHALL give the count and whether it would merge cleanly behind
everything queued, from `sluice submit <branch> --check --json`. A
conflict SHALL name what it is contested with and the first file in
conflict, with how many more. The forecast SHALL run again only when the
worktree's head, the branch's `at`, or the set of submissions waiting on
the branch changes.

#### Scenario: Clean

- **WHEN** the worktree is 3 commits ahead of `main` and the forecast finds
  no conflict
- **THEN** the token is `3 ahead · clean`

#### Scenario: Conflicts

- **WHEN** the forecast finds a conflict with queued `#95` in
  `docs/architecture/builds.md` and one other file
- **THEN** the token is `3 ahead · conflicts #95 in builds.md +1`

#### Scenario: Conflicts with what landed

- **WHEN** the forecast finds a conflict with `main` alone in
  `docs/spec/identity.md`
- **THEN** the token is `3 ahead · conflicts main in identity.md`

#### Scenario: Nothing changed

- **WHEN** a new document arrives in which a combination of the branch
  has started under test, but the worktree's head, the branch's `at` and
  its submissions waiting are unchanged
- **THEN** no forecast runs

#### Scenario: Forecasts off

- **WHEN** `sources.toml` sets `ahead_check = false`
- **THEN** no forecast runs and the token is `3 ahead`
