## ADDED Requirements

### Requirement: sluice confirm confirms an outcome by its key

`sluice confirm <key>...`, run in a worktree, SHALL confirm each key's
delivery for the reader it names that the worktree reads as, as `sluice
inbox` reads them, on whichever branch holds the key, making the
reader's name known where it was not. It SHALL print `confirmed <key>`
with the outcome's submission, branch and state, and exit 0 (51; weir
102, 106, 110). An expired outcome SHALL be confirmed as an owed one
(weir 104).

#### Scenario: A landing confirmed

- **WHEN** submission 12 from worktree branch `unit-a` landed on `main`
  and is owed, and `sluice confirm 12:landed` runs in that worktree
- **THEN** it prints a line naming `12:landed`, `#12`, `main` and
  `landed`, exits 0, and `sluice inbox` in that worktree then prints
  nothing of 12

#### Scenario: An expired blame confirmed

- **WHEN** submission 13's blame expired, owed to `unit-a`, and `sluice
  confirm 13:failed` runs in that worktree
- **THEN** it exits 0, and `sluice status main` shows nothing owed for
  13

#### Scenario: Two keys at once

- **WHEN** `unit-a` is owed `12:landed` on `main` and `15:conflict` on
  `bolt/x`, and `sluice confirm 12:landed 15:conflict` runs there
- **THEN** both are confirmed, each on its own branch, and it exits 0

### Requirement: A second confirmation writes nothing

Where the reader has confirmed the key, `sluice confirm` SHALL write
nothing, say when the reader first confirmed it, and exit 0 (weir 102).

#### Scenario: Confirmed twice

- **WHEN** `unit-a` confirmed `12:landed` at `2026-11-08T10:40:12Z`,
  and `sluice confirm 12:landed` runs there again before 12 leaves the
  live state
- **THEN** it says `12:landed` was confirmed at `2026-11-08T10:40:12Z`,
  writes nothing, and exits 0

### Requirement: A key no branch holds is not found

A key that no branch's deliveries hold SHALL be answered not found,
naming the key, with `sluice inbox` as the command that moves the
reader on, exit 1. Where a branch could not be read, it SHALL say which,
in the reader's own words, and exit 3, since whether the key is there
is not known (rule core.11).

#### Scenario: A key never recorded

- **WHEN** `sluice confirm 99:landed` runs where no branch holds a
  delivery under `99:landed`
- **THEN** it says no outcome is owed under `99:landed`, names `sluice
  inbox`, and exits 1

### Requirement: Another reader's outcome is refused

A key whose delivery names none of the readers the worktree reads as
SHALL be refused, naming the delivery's reader, with `sluice confirm
<key> --as operator` as the command that moves the operator on, exit 2,
and nothing written.

#### Scenario: Another worktree's blame

- **WHEN** `13:failed` is owed to `unit-b`, and `sluice confirm
  13:failed` runs in the worktree on `unit-a`
- **THEN** it is refused naming `unit-b`, exits 2, and 13 stays owed

### Requirement: The operator confirms for an outcome's own reader

`sluice confirm <key> --as operator`, run anywhere in the repository,
SHALL confirm the delivery for each of its readers that has not
confirmed it, and the branch's log SHALL name the operator as who
confirmed (S77).

#### Scenario: A removed worktree's blame

- **WHEN** worktree `unit-b` was removed before anyone collected
  submission 13's blame, and `sluice confirm 13:failed --as operator`
  runs in the main checkout
- **THEN** it exits 0, `sluice status main` shows nothing owed for 13,
  and `main`'s log holds a `confirmed` line for 13 whose `by` is
  `operator`

### Requirement: With --json, the answer is one document

With `--json`, `sluice confirm` SHALL print one document, `confirmed`,
a list holding for each key `key`, `branch`, `reader`, `at` and
`first`, true where this call confirmed it; a refusal or a key not
found SHALL be the error document every command prints, with its
`kind` and `next`.

#### Scenario: A confirmation's document

- **WHEN** `sluice confirm 12:landed --json` confirms `unit-a`'s landing
  on `main`
- **THEN** it prints a document whose `confirmed` holds one element
  with `key` `12:landed`, `branch` `main`, `reader` `unit-a`, `at`, and
  `first` true
