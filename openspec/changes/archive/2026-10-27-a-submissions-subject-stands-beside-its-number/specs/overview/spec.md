## ADDED Requirements

### Requirement: A branch names the subjects of its recent submissions

In `sluice overview --json`, each branch SHALL have `recent_subjects`,
an object mapping each submission its `recent` lines name, as
`recent_from` counts them, to the subject its submit gave, as written,
whether the submission is still live or finished. A submission whose
submit gave none, or whose finished line was written before subjects
were kept, SHALL have no key, and a reader names it by `recent_from`.
`recent` and `recent_from` SHALL stay as they are.

#### Scenario: A landing with a subject and one without

- **WHEN** #1 from `unit/a`, given `Batch review cards`, and #2 from `unit/b`, given none, landed together
- **THEN** the branch's `recent_subjects` is `{"1": "Batch review cards"}` and its `recent_from` is `{"1": "unit/a", "2": "unit/b"}`

#### Scenario: A landed submission moved to the finished records

- **WHEN** #1, given `Batch review cards`, landed and has moved to the finished records, and a recent line names it
- **THEN** `recent_subjects` still gives #1 as `Batch review cards`

#### Scenario: Finished before subjects were kept

- **WHEN** a recent line names #1, whose line in the finished index has no subject
- **THEN** `recent_subjects` has no key for #1, and `recent_from` names its worktree

#### Scenario: No recent lines

- **WHEN** a branch has a submission queued and nothing landed, blamed or conflicting
- **THEN** its `recent_subjects` is `{}`
