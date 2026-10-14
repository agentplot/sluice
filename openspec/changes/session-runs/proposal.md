# Proposal

## Why

An agent working in its own worktree runs the repository's checks while
it works, and a suite that brings up a stack of services and a container
engine runs there with none of what sluice gives a candidate's checkout:
no bound on how many run at once on the host, no ports of its own, no
limits, and nothing that ends what the run started. Several agents on
one host collide on ports, overload the machine, and leave stacks running
until the worktree is removed. The design for this is drafted as
`design/sluice/proposals/session-runs-1.md` in agentplot/blueprints,
clauses 89 to 96; this change builds the one-host part of it.

## What Changes

- `sluice exec <branch>`, run in an agent's worktree, runs the checks
  the worktree's head affects against the branch; `sluice exec --
  <command>` runs any command. Either counts against the host's run
  bound for as long as it runs, receives the checkout's ports, runs under
  the policy's executor and limits, and ends with its process group (89).
- An agent's worktree receives a port for each name the policy lists on
  its first run and keeps them until it is released (90).
- The host's config gains `[runs] reserved`, how much of the bound
  agents' runs never use, so submissions' runs start first (91).
  Default 0.
- A run of the checks in a worktree that holds exactly its head is
  recorded under its keys as an open run; a dirty worktree runs and
  records nothing; a command records nothing (92). A later submission
  from that head replays what the record covers (93).
- Nothing is torn down between runs (94). `sluice release`, run in the
  worktree by the agent or its driver, runs the policy's release command
  under the executor, ends every session run's process group, removes
  the scratch directory and lets the ports go; the worktree stays (95).
  `sluice exec --release` releases when the run ends.
- `wt submit` runs `sluice release` in the worktree before worktrunk
  removes it, so a `post-remove` hook finds nothing of the agent's runs
  left (95, C.5 worktrunk).
- Site pages: commands, configuration, state, the walkthrough's services
  section and the worktrunk guide gain `sluice exec` and `sluice
  release`; an ADR records the decisions.

Out of scope, waiting on the many-hosts work (`many-hosts-2.md`, remote
submit profile, C.8 deployments): `sluice exec --remote` (96), a session
run taken by the fleet's executors on a pushed revision. It is specified
by the blueprint and gets its own change once a remote submit exists.
Also out of scope, left open by the proposal: a worktree whose agent is
gone and never released, and whether an idle stack between runs counts
against anything.

## Capabilities

### New Capabilities
- `session-run`: an agent's own run of its checks or a command in its
  worktree, under the host's bound, the checkout's ports and the policy's
  executor, with its record when the worktree holds exactly its head.
- `worktree-release`: releasing an agent's worktree on request: the
  policy's release command, the runs' process groups, the scratch
  directory and the ports, with the worktree left standing; and the
  worktrunk driver releasing before it removes.
- `run-bound`: how many runs a host starts at once, counted from the
  host's state across submissions' runs and agents' runs, with a reserve
  submissions' runs alone may use.

### Modified Capabilities
<!-- none: the repository has no specs yet; existing behaviour is described by the statement and the ADRs -->

## Impact

- The CLI crate: two commands, `exec` and `release`, and `wt-submit`'s
  landing path.
- The place seam: a profile for an agent's own worktree (no materialize;
  ports held across runs; release without removal).
- The check seam and the executor: a run started for a session, with
  the same limits and environment as a candidate's, and a record under
  its keys only when the worktree is clean at its head.
- The host's state (`$XDG_STATE_HOME/sluice/sessions/`): session runs,
  counted toward the bound through the check seam. The records: their
  check-run records in the repository's log.
- The host config (`~/.config/sluice/config.toml`): `[runs] reserved`.
- Docs: ADR, `docs/architecture/core.md` (core.10 and core.16 gain the
  session's worktree), the site's commands, configuration, state,
  walkthrough and worktrunk pages.
