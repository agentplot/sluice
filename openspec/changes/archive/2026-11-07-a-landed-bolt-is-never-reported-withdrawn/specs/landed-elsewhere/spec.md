## Purpose

Lands, as the commit that landed it, a submission whose work its branch
already holds by whichever landing took it, so that work is never
formed, run or landed again and never withdrawn (122, 5).

## ADDED Requirements

### Requirement: A submission whose work the branch already holds is landed

A round SHALL land each submission in the queue, not landing and in no
combination formed on the branch's revision, whose head that revision
holds, before it lands anything beside the queue or forms any
combination. Its record SHALL say `state` `landed`, `landed_as` the
commit that landed it, the oldest of the branch's first-parent history
holding its head, and `landed_elsewhere` `true`. It SHALL never be
withdrawn (122, 5).

#### Scenario: Another queue lands it

- **WHEN** submission `<n>` is queued, and the branch moves to a merge
  of its head that another queue of the branch made, and on past it
- **THEN** the next round records `<n>` landed as that merge, with
  `landed_elsewhere`, and never withdrawn

#### Scenario: A record written before

- **WHEN** a submission's record written before this has no
  `landed_elsewhere`
- **THEN** it reads as `false`

### Requirement: Nothing is formed, run or landed of work the branch holds

No combination of a submission whose work the branch holds SHALL be
formed, run or landed. A combination under test that holds it SHALL be
dropped and its checkout released, and the submissions behind it formed
again on the branch as it stands (6, 10, 22).

#### Scenario: Nothing run

- **WHEN** submission `<n>` is queued and another queue of the branch
  lands its head
- **THEN** the next round runs nothing for `<n>` and moves the branch
  nowhere

#### Scenario: Under test when another queue lands it

- **WHEN** submissions `<a>` and `<b>` are under test, and the branch
  moves to a merge of `<a>`'s head
- **THEN** the next round records `<a>` landed as that merge, drops the
  combinations formed before the move and releases their checkouts, and
  tests `<b>` alone on the branch as moved, which then lands as its own

### Requirement: Work found landed is logged and delivered

The round that finds a submission's work landed SHALL log `found_landed`
with its number and the commit that landed it, and SHALL deliver its
outcome as any landing's (51).

#### Scenario: Its session is told

- **WHEN** a round finds submission `<n>`'s work landed as `M`
- **THEN** the log holds `found_landed` naming `<n>` and `M`, and `<n>`'s
  session is told it landed as `M`
