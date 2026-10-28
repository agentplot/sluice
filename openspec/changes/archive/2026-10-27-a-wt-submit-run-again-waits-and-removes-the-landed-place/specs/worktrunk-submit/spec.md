## Purpose

What `wt submit`, worktrunk's front door to sluice, does beyond `sluice submit`: it removes a landed worktree, it follows or answers with a standing submission when run again rather than submitting again, and it waits under its own `--timeout` (C.5, worktrunk; 25; 124; record 0065).

## ADDED Requirements

### Requirement: A submission stands at the revision a submit would send

A worktree's submission to a branch SHALL stand when it is the worktree's last submission to that branch, it is of the revision a submit from that worktree would send now, and it is queued, waiting, under test, passed, landing or landed. That revision SHALL be the worktree's head, or, in a worktree that has checked out a branch sluice lands on, the remote's revision of that branch (124). `wt submit` SHALL read it from sluice's status of the worktree.

#### Scenario: A worktree on a branch sluice lands on, its copy behind

- **WHEN** a worktree that has checked out `bolt/x`, which sluice lands on and keeps on `origin`, submitted `bolt/x` to `main` with `--no-wait` at origin's revision while this host's copy of `bolt/x` is behind it, and `wt submit main --timeout 9m --json` runs there with that submission still queued
- **THEN** no new submission is made, the first is not withdrawn, and the command follows it

### Requirement: A run that cannot read what stands submits nothing

Where sluice gives no answer `wt submit` can read of what stands, a run that would submit SHALL submit nothing and exit 3 with sluice's words, since a submit at the revision standing would withdraw it (67).

#### Scenario: sluice's answer cannot be read

- **WHEN** `sluice submit main --status --json` fails in the worktree with an error other than nothing submitted, and `wt submit main --timeout 9m` runs there
- **THEN** it exits 3 with sluice's words, and nothing is submitted or withdrawn

### Requirement: A run again follows a standing submission

Where a submission stands, `wt submit <branch>` SHALL submit nothing and withdraw nothing. It SHALL follow that submission as `sluice wait <branch>` does, with that wait's output and exits, and SHALL remove the worktree when it lands (C.5, worktrunk).

#### Scenario: Submitted without waiting, then run again

- **WHEN** an agent ran `wt submit main --no-wait --json` and got submission `<id>`, and then runs `wt submit main` in the same worktree with nothing committed since
- **THEN** no new submission is made, `<id>` is not withdrawn, the command runs rounds until `<id>` lands, prints `passed · landed as <rev>`, releases the worktree, removes it, and exits 0

#### Scenario: Already landed

- **WHEN** the worktree's submission `<id>` landed after a `--no-wait` submit, and `wt submit main` runs in that worktree
- **THEN** it runs no round, releases the worktree, removes it, and exits 0

#### Scenario: Landed, kept

- **WHEN** the worktree's submission `<id>` landed, and `wt submit main --no-remove` runs in that worktree
- **THEN** it exits 0, and the worktree stays and is not released

### Requirement: A no-wait run again answers with the standing submission

Where a submission stands, `wt submit <branch> --no-wait` SHALL submit nothing, withdraw nothing and remove nothing. It SHALL print what `wt submit <branch> --status` prints of that submission, under `--json` its record, and exit 0.

#### Scenario: Submitted twice without waiting

- **WHEN** an agent ran `wt submit main --no-wait --json` and got submission `<id>`, and runs `wt submit main --no-wait --json` again with nothing committed since
- **THEN** it exits 0 and prints the record of `<id>`, still queued, and `sluice status main` lists one submission from that worktree

#### Scenario: Landed, then a no-wait run again

- **WHEN** the worktree's submission `<id>` landed, and `wt submit main --no-wait --json` runs there
- **THEN** it exits 0, prints the record of `<id>` with `state` `landed`, and the worktree stays

### Requirement: Only a new head submits again where a submission stands

Where a submission stands, a `wt submit <branch>` that names `--after` or `--priority` SHALL submit nothing, exit 64, and say that the submission stands at this revision and that `sluice amend <id>` changes its place in the queue.

#### Scenario: Run again with --after

- **WHEN** a worktree's submission `<id>` is queued, and `wt submit main --no-wait --after 7` runs there with nothing committed since
- **THEN** it exits 64, `<id>` stays queued and is not withdrawn, and the message names `sluice amend <id>`

