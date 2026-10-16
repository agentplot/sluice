# plugin-usage Specification

## Purpose
An agent in Claude Code landing its work through sluice.

## Requirements

### Requirement: An agent submits rather than merges

In a repository with a `.config/sluice.toml`, an agent asked to land its
work SHALL commit everything and submit with `sluice submit` or `wt
submit`, and SHALL NOT merge or push onto a protected branch.

#### Scenario: Asked to merge

- **WHEN** an agent in a worktree of a repository whose policy protects
  `main` is asked to merge its branch into `main`
- **THEN** it runs `sluice submit main` (or `wt submit main`) and reports
  the outcome by its exit code

### Requirement: A failure is read before it is fixed

On a failed submission, an agent SHALL read the blamed tests and their kept
logs, reproduce them with `sluice exec`, fix, and submit again; on a
conflict it SHALL merge the branch in and submit again.

#### Scenario: Blamed

- **WHEN** `sluice submit` exits 1 naming `tests:t-cards` and a log path
- **THEN** the agent opens that log, runs `sluice exec <branch>`, and
  submits again after a fix
