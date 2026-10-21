## ADDED Requirements

### Requirement: A running task says when it started

A run's supervisor SHALL note, at most once a second while the run's
output grows and once as it ends, how long the output was and when, in
the run's own directory, removed with it; it SHALL note nothing of
what the output says. The moon runner SHALL give a running task the
time of the first note that reaches the end of its start checkpoint.
A run with no notes SHALL give its running tasks no start. Nothing a
round writes and no record SHALL change (61, 62; ADR 0028).

#### Scenario: A task started a minute ago

- **WHEN** moon started `b:test` in a run about a minute ago and it has
  not ended
- **THEN** after the next round, `sluice overview --json` lists
  `b:test` as `running` with a `started` within a second of when its
  checkpoint was printed

#### Scenario: A run with no notes

- **WHEN** a run's directory holds no notes, as a run started by an
  older sluice does
- **THEN** its running tasks have no `started`, and the rest of its
  tasks read as before

### Requirement: A combination's progress is one line

A combination under test's progress SHALL read as one line: how many
of its tasks have ended of how many, up to three running tasks,
longest first, each with how long it has run, and the combination's
time against its limit, as
`7/12 tasks · running b:test 4m02s · 6m10s of 30m`. Where no tasks are
kept, it SHALL give the combination's time alone.

#### Scenario: More than three running

- **WHEN** five of a combination's tasks are running
- **THEN** its line names the three that have run longest, then
  `+2 more`

#### Scenario: No tasks kept

- **WHEN** a branch runs its checks with the command runner and
  `main + #3` is under test
- **THEN** its line gives the combination's time against its limit
  and no count

### Requirement: The text views show a combination's progress

`sluice status <branch>`, `sluice status <n>`, a waiting
`sluice submit`, `sluice wait` and `sluice overview` SHALL show a
combination under test's progress line, read from the tasks the
overview reads. No JSON form SHALL change, and none of these SHALL
write anything (rules core.11, core.14).

#### Scenario: The branch's status

- **WHEN** `main + #3` is under test with 1 of its 3 tasks ended and
  `b:test` running
- **THEN** `sluice status main` prints a line
  `testing main + #3 · 1/3 tasks · running b:test …` under its header,
  and its row for #3 is as before

#### Scenario: A submission's status

- **WHEN** the same combination is under test
- **THEN** `sluice status 3` prints that line for #3's combination

#### Scenario: A waiting command

- **WHEN** `sluice wait 3` waits while #3's tasks end one by one
- **THEN** it prints the line each time the count ended or the tasks
  running change, and not at every round
