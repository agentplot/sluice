# moon-runner Specification

## Purpose
How the moon runner runs a combination's checks, in moon's CI mode so a
repository's own CI settings hold, and what it reads back from moon for
each task and for the run.

## Requirements

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

### Requirement: A submission is refused when its branch requires a check moon keeps out of CI

`sluice submit` SHALL refuse a submission when the branch's required
checks, read from the branch's current revision (56), include a task
that moon, in the submitting worktree, marks `runInCI: false` or
`runInCI: skip`. The refusal SHALL name each such task and the line of
`.config/sluice.toml` that requires it, with the tag when the task is
required through one, and say how to go on. Nothing SHALL be queued, and
on a branch that takes pull requests nothing SHALL be pushed.

#### Scenario: A required task kept out of CI

- **WHEN** `main`'s policy requires `root:integration` on line 59 of
  `.config/sluice.toml`, and the submitting worktree's `moon.yml` marks
  `root:integration` with `runInCI: false`
- **THEN** `sluice submit main` is refused naming `root:integration` and
  line 59, `sluice status main` lists no new submission, and the submit
  exits 2

#### Scenario: A task kept out through a required tag

- **WHEN** `main` requires `#check-fast` on line 39, and `app:serve`
  carries that tag with `runInCI: false`
- **THEN** `sluice submit main` is refused naming `app:serve` as
  required through `#check-fast` on line 39

#### Scenario: A branch that takes pull requests

- **WHEN** `main` takes pull requests and requires a task the submitting
  worktree keeps out of CI
- **THEN** `sluice submit main` is refused the same way, the worktree's
  branch is not pushed, and no request is opened or marked ready

### Requirement: A submission whose required checks all run is taken as before

A submission whose worktree lets every required task run in CI SHALL be
taken as before, whatever the branch's own revision marks. When moon
cannot be asked in the worktree, the submission SHALL be taken, with a
warning saying why the check could not be made.

#### Scenario: The submission that lets the task run

- **WHEN** `main`'s revision marks `root:integration` with
  `runInCI: false`, and the submission changes `moon.yml` so it runs in
  CI
- **THEN** the submission is taken, and its combination runs
  `root:integration`

#### Scenario: Every required check runs

- **WHEN** no task `main` requires is kept out of CI in the submitting
  worktree
- **THEN** the submission is taken and answered exactly as before

#### Scenario: moon cannot be asked

- **WHEN** the policy's toolchain fails in the submitting worktree
- **THEN** the submission is taken, and the submit's answer warns that
  it could not check which required tasks moon keeps out of CI, with the
  toolchain's error
