# Spec Delta

## Purpose

Keeping a Claude Code session off the branches sluice protects (core.4).

## ADDED Requirements

### Requirement: A push or merge onto a protected branch is refused

The plugin's Bash guard SHALL refuse `git push` to a protected branch,
`git merge` or `git update-ref` onto one, and `gh pr merge` into one, naming
`sluice submit <branch>` instead, and SHALL let every other command through.

#### Scenario: A push to main

- **WHEN** in a repository whose policy protects `main`, a session runs
  `git push origin HEAD:main`
- **THEN** the call is refused with a reason naming `sluice submit main`

#### Scenario: A push of the agent's own branch

- **WHEN** the same session runs `git push origin unit/x`
- **THEN** the call goes through

#### Scenario: Not a sluice repository

- **WHEN** a session in a repository with no `.config/sluice.toml` runs
  `git push origin main`
- **THEN** the call goes through

### Requirement: A session is told which branches are protected

At session start in a repository whose policy protects branches, the plugin
SHALL add one short note naming them and the submit command.

#### Scenario: Session start

- **WHEN** a session starts in a worktree of a repository protecting `main`
- **THEN** its context says `main` lands through `sluice submit main`
