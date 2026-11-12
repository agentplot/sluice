## ADDED Requirements

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
