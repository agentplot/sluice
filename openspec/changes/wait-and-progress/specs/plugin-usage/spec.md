## ADDED Requirements

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
