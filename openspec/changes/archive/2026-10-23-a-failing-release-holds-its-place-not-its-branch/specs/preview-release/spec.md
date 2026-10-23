# Spec Delta

## MODIFIED Requirements

### Requirement: Previews nobody can use are released

sluice SHALL release a preview whose command exited non-zero or exceeded
its time limit, keeping the command's output in the preview's state. A
preview whose checkout no longer exists SHALL have its stored release
command run once, in its scratch directory with its ports, and SHALL
then be freed, its runs ended and its ports and names let go, whatever
that command did, and never tried again (123). This SHALL happen when
the outcome is read: by `sluice preview`, `sluice previews`, or the next
round on the host (101).

#### Scenario: A command fails

- **WHEN** a preview command exits 1
- **THEN** the preview is reported `failed` with the command's output and
  its release command has run, its ports and names freed (S33)

#### Scenario: A checkout removed by hand

- **WHEN** a preview's checkout is removed with `git worktree remove`
- **THEN** the next `sluice previews` runs its stored release command
  once in its scratch directory, frees its ports and names, and no later
  read runs its release again, even when that command failed
