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

### Requirement: An agent with a capped tool call waits with sluice wait

An agent whose tool calls cannot outlast a verdict SHALL submit with
`--no-wait --json` and then run `sluice wait <n> --timeout 9m --json`
again for as long as it exits 5, reading the outcome from the record's
`state` and the exit. It SHALL NOT read the outcome by searching
`sluice status`'s listing, which a decided submission leaves.

#### Scenario: A long gate

- **WHEN** an agent with ten-minute tool calls submits to a branch
  whose gate takes forty minutes
- **THEN** it runs `sluice submit main --no-wait --json`, then
  `sluice wait <id> --timeout 9m --json` until it exits other than 5,
  and reports the record's `state`

### Requirement: An agent started in a worktree submits its own work

When Claude starts a background agent to work in a worktree of a
repository with a `.config/sluice.toml`, the prompt it gives that agent
SHALL tell it to commit its work and submit it through sluice from that
worktree when it is done, and to report the outcome.

#### Scenario: Two spikes in two worktrees

- **WHEN** a session in the main checkout is asked to build two spikes, each in its own worktree off `dev/ai-spike`, with background agents
- **THEN** the prompt for each agent tells it to submit with `sluice submit dev/ai-spike` from its worktree when its work is done and to report the outcome

### Requirement: Work whose agent has ended is submitted by an agent

When work is committed in a worktree whose agent has ended, Claude SHALL
start an agent in that worktree to submit it through sluice. It SHALL
NOT ask the user to run `sluice submit`, in that worktree or any other.

#### Scenario: The background agents have ended

- **WHEN** two background agents have committed their work in their worktrees and ended, and the user asks to land it on `dev/ai-spike`
- **THEN** Claude starts an agent in each worktree that runs `sluice submit dev/ai-spike` there and reports the outcome, and tells the user nothing to run
