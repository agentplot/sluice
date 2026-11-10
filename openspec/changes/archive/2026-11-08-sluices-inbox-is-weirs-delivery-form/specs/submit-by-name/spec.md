## MODIFIED Requirements

### Requirement: The outcome of a submission by name is delivered by nothing

The round that decides a submission by name landed, blamed or in
conflict SHALL record no delivery for it, push no notice for it, and
log no `pending`, `handed` or `confirmed` line for it; it SHALL log the
decision as for any submission. `sluice inbox`, run anywhere, SHALL
hand over nothing of it, and `sluice confirm` SHALL find no outcome
owed under its key. A landed submission by name SHALL leave the live
state as any finished submission does (130).

#### Scenario: Blamed, and no inbox takes it

- **WHEN** submission 32, `bolt/x` by name, is blamed on `main` for
  `cli:test` by `sluice step main`, and `sluice inbox --json` then runs
  in the repository's git directory, in the main checkout, and in a
  worktree that has checked out `bolt/x`
- **THEN** `main`'s log holds a `blamed` line for 32 and no `pending`,
  `handed` or `confirmed` line for it, each `sluice inbox --json`
  prints `outcomes` `[]`, and `sluice status 32 --json` gives no
  `delivery`

#### Scenario: Landed, it leaves the live records

- **WHEN** submission 34, `bolt/x` by name, lands on `main`, and a later
  round writes `main`'s records
- **THEN** 34 is in `main`'s finished records and its index, and `sluice
  status main --all --json` gives it `landed` with its `landed_as`

### Requirement: A submission by name to a branch that takes pull requests opens its request from the branch

Where `<target>` takes pull requests, a submit by name SHALL open or
update the pull request from `<branch>` into `<target>`, push no branch,
mark it ready and title it by its subject or `<branch>`, as a submit
from a worktree on `<branch>` does (124). The submission sluice takes for
it SHALL be the request's, its `from.head` `<branch>`; its outcome
SHALL be owed to the request, posted on it and confirmed on that post's
receipt, and `sluice inbox` SHALL hand over nothing of it in any
worktree.

#### Scenario: A request opened by name

- **WHEN** `main` takes pull requests, `bolt/x` is a branch sluice lands
  on with no request open, and `sluice submit main --head bolt/x
  --no-wait --json` runs in the repository's git directory
- **THEN** a pull request from `bolt/x` into `main` titled `bolt/x` is
  open and ready, no branch is pushed, and once intake takes it in, the
  submission's `from.head` is `bolt/x`

#### Scenario: No worktree on the branch collects its outcome

- **WHEN** that request's submission is blamed, its outcome's post on
  the request fails, and `sluice inbox --json` runs in a worktree that
  has checked out `bolt/x`
- **THEN** it prints `outcomes` `[]`, and the outcome stays owed to the
  request, shown in `main`'s status

## ADDED Requirements

### Requirement: A wait records nothing of a submission by name

A wait, a waiting submit or `sluice wait <n>` that reports a submission
by name SHALL record nothing and SHALL NOT say it stays owed.

#### Scenario: A wait in a bare clone reports it and marks nothing

- **WHEN** submission 33, `bolt/x` by name, lands on `main`, and `sluice
  wait 33` runs in the repository's git directory
- **THEN** it reports 33 landed and exits 0, says nothing of 33 staying
  owed, and `main`'s deliveries hold nothing under `33:landed`
