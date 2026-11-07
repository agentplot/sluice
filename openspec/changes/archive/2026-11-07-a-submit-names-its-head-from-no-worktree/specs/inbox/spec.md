## MODIFIED Requirements

### Requirement: sluice inbox collects a worktree's pending outcomes

`sluice inbox`, run in a worktree, SHALL deliver every submission on
any branch that landed, was blamed or conflicts, is not marked
delivered, and is this worktree's: one it submitted, whose recorded
work is its branch and which names no `from.head`; or a pull request's
opened from its branch, where that branch is one sluice lands on
nowhere. A submission withdrawn, or still queued or under test, SHALL
NOT be delivered.

#### Scenario: A restarted session collects a landing it missed

- **WHEN** submission 12, submitted from worktree branch `unit-a` with
  no multiplexer, lands on `main`, and a session then runs `sluice inbox`
  in that worktree
- **THEN** the command delivers submission 12's landing on `main`

#### Scenario: Another worktree's outcomes are not delivered

- **WHEN** submission 13 from worktree branch `unit-b` is blamed, and
  `sluice inbox` runs in the worktree on branch `unit-a`
- **THEN** the command delivers nothing of submission 13

#### Scenario: Outcomes on several branches are collected together

- **WHEN** worktree branch `unit-a` has a pending landing on `main` and
  a pending conflict on `bolt/x`
- **THEN** one run of `sluice inbox` in that worktree delivers both

#### Scenario: A failure resubmitted is not delivered

- **WHEN** submission 14 from `unit-a` was blamed, and `unit-a` submitted
  again, which withdrew 14
- **THEN** `sluice inbox` in that worktree delivers nothing of 14

#### Scenario: A submission still under test is not delivered

- **WHEN** submission 15 from `unit-a` is under test
- **THEN** `sluice inbox` in that worktree delivers nothing of 15

#### Scenario: A pull request from the worktree's branch is collected

- **WHEN** worktree branch `unit-a`, which no `[branches]` table names,
  pushed `unit-a` and opened a pull request into `main`, whose
  submission 16 is blamed and not delivered, and `sluice inbox` runs in
  that worktree
- **THEN** the command delivers submission 16's failure

## ADDED Requirements

### Requirement: No worktree collects a submission by name or a request from a branch sluice lands on

`sluice inbox` SHALL NOT deliver, in any worktree, a submission by name,
whose `from.head` and `from.worktree` are both the branch it named
(124), the worktree on that branch among them; nor a pull request's
opened from a branch sluice lands on, since no worktree pushed that
branch (1, 124).

#### Scenario: A submission by name is no worktree's

- **WHEN** submission 17, `bolt/x` submitted to `main` by name, landed,
  and `sluice inbox` runs in a worktree that has checked out `bolt/x`
- **THEN** the command delivers nothing of submission 17

#### Scenario: A pull request from a branch sluice lands on is no worktree's

- **WHEN** `bolt/x` is a branch sluice lands on, submission 18 is a pull
  request's from `bolt/x` into `main`, blamed and not delivered, and
  `sluice inbox` runs in a worktree that has checked out `bolt/x`
- **THEN** the command delivers nothing of submission 18
