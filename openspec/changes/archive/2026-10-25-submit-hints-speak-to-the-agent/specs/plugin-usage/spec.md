## ADDED Requirements

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
