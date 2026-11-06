## ADDED Requirements

### Requirement: A branch sluice lands on is submitted by name from anywhere in the repository

`sluice submit <target> --head <branch>`, without `--check`, SHALL
submit `<branch>`, a branch sluice lands on, to `<target>`, run from any
folder of the repository: a worktree on any branch, the main checkout,
or a bare repository's git directory. It SHALL read no worktree's head
and run no `git status` (core.48). Under `--no-wait --json` it SHALL
answer as any submit that does not wait, `{"id", "branch", "ahead"}`.

#### Scenario: From a bare repository's git directory

- **WHEN** `bolt/x` and `main` are branches sluice lands on in a bare
  repository with no worktree on `bolt/x`, and `sluice submit main
  --head bolt/x --no-wait --json` runs in its git directory
- **THEN** it prints `{"id": <n>, "branch": "main", "ahead": [...]}`,
  exits 0, and submission `<n>`'s head is `bolt/x`'s revision

#### Scenario: From a worktree on another branch

- **WHEN** the same command runs in a worktree on `unit/a` that holds an
  untracked file
- **THEN** the submission's head is `bolt/x`'s revision, not the
  worktree's head, and nothing the worktree holds refuses it

#### Scenario: A waiting submit by name

- **WHEN** `sluice submit main --head bolt/x` runs with no `--no-wait`
  on a host where no `sluice run` lives
- **THEN** it runs `main`'s rounds while it waits, reports the outcome,
  and exits as a waiting submit does for it

### Requirement: A submission by name is of its branch as the policy holds it

A submission by name's head SHALL be `<branch>` as its policy holds it:
the remote's revision where the policy names a `remote`, the host's own
ref where it names none (124). The submit SHALL push no branch, write
no ref of `<branch>` or `<target>` on the host, and say nothing of the
host's copy of `<branch>` (1, core.4).

#### Scenario: The remote's revision where the policy names a remote

- **WHEN** `bolt/x`'s policy names `remote = "origin"`, origin's
  `bolt/x` is at `8079db3`, this host's own `bolt/x` is two landings
  behind, and `sluice submit main --head bolt/x --no-wait` runs
- **THEN** the submission's head is `8079db3`, no branch is pushed,
  origin's `bolt/x` and this host's `bolt/x` stay where they were, and
  the output says nothing of this host's copy

### Requirement: Only a branch sluice lands on is submitted by name

A submit by name SHALL be refused, exit 2, with nothing written: where
`<branch>` does not exist, saying `no branch <branch> to submit`; where
it is `<target>`, saying `<branch> is not submitted to itself`; and
where no `[branches]` table at its revision names it, saying `<branch>
is no branch sluice lands on: a branch of a worktree's own is submitted
from that worktree`.

#### Scenario: A branch of a worktree's own

- **WHEN** `sluice submit main --head unit/a --json` runs, and no table
  names `unit/a`
- **THEN** it prints an error document whose message says `unit/a is no
  branch sluice lands on`, exits 2, and `main`'s records keep their
  version

#### Scenario: No such branch

- **WHEN** `sluice submit main --head bolt/gone` runs
- **THEN** it says `no branch bolt/gone to submit` and exits 2

#### Scenario: The target itself

- **WHEN** `sluice submit main --head main` runs
- **THEN** it says `main is not submitted to itself` and exits 2

### Requirement: A submit by name takes a submit's options and no worktree

`--head` given with `--worktree`, or with `--status`, SHALL be a usage
error, exit 64, naming why, with nothing submitted. `--after`,
`--priority`, `--no-wait`, `--json` and one `--subject` SHALL be taken
as for any submit.

#### Scenario: With a worktree named

- **WHEN** `sluice submit main --head bolt/x --worktree ../unit-a` runs
- **THEN** it exits 64, saying `--head` takes no `--worktree`, and
  submits nothing

#### Scenario: With a subject

- **WHEN** `sluice submit main --head bolt/x --subject "The runner's
  credential" --no-wait` runs, and the submission lands
