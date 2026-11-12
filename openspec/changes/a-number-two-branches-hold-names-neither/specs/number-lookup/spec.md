## ADDED Requirements

### Requirement: A number more than one branch holds names none of them alone

A lookup of a submission by its number SHALL read every branch it can.
Where more than one branch holds the number, the lookup SHALL be
refused and nothing done, naming each branch that holds it, in the
records' order, and each branch that could not be read with its words
(26; record 0114; rule core.11). This holds for `wait`, `status`,
`amend` and `withdraw`, and for a submission whose records moved to its
remote. A number one branch holds SHALL be found as before.

#### Scenario: Withdraw refused naming both branches

- **GIVEN** submission 26 queued on `bolt/a` and submission 26 landed
  on `bolt/b`
- **WHEN** `sluice withdraw 26` runs
- **THEN** it says `refused: submission 26 is on more than one branch:
  bolt/a, bolt/b; name one with --branch, as sluice withdraw 26
  --branch bolt/a`, exits 2, and both submissions are as they were

#### Scenario: Status and amend refused the same way

- **GIVEN** submission 26 queued on `bolt/a` and on `bolt/b`
- **WHEN** `sluice status 26` and `sluice amend 26 --priority 5` run
- **THEN** each is refused naming `bolt/a` and `bolt/b` and exits 2, and
  neither submission's priority changes

#### Scenario: A branch that could not be read is named beside them

- **GIVEN** submission 26 on `bolt/a` and on `bolt/b`, and `bolt/c`'s
  records refused with the words `<words>`
- **WHEN** `sluice status 26` runs
- **THEN** it is refused naming `bolt/a` and `bolt/b`, then says
  `bolt/c could not be read: <words>`, and exits 2

#### Scenario: A number one branch holds

- **GIVEN** submission 27 queued on `bolt/a` alone, and submission 26 on
  `bolt/a` and on `bolt/b`
- **WHEN** `sluice withdraw 27` runs
- **THEN** it withdraws `bolt/a`'s 27 and exits 0, as before

### Requirement: A wait on a number more than one branch holds exits 64

`sluice wait <n>` on a number more than one branch holds SHALL be
refused as the other commands are, and SHALL exit 64, never 2, which is
a conflict's exit, nor any other outcome's. `sluice wait --help` SHALL
say exit 64 is also a number more than one branch holds, given with no
`--branch`.

#### Scenario: Never read as a conflict

- **GIVEN** submission 26 queued on `bolt/a` and on `bolt/b`
- **WHEN** `sluice wait 26 --json` runs
- **THEN** it prints the error document with kind `refused`, runs no
  round, and exits 64

### Requirement: The refusal's document names a command per branch

Under `--json`, the refusal SHALL print the error document with kind
`refused`, its message the refusal's lines joined by `; `, and as
`next` the command that was asked with the number and `--branch
<branch>`, once for each branch that holds the number, in the records'
order.

#### Scenario: Under --json

- **GIVEN** submission 26 on `bolt/a` and on `bolt/b`
- **WHEN** `sluice withdraw 26 --json` runs
- **THEN** it prints `{"error":{"kind":"refused",...}}` whose message
  names `bolt/a` and `bolt/b`, whose `next` is `["sluice withdraw 26
  --branch bolt/a","sluice withdraw 26 --branch bolt/b"]`, and exits 2

### Requirement: The commands take the branch

`sluice wait`, `sluice status`, `sluice amend` and `sluice withdraw`
SHALL take `--branch <branch>` with a submission's number, and SHALL
then read that branch alone: they SHALL act on, follow or show that
branch's submission of the number, and SHALL name no other branch,
whether it holds the number or could not be read. A number one branch
holds SHALL need no `--branch`.

#### Scenario: Each command acts on the branch named

- **GIVEN** submission 26 queued on `bolt/a` and on `bolt/b`
- **WHEN** `sluice amend 26 --branch bolt/b --priority 5`, then
  `sluice withdraw 26 --branch bolt/a` run, then `sluice status 26
  --branch bolt/b --json`
- **THEN** `bolt/b`'s 26 has priority 5 and is queued, `bolt/a`'s 26 is
  withdrawn with its priority unchanged, and the status shows `bolt/b`'s
  record

#### Scenario: A wait follows the branch named to its landing

- **GIVEN** submission 26 queued on `bolt/a` and on `bolt/b`
- **WHEN** `sluice wait 26 --branch bolt/b --json` runs
- **THEN** it follows `bolt/b`'s 26, prints its record once it lands,
  and exits 0

#### Scenario: Another branch that cannot be read is not named

- **GIVEN** submission 26 on `bolt/a`, and `bolt/c`'s records refused
- **WHEN** `sluice status 26 --branch bolt/a` runs
- **THEN** it shows `bolt/a`'s 26 and says nothing of `bolt/c`

#### Scenario: The branch named holds no such number

- **GIVEN** `bolt/a` reads and holds no submission 26
- **WHEN** `sluice status 26 --branch bolt/a` runs
- **THEN** it says `no submission 26 on bolt/a` and exits 1

#### Scenario: The branch named cannot be read

- **GIVEN** `bolt/c`'s records refused with the words `<words>`
- **WHEN** `sluice wait 26 --branch bolt/c` runs
- **THEN** it says submission 26 is on none of the branches sluice
  could read, then `bolt/c could not be read: <words>`, and exits 3

### Requirement: --branch goes with a number

`--branch` given to `sluice wait` or `sluice status` whose target is a
branch, or to `sluice status` with no target, SHALL be a command line
sluice cannot read: the command SHALL say `--branch` names the branch of
a submission's number, and SHALL exit 64.

#### Scenario: A branch target

- **WHEN** `sluice wait main --branch bolt/a` runs
- **THEN** it says `--branch names the branch of a submission's number;
  main is a branch` and exits 64

### Requirement: A command given the branch names it in what it suggests

A command given `--branch` SHALL carry `--branch <branch>` in every
command it suggests for that submission, in its text and in its
document's `next`.

#### Scenario: Under test

- **GIVEN** submission 26 under test on `bolt/b`, and submission 26
  queued on `bolt/a`
- **WHEN** `sluice amend 26 --branch bolt/b --priority 5` runs
- **THEN** it is refused as under test, suggesting `sluice status 26
  --branch bolt/b` and `sluice withdraw 26 --branch bolt/b`

### Requirement: What follows a submission reads its own branch

A wait's follow, a waiting submit's reads of the submission it made, and
the pull request driver's withdrawal of a request's submission SHALL
read and act on the submission's own branch alone, whatever other branch
holds its number (26; record 0114).

#### Scenario: A waiting submit lands its own

- **GIVEN** submission 1 queued on `bolt/a`
- **WHEN** a waiting `sluice submit bolt/b` is given number 1 too
- **THEN** it follows `bolt/b`'s 1 to its landing, and exits 0, and
  `bolt/a`'s 1 stays queued
