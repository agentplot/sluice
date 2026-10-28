## ADDED Requirements

### Requirement: A wait removes no worktree

`sluice wait` SHALL NOT remove, release or clean the worktree a submission came from, whatever it waits on and however that submission ends. Removing a landed worktree is its driver's: under worktrunk, `wt submit` run again in that worktree (25; rule core.16; record 0065).

#### Scenario: Landed, waited on with sluice wait

- **WHEN** a worktree that ran `sluice exec main` submits with `--no-wait`, and `sluice wait <id>` runs until `<id>` lands
- **THEN** it exits 0, the worktree is still there with its head, and `sluice status` still lists its ports
