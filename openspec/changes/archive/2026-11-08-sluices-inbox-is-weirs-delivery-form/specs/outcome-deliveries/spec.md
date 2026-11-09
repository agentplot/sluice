## ADDED Requirements

### Requirement: A decided outcome is recorded once as a delivery

When a round decides a submission landed, blamed or in conflict, it
SHALL record in the branch's state one `weir-delivery/1` record by
`weir-delivery`'s `record`: key `<id>:landed`, `<id>:failed` or
`<id>:conflict`; one reader, the submission's `from.worktree`; the
notice as its message; recorded at the round's time, expiring after
the host's span (51; weir 100, 104, 108; record 0102). A round that
runs again SHALL record nothing more under that key (weir 101).

#### Scenario: A landing from a worktree

- **WHEN** submission 12, submitted from worktree branch `unit-a`,
  lands on `main` as `abc1234` on a host with no `[deliveries]`
  setting
- **THEN** `main`'s deliveries hold one `weir-delivery/1` record with
  key `12:landed`, readers `["unit-a"]`, message `[sluice] #12 landed
  on main as abc1234.`, and `expires_at` 168 hours after its
  `recorded_at`

#### Scenario: A round stopped after its write runs again

- **WHEN** the step that decided submission 13 blamed stops after
  writing `main`'s state, and `sluice step main` runs again
- **THEN** `main`'s deliveries hold one record under `13:failed`

### Requirement: A pull request's outcome is owed to the request

A submission intake took in from a pull request SHALL have its
delivery's reader be the request as sluice names it, `pull/<n>` (51,
124, C.5).

#### Scenario: A request is blamed

- **WHEN** submission 20, intake's of pull request 41, is blamed for
  `cli:test`
- **THEN** its delivery is under `20:failed` with readers `["pull/41"]`

### Requirement: A submission by name and a withdrawal record no delivery

The round SHALL record no delivery for a submission by name, whatever
its outcome, and none for a withdrawal (124; rule core.50).

#### Scenario: A blame by name

- **WHEN** submission 32, `bolt/x` by name, is blamed on `main`
- **THEN** `main`'s deliveries hold nothing under `32:failed`, and
  `sluice status 32 --json` gives no `delivery`

#### Scenario: A resubmission withdraws the earlier

- **WHEN** worktree branch `unit-a` submits again, which withdraws its
  queued submission 14
- **THEN** `main`'s deliveries hold nothing for 14

### Requirement: A resubmission confirms the outcome it withdraws

When a submit from a worktree withdraws that worktree's earlier
submission that was blamed or in conflict, it SHALL confirm that
submission's outcome for its reader in the same write, since the reader
has acted on it by submitting again (5, 67; weir 102). The branch's log
SHALL hold a `confirmed` line for it whose `by` is `resubmission`.

#### Scenario: A blame resubmitted

- **WHEN** submission 14 from `unit-a` was blamed and is owed, and
  `unit-a` submits again, which withdraws 14
- **THEN** 14's `delivery` holds `unit-a`'s confirmation, and `main`'s
  log holds a `confirmed` line for 14 whose `by` is `resubmission`

### Requirement: The deliveries are kept among the branch's live state

A branch's deliveries SHALL be kept in `deliveries.jsonl` among its live
state, each record one JSON object on its own line as weir's C.8 spells
it, in the order added, read and written under the branch's lock with
the rest of its state, so a host that reads the branch's state reads
them (core.11). A line that does not read SHALL stop the branch's reads,
naming `deliveries.jsonl` and weir's words.

#### Scenario: A branch whose state is on its remote

- **WHEN** submission 12 from `unit-a` lands on `main`, whose state is
  on its remote, on host A, and `sluice inbox --json` runs in a
  worktree on `unit-a` on host B
- **THEN** host B's inbox gives 12's landing

#### Scenario: A line that does not read

- **WHEN** `main`'s `deliveries.jsonl` holds a line whose `format` is
  `weir-delivery/2`
- **THEN** `sluice status main` fails naming `deliveries.jsonl` and
  `format`

### Requirement: The branch's store passes weir's pack

The branch's store of deliveries SHALL pass every delivery case of
weir's pack, run through a fresh store over a branch's state for each
case (weir 112, 113; S25).

#### Scenario: The pack's cases

- **WHEN** `weir_pack::conform_deliveries` runs the embedded pack's
  delivery cases through a store over a fresh branch's state on the
  fakes' state seam
- **THEN** every case passes

### Requirement: Only a done delivery leaves the branch's state

A delivery SHALL leave the branch's state only once every one of its
readers has confirmed it, and then whole, with its hand-overs and its
confirmations: in the write that moves its submission to the finished
state, or in the first write after it is done where its submission has
moved already (weir 107; I9; 130). A delivery not done SHALL stay
whatever becomes of its submission, and a `weir-known/1` record SHALL
never leave.

#### Scenario: A landing confirmed

- **WHEN** submission 12's landing is confirmed by `unit-a`, and a later
  round moves 12 to the finished state
- **THEN** `main`'s deliveries hold no record under `12:landed`, and
  still hold `unit-a`'s `weir-known/1`

#### Scenario: A landing nobody confirms

- **WHEN** submission 12 landed, was handed over by `notice`, is
  confirmed by nobody, and a later round moves 12 to the finished state
- **THEN** `main`'s deliveries still hold its `weir-delivery/1` and its
  hand-over

### Requirement: The host sets an outcome's span

The span an outcome is owed before it expires SHALL be the host's
`[deliveries] expire_after`, a duration such as `"168h"` or `"90m"`,
and 168 hours where it is unset, fixed by the host whose round records
the delivery. A value that is not such a duration, or is under one
second, SHALL be refused naming `[deliveries] expire_after` (weir 104).

#### Scenario: A day's span

- **WHEN** the host's settings hold `[deliveries]` with `expire_after =
  "24h"`, and submission 12 lands
- **THEN** its delivery's `expires_at` is 24 hours after its
  `recorded_at`

#### Scenario: A span that does not read

- **WHEN** the host's settings hold `expire_after = "a week"`
- **THEN** sluice refuses its settings, naming `[deliveries]
  expire_after`
