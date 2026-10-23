# Spec Delta

## Purpose

Lets a submitter hand sluice several worktrees' heads as one act, in an
order, so that all of them are queued or none is, and no combination is
formed on any of them until every one is queued (125).

## ADDED Requirements

### Requirement: Several worktrees are submitted as one act

`sluice submit <branch>` given `--worktree` more than once SHALL submit
each named worktree's head to `<branch>`, in the order given, as one
act. One `--worktree`, or none, SHALL submit as `sluice submit` does
today.

#### Scenario: Three worktrees in order

- **WHEN** `sluice submit main --worktree ../b1 --worktree ../b2 --worktree ../b3 --no-wait` runs and each worktree holds only its commits
- **THEN** three submissions to `main` are queued, numbered in the order given, and the output names each one with its worktree

#### Scenario: One worktree

- **WHEN** `sluice submit main --worktree ../b1` runs
- **THEN** it submits and answers exactly as `sluice submit main` run in `../b1` does

### Requirement: All are queued or none is

When any named worktree would be refused (changes beyond its commits, no
branch, named twice, or an `--after` the branch does not hold), the act
SHALL queue nothing, push nothing, name every refused worktree with
why, and exit 2.

#### Scenario: One worktree holds uncommitted changes

- **WHEN** `sluice submit main --worktree ../b1 --worktree ../b2` runs and `../b2` holds an uncommitted file
- **THEN** no submission is queued for either, the output names `../b2` and the file, and it exits 2

#### Scenario: A worktree named twice

- **WHEN** `sluice submit main --worktree ../b1 --worktree ../b1` runs
- **THEN** nothing is queued and the output says `../b1` is named twice

### Requirement: No combination is formed until every one is queued

The submissions of one act SHALL become visible to every reader of the
branch's state at once: no round, waiting submit or `sluice wait`, on
this host or another, SHALL ever read some of them queued and not the
rest, and so none forms a combination on any of them alone.

#### Scenario: A round running beside the act

- **WHEN** `sluice run` is running for `main` while three worktrees are submitted as one act
- **THEN** the first combination formed on any of them holds all three, in the order given, and the records hold no combination of the first alone

#### Scenario: S59, one candidate for the set

- **WHEN** three approved bolts are submitted as one act to a branch with `batch = true`, deepest first
- **THEN** the next round forms one combination of all three and tests it once; it passes, and the three land in one move, in the order given

### Requirement: Each lands after the one before

Each submission of an act SHALL be ordered after the one named before
it by its order of submission. `--after` and `--priority` SHALL apply to
every submission of the act alike. No submission of the act SHALL name
another of the act as one it lands after.

#### Scenario: Waiting on an earlier submission

- **WHEN** `sluice submit main --after 40 --worktree ../b1 --worktree ../b2` runs and submission 40 is still queued
- **THEN** both wait for 40, and once 40 is ahead of them they come in the order `../b1`, `../b2`

#### Scenario: Another submission with higher priority

- **WHEN** a later submission with higher priority is queued before the group is in any combination under test
- **THEN** it may come ahead of the group's submissions, which keep their order among themselves

### Requirement: Each is thereafter its own submission

Once queued, each submission of an act SHALL be blamed, set aside for a
conflict, withdrawn, told its outcome and waited on alone, as any
submission is.

#### Scenario: S59, the middle one conflicts

- **WHEN** three heads are submitted as one act and the second conflicts with the first as the combinations form
- **THEN** the second alone is set aside as a conflict, naming the first and the files contested, and the first and third land

#### Scenario: The middle one is blamed

- **WHEN** three heads are submitted as one act and the combination of all three fails on the second's work
- **THEN** the second alone is blamed, and the first and third are tested again without it and land

#### Scenario: One withdrawn

- **WHEN** `sluice withdraw <n>` names the first submission of an act
- **THEN** it alone is withdrawn, and the others are tested and land without it

#### Scenario: Waited on alone

- **WHEN** `sluice wait <n>` names the second submission of an act
- **THEN** it reports that submission's outcome alone and exits as a wait on any submission does

### Requirement: The answer names every submission and its outcome

A waiting act SHALL run rounds until every one of its submissions is
decided, then report each in the order given, a conflict naming what it
is contested with. It SHALL exit 0 when every one landed, otherwise
with the exit of the first in the order given that did not land, and
130 when stopped with Ctrl-C. With `--no-wait` it SHALL exit 0 once all
are queued.

#### Scenario: All land

- **WHEN** a waiting act of two submissions runs and both land
- **THEN** it prints each one's outcome in order and exits 0

#### Scenario: The second conflicts

- **WHEN** a waiting act of three submissions runs and the second is set aside as a conflict while the others land
- **THEN** it prints the first landed, the second's conflict with the files and the submission it is contested with, the third landed, and exits 2

### Requirement: The act's document

Under `--json` the act SHALL print one document on stdout however it
ends: `{"branch": <branch>, "submissions": [...]}`, one item per
worktree in the order given, each with `worktree`. With `--no-wait` an
item SHALL hold `id` and `ahead`; when waiting, it SHALL be the
submission's record as `sluice status <n> --json` prints it. A refusal
SHALL print the refused error document naming each refused worktree.

#### Scenario: No wait

- **WHEN** `sluice submit main --worktree ../b1 --worktree ../b2 --no-wait --json` runs
- **THEN** it prints one document whose `submissions` holds two items, `../b1`'s first, each with `id`, `worktree` and `ahead`

#### Scenario: Waited to the end

- **WHEN** `sluice submit main --worktree ../b1 --worktree ../b2 --json` waits until both land
- **THEN** it prints one document whose `submissions` are the two records, each with `state` `landed`

### Requirement: Several worktrees take neither --check nor --status

`sluice submit` given `--worktree` more than once with `--check` or
`--status` SHALL exit 64 and say each takes one worktree.

#### Scenario: Check over two worktrees

- **WHEN** `sluice submit main --check --worktree ../b1 --worktree ../b2` runs
- **THEN** it exits 64, saying `--check` takes one worktree, and writes nothing

### Requirement: Under pull requests the act opens every pull request

To a branch that takes pull requests, the act SHALL push each worktree's
branch and open its pull request, or update the open one, each marked as
one of the group with its place and the group's size, and SHALL answer
naming every pull request with what it waits for. A refused worktree
SHALL push nothing for any.

#### Scenario: Three pull requests

- **WHEN** `sluice submit main --worktree ../b1 --worktree ../b2 --worktree ../b3 --no-wait` runs and `main` takes pull requests
- **THEN** the three branches are on the remote, each has an open pull request into `main` marked 1, 2 and 3 of 3 of one group, and the output names the three pull requests with what each waits for

#### Scenario: A waiting act follows each pull request

- **WHEN** the same act runs without `--no-wait` and all three pull requests land
- **THEN** it reports each one's states as it reaches them, then each one's outcome in the order given, and exits 0
