# Spec Delta

## Purpose

A submission whose work no required check reads lands at once beside
the queue, with nothing run, and the combinations under test keep their
results and land with it (140, S68; record 0075; rule core.39).

## ADDED Requirements

### Requirement: Work no required check reads lands at the first round

A queued submission in no combination, whose after-submissions have
landed, SHALL land at the first round that reads it, with nothing run,
when the task runner's affected reading, under the gate at the branch's
current revision, names no required check between that revision and the
revision with the work, nor between each combination formed and not
landed, as it would land now, and that combination with the work.

#### Scenario: Docs beside two code submissions under test

- **WHEN** two code submissions are under test together on the branch
  and a session submits a change to a file no required check reads
- **THEN** the next round lands it, starts no run and takes no checkout,
  and the combination under test is still under test

#### Scenario: Read at the branch's revision

- **WHEN** a submission changes a file a required check reads
- **THEN** it takes the queue as before and carries no `beside`

#### Scenario: Mixed work

- **WHEN** a submission changes a docs file and a file a required check
  reads
- **THEN** it takes the queue whole and nothing of it lands beside

### Requirement: What never lands beside

A submission SHALL take the queue when its work changes the branch's
`.config/sluice.toml` or a setting the task runner names as its own (its
configuration, the toolchain), when the branch's revision is covered by
no result, or when the task runner reads every check as affected, as the
command runner does.

#### Scenario: The policy

- **WHEN** a submission changes only `.config/sluice.toml`
- **THEN** it takes the queue

#### Scenario: A revision no result covers

- **WHEN** the branch was moved by something other than sluice and a
  submission changes only docs
- **THEN** it takes the queue

#### Scenario: A runner that reads every check

- **WHEN** the branch's runner is the command runner
- **THEN** no submission lands beside the queue

#### Scenario: The reading fails

- **WHEN** the task runner cannot answer its affected reading
- **THEN** the submission takes the queue and the round goes on

### Requirement: A combination under test keeps its result and lands with the work

A combination formed before a landing beside SHALL keep its runs, keys
and result, never be formed or tested again for that work, and land
from the revision the landing beside made, one commit per submission,
its tree the tested tree with the work landed beside it.

#### Scenario: The combination lands carrying the docs

- **WHEN** docs land beside a combination under test, and the
  combination passes
- **THEN** it lands with no run started again, its first commit's
  parent is the docs' landing and its tree holds both the docs and the
  code

#### Scenario: Two landings beside in a row

- **WHEN** two docs submissions land beside one combination under test
- **THEN** the combination lands carrying both

#### Scenario: A combination named on the new revision

- **WHEN** a combination formed before a landing beside is shown
- **THEN** it is named as the revision the landing made extended by its
  submissions

### Requirement: Work that waits says which submission it waits behind

A submission whose work a formed combination's checks would read, or
that does not merge cleanly with the work of a submission ahead of it,
SHALL take the queue, and `sluice status` and a waiting submit SHALL say
"waits behind #<n>: its checks read this change" or "conflicts with
#<n>'s work"; its JSON SHALL carry `beside: {behind: <n>, why: "read"}`
or `why: "conflict"`.

#### Scenario: A combination's checks would read it

- **WHEN** a combination under test adds a check that reads the docs,
  and a session submits a docs change
- **THEN** the docs submission takes the queue, behind that
  combination's newest submission, with `why: "read"`

#### Scenario: A conflict with a submission ahead

- **WHEN** a docs submission edits a file the second submission under
  test also edits
- **THEN** it waits behind that submission with `why: "conflict"`, and
  is formed on that submission's work as any submission is, a conflict
  found there removing it as a conflict

### Requirement: A group's leading heads land beside in order

Of several heads submitted as one act, the leading heads that each
qualify SHALL land beside the queue in the act's order; from the first
that does not, every head SHALL take the queue.

#### Scenario: Two docs heads

- **WHEN** a group submit gives two heads, each changing only docs
- **THEN** both land beside the queue in one round, in the act's order

#### Scenario: Docs after code

- **WHEN** a group submit gives a code head, then a docs head
- **THEN** both take the queue

### Requirement: Landing beside is a landing

A landing beside SHALL move the branch only from the revision read,
on a branch on its remote by a leased push, made only by the instance
advancing the branch; a move refused SHALL be reported and the
submission read again at the next round. No signer is asked.

#### Scenario: The branch moved before the landing

- **WHEN** the branch moves between the round's decision and its
  landing beside
- **THEN** the landing is refused and recorded, and the submission is
  queued and read again

### Requirement: The outcome says it landed beside the queue

The outcome of a landing beside SHALL be `landed`, delivered as any
outcome is; `sluice submit`, `sluice wait` and the notice SHALL say
"landed as <rev> beside the queue: no required check reads its
changes", and their JSON SHALL carry `beside: true`. A pull request's
report and `sluice` check SHALL say it landed with no check run.

#### Scenario: The waiting submit's words

- **WHEN** a waiting submit's submission lands beside the queue
- **THEN** it prints "landed as <rev> beside the queue: no required
  check reads its changes" and exits 0

#### Scenario: JSON

- **WHEN** `sluice wait <id> --json` reports a submission landed beside
  the queue
- **THEN** its document has `"state": "landed"` and `"beside": true`

#### Scenario: The notice

- **WHEN** the outcome is pushed to its session
- **THEN** the notice reads "[sluice] #<id> landed on <branch> as <rev>
  beside the queue: no required check reads its changes."
