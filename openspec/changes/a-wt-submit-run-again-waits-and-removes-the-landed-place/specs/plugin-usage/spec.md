## MODIFIED Requirements

### Requirement: An agent with a capped tool call waits with sluice wait

An agent whose tool calls cannot outlast a verdict SHALL submit with
`--no-wait --json` and then run `sluice wait <n> --timeout 9m --json`
again for as long as it exits 5, reading the outcome from the record's
`state` and the exit. Under worktrunk it SHALL run `wt submit <branch>
--timeout 9m --json` in place of `sluice wait`, which removes its
worktree once it lands, and SHALL NOT remove a landed worktree by hand.
It SHALL NOT read the outcome by searching `sluice status`'s listing,
which a decided submission leaves.

#### Scenario: A long gate

- **WHEN** an agent with ten-minute tool calls submits to a branch
  whose gate takes forty minutes
- **THEN** it runs `sluice submit main --no-wait --json`, then
  `sluice wait <id> --timeout 9m --json` until it exits other than 5,
  and reports the record's `state`

#### Scenario: A long gate under worktrunk

- **WHEN** an agent with ten-minute tool calls, in a worktree worktrunk
  made, submits to a branch whose gate takes forty minutes
- **THEN** it runs `wt submit main --no-wait --json`, then `wt submit
  main --timeout 9m --json` until it exits other than 5, reports the
  record's `state`, and runs no `wt remove`

#### Scenario: The session-start note says so

- **WHEN** a session starts in a repository with a `.config/sluice.toml`
- **THEN** the note sluice's plugin adds tells an agent under worktrunk
  to wait with `wt submit <branch> --timeout 9m --json` in the
  background again while it exits 5, and never to remove a landed
  worktree by hand
