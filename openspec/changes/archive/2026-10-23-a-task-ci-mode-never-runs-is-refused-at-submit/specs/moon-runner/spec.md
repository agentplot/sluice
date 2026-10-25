# Spec Delta

## ADDED Requirements

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
