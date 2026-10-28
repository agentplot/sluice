# Spec Delta

## Purpose

How a session that starts, after a restart or on another host, or a
caller's place that never waited, collects every outcome pending for its
own submissions by one command, `sluice inbox`, each then marked
delivered so it is delivered once (51, 135; rule core.31; ADRs 0068,
0070; flume 133, flume C.6).

## ADDED Requirements

### Requirement: sluice inbox collects a worktree's pending outcomes

`sluice inbox`, run in a worktree, SHALL deliver every submission on
any branch that landed, was blamed or conflicts, is not marked
delivered, and whose recorded submitter is this worktree's work: its
branch, or the branch a pull request was opened from. A submission
withdrawn, or still queued or under test, SHALL NOT be delivered.

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

### Requirement: An outcome is delivered in a wait's words

Without `--json`, each outcome SHALL be printed as the notice the round
pushes into a pane for it, naming the submission, its branch and its
outcome, followed by what `sluice wait <id>` prints for it, outcomes
apart by a blank line, and nothing at all when nothing is pending.

#### Scenario: A blame is delivered with its failed checks and logs

- **WHEN** submission 16 from `unit-a` was blamed for `cli:test`, and
  `sluice inbox` runs in that worktree
- **THEN** the output opens with a line naming `#16`, `main`, `failed`
  and `cli:test`, followed by the lines `sluice wait 16` prints for it,
  its check's log among them

### Requirement: With --json, the inbox prints one document of records

With `--json`, `sluice inbox` SHALL print one document holding
`outcomes`, a list, `[]` when nothing is pending, and `unread` only when
a branch could not be read. Each element SHALL be the submission's
record as `sluice wait <id> --json` prints it, with `delivered` and
`delivered_to` as this run marks them, and `notice`, the one line.

#### Scenario: Nothing pending

- **WHEN** `sluice inbox --json` runs where nothing is pending
- **THEN** it prints a document whose `outcomes` is `[]`, holds no
  `unread`, and exits 0

### Requirement: The JSON document is a contract that only gains fields

Each element of `outcomes` SHALL hold `id`, `branch`, `state`
(`landed`, `failed` or `conflict`), `notice`, `delivered`,
`delivered_to` and `from`; a landed one `landed_as`; a failed one
`result` (`failed`, or `timed out`) and `failed`, each with its `task`
and, where kept, its `log`; a conflicting one `conflict` with its
`paths` and its `with`. A later sluice SHALL keep these names and
meanings, and MAY add fields.

#### Scenario: The JSON document lists the records

- **WHEN** `sluice inbox --json` delivers submission 12's landing on
  `main` as `abc1234`
- **THEN** it prints one document whose `outcomes` holds one element
  with `id` 12, `branch` `main`, `state` `landed`, `landed_as`
  `abc1234`, `delivered`, `delivered_to`, and a `notice` naming `#12`
  and `landed`

#### Scenario: A conflict names its paths

- **WHEN** `sluice inbox --json` delivers submission 14's conflict with
  submission 11 in `src/lib.rs`
- **THEN** its element holds `state` `conflict`, `conflict.paths`
  holding `src/lib.rs`, and `conflict.with` holding submission 11

### Requirement: Each outcome collected is marked delivered to the collecting session

Each outcome the command delivers SHALL be marked delivered on its
submission with the time and the session it was delivered to: the
multiplexer's name for the collecting session where it runs in one,
otherwise the worktree's path. The branch's log SHALL hold a `delivered`
line naming the same notification the round's push uses for that
outcome. A finished submission SHALL be marked in its own record.

#### Scenario: A landing collected outside any multiplexer names the worktree

- **WHEN** `sluice inbox` runs with no multiplexer in worktree
  `/w/unit-a` and delivers submission 12's landing
- **THEN** `sluice status 12 --json` shows `delivered` with the time and
  `delivered_to` `/w/unit-a`, and `main`'s log holds a `delivered` line
  for 12

