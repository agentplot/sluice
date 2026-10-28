# Spec Delta

## Purpose

A branch whose policy, revision or state sluice cannot read is refused
alone: no lookup of a submission by its number, no wait, no status read
and no round of any other branch stops on it (26, S64).

## ADDED Requirements

### Requirement: A submission is found past a branch that cannot be read

A lookup of a submission by its number SHALL read every branch with
state it can, SHALL pass by a branch whose policy, revision or state it
cannot read, and SHALL answer for a submission on a branch it can read
as when every branch reads, saying nothing of the branch passed by (26;
rule core.11). This holds for `sluice wait <n>`, `sluice status <n>`,
`sluice amend <n>`, `sluice withdraw <n>`, a waiting `sluice submit`
reading its own submission, and a submission whose state moved to its
remote.

#### Scenario: A wait on another branch follows its submission to its verdict

- **GIVEN** submission `<n>` queued on `main`, and `bolt/b`'s state
  cannot be read
- **WHEN** `sluice wait <n> --json` runs
- **THEN** it finds `<n>`, runs `main`'s rounds, and prints `<n>`'s
  record once it lands, exiting 0, as when `bolt/b` reads (S64)

#### Scenario: Status, amend and withdraw find it the same way

- **GIVEN** submission `<n>` queued on `main`, and `bolt/b`'s state
  cannot be read
- **WHEN** `sluice status <n>`, `sluice amend <n> --priority 1` and
  `sluice withdraw <n>` run in turn
- **THEN** each answers for `<n>` as when `bolt/b` reads, and none says
  anything of `bolt/b`

#### Scenario: A branch whose routing is refused stops no wait on another branch

- **GIVEN** a repository whose state is on its remote, and the host's
  own ref of `bolt/b` holding a policy this sluice refuses
- **WHEN** `sluice wait <n>` runs on a submission to `main`
- **THEN** it finds `<n>` and follows it to its verdict

### Requirement: A number found on no readable branch names the branches that could not be read

When a lookup by number finds the submission on no branch it could
read, and some branch could not be read, the command SHALL say the
submission is on none of the branches sluice could read, SHALL name
each branch that could not be read with the words it was refused with,
whole and unchanged (133), SHALL NOT say there is no such submission,
and SHALL exit 3 (26).

#### Scenario: A submission on the branch that cannot be read

- **GIVEN** submission `<n>` on `bolt/b`, whose state cannot be read
  with the words `<words>`
- **WHEN** `sluice wait <n>` runs
- **THEN** it says `<n>` is on none of the branches sluice could read,
  then `bolt/b could not be read: <words>`, and exits 3

#### Scenario: Every branch read

- **GIVEN** every branch with state reads, and none holds submission
  9999
- **WHEN** `sluice wait 9999` runs
- **THEN** it says `no submission 9999` and exits 1

### Requirement: The unfound number's document is the internal error

Under `--json`, a number found on no readable branch while some branch
could not be read SHALL print the error document with kind `internal`,
its message the same words the text gives, and `sluice overview` as
its next command.

#### Scenario: Under --json

- **GIVEN** `bolt/b`'s state cannot be read with the words `<words>`,
  and no readable branch holds submission 9999
- **WHEN** `sluice status 9999 --json` runs
- **THEN** it prints `{"error":{"kind":"internal",...}}` whose message
  names `bolt/b` and holds `<words>`, and exits 3

### Requirement: sluice run passes a branch by and goes on

`sluice run` SHALL pass by a branch whose status read or round fails,
and SHALL go on in the same pass to every other branch, whose rounds
run as when that branch reads (26; rule core.14). A branch passed by
SHALL NOT change the exit `sluice run` ends with.

#### Scenario: One branch refused, the rest move

- **GIVEN** submissions queued on `main` and on `bolt/a`, and
  `bolt/b`'s policy refused
- **WHEN** `sluice run --rounds 3` runs
- **THEN** the submissions on `main` and `bolt/a` land, and it exits 0
  (S64)

### Requirement: sluice run says a branch passed by once while it stands

`sluice run` SHALL say a branch it passed by on one line, `<branch>
could not be read, passed by:` and the failure's words, whole and
unchanged, once while the same words stand: again when the words
change, and again when the branch is refused anew after a pass that
read it (26, 121).

#### Scenario: Said once over several passes

- **GIVEN** `bolt/b` refused with the same words on every pass
- **WHEN** `sluice run --rounds 3` runs
- **THEN** `bolt/b could not be read, passed by: <words>` is said once

#### Scenario: Read again, refused again

- **GIVEN** `bolt/b` passed by and said, then read on a later pass,
  then refused again with the same words
- **WHEN** the next pass reaches it
- **THEN** its line is said again

### Requirement: A round reads no other branch it cannot read

A round of a branch SHALL NOT fail because another branch cannot be
read. The runs a `sluice run` instance has starting on another branch
SHALL count nothing for a branch it cannot read, and the check of the
host's systems against each branch's policy SHALL pass such a branch
by (26; rule core.14).

#### Scenario: sluice run on a host whose config names its systems

- **GIVEN** a host whose config names its systems, and `bolt/b`'s
  policy refused
- **WHEN** `sluice run --rounds 1` runs
- **THEN** it checks the systems against every other branch's policy
  and runs their rounds

### Requirement: A release whose branch cannot be read waits for a later pass

A removed worktree's checkout or an unusable preview whose branch
cannot be read SHALL be left for a later pass, as one whose branch's
lock another round holds is, and every other SHALL be released, by a
round, `sluice exec`, `sluice preview` and `sluice previews` alike (26;
rule core.14).

#### Scenario: A removed worktree's checkout on a refused branch

- **GIVEN** a removed worktree's checkout on `bolt/b`, whose state
  cannot be read, and a submission queued on `main`
- **WHEN** a round of `main` runs
- **THEN** the round of `main` runs, and the checkout on `bolt/b` is
  kept for a later pass, released once `bolt/b` reads

### Requirement: The branches are listed when the default branch's policy cannot be read

The list of branches with state SHALL NOT fail because the default
branch's policy cannot be read: it SHALL list this host's own and those
on the remotes this host has read a state from before, the default
branch's refusal SHALL be said wherever that branch is read, and the
list SHALL fail only when the state itself cannot be listed (26).

#### Scenario: A refused main hides no branch this host has read

- **GIVEN** a repository whose state is on its remote, which this host
  has read before, and the host's own ref of `main` holding a policy
  this sluice refuses
- **WHEN** `sluice wait <n>` runs on a submission to `bolt/a`
- **THEN** it finds `<n>` and follows it, and `sluice overview` shows
  `main` with its refusal and `bolt/a` whole
