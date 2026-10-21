# forecast Specification

## Purpose
Whether a head, or a branch no worktree holds, would conflict if
submitted now, and what the forecast was formed from: told to whoever can
act on it, and deciding nothing (64, 65, 47a; ADR 0039).

## Requirements

### Requirement: A branch is forecast with no worktree of it

`sluice submit <branch> --check --head <from>` SHALL forecast the
revision of the branch `<from>` against `<branch>` and every submission
queued on it, as `--check` forecasts a worktree's head (64, 65, 47a),
leaving out the submissions made from `<from>` (67). It SHALL need no
worktree holding `<from>`, write no record and move no branch. A
`<from>` that does not exist SHALL be refused with exit 2.

#### Scenario: A bolt against main

- **WHEN** `bolt/y` changes `builds.md`, `#1` on `main` changes the same
  lines, and no worktree holds `bolt/y`
- **THEN** `sluice submit main --check --head bolt/y` prints
  `would conflict: builds.md with #1 (<agent>, <worktree>)` and exits 2,
  and `main`'s records keep their version

#### Scenario: Its own submission left out

- **WHEN** a submission made from `bolt/y` is queued on `main`, and
  `bolt/y` has since moved
- **THEN** the forecast of `bolt/y` is formed without that submission

#### Scenario: No such branch

- **WHEN** `sluice submit main --check --head bolt/gone --json` runs
- **THEN** it prints an error document whose message is `no branch
  bolt/gone to forecast`, and exits 2

### Requirement: A forecast says what it was formed from

`sluice submit --check --json` SHALL print, beside `conflict`, `head`
(the revision forecast), `onto` (the branch's revision it was formed on)
and `ahead` (the submissions formed ahead of it, in order). These SHALL
be added fields, and `conflict` SHALL be as before: null, or `{paths,
with}`.

#### Scenario: Clean behind one

- **WHEN** `#1` is queued on `main` and the worktree's head merges
  cleanly behind it
- **THEN** the document is `{"conflict": null, "head": "<head>", "onto":
  "<main's revision>", "ahead": [1]}`

### Requirement: A forecast takes the round's combinations

A forecast SHALL take each combination a round formed on the branch's
current revision with exactly the submissions ahead of the head up to
it, and form again only what no such combination holds. What it reports
SHALL be the same as forming every one again.

#### Scenario: Two combinations formed

- **WHEN** a round has formed the combinations of `#1` and of `#1` and
  `#2` on `main`'s revision, and a third worktree is forecast
- **THEN** the forecast forms one merge, the head on the second
  combination

### Requirement: exec forecasts before it runs

Before it waits for room, `sluice exec <branch>` SHALL forecast the
worktree's head on the branch's revision its run is held to, say `would
conflict: <paths> with <what>` when it would conflict, and run as it
would otherwise. `--json` SHALL carry it as `forecast`, in `submit
--check --json`'s shape, or null when none was formed. It SHALL never
change the run or its exit, and nothing of it SHALL be recorded (64;
session-runs-1 clause 92).

#### Scenario: The branch moved under the work

- **WHEN** `main` has landed a change to `a.txt` since the worktree's
  head changed it differently, and the agent runs `sluice exec main`
- **THEN** exec first says `would conflict: a.txt with main`, then runs
  the checks, and exits as their outcome says

#### Scenario: Nothing to say

- **WHEN** the head merges cleanly
- **THEN** exec says nothing of a forecast, and `--json` gives
  `"forecast": {"conflict": null, …}`
