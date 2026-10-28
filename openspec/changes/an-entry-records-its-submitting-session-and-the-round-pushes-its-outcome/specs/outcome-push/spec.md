# Spec Delta

## Purpose

How a submission's outcome reaches the session that submitted it: the
session recorded at submit, the outcome pushed into its pane by the
round that decides it, and the submission marked told only when that
push delivered (20, 50, 51, C.5 worktrunk; rule core.31; ADR 0068).

## ADDED Requirements

### Requirement: A submission records the session that submitted it

A submit run in a herdr pane SHALL record, on the submission, the host
it ran on and the session as herdr names it: the herdr server and the
pane. A submit run in no multiplexer SHALL record no session. The
session SHALL be shown in the submission's record in `sluice wait
--json` and `sluice status --json`.

#### Scenario: A submit in a herdr pane records its pane

- **WHEN** `wt submit main --no-wait --json` runs in herdr pane `w1:p2`
  of the herdr server whose socket is `/run/herdr.sock`, on host `deneb`
- **THEN** the submission's record names host `deneb` and a session
  naming that server and pane `w1:p2`

#### Scenario: A submit outside any multiplexer records no session

- **WHEN** `wt submit main --no-wait --json` runs with no herdr
  variables in its environment
- **THEN** the submission's record names no session

### Requirement: The round that decides an outcome pushes it into the submitter's pane

When a round decides a submission landed, failed or conflicted, and the
submission recorded a session on this host, the round SHALL push the
outcome into that pane as a prompt to the agent there: one line naming
the submission's number, its branch and its outcome, and for a landing
the revision it landed as, for a failure the failed checks, for a
conflict the submissions it conflicts with.

#### Scenario: A landing is pushed into the submitting pane

- **WHEN** submission 12, submitted from herdr pane `w1:p2` on this
  host, lands on `main` as `abc1234`
- **THEN** the round prompts the agent in pane `w1:p2` once, with a line
  naming `#12`, `main`, `landed` and `abc1234`

#### Scenario: A failure is pushed with its failed checks

- **WHEN** submission 13, submitted from a herdr pane on this host, is
  blamed for `cli:test`
- **THEN** the line pushed into its pane names `#13`, `failed` and
  `cli:test`

#### Scenario: No other pane is prompted

- **WHEN** submissions 12 and 13 are tested together and 13 is blamed
- **THEN** only 13's pane is told of the failure, and 12's pane is told
  nothing of it

### Requirement: A submission is marked told only when its outcome was delivered

A submission SHALL be marked told only when its outcome was delivered:
herdr took the prompt into the pane, or the pull request driver posted
it on the request. The record SHALL say when and to which session. An
outcome not delivered SHALL leave the submission decided and untold.

#### Scenario: A delivered outcome is recorded with its session

- **WHEN** herdr takes the prompt for submission 12 into pane `w1:p2`
- **THEN** submission 12's record shows `told` with the time and
  `told_to` with the session naming pane `w1:p2`

#### Scenario: A submission outside any multiplexer stands untold

- **WHEN** submission 14, which recorded no session, lands
- **THEN** no pane is prompted, its record shows it landed and shows
  neither `told` nor `told_to`

#### Scenario: A refused prompt marks nothing

- **WHEN** herdr refuses the prompt for submission 15, its pane gone or
  its agent waiting on a question
- **THEN** submission 15 shows no `told`, and the branch's log has one
  line saying its outcome was not delivered, with herdr's words

#### Scenario: A session on another host is not prompted from this one

- **WHEN** the round that decides submission 16 runs on a host other
  than the one 16 recorded
- **THEN** no pane on the round's host is prompted, and 16 shows no
  `told`

### Requirement: An outcome is pushed once

A round SHALL push an outcome at most once: one that was not delivered
SHALL NOT be pushed again by a later round, and none SHALL be pushed for
a submission already marked told. A round that stopped between a
delivery and its record MAY push it again under the same submission and
outcome.

#### Scenario: An undelivered outcome is not pushed again

- **WHEN** the push for submission 15 was refused, and later rounds run
  on `main`
- **THEN** no later round prompts any pane for submission 15

#### Scenario: An outcome already told is not pushed

- **WHEN** submission 17 is marked told before the round that would push
  its outcome
- **THEN** the round prompts no pane for 17 and its `told` is unchanged

### Requirement: A record written before reads as it was

A submission recorded before sessions were, with `told` and no
`told_to`, SHALL read and show as it was written, and a record that
holds `told_to` SHALL stay readable by a sluice that does not know it.

#### Scenario: An old told record is read

- **WHEN** a branch's records hold submission 9 with `"told":
  "2026-10-27T10:00:00Z"` and no `told_to`
- **THEN** `sluice wait 9 --json` shows `told` as
  `2026-10-27T10:00:00Z` and no `told_to`
