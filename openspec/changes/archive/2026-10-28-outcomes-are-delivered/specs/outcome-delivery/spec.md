# Spec Delta

## Purpose

How a submission's outcome is delivered to the session that submitted
it: the session recorded at submit, a notice pushed into its pane by the
round that decides the outcome, a reporting wait, and the submission
marked delivered only on a carrier's word, pending until then (20, 50,
51, the C.5 worktrunk row; rule core.31; ADRs 0068, 0070).

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

- **WHEN** `sluice submit main --no-wait --json` runs with no herdr
  variables in its environment
- **THEN** the submission's record names no session

### Requirement: The round that decides an outcome pushes a notice into the submitter's pane

When a round decides a submission landed, failed or conflicted, and the
submission recorded a session on this host, the round SHALL push a
notice into that pane as a prompt to the agent there: one line opening
`[sluice]`, naming the submission's number, its branch and its outcome,
and for a landing the revision it landed as, for a failure the failed
checks or that the time limit ended it, for a conflict the submissions
it conflicts with. No other session SHALL be sent the notice.

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
- **THEN** only 13's pane is sent the failure, and 12's pane is sent
  nothing of it

#### Scenario: A round run from another pane reaches the recorded server

- **WHEN** the round that decides submission 12 runs in a herdr pane of
  another server
- **THEN** the prompt goes to the server and pane submission 12
  recorded, and none of the round's own pane

### Requirement: A submission is marked delivered only when its outcome was delivered

A submission SHALL be marked delivered only when a carrier delivered its
outcome: herdr took the prompt into the pane, the pull request driver
posted it on the request, `sluice inbox` printed it, or a wait reported
it. The record SHALL say when, `delivered`, and to which session,
`delivered_to`. An outcome not delivered SHALL stay pending: decided,
with neither field, and one line in the branch's log saying it was not
delivered and why, in the carrier's words.

#### Scenario: A delivered outcome is recorded with its session

- **WHEN** herdr takes the prompt for submission 12 into pane `w1:p2`
- **THEN** submission 12's record shows `delivered` with the time and
  `delivered_to` with the session naming pane `w1:p2`

#### Scenario: A pull request's outcome is delivered to the request

- **WHEN** the pull request driver posts submission 20's landing on
  pull request 41
- **THEN** submission 20 shows `delivered_to` `pull/41`

#### Scenario: A submission outside any multiplexer stays pending

- **WHEN** submission 14, which recorded no session, lands
- **THEN** no pane is prompted, its record shows it landed and shows
  neither `delivered` nor `delivered_to`, and the branch's log has one
  `pending` line for it

#### Scenario: A refused prompt marks nothing

- **WHEN** herdr refuses the prompt for submission 15 with `agent is
  blocked`
- **THEN** submission 15 shows no `delivered`, and the branch's log has
  one `pending` line for it holding `agent is blocked`

#### Scenario: A session on another host is not prompted from this one

- **WHEN** the round that decides submission 16 runs on a host other
  than the one 16 recorded
- **THEN** no pane is prompted, and 16 shows no `delivered`

### Requirement: An outcome is pushed once

A round SHALL push an outcome at most once: one not delivered SHALL NOT
be pushed again by a later round, and none SHALL be pushed for a
submission already delivered. A round that stopped between a delivery
and its record MAY push it again under the same submission and outcome.

#### Scenario: A pending outcome is not pushed again

- **WHEN** the push for submission 15 was refused, and three later
  rounds run on `main`
- **THEN** no later round prompts any pane for submission 15, and the
  log holds one `pending` line for it

#### Scenario: An outcome already delivered is not pushed

- **WHEN** submission 17 is marked delivered before the round that
  would push its outcome
- **THEN** the round prompts no pane for 17 and its `delivered` is
  unchanged

### Requirement: A wait that reports its own submission's outcome marks it delivered

When `sluice wait`, a waiting `sluice submit` or `wt submit` reports a
pending landed, failed or conflicting submission whose submitter is the
waiting worktree's work (its branch, or the branch a pull request was
opened from), it SHALL mark it delivered once its report is written:
`delivered_to` the multiplexer's name for the waiting session where it
runs in one, else the worktree's path.

#### Scenario: A waiting submit delivers its landing

- **WHEN** `sluice submit main --json` in worktree `/w/unit-a`, with no
  multiplexer, waits and reports submission 12 landed
- **THEN** its record shows `delivered` and `delivered_to` `/w/unit-a`,
  and `sluice inbox` in that worktree then prints nothing

### Requirement: A wait marks nothing that is not its own to deliver

A wait that reports an outcome already delivered SHALL report it as it
stands and mark nothing. A wait on another worktree's submission, or
one that ends undecided, SHALL mark nothing.

#### Scenario: A wait on another's submission marks nothing

- **WHEN** `sluice wait 13` runs in worktree `unit-b` and reports
  submission 13, submitted from `unit-a`, failed
- **THEN** submission 13 shows no `delivered`

#### Scenario: A wait reads an outcome already delivered

- **WHEN** the round pushed submission 12's landing into its pane, and
  the session then runs `sluice wait 12 --json`
- **THEN** it reports 12 landed with the `delivered_to` the push
  recorded, unchanged

### Requirement: A record written before reads as it was

A submission written by an earlier sluice, with `told` and `told_to` or
`told` alone, SHALL read and show as delivered at that time and to that
session, and a log line `"event":"told"` SHALL read as a `delivered`
line. sluice SHALL write `delivered` and `delivered_to`.

#### Scenario: An old record is read

- **WHEN** a branch's records hold submission 9 with `"told":
  "2026-10-27T10:00:00Z"` and no `told_to`
- **THEN** `sluice wait 9 --json` shows `delivered` as
  `2026-10-27T10:00:00Z` and no `delivered_to`, and `sluice inbox` does
  not deliver 9 again
