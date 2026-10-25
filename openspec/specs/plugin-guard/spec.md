# plugin-guard Specification

## Purpose
Keeping a Claude Code session off the branches sluice protects (core.4).

## Requirements

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
SHALL add one short note naming them and the submit command, and saying
that an agent the session starts to work in a worktree submits its own
work through sluice from there, never the user.

#### Scenario: Session start

- **WHEN** a session starts in a worktree of a repository protecting `main`
- **THEN** its context says `main` lands through `sluice submit main`

#### Scenario: Agents the session starts

- **WHEN** a session starts in a repository protecting `main`
- **THEN** its context says an agent it starts in a worktree submits its own work through sluice from that worktree, and that the user is never asked to run `sluice submit`

### Requirement: A session cannot set aside a result

The guard SHALL refuse a Bash command in an agent's session that runs
`sluice set-aside`, and SHALL say that setting aside a result is the
operator's.

#### Scenario: An agent sets aside its own fail

- **WHEN** an agent runs `sluice set-aside cli:tools --at 2026-10-17T13:05:32Z --reason x`
- **THEN** the guard refuses it and names the operator
