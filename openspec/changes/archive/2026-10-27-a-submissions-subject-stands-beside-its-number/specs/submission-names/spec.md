## Purpose

How sluice's own text output names a submission to a person: by its
number with its subject, or with the branch it was submitted from where
its submit gave none, so a reader of `sluice status`, `sluice overview`
or `sluice` with no command knows what each number is (26).

## ADDED Requirements

### Requirement: A submission is named by its number and its subject

Wherever the text of `sluice status`, `sluice overview` or `sluice` with
no command shows a submission, it SHALL show its number and, after it,
its subject as its submit gave it, or the branch it was submitted from,
as its record's `from.worktree` names it, where the submit gave none
(26). A line SHALL name a submission so the first time it shows it; the
same number shown again on that line SHALL stand alone.

#### Scenario: A subject given

- **WHEN** an agent on `unit/review` submitted #12 to `main` with the subject `Batch review cards`, and #12 is queued
- **THEN** `sluice status main` shows #12's row beginning `#12 Batch review cards`

#### Scenario: No subject given

- **WHEN** an agent on `unit/forecast` submitted #13 to `main` with no subject
- **THEN** `sluice status main` shows #13's row beginning `#13 unit/forecast`

#### Scenario: A row's own number again in its detail

- **WHEN** #12, given `Batch review cards`, is under test in `main + #12`
- **THEN** its row in `sluice status main` reads `#12 Batch review cards`, then its state, agent and worktree, then `main + #12` with no second subject

#### Scenario: A record from before subjects were kept

- **WHEN** a submission's record has no subject field
- **THEN** every view names it by its branch, and the record is read as before

### Requirement: A subject is shown on one line

A subject SHALL be shown on the one line that names its submission:
each run of line breaks or other control characters in it SHALL be
shown as one space, and nothing else of it changed. Its record SHALL
keep it as the submit gave it.

#### Scenario: A subject with a line break

- **WHEN** #12's subject is `Batch review\ncards`
- **THEN** `sluice status main` shows `#12 Batch review cards` on one line, and `sluice status 12 --json` gives its `subject` as `Batch review\ncards`

### Requirement: `sluice status` names each submission with its subject

`sluice status <branch>`, with or without `--all`, and `sluice status
<id>` SHALL begin each submission's row with its number and its subject,
the subjects padded so the columns after them line up. Each submission
the row's detail names, in a combination, `after`, `with` or `held for`,
and each submission of a combination-under-test line, SHALL be named by
its subject too, whether that submission is live or finished.

#### Scenario: A waiting submission and the one it waits on

- **WHEN** #14 from `unit/cards` waits on #12, given `Batch review cards`, which has landed and moved to the finished records
- **THEN** #14's row in `sluice status main --all` ends `after #12 Batch review cards`

#### Scenario: A combination under test

- **WHEN** `main + #12 + #13` is under test, #12 given `Batch review cards` and #13 given none from `unit/forecast`
- **THEN** `sluice status main` shows the line `testing main + #12 Batch review cards + #13 unit/forecast` followed by its progress

#### Scenario: Rows line up

- **WHEN** #12's subject is `Batch review cards` and #13 has none, from `unit/forecast`
- **THEN** in `sluice status main` the state column of #12's row and of #13's row begin at the same place

### Requirement: `sluice overview` names each submission with its subject

The text of `sluice overview` SHALL name each submission of a
combination-under-test line, and each submission a recent `landed`,
`blamed` or `conflict` line names, by its number and its subject, a
landing's several submissions separated by commas, whether each is live
or finished.

#### Scenario: A landing of two

- **WHEN** #12, given `Batch review cards`, and #13, given none from `unit/forecast`, landed together as `c41e8f2`
- **THEN** `sluice overview` shows the recent line `landed #12 Batch review cards, #13 unit/forecast as c41e8f2`

#### Scenario: A blame

- **WHEN** #11, given `Fix the forecast's dates`, was blamed on `9a03c1e`
- **THEN** `sluice overview` shows the recent line `blamed #11 Fix the forecast's dates on 9a03c1e`

### Requirement: The home view names its submission with its subject

`sluice` with no command, in a worktree with a submission, SHALL name
it as `#<id> <subject or branch> to <branch> · <state>`. Under `--json`
its `submission` SHALL carry `subject`, the submit's words as written,
or null where it gave none; its other fields SHALL be as before.

#### Scenario: A submission with a subject

- **WHEN** `unit/forecast` submitted #14 to `bolt/concurrent-units` with the subject `Forecast concurrent units`, and #14 is under test
- **THEN** `sluice` there shows `#14 Forecast concurrent units to bolt/concurrent-units · testing`, and `sluice --json` gives its `submission.subject` as `Forecast concurrent units`

#### Scenario: A submission with none

- **WHEN** `unit/forecast` submitted #14 with no subject
- **THEN** `sluice` there shows `#14 unit/forecast to bolt/concurrent-units · testing`, and `sluice --json` gives `submission.subject` as null
