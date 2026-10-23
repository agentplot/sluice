# preview-release Specification

## Purpose
Ends a preview when its caller says the person is done with it, lists
the host's previews so a caller can find what it forgot, and ends on
sluice's own only the previews nobody can use.

## Requirements

### Requirement: Release at the caller's asking

`sluice release <preview>` SHALL run the release command of the
previewed revision's policy in the checkout under the host's executor,
with the preview's environment, names and ports; end everything started
in it; remove the checkout; and let its ports and names go (101, 34).
Releasing a preview that does not exist SHALL exit 0 saying there is
nothing to release.

#### Scenario: The next landing replaces a preview

- **WHEN** a caller asks for R2's preview and then releases R1's
- **THEN** R1's release command removes its containers and routes, its
  checkout is gone, and its ports and names are free for another
  checkout (S31)

#### Scenario: Release twice

- **WHEN** a caller releases a preview that was already released
- **THEN** sluice says there is nothing to release and exits 0

### Requirement: sluice never ends a usable preview on its own

No round, `sluice run`, `sluice step`, submission or landing SHALL
release a preview whose checkout exists and whose command did not fail,
however long it has been held.

#### Scenario: The branch moves on

- **WHEN** a preview of R1 is up and the branch lands R2 and R3
- **THEN** R1's preview is still up and still lists R1

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

### Requirement: The host's previews are listed

`sluice previews` SHALL list every preview on the host for the
repository with its name, branch, revision, addresses, state and the
time it was made; `--json` SHALL print one document on stdout. A failed
preview, its checkout and ports already let go, SHALL stay listed with
its output until its caller releases it.

#### Scenario: Reconciling after a crash

- **WHEN** a caller restarts and runs `sluice previews --json`
- **THEN** it sees R0's and R2's previews and can release R0's, whose card
  is gone (S32)