- **THEN** its landing commit's first line names `The runner's
  credential`

### Requirement: A submission by name records no worktree and no session

A submission by name SHALL record its branch as its work and its head,
`from.worktree` and `from.head` both `<branch>`; no `from.path` and no
`from.session`, wherever it ran, a herdr pane among them; `from.host`
the host it ran on; and `from.agent` the agent the session's harness
names in `SLUICE_AGENT`, else `<branch>`. `sluice status --json`, `sluice
wait --json` and `sluice overview --json` SHALL show that record.

#### Scenario: The record in a herdr pane

- **WHEN** `sluice submit main --head bolt/x --no-wait --json` runs in
  herdr pane `w1:p2` on host `deneb`, in a worktree on `unit/a`, with no
  `SLUICE_AGENT`
- **THEN** `sluice status <n> --json` shows `from` as `{"agent":
  "bolt/x", "worktree": "bolt/x", "head": "bolt/x", "host": "deneb"}`,
  with no `path` and no `session`

### Requirement: A submission by name and one from a worktree on the branch are one work

A submission by name SHALL withdraw, as a resubmission, the submission
to the same target that is neither landed, landing nor withdrawn whose
work is `<branch>`, made by name or from a worktree on `<branch>`; one
from a worktree on `<branch>` SHALL withdraw a submission by name of it
the same way (67). The attempt a submission shows SHALL count both
(54).

#### Scenario: A retry after a failure

- **WHEN** submission 30, `bolt/x` by name, was blamed on `main`, a fix
  lands on `bolt/x`, and `sluice submit main --head bolt/x` runs
- **THEN** 30 is withdrawn `by` `resubmission`, the new submission's
  head is `bolt/x`'s new revision, and its attempt is 2

#### Scenario: A worktree on the branch submits after a submission by name

- **WHEN** submission 31, `bolt/x` by name, is queued on `main`, and a
  worktree that has checked out `bolt/x` runs `sluice submit main
  --no-wait`
- **THEN** 31 is withdrawn `by` `resubmission`

### Requirement: A forecast and a submit by name read one head

`sluice submit <target> --check --head <branch>` SHALL forecast the
revision a submit by name of `<branch>` would submit, leaving out the
submissions to `<target>` that submit would withdraw. Its `head` under
`--json` SHALL be that revision. A forecast SHALL still take any branch
the host holds, one sluice lands on nowhere among them (64).

#### Scenario: The forecast names what the submit sends

- **WHEN** `bolt/x`'s policy names a remote, and `sluice submit main
  --check --head bolt/x --json` and then `sluice submit main --head
  bolt/x --no-wait --json` run
- **THEN** the forecast's `head` is the submission's head

### Requirement: The outcome of a submission by name is delivered by nothing

The round that decides a submission by name landed, blamed or in
conflict SHALL push no notice for it, log no `pending` line for it, and
never mark it delivered; it SHALL log the decision as for any
submission. `sluice inbox`, run anywhere, SHALL deliver nothing of it.
A wait, a waiting submit or `sluice wait <n>` that reports it SHALL mark
nothing and SHALL NOT say it stays pending for an inbox. A landed
submission by name SHALL leave the live records as any finished
submission does (130).

#### Scenario: Blamed, and no inbox takes it

- **WHEN** submission 32, `bolt/x` by name, is blamed on `main` for
  `cli:test` by `sluice step main`, and `sluice inbox --json` then runs
  in the repository's git directory, in the main checkout, and in a
  worktree that has checked out `bolt/x`
- **THEN** `main`'s log holds a `blamed` line for 32 and no `pending` or
  `delivered` line for it, each `sluice inbox --json` prints `outcomes`
  `[]`, and 32 shows no `delivered`

#### Scenario: A wait in a bare clone reports it and marks nothing

- **WHEN** submission 33, `bolt/x` by name, lands on `main`, and `sluice
  wait 33` runs in the repository's git directory
- **THEN** it reports 33 landed and exits 0, says nothing of 33 staying
  pending, and 33 shows no `delivered`

#### Scenario: Landed, it leaves the live records

- **WHEN** submission 34, `bolt/x` by name, lands on `main`, and a later
  round writes `main`'s records
- **THEN** 34 is in `main`'s finished records and its index, and `sluice
  status main --all --json` gives it `landed` with its `landed_as`

### Requirement: A status gives each decided submission in an inbox's words

`sluice status <branch> --json`, with `--all` or without, and `sluice
status <n> --json` SHALL give each landed, failed or conflicting
submission's `notice`, the words `sluice inbox` gives it, beside its
`failed`, `conflict` with its `paths`, and `tested` with its `on`, as
its record holds them. A withdrawn, queued or testing submission SHALL
have no `notice`. These SHALL be fields added to the documents as they
are.

#### Scenario: A failure by name read from the branch's records

- **WHEN** submission 32, `bolt/x` by name, was blamed on `main` for
  `cli:test`, and `sluice status main --all --json` runs
- **THEN** the element with `id` 32 has `state` `failed`, `from.head`
  `bolt/x`, a `notice` naming `#32` and `cli:test`, `failed` naming
  `cli:test`, and `tested.on` the revision of `main` it was tested on

#### Scenario: A conflict by name read from the branch's records

- **WHEN** submission 35, `bolt/x` by name, conflicts on `main` in
  `builds.md`, and `sluice status 35 --json` runs
- **THEN** the document has `state` `conflict`, `conflict.paths`
  `["builds.md"]`, and a `notice` naming `#35` and `builds.md`

### Requirement: A submission by name to a branch that takes pull requests opens its request from the branch

Where `<target>` takes pull requests, a submit by name SHALL open or
update the pull request from `<branch>` into `<target>`, push no branch,
mark it ready and title it by its subject or `<branch>`, as a submit
from a worktree on `<branch>` does (124). The submission sluice takes for
it SHALL be the request's, its `from.head` `<branch>`; its outcome
SHALL be posted on the request and marked delivered there, and `sluice
inbox` SHALL deliver nothing of it in any worktree.

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
- **THEN** it prints `outcomes` `[]`

### Requirement: No worktree's view takes a submission by name as its own

The home view, `sluice` with no command in a worktree, SHALL NOT show a
submission by name as the worktree's, wherever it runs.

#### Scenario: The home view on the branch

- **WHEN** submission 36, `bolt/x` by name, is queued on `main`, and
  `sluice` runs in a worktree that has checked out `bolt/x` and
  submitted nothing
- **THEN** it shows no submission of the worktree's

### Requirement: The way on of a submission by name is its caller's

`sluice status <n>` and a wait's text for a submission by name that
failed, timed out or conflicts SHALL name its way on as a fix landed on
`<branch>` and `sluice submit <target> --head <branch>` again; for a
conflict, after merging `<target>` at the revision it was tested on, or
the target's revision where it was not tested. They SHALL NOT name an
agent working in `<branch>`.

#### Scenario: The way on of a failure by name

- **WHEN** submission 32, `bolt/x` by name, was blamed on `main`, and
  `sluice status 32` runs
- **THEN** its way on names a fix landed on `bolt/x` and `sluice submit
  main --head bolt/x`, and says nothing of an agent working in `bolt/x`
