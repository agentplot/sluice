## MODIFIED Requirements

### Requirement: The round that decides an outcome pushes a notice into the submitter's pane

When a round decides a submission landed, failed or conflicted, and the
submission recorded a session on this host, the round SHALL push a
notice into that pane as a prompt to the agent there: one line opening
`[sluice]`, naming the submission's number, its branch and its outcome,
and for a landing the revision it landed as, for a failure the failed
checks or that the time limit ended it, for a conflict the submissions
it conflicts with, and ending with `sluice confirm <key>` for the
outcome's key. No other session SHALL be sent the notice.

#### Scenario: A landing is pushed into the submitting pane

- **WHEN** submission 12, submitted from herdr pane `w1:p2` on this
  host, lands on `main` as `abc1234`
- **THEN** the round prompts the agent in pane `w1:p2` once, with a line
  naming `#12`, `main`, `landed`, `abc1234` and `sluice confirm
  12:landed`

#### Scenario: A failure is pushed with its failed checks

- **WHEN** submission 13, submitted from a herdr pane on this host, is
  blamed for `cli:test`
- **THEN** the line pushed into its pane names `#13`, `failed`,
  `cli:test` and `sluice confirm 13:failed`

#### Scenario: No other pane is prompted

- **WHEN** submissions 12 and 13 are tested together and 13 is blamed
- **THEN** only 13's pane is sent the failure, and 12's pane is sent
  nothing of it

#### Scenario: A round run from another pane reaches the recorded server

- **WHEN** the round that decides submission 12 runs in a herdr pane of
  another server
- **THEN** the prompt goes to the server and pane submission 12
  recorded, and none of the round's own pane

### Requirement: An outcome is pushed once

A round SHALL push an outcome at most once: one its carrier refused
SHALL NOT be pushed again by a later round, and none SHALL be pushed for
an outcome already handed to its reader by any way, or confirmed. A
round that stopped between a push and its record MAY push it again
under the same key.

#### Scenario: A pending outcome is not pushed again

- **WHEN** the push for submission 15 was refused, and three later
  rounds run on `main`
- **THEN** no later round prompts any pane for submission 15, and the
  log holds one `pending` line for it

#### Scenario: An outcome already delivered is not pushed

- **WHEN** `sluice inbox` in submission 17's worktree handed over its
  landing before the round that would push it
- **THEN** the round prompts no pane for 17, and 17's `delivery` holds
  the one hand-over by `inbox`

### Requirement: A wait marks nothing that is not its own to deliver

A wait that reports an outcome its reader has confirmed SHALL report it
as it stands and record nothing. A wait on another worktree's
submission, or one that ends undecided, SHALL record no hand-over and
no confirmation.

#### Scenario: A wait on another's submission marks nothing

- **WHEN** `sluice wait 13` runs in worktree `unit-b` and reports
  submission 13, submitted from `unit-a`, failed
- **THEN** 13's `delivery` holds no hand-over by `wait`

#### Scenario: A wait reads an outcome already delivered

- **WHEN** `unit-a` confirmed `12:landed`, and the session then runs
  `sluice wait 12 --json`
- **THEN** it reports 12 landed with its `delivery` as it stood, and
  records nothing

### Requirement: A wait where no files are checked out reads as a worktree on its branch

A wait or a waiting submit run in a folder with no files checked out, a
bare repository or a git directory, SHALL read whose its submission is
as the worktree on the branch the folder's `HEAD` names would, reading
neither the folder's files nor `HEAD`'s commit, and SHALL hand over what
that worktree's wait would, and nothing else. It SHALL NOT fail, nor
say an outcome stays owed, because no files are checked out (2, 51;
record 0099).

#### Scenario: Another worktree's landing read in the git directory

- **WHEN** submission 12 from worktree branch `unit-a` landed and is
  owed, and `sluice wait 12` runs in the repository's git directory,
  whose `HEAD` names `main`
- **THEN** it reports 12 landed and exits 0, says nothing of 12 owed, of
  `git status` or of files checked out, and records nothing; `sluice
  inbox` in `unit-a`'s worktree then hands it over

#### Scenario: The git directory's own landing

- **WHEN** submission 12 from worktree branch `unit-a` landed and is
  owed, and `sluice wait 12` runs with `SLUICE_AGENT` `agent-a` and no
  multiplexer in `unit-a`'s own git directory, which has no files
  checked out and whose `HEAD` names `unit-a`
- **THEN** it reports 12 landed, 12's `delivery` holds a hand-over by
  `wait`, and `main`'s log holds a `handed` line for 12 whose `to` is
  `agent-a`

#### Scenario: A HEAD whose branch has no commit

- **WHEN** submission 12 landed, and `sluice wait 12` runs in a folder
  with no files checked out whose `HEAD` names a branch with no commit
- **THEN** it reports 12 landed and exits 0, says nothing of what it
  could not read, and records nothing

## ADDED Requirements

### Requirement: A hand-over is recorded on its carrier's word and confirms nothing

A hand-over SHALL be recorded by `weir-delivery`'s `hand` only when a
carrier handed the outcome over, by its way: `notice` (herdr took the
prompt), `pull-request` (the driver posted it), `inbox` or `wait`. It
SHALL leave the outcome owed unless its call says its reader confirms
on receipt. An outcome not handed over SHALL stay owed with none, and
one `pending` line in the log saying why in the carrier's words (51;
weir 103, 109).

