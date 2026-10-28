# Spec Delta

## Purpose

How a session that starts, after a restart or on another host, is told
every outcome of its own submissions that nothing told it yet, by one
command, and how each is then marked told so it is told once (51, 135;
rule core.31; ADR 0068).

## ADDED Requirements

### Requirement: One command tells a session its untold outcomes

`sluice untold`, run in a worktree, SHALL tell every submission on any
branch that landed, was blamed or conflicts, is not marked told, and
whose recorded submitter is this worktree's work: its branch, or the
branch a pull request was opened from. A submission withdrawn, or still
in a queue or under test, SHALL NOT be told.

#### Scenario: A restarted session is told a landing it missed

- **WHEN** submission 12, submitted from worktree branch `unit-a` with
  no multiplexer, lands on `main`, and a session then runs `sluice
  untold` in that worktree
- **THEN** the command tells submission 12's landing on `main`

#### Scenario: Another worktree's outcomes are not told

- **WHEN** submission 13 from worktree branch `unit-b` is blamed, and
  `sluice untold` runs in the worktree on branch `unit-a`
- **THEN** the command tells nothing of submission 13

#### Scenario: Outcomes on several branches are told together

- **WHEN** worktree branch `unit-a` has an untold landing on `main` and
  an untold conflict on `bolt/x`
- **THEN** one run of `sluice untold` in that worktree tells both

#### Scenario: A failure resubmitted is not told

- **WHEN** submission 14 from `unit-a` was blamed, and `unit-a` submitted
  again, which withdrew 14
- **THEN** `sluice untold` in that worktree tells nothing of 14

#### Scenario: A submission still queued is not told

- **WHEN** submission 15 from `unit-a` is under test
- **THEN** `sluice untold` in that worktree tells nothing of 15

### Requirement: An outcome is told in a wait's words

Without `--json`, each outcome SHALL be told by the one line the round
pushes into a pane for it, naming the submission, its branch and its
outcome, followed by what `sluice wait <id>` prints for it, outcomes
apart by a blank line. With `--json`, the command SHALL print one
document, `{"outcomes": [...]}`, each element the submission's record as
`sluice wait <id> --json` prints it, now marked told.

#### Scenario: A blame is told with its failed checks and logs

- **WHEN** submission 16 from `unit-a` was blamed for `cli:test`, and
  `sluice untold` runs in that worktree
- **THEN** the output opens with a line naming `#16`, `main`, `failed`
  and `cli:test`, followed by the lines `sluice wait 16` prints for it,
  its check's log among them

#### Scenario: The JSON document lists the records

- **WHEN** `sluice untold --json` tells submission 12's landing
- **THEN** it prints `{"outcomes": [...]}` holding submission 12's
  record with `state` `landed`, `told` and `told_to` set

### Requirement: Each outcome told is marked told to the asking session

Each outcome the command tells SHALL be marked told on its submission
with the time and the session it was told to: the multiplexer's name for
the asking session where it runs in one, otherwise the worktree's path.
The branch's log SHALL say it was told, naming the same notification the
round's push uses for that outcome. A finished submission SHALL be
marked in its own record.

#### Scenario: A landing told outside any multiplexer names the worktree

- **WHEN** `sluice untold` runs with no multiplexer in worktree
  `/w/unit-a` and tells submission 12's landing
- **THEN** `sluice status 12 --json` shows `told` with the time and
  `told_to` `/w/unit-a`, and `main`'s log holds a `told` line for 12
  naming `12:landed`

#### Scenario: A session in a herdr pane is named by its pane

- **WHEN** `sluice untold` runs in herdr pane `w1:p2` and tells
  submission 12
- **THEN** submission 12's `told_to` names that herdr server and pane
  `w1:p2`

### Requirement: An outcome is told once

The command SHALL answer an outcome already told with nothing: run again,
or for an outcome the round's push delivered, it SHALL print nothing
without `--json`, `{"outcomes":[]}` with it, and exit 0. The round SHALL
NOT push an outcome the command told. While it tells and marks a
branch's outcomes, the command SHALL hold that branch's lock, so a push
and the command never both deliver one outcome; one stopped between
telling and marking SHALL tell it again under the same notification.

#### Scenario: Asking twice tells once

- **WHEN** `sluice untold` told submission 12, and runs again in the
  same worktree
- **THEN** the second run prints nothing and exits 0

#### Scenario: An outcome the push delivered is not told again

- **WHEN** the round pushed submission 12's landing into its pane and
  marked it told, and the session then runs `sluice untold`
- **THEN** the command prints nothing

#### Scenario: The round does not push what the command told

- **WHEN** submission 12 landed and its outcome is still to be pushed,
  and `sluice untold` tells it before the next round
- **THEN** the next round prompts no pane for submission 12, and its
  `told_to` stays the one the command recorded

### Requirement: A branch that cannot be read stops no other

When a branch's records cannot be read, the command SHALL still tell the
outcomes of every branch it could read, SHALL say on stderr which branch
it could not read in the reader's own words, and SHALL exit 3. Under
`--json`, the document SHALL also hold `unread`, each with the branch
and those words.

#### Scenario: One unreadable branch

- **WHEN** worktree `unit-a` has an untold landing on `main`, and the
  records of `bolt/x` cannot be read
- **THEN** `sluice untold` tells the landing on `main`, says on stderr
  that `bolt/x` could not be read and why, and exits 3

### Requirement: A worktree on no branch is told nothing

A worktree on no branch has submitted nothing, so `sluice untold` run
there SHALL print nothing and exit 0.

#### Scenario: A detached worktree asks

- **WHEN** `sluice untold` runs in a worktree on no branch
- **THEN** it prints nothing and exits 0
