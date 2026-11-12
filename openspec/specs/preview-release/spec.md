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
its time limit, keeping the command's output in the preview's state.
This SHALL happen when the outcome is read: by `sluice preview`,
`sluice previews`, or the next round on the host (101).

#### Scenario: A command fails

- **WHEN** a preview command exits 1
- **THEN** the preview is reported `failed` with the command's output and
  its release command has run, its ports and names freed (S33)

### Requirement: A preview whose checkout is gone is freed

A preview whose checkout no longer exists SHALL have its stored release
command run once, in its scratch directory with its ports, and SHALL
then be freed, its runs ended and its ports and names let go, whatever
that command did, and never tried again (123). This SHALL happen when
the outcome is read: by `sluice preview`, `sluice previews`, or the next
round on the host (101).

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

### Requirement: A preview's containers end with it, however it is let go

Whichever way a preview is let go, sluice SHALL remove every container
the host's container engine holds for it. The four ways are `sluice
release`, a release because its command failed, a free because its
checkout is gone, and `sluice release --accept-failure`. Each container
SHALL be removed after the release command, where one runs, and before
the checkout is removed and the ports and names are let go (34;
previews-1 clause 101; rule core.10).

#### Scenario: The release command leaves a container

- **WHEN** a preview is up, the engine holds a container that mounts a
  file of its checkout, and its caller runs `sluice release <preview>`
  with a release command that does not remove it
- **THEN** the release command runs and the container is removed, and
  only then are the checkout removed and the port and names freed

#### Scenario: A checkout removed by hand

- **WHEN** a preview's checkout is removed with `git worktree remove`
  while the engine holds a container that mounts a file of it, and the
  release command is a script in the checkout
- **THEN** the next `sluice previews` runs the stored release command
  once in the preview's scratch directory, where it fails, removes the
  container all the same, and frees the preview

#### Scenario: A release failure accepted

- **WHEN** a preview is held because its release command fails, the
  engine holds a container that publishes one of its ports, and the
  operator runs `sluice release <preview> --accept-failure`
- **THEN** the container is removed, no release command runs, and the
  preview is freed

#### Scenario: A failed command

- **WHEN** a preview's command starts a container labelled `sluice.place`
  with its `SLUICE_PLACE` and then exits 1
- **THEN** the preview is reported `failed` with its output, its release
  command has run, and the container is removed

### Requirement: A container is a preview's by its mount, its port or its label

A container SHALL be the preview's when the engine reports that it does
one of these: mounts a path inside the preview's checkout or its scratch
directory, publishes one of the preview's ports on the host, or carries
the label `sluice.place` with the preview's `SLUICE_PLACE` as its value.
sluice SHALL leave every other container as it is.

#### Scenario: Another checkout's container is left

- **WHEN** a preview is released while the engine holds one container
  that mounts a file of its checkout, one that publishes its `web` port,
  one labelled `sluice.place` with its `SLUICE_PLACE`, and one of another
  checkout that does none of these
- **THEN** the first three are removed and the fourth is left as it is

### Requirement: Every engine that answers is asked

sluice SHALL ask every engine that answers where a run's engine is
looked for. It SHALL stop a running container and remove it with its
anonymous volumes. A container already gone SHALL count as removed.
Where no engine answers, there is nothing to remove, and the preview
SHALL be let go as before.

#### Scenario: A host with no container engine

- **WHEN** no engine answers at any place sluice looks for one and a
  caller releases a preview
- **THEN** the preview is released as before and `sluice release` exits 0

### Requirement: A container sluice could not remove keeps its preview

When an engine answers but a container of the preview's cannot be listed
or removed, sluice SHALL keep the preview: its checkout, its ports and
names, and its record. What failed SHALL name each container left and
the engine's words.

#### Scenario: The engine refuses a removal

- **WHEN** a preview is up, the engine holds a container that publishes
  one of its ports and answers its removal with a server error, and its
  caller runs `sluice release <preview>`
- **THEN** `sluice release` exits 1 naming the container and the
  engine's words, `sluice previews` still lists the preview with its
  release failure, and its port stays held

### Requirement: A preview kept by its container is tried again

A release by `sluice release` or after a failed command that a container
kept SHALL be tried again as a failing release command is, and held once
its branch's tries are spent (123). A free because the checkout is gone,
or by `--accept-failure`, SHALL fail with what failed. The next `sluice
previews`, `sluice preview` or round on the host SHALL try a gone
preview's containers again. Its stored release command SHALL never run a
second time.

#### Scenario: The engine removes it on a later try

- **WHEN** a preview's checkout is gone, the engine refuses to remove its
  container on the first `sluice previews` and removes it on the second
- **THEN** the first listing keeps the preview, naming what failed, the
  second frees it, and the stored release command ran once in all
