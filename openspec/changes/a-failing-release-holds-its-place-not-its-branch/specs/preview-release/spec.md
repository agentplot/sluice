# Spec Delta

## MODIFIED Requirements

### Requirement: Previews nobody can use are released

sluice SHALL release a preview whose command exited non-zero or exceeded
its time limit, keeping the command's output in the preview's state, and
SHALL free a preview whose checkout no longer exists, running nothing in
its name: no release command, only its runs ended and its ports and
names let go (123). This SHALL happen when the outcome is read: by
`sluice preview`, `sluice previews`, or the next round on the host (101).

#### Scenario: A command fails

- **WHEN** a preview command exits 1
- **THEN** the preview is reported `failed` with the command's output and
  its release command has run, its ports and names freed (S33)

#### Scenario: A checkout removed by hand

- **WHEN** a preview's checkout is removed with `git worktree remove`
- **THEN** the next `sluice previews` frees its ports and names without
  running its release command, and the preview is never released again
