# Spec Delta

## Purpose

Says what happens when sluice releases a checkout it made or a
worktree's place it adopted: a release that fails holds the checkout,
never the branch, and a checkout whose working copy is gone is freed
with nothing run (123, 25, 34).

## ADDED Requirements

### Requirement: A failed release holds its checkout, named with what failed

When the release of a combination's checkout fails, sluice SHALL keep
the checkout and SHALL record in the branch's state its name, what
failed and how many times its release was tried; the branch's log SHALL
hold a `release_failed` event naming it and what failed (123).

#### Scenario: The release command fails

- **WHEN** a combination's result is read and its checkout's release
  command exits 2 printing `usage: scripts/preview up`
- **THEN** the checkout still exists, `sluice status <branch> --json`
  lists it with that output and one try, and the log holds
  `release_failed` naming it

### Requirement: A failed release is tried again a bounded number of times

sluice SHALL try a held combination's checkout's release again on each
round, as many more times as the branch's `release_retries` states, read
from the branch's current revision, or 3 times when it states none. Once
they are spent sluice SHALL stop trying and SHALL report the checkout
held, once, in the round's output and in its branch's log (123, 56).

#### Scenario: No retries named

- **WHEN** a checkout's release fails on every try and the policy names
  no `release_retries`
- **THEN** the release is run four times in all, over four rounds, and
  the fifth and later rounds run nothing for it and print nothing of it

#### Scenario: Retries named

- **WHEN** the branch's policy says `release_retries = 1` and the
  release fails twice
- **THEN** the checkout is reported held after the second failure, with
  what failed

#### Scenario: A retry succeeds

- **WHEN** a checkout's release fails once and succeeds on the next
  round
- **THEN** the checkout is gone, its ports are free, the branch's state
  no longer lists it, and the log holds `released` naming it

### Requirement: A held checkout never holds its branch

Whatever a release does, the round SHALL go on with the branch's work:
it SHALL land what passed, tell outcomes, and form and start
combinations in other checkouts. A failed release SHALL never be
reported as the submission's outcome (123, 122).

#### Scenario: The queue goes on

- **WHEN** a combination's checkout's release fails and another
  submission is waiting
- **THEN** the same round starts the waiting submission's combination in
  a new checkout, and it lands when it passes

#### Scenario: A checkout left from a run that never started

- **WHEN** a round stopped after deciding a checkout and before its run
  started, and the checkout's release fails on the next round
- **THEN** the run is opened again and started in another checkout, and
  the failed checkout is held as above

### Requirement: A returning host's held checkout stops only that host

A host that releases the checkouts it left before it claims (81) SHALL
treat a failed release as above: held, named, tried again, then reported.
That host SHALL claim no run while it holds such a checkout (81); every
other host SHALL go on claiming the branch's runs.

#### Scenario: Back with a checkout that will not release

- **WHEN** host A returns with a checkout left from a lapsed claim whose
  release fails, and host B is up
- **THEN** host A claims nothing and shows the checkout held, and host B
  claims and reports the branch's open runs

### Requirement: The operator releases a held checkout by its name

`sluice release <name>` SHALL release the held checkout of that name on
this host, running its release once more; on success the checkout is
gone and its branch's state no longer lists it, and on failure it stays
held with what failed and the command exits 1. A name that names no
checkout SHALL exit 0 saying there is nothing to release (123).

#### Scenario: Released once more

- **WHEN** the operator fixes what the release needed and runs `sluice
  release sluice/try/main/48`
- **THEN** the release command runs, the checkout is removed, its ports
  are free, and the log holds `released` naming it

#### Scenario: Still failing

- **WHEN** the operator runs `sluice release sluice/try/main/48` and the
  release fails again
- **THEN** the command prints what failed, exits 1, and the checkout is
  still listed held

### Requirement: The operator may accept a release's failure

`sluice release <name> --accept-failure` SHALL free what sluice itself
holds for the checkout without running its release command: end its
runs, remove the checkout sluice made, its scratch directory and record,
and let its ports go. The log SHALL hold `released` naming it and the
failure accepted. In a worktree, `sluice release --accept-failure` SHALL
do the same for the worktree's place and leave the worktree (123).

#### Scenario: Accepted

- **WHEN** a checkout is held because its release command fails, and the
  operator runs `sluice release sluice/try/main/48 --accept-failure`
- **THEN** the release command is not run, the checkout is removed, its
  ports are free for the next checkout, and `sluice status main` no
  longer lists it

### Requirement: A worktree's or a preview's failed release holds it

A failed `sluice release` in a worktree, or a failed release of a
preview, SHALL leave the place held with what failed: `sluice status`
lists the worktree with it, and `sluice previews` the preview. sluice
SHALL try a preview's release again as a combination's checkout's is,
and SHALL never release a worktree whose directory exists on its own
(123, 25, 101).

#### Scenario: A worktree's release fails

- **WHEN** an agent runs `sluice release` in its worktree and the
  release command exits 1
- **THEN** the command exits 1 with the command's output, the worktree
  keeps its ports, and `sluice status` lists it with what failed, and no
  round releases it

### Requirement: A checkout whose working copy is gone is freed at once

When the working copy of a checkout sluice holds no longer exists,
whether a worktree's, a combination's or a preview's, the next run or
round on the host SHALL end its runs and let go its scratch directory,
ports and record, and SHALL run nothing in its name: no toolchain and no
release command. Its branch's log SHALL hold `gone` naming it, and
sluice SHALL never try it again (123).

#### Scenario: A worktree removed by hand

- **WHEN** an agent's worktree that ran `sluice exec` is removed with
  `rm -rf`, and another agent runs `sluice submit` and waits
- **THEN** the first round says once that the worktree is gone and its
  ports are free, the toolchain and the release command never run, the
  log holds `gone` naming it, and no later round mentions it

#### Scenario: A combination's checkout deleted

- **WHEN** a combination's checkout directory is deleted before its
  result is read
- **THEN** on release nothing is run in it, git no longer lists it as a
  worktree, its ports are free, and the log holds `gone` naming it
