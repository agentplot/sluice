## MODIFIED Requirements

### Requirement: The exits tell the outcomes apart

A waiting `sluice submit` and `sluice wait` SHALL exit 0 when the
submission landed, 1 when it failed or its pull request was closed, 2
on a conflict, 4 when it was withdrawn and no work of it goes on, 5 when
`--timeout` ran out, and 130 when stopped with Ctrl-C. A submission that
does not exist SHALL exit 1 with the not-found error document under
`--json`.

#### Scenario: Withdrawn while waited on

- **WHEN** submission `<n>` is withdrawn by the operator while `sluice
  wait <n>` waits, and no later submission and no pull request carries
  its head
- **THEN** it exits 4, and a waiting `sluice submit` withdrawn the same
  way exits 4

#### Scenario: Blamed

- **WHEN** submission `<n>` is blamed while `sluice wait <n>` waits
- **THEN** it exits 1 and names the failed tasks as a waiting submit
  does

#### Scenario: No such submission

- **WHEN** `sluice wait 9999 --json` runs and there is no submission
  9999
- **THEN** it prints `{"error":{"kind":"not_found",...}}` and exits 1

## ADDED Requirements

### Requirement: A wait reports landed a withdrawn submission whose work the branch holds

A waiting `sluice submit` and `sluice wait`, reading their submission
withdrawn while its branch holds its head, SHALL report it landed as the
commit that landed it: its record with `state` `landed`, that
`landed_as` and `landed_elsewhere`, the words `landed as <rev>: <branch>
holds its work, by a landing not its own`, and exit 0. The record kept
SHALL stay withdrawn (122, 5, 67).

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

### Requirement: A wait follows the submission that carries a withdrawn one's work

Where a later submission from the withdrawn one's place holds its head,
a waiting `sluice submit` and `sluice wait` SHALL say `#<a> withdrawn by
<who>: #<b> carries its work, so this wait follows it`, follow `#<b>`
to its outcome and report it (67).

#### Scenario: Withdrawn by its session's next submit

- **WHEN** a session submits `<a>`, then submits a commit on top of it
  as `<b>`, and `sluice wait <a>` runs
- **THEN** it says `#<a> withdrawn by resubmission: #<b> carries its
  work`, follows `<b>`, and exits 0 once `<b>` lands

### Requirement: A wait follows the pull request that carries a withdrawn submission's work

For a pull request's submission withdrawn while the pull request is
open and no draft on a head that holds its head, a waiting `sluice
submit` and `sluice wait` SHALL follow the pull request as `sluice wait
<branch>` does, to its submission on this host or to what GitHub shows
of it while another host takes it in (137).

#### Scenario: A pull request's new head

- **WHEN** a session's `wt submit` opened pull request `#12`, sluice took
  it in as `<a>`, the session submits a new head on it, and `sluice wait
  <a> --json` runs
- **THEN** it follows `#12` to the submission of its new head and exits
  0 once that lands, printing that submission's record