#### Scenario: A session in a herdr pane is named by its pane

- **WHEN** `sluice inbox` runs in herdr pane `w1:p2` and delivers
  submission 12
- **THEN** submission 12's `delivered_to` names that herdr server and
  pane `w1:p2`

### Requirement: An outcome is delivered once

The command SHALL print nothing for an outcome already delivered: run
again, or for an outcome the round's push or a wait delivered, it SHALL
print nothing without `--json`, `outcomes` `[]` with it, and exit 0.

#### Scenario: Collecting twice delivers once

- **WHEN** `sluice inbox` delivered submission 12, and runs again in the
  same worktree
- **THEN** the second run prints nothing and exits 0

#### Scenario: An outcome the push delivered is not delivered again

- **WHEN** the round pushed submission 12's landing into its pane and
  marked it delivered, and the session then runs `sluice inbox`
- **THEN** the command prints nothing

### Requirement: The command and the round never both deliver one outcome

While it prints and marks a branch's outcomes, the command SHALL hold
that branch's lock, so the round SHALL NOT push an outcome the command
delivered, nor the command print one the round pushed. A command stopped
between printing and marking SHALL leave the outcome pending, to be
delivered again under the same notification.

#### Scenario: The round does not push what the command delivered

- **WHEN** submission 12 landed in a herdr pane whose push was refused,
  and `sluice inbox` then delivers it
- **THEN** no later round prompts any pane for submission 12, and its
  `delivered_to` stays the one the command recorded

### Requirement: A branch that cannot be read stops no other

When a branch's records cannot be read, the command SHALL still deliver
the outcomes of every branch it could read, SHALL say on stderr which
branch it could not read in the reader's own words, and SHALL exit 3.
Under `--json`, the document SHALL also hold `unread`, each with the
branch and those words.

#### Scenario: One unreadable branch

- **WHEN** worktree `unit-a` has a pending landing on `main`, and the
  records of `bolt/x` cannot be read
- **THEN** `sluice inbox` delivers the landing on `main`, says on stderr
  that `bolt/x` could not be read and why, and exits 3

### Requirement: A worktree on no branch has nothing pending

A worktree on no branch has submitted nothing, so `sluice inbox` run
there SHALL print nothing (`outcomes` `[]` under `--json`) and exit 0.

#### Scenario: A detached worktree collects

- **WHEN** `sluice inbox` runs in a worktree on no branch
- **THEN** it prints nothing and exits 0

### Requirement: A place submitted with no session collects its outcome by inbox

A submission made by `sluice submit <branch> --no-wait --json` with no
multiplexer's variables in its environment SHALL record no session; the
round that decides it SHALL push nothing, log one `pending` line, and
try no more; `sluice inbox --json` run in the submitting worktree SHALL
then deliver the outcome once and mark it delivered to that worktree's
path. `sluice step <branch>` SHALL run one round of the branch's queue,
its outbox included, as it does without this change.

#### Scenario: A caller that never waits collects a failure

- **WHEN** a caller runs `sluice submit main --no-wait --json` in
  worktree `/repo/places/unit-a` with no herdr variables, then `sluice
  step main` until submission 21 is blamed for `cli:test`, then `sluice
  inbox --json` in that worktree twice
- **THEN** `main`'s log holds one `pending` line for 21 and no further
  attempt; the first `sluice inbox --json` prints one element with `id`
  21, `state` `failed`, `failed` holding `cli:test`, a `notice` naming
  `#21`, and `delivered_to` `/repo/places/unit-a`; the second prints
  `outcomes` `[]`

#### Scenario: A caller that never waits collects a landing

- **WHEN** the same caller's submission 22 lands on `main` as `def5678`
  by `sluice step main`, and `sluice inbox --json` runs in its worktree
- **THEN** it prints one element with `id` 22, `state` `landed` and
  `landed_as` `def5678`
