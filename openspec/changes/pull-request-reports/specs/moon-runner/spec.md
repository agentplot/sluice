# Spec Delta

## Purpose

How the moon runner runs a combination's checks, in moon's CI mode so a
repository's own CI settings hold, and what it reads back from moon for
each task and for the run.

## ADDED Requirements

### Requirement: moon runs a combination's checks in its CI mode

The moon runner SHALL run a combination's checks with moon's CI mode on.
The targets SHALL stay the branch's required checks, narrowed to those
the combination affects (36). A task whose project marks it
`runInCI: false` SHALL NOT be run, and SHALL read as missing (27).

#### Scenario: A task kept out of CI

- **WHEN** a branch requires `#check-fast`, and `app:serve` carries that tag with `runInCI: false`
- **THEN** a combination's run does not run `app:serve`, and its result reads missing

#### Scenario: Only what is affected

- **WHEN** a combination changes files only under `app/`
- **THEN** the run runs the required checks of `app` and of what depends on it, and no other

### Requirement: sluice warns of a required check moon keeps out of CI

When a branch's required checks include a task marked
`runInCI: false`, `sluice status` and a waiting `sluice submit` SHALL
name the task and say it is never run, so the branch can never pass
while it is required.

#### Scenario: The warning

- **WHEN** `main` requires `#check-fast` and `app:serve` in it is marked `runInCI: false`
- **THEN** `sluice status main` names `app:serve` as required and never run

### Requirement: moon's times are read with each result

The moon runner SHALL read from moon's run report, for each task, how
long it took and how many times it was tried. For the run, it SHALL
read how long the run took, moon's estimate of the time it saved, and
the files moon took as changed. A replayed task's time SHALL be what
moon says restoring it took. A task moon gives no time for SHALL have
none.

#### Scenario: A retried task

- **WHEN** moon ran `app:test` twice, the second time passing, in 2m 3s in all
- **THEN** its result is recorded as passed, with 2m 3s and 2 attempts

#### Scenario: The run's estimate

- **WHEN** moon's report gives the run 4m 12s and a comparison estimate of 9m 40s
- **THEN** the combination's record holds both, and its report shows 5m 28s saved
