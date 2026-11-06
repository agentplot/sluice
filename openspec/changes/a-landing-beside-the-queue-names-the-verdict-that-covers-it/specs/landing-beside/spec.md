## ADDED Requirements

### Requirement: A landing beside's result reads as passed with nothing run

A landing beside's commit SHALL carry `Sluice-Submission` and
`Sluice-From` as any landing's does and no `Sluice-Tested`, and its
`Sluice-Result` SHALL open with `passed (0 run, 0 replayed) beside the
queue: no required check reads its changes`. A landing through the
queue SHALL write its trailers as before (23, 140; record 0083).

#### Scenario: A reader of the head reads it as passed

- **WHEN** `git log -1 --format='%(trailers:key=Sluice-Result,valueonly)'`
  reads the branch's head after a landing beside
- **THEN** it prints one line whose first word is `passed`, and
  `%(trailers:key=Sluice-Tested,valueonly)` prints nothing

#### Scenario: A combination lands after a landing beside

- **WHEN** docs land beside a combination under test, and the
  combination then passes and lands as `M`
- **THEN** `M`'s commit carries `Sluice-Tested` naming its combination
  and its own `passed` result, as before

### Requirement: A landing beside names the result that covers it

After those words, a landing beside's `Sluice-Result` SHALL say `;
covered by the result at <revision>, <words>`, where `<revision>` is, in
full, the branch's revision that the result covering the branch's
revision landed as, and `<words>` is that landing's own `Sluice-Result`.
A landing beside after a landing beside SHALL name the same covering
result (23, 70, 140; record 0083).

#### Scenario: Docs landed beside after a tested landing

- **WHEN** a code submission lands through the queue as `L`, its
  `Sluice-Result` `passed (2 run, 0 replayed)`, and a session then
  submits a change to a file no required check reads, which lands beside
  the queue
- **THEN** the docs' landing commit has no `Sluice-Tested`, and its
  `Sluice-Result` is `passed (0 run, 0 replayed) beside the queue: no
  required check reads its changes; covered by the result at <L in
  full>, passed (2 run, 0 replayed)`

#### Scenario: Two landings beside in a row

- **WHEN** two docs submissions land beside the queue one after the
  other on a branch whose last landing through the queue is `L`
- **THEN** both landing commits are covered by the result at `L`, and
  the second names no revision of the first

#### Scenario: Covered by a combination that landed after a landing beside

- **WHEN** docs land beside a combination under test, the combination
  then passes and lands as `M`, and another docs submission then lands
  beside
- **THEN** the later docs' commit is covered by the result at `M`, with
  `M`'s own words

### Requirement: A request landed beside names the result that covers it

A pull request's `sluice` check's summary and its report SHALL say the
request landed beside the queue with no check run because no required
check reads its changes, and name the result that covers it, by its
revision and its words, as the landing commit does. The report SHALL be
the check's text and the request's one comment, as any result's report
is, with no row; no check per required check SHALL be posted for it
(118).

#### Scenario: The request's check names the covering result

- **WHEN** a pull request's submission lands beside the queue on `main`
  as `D`, covered by the result at `L`, `passed (141 run, 0 replayed)`
- **THEN** its `sluice` check succeeds, titled "landed as <D short>
  beside the queue", with the summary "Landed on main as `<D short>`
  beside the queue, with no check run: no required check reads its
  changes. Covered by the result at `<L short>`, passed (141 run, 0
  replayed).", its text and the request's comment are the report, whose
  head reads "sluice: landed on main as <D short> with no check run: no
  required check reads its changes" followed by "Covered by the result
  at `<L short>`, passed (141 run, 0 replayed).", `<L short>` linked to
  its commit, with no row, and no check is posted on its head for any
  required check

#### Scenario: A request's earlier report is brought up to date

- **WHEN** a request whose earlier head was blamed, its report comment
  saying so, gets a new head that changes only docs, and that head
  lands beside the queue
- **THEN** the same comment is edited to the landing beside's report,
  and no second comment is posted

## MODIFIED Requirements

### Requirement: What never lands beside

A submission SHALL take the queue when its work changes the branch's
`.config/sluice.toml` or a setting the task runner names as its own (its
configuration, the toolchain), when the branch's revision is covered by
no result, or by one whose words and revision the branch's state does
not name, or when the task runner reads every check as affected, as the
command runner does.

#### Scenario: The policy

- **WHEN** a submission changes only `.config/sluice.toml`
- **THEN** it takes the queue

#### Scenario: A revision no result covers

- **WHEN** the branch was moved by something other than sluice and a
  submission changes only docs
- **THEN** it takes the queue

#### Scenario: State that names no covering result

- **WHEN** the branch's state says its revision is covered, as a sluice
  from before this change wrote it, naming no result, and a submission
  changes only docs
- **THEN** it takes the queue and lands through it, and the next docs
  submission after that landing lands beside, covered by the result at
  that landing

#### Scenario: A runner that reads every check

- **WHEN** the branch's runner is the command runner
- **THEN** no submission lands beside the queue

#### Scenario: The reading fails

- **WHEN** the task runner cannot answer its affected reading
- **THEN** the submission takes the queue and the round goes on

### Requirement: The outcome says it landed beside the queue

The outcome of a landing beside SHALL be `landed`, delivered as any
outcome is; `sluice submit`, `sluice wait` and the notice SHALL say
"landed as <rev> beside the queue: no required check reads its
changes", and their JSON SHALL carry `beside: true`. A pull request's
`sluice` check SHALL succeed, titled "landed as <rev> beside the queue".

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