### Requirement: wt submit waits under its own time limit

`wt submit <branch> --timeout <duration>` SHALL wait at most that long, wherever it waits, following a standing submission or the one it makes. Once the limit passes with the submission undecided, it SHALL exit 5, print what `sluice wait <branch> --timeout <duration>` prints, the submission's record under `--json`, and remove nothing. `--timeout` takes what `sluice wait --timeout` takes, and is refused with exit 64 on a run that does not wait.

#### Scenario: Blamed while followed

- **WHEN** the standing submission is blamed for a failure while `wt submit main --timeout 9m` follows it
- **THEN** it exits 1 as `sluice wait` does, and the worktree stays

#### Scenario: With --no-wait

- **WHEN** `wt submit main --no-wait --timeout 9m` runs
- **THEN** it exits 64, saying `--timeout` is for a run that waits, and nothing is submitted

### Requirement: A first submit under a time limit submits and then waits

Where no submission stands, `wt submit <branch> --timeout <duration>` SHALL submit as `sluice submit <branch> --no-wait` does and then follow the new submission under that limit. Where that submit fails, it SHALL end with the submit's output and exit and wait for nothing. Under `--json`, its standard output SHALL be one document, the wait's.

#### Scenario: First submit under a time limit, then landed

- **WHEN** a worktree has never submitted to `main`, its check takes a minute, and the agent runs `wt submit main --timeout 2s --json`, then again once that exits
- **THEN** the first run makes one submission, exits 5 and prints only that submission's record with `state` `queued` or `testing`, the worktree stays, and a later run makes no new submission, exits 0 once it lands, and the worktree is gone

### Requirement: A new head, or a decided submission, is submitted again

`wt submit <branch>` SHALL submit as `sluice submit <branch>` does when the revision a submit from the worktree would send is not that of its last submission to the branch, or that submission failed, conflicted or was withdrawn, or the worktree has none (54, 67).

#### Scenario: A new commit

- **WHEN** a worktree's submission `<one>` is queued, the agent commits again, and `wt submit main --no-wait --json` runs
- **THEN** a new submission is made and `<one>` is withdrawn

#### Scenario: Blamed, the same head submitted again

- **WHEN** a worktree's submission was blamed, and `wt submit main` runs again with nothing committed since
- **THEN** a new submission of the same head is made and tested

### Requirement: A pull request stands while it is open and ready at the revision a submit would send

On a branch that takes pull requests, the worktree's submission SHALL stand while its pull request is open, out of draft, and at the revision a submit from the worktree would send, taken in yet or not, and once its submission landed there. `wt submit <branch>` run again there SHALL follow it as `sluice wait <branch>` follows a pull request, and under `--no-wait` answer with it; a draft or another head SHALL be submitted.

#### Scenario: Waiting for a review, run again under a time limit

- **WHEN** an agent's `wt submit main --no-wait` opened pull request `<n>`, which waits for a review, and the agent runs `wt submit main --timeout 2s --json`
- **THEN** nothing is pushed and the pull request is not edited, the command exits 5, and it prints `pull_request` `<n>`, `branch` and `state` `waiting: needs 1 review`

#### Scenario: Waiting for a review, run again without waiting

- **WHEN** an agent's pull request `<n>` waits for a review, and the agent runs `wt submit main --no-wait --json` with nothing committed since
- **THEN** nothing is pushed and the pull request is not edited, the command exits 0, and it prints `pull_request` `<n>` with its `state`

#### Scenario: Sent back as a draft

- **WHEN** an agent's pull request was blamed and marked a draft, and the agent runs `wt submit main` with nothing committed since
- **THEN** the pull request is marked ready for review at that head, as a submit does

### Requirement: A worktree is removed only on its own landing

`wt submit` SHALL remove a worktree only when sluice records the worktree's last submission to the branch as landed and of the revision a submit from the worktree would send now, after `sluice release` in it succeeded, and only where it waited, `--no-remove` is not given and worktrunk's `[merge] remove` is not false; a run that follows a submission SHALL remove on the same terms as a waiting submit (C.4; 25).

#### Scenario: A commit made while followed

- **WHEN** `wt submit main --timeout 9m` follows submission `<id>`, and the agent commits in the worktree before `<id>` lands
- **THEN** when `<id>` lands the command exits 0, says sluice records no landing of the worktree's head, and the worktree stays
