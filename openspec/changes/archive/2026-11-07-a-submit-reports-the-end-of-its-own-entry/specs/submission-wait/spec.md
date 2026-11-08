## MODIFIED Requirements

### Requirement: A wait reports landed a withdrawn submission whose work the branch holds

A waiting `sluice submit` and `sluice wait`, reading their submission
withdrawn while its branch holds its head, SHALL report it landed as the
commit that landed it: its record with `state` `landed`, that
`landed_as` and `landed_elsewhere`, the words `landed as <rev>:
<branch> holds its work`, and exit 0. They SHALL read the branch as a
round reads it, from its remote, before they call a closed pull request
closed without landing. The record kept SHALL stay withdrawn (122, 5,
67).

#### Scenario: Withdrawn after another queue landed it

- **WHEN** submission `<n>` of pull request `#330` was withdrawn, and
  another queue of `main` has landed `#330`'s head as `562606d`
- **THEN** `sluice wait <n> --json` prints `<n>`'s record with `state`
  `landed` and `landed_as` `562606d` and exits 0, and `sluice status
  <n>` still says withdrawn

#### Scenario: Its pull request closed as landed

- **WHEN** that pull request is closed, and `sluice wait main` runs in
  its worktree
- **THEN** it says `landed as 562606d` and exits 0, never `closed
  without landing`

#### Scenario: Closed before this host fetched the branch

- **WHEN** `main` takes pull requests, a driver of the repository is
  live on the host, this host took pull request `#12` in as `<n>`,
  another queue of `main` lands `#12`'s head as `<rev>` on the remote
  and GitHub reads `#12` closed, and no command of this host has
  fetched `main` since
- **THEN** `sluice wait main` in `#12`'s worktree says `landed as
  <rev>` and exits 0, never `closed without landing`

### Requirement: A wait follows the submission that carries a withdrawn one's work

Where a later submission from the withdrawn one's place holds its head,
a waiting `sluice submit` and `sluice wait` SHALL say `#<a> withdrawn by
<who>: #<b> carries its work, so this wait follows it` and follow
`#<b>`. They SHALL end with `#<a>`, never `#<b>`, as their outcome:
landed as the commit that landed its head once `#<b>` lands, exit 0;
withdrawn once `#<b>` is blamed, in conflict, or withdrawn with nothing
carrying it on, naming how `#<b>` ended, exit 4 (67, 122, 51).

#### Scenario: Withdrawn by its session's next submit

- **WHEN** a session submits `<a>`, then submits a commit on top of it
  as `<b>`, and `sluice wait <a> --json` runs
- **THEN** it exits 0 once `<b>` lands, printing `<a>`'s record with
  `state` `landed`, `landed_elsewhere`, `landed_as` the commit that
  landed `<b>`, and `carried_by` `{"id": <b>, "state": "landed"}`, and
  `sluice status <a>` still says withdrawn

#### Scenario: Its words

- **WHEN** the same, and `sluice wait <a>` runs without `--json`
- **THEN** it says `#<a> withdrawn by resubmission: #<b> carries its
  work, so this wait follows it`, ends `landed as <rev>: main holds its
  work, by #<b>'s landing`, and exits 0

#### Scenario: What carried its work was blamed

- **WHEN** a session submits `<a>`, then a commit on top of it as `<b>`
  that fails a check, and `sluice wait <a> --json` runs
- **THEN** it exits 4 once `<b>` is blamed, printing `<a>`'s record
  with `state` `withdrawn` and `carried_by` `{"id": <b>, "state":
  "failed"}`, and without `--json` it says `#<a> withdrawn by
  resubmission: #<b>, which carried its work, failed`

#### Scenario: Out of time while it follows

- **WHEN** `<b>` carries `<a>`'s work and is still under test, and
  `sluice wait <a> --timeout 2s --json` runs
- **THEN** it exits 5, printing `<a>`'s record with `carried_by`
  naming `<b>` and its `state` as it stands, and without `--json` it
  ends `still waiting: sluice wait <a>`

### Requirement: A wait follows the pull request that carries a withdrawn submission's work

For a pull request's submission withdrawn while the pull request is
open and no draft on a new head that holds its head, or on its own head
after this host handed it over (137), a waiting `sluice submit` and
`sluice wait` SHALL follow the pull request as `sluice wait <branch>`
does, and SHALL end with that submission as a wait that follows a later
submission ends. A withdrawal that leaves it on its own head otherwise
SHALL be reported withdrawn (120).

#### Scenario: Withdrawn by the operator, still open

- **WHEN** the operator withdraws the submission `<a>` of a pull request
  a person opened, which stays open and ready on its head, and `sluice
  wait <a>` runs
- **THEN** it exits 4 at once

#### Scenario: A pull request's new head

- **WHEN** a session's `wt submit` opened pull request `#12`, sluice took
  it in as `<a>`, the session submits a new head on it, which sluice
  takes in as `<b>`, and `sluice wait <a> --json` runs
- **THEN** it follows `#12` and exits 0 once `<b>` lands, printing
  `<a>`'s record with `state` `landed`, `landed_elsewhere` and
  `carried_by` `{"id": <b>, "state": "landed"}`

#### Scenario: Closed on its new head

- **WHEN** the same, and `#12` is closed before `<b>` lands
- **THEN** it exits 4, printing `<a>`'s record with `state` `withdrawn`
  and `carried_by` `{"pull_request": 12, "state": "closed"}`

## ADDED Requirements

### Requirement: A waiting submit to a branch that takes pull requests ends with the submission it made

A waiting `sluice submit` to a branch that takes pull requests, and
`sluice wait <branch>` there, SHALL take as their own the first
submission of the pull request this host's records hold as they first
read it, and once it is read SHALL wait on it as `sluice wait <id>`
does. They SHALL never end with a submission of the pull request on
another head as their own outcome (135, 67).

#### Scenario: The branch moves under its landing

- **WHEN** `sluice submit main --head bolt/x` waits, sluice took its
  pull request `#12` in as `<a>`, a commit then lands on `bolt/x` on
  GitHub, sluice takes `#12` in again as `<b>` on that head, and `<b>`
  lands
- **THEN** the submit exits 0 and ends `landed as <rev>: main holds its
  work, by #<b>'s landing`, and under `--json` it prints `<a>`'s record
  with `state` `landed` and `carried_by` `{"id": <b>, "state":
  "landed"}`

#### Scenario: Landed by another queue, in a bare clone

- **WHEN** `sluice submit main --head bolt/x` waits in the repository's
  git directory, sluice took its pull request `#12` in as `<a>`, and
  another queue of `main` lands `#12`'s head as `<rev>`
- **THEN** the submit exits 0, says `landed as <rev>` and never
  `withdrawn`, and `sluice status <a> --json` gives `state` `landed`
  with `landed_elsewhere`

### Requirement: A wait marks delivered only its own outcome

A waiting `sluice submit` and `sluice wait` that follow a later
submission or a pull request SHALL mark delivered only the outcome they
report as their own. The outcome of the submission they followed SHALL
stay pending for its own delivery, the round's notice or `sluice inbox`
(51).

#### Scenario: The resubmission's landing stays pending

- **WHEN** a session submits `<a>`, then a commit on top of it as `<b>`,
  and `sluice wait <a>` runs in its worktree until `<b>` lands
- **THEN** `sluice status <b> --json` shows no `delivered`, and `sluice
  inbox` in that worktree then gives `<b>`'s landing and marks it
  delivered