#### Scenario: A notice taken into the pane

- **WHEN** herdr takes the prompt for submission 12 into pane `w1:p2`
- **THEN** 12's `delivery` is `owed`, with one hand-over by `notice`,
  and `main`'s log holds a `handed` line for 12 naming `w1:p2`

#### Scenario: A submission outside any multiplexer

- **WHEN** submission 14, which recorded no session, lands
- **THEN** no pane is prompted, 14's `delivery` is `owed` with no
  hand-over, and the branch's log has one `pending` line for 14

#### Scenario: A refused prompt records nothing

- **WHEN** herdr refuses the prompt for submission 15 with `agent is
  blocked`
- **THEN** 15's `delivery` holds no hand-over, and the branch's log has
  one `pending` line for it holding `agent is blocked`

#### Scenario: A session on another host is not prompted from this one

- **WHEN** the round that decides submission 16 runs on a host other
  than the one 16 recorded
- **THEN** no pane is prompted, and 16's `delivery` holds no hand-over

### Requirement: The log says where each hand-over reached

Each hand-over SHALL be logged on the branch as one `handed` line naming
the submission, the key, the way and where it reached, the pane, the
request, the path or the agent; each confirmation as one `confirmed`
line naming the submission, the key, the reader and who confirmed it.

#### Scenario: A collect outside any multiplexer

- **WHEN** `sluice inbox` runs with no multiplexer in worktree
  `/w/unit-a` and hands over submission 12's landing
- **THEN** `main`'s log holds a `handed` line for 12 with key
  `12:landed`, `by` `inbox` and `to` `/w/unit-a`

### Requirement: A pull request's post confirms on receipt

The pull request driver's post of an outcome on its request SHALL be a
hand-over by `pull-request` that confirms on receipt, recorded with the
request's confirmation in the same write, since the request keeps the
post for whoever reads it (51, C.5; weir 103).

#### Scenario: A request's landing is posted

- **WHEN** the pull request driver posts submission 20's landing on
  pull request 41
- **THEN** 20's `delivery` is `done`, its reader `pull/41`, with one
  hand-over by `pull-request` and its confirmation, and `sluice inbox`
  in a worktree on the request's head branch gives nothing of 20

#### Scenario: A post that fails

- **WHEN** the post of submission 21's blame on pull request 42 fails
  and the driver answers that it did not deliver it
- **THEN** 21 stays owed to `pull/42` with no hand-over

### Requirement: A wait hands over its own submission's outcome

When `sluice wait`, a waiting `sluice submit` or `wt submit` reports an
owed landed, failed or conflicting submission whose reader the waiting
worktree reads as (its branch, or the request opened from it), it SHALL
record a hand-over by `wait` once its report is written, and leave the
outcome owed. With `--confirm`, the reader SHALL confirm it on receipt
in the same call. `wt submit` SHALL confirm on receipt a landing whose
worktree it removes, since the reader's worktree goes with it.

#### Scenario: A waiting submit hands over its landing

- **WHEN** `sluice submit main --json` in worktree `/w/unit-a`, with no
  multiplexer, waits and reports submission 12 landed
- **THEN** 12's `delivery` is `owed` with one hand-over by `wait`, and
  `sluice inbox` in that worktree then hands 12 over again

#### Scenario: A wait that confirms on receipt

- **WHEN** `sluice wait 12 --confirm --json` in worktree `unit-a`
  reports submission 12 landed
- **THEN** 12's `delivery` holds a hand-over by `wait` and `unit-a`'s
  confirmation, and `sluice inbox` in that worktree prints nothing of 12

#### Scenario: wt submit removes a landed worktree

- **WHEN** `wt submit main` in worktree `unit-a` reports submission 12
  landed and removes the worktree
- **THEN** 12's `delivery` holds a hand-over by `wait` and `unit-a`'s
  confirmation

#### Scenario: wt submit reports a blame

- **WHEN** `wt submit main` in worktree `unit-a` reports submission 13
  blamed, and keeps the worktree
- **THEN** 13's `delivery` is `owed` with one hand-over by `wait`

## REMOVED Requirements

### Requirement: A submission is marked delivered only when its outcome was delivered

**Reason**: An outcome is no longer marked delivered; each hand-over is
recorded by its way and leaves it owed until its reader confirms it
(51; weir 102, 103).

**Migration**: "A hand-over is recorded on its carrier's word and
confirms nothing" and "A pull request's post confirms on receipt"; a
submission's `delivery` in place of `delivered` and `delivered_to`.

### Requirement: A wait that reports its own submission's outcome marks it delivered

**Reason**: A wait hands over and confirms only where it says so
(weir 103).

**Migration**: "A wait hands over its own submission's outcome".

### Requirement: A record written before reads as it was

**Reason**: sluice reads no `delivered`, `delivered_to`, `told` or
`told_to` into weir's form, and no code carries the move (principle 12,
proposed; record 0102).

**Migration**: design D12's agent step records a delivery for each
outcome decided before the upgrade that no carrier had handed over.
