## ADDED Requirements

### Requirement: The popup names each submission with its subject

Every submission the popup names SHALL stand with its subject, or its
branch where its submit gave none (26): a combination as
`main + #12 <subject> + #13 <subject>`, a QUEUED row as `#14 <subject>`
with the subjects padded so the states line up, and a RECENT line's
submissions separated by commas. A submission the document gives no
subject or branch for SHALL be shown by its number alone.

#### Scenario: A landing of two

- **WHEN** #12, given `Batch review cards`, and #13, given none from
  `unit/forecast`, landed together as `c41e8f2`
- **THEN** RECENT shows `✓ <time> #12 Batch review cards, #13
  unit/forecast landed as c41e8f2`

#### Scenario: A document from a sluice that names no subjects

- **WHEN** the document's branch has no `recent_subjects` and no
  `recent_from`, and a recent line says #11 landed
- **THEN** RECENT shows `#11 landed as <revision>`, as before

### Requirement: The popup draws a subject on one line

The popup SHALL draw a submission's subject on one line, each run of
control characters in it as one space.

#### Scenario: A subject with control characters

- **WHEN** #14's subject holds a line break and an escape sequence
- **THEN** its QUEUED row is one line, with a space where each run of
  them was, and the popup's colors are unchanged by it

### Requirement: The segment names each submission with its subject

Each submission the tab-bar segment's line names, in a combination, in
what is waiting and in the last event, SHALL stand with its subject, or
its branch where its submit gave none, as the popup names it (26).

#### Scenario: What is waiting and the last event

- **WHEN** #14, given `Herdr popup`, and #15, from `unit/toasts` with no
  subject, are queued, and the last event is #11, given `Fix the
  forecast's dates`, landed as `9ecc34a`
- **THEN** the segment's line holds `· #14 Herdr popup, #15 unit/toasts
  waiting · last #11 Fix the forecast's dates landed as 9ecc34a`

## MODIFIED Requirements

### Requirement: The popup shows the focused repository's queue

The `status` action SHALL open a popup for the focused pane's repository
listing, per branch, TESTING (each combination under test with its tests,
how many of its tasks have ended with a bar of ten cells when its document
lists them, its elapsed time against its limit, and beneath it each task
with its state), QUEUED (the submissions in the queue with their states)
and RECENT (the latest landings, blames and conflicts), worded as the
Claude Code board's `/sluice` pane.

#### Scenario: Testing

- **WHEN** the repository's document shows `main` testing `#12 + #13` for
  130 s of a 900 s limit with tests `#check-fast #check-tools`, #12 given
  `Batch review cards` and #13 given none from `unit/forecast`, `#14`
  queued from `pull/41` with the subject `Herdr popup`, `#11` landed as
  `9ecc34a` with the subject `Fix the forecast's dates`, and `#9` blamed,
  submitted from `unit/sidebar` with no subject
- **THEN** the popup shows `▶ testing main + #12 Batch review cards + #13
  unit/forecast  2m10s / 15m` under TESTING with the tests beneath it, a
  row beginning `#14 Herdr popup` with its state `queued` under QUEUED,
  and `✓ 19:41 #11 Fix the forecast's dates landed as 9ecc34a` and
  `✗ 19:42 #9 unit/sidebar blamed on 4f1c2aa` under RECENT

#### Scenario: Testing, with tasks

- **WHEN** that combination's document lists five tasks, three of them
  ended and `sluice:e2e` running
- **THEN** the popup shows `▶ testing main + #12 Batch review cards + #13
  unit/forecast  3/5 ▓▓▓▓▓▓░░░░  2m10s / 15m`, and under its tests the
  ended tasks in the order they ended with how long each took, then
  `… sluice:e2e  running`, then the waiting task

#### Scenario: A list it cannot read

- **WHEN** a document's tasks include an entry with no target
- **THEN** the popup leaves that entry out of the count and the list

#### Scenario: Two hosts

- **WHEN** the watcher holds documents for the repository from two hosts
- **THEN** the popup shows one block per host, each headed by its
  repository and host

### Requirement: The tab-bar segment

The plugin SHALL provide a command for herdr's `tab_bar_right` that prints
one line for the focused pane's repository, worded as the board's band,
with a combination's task count and bar and never its tasks, with the
band's clause when agents' checks hold the queue back, and prints nothing
while no branch there has a queue. It SHALL read only the documents of
the focused pane's repository, so a document of another repository never
changes or blanks its line.

#### Scenario: Queue

- **WHEN** the focused repository's `main` has 3 submissions with
  `#12 + #13` testing for 130 s of 900 s, three of its five tasks ended,
  #12 given `Batch review cards` and #13 given none from `unit/forecast`
- **THEN** the segment prints `sluice main · 3 submissions · testing
  main + #12 Batch review cards + #13 unit/forecast 3/5 ▓▓▓▓▓▓░░░░
  2m10s/15m` followed by what is waiting and the last event

#### Scenario: Held back by checks

- **WHEN** the same queue has no free run on the host and one agent's
  checks hold a run
- **THEN** the segment's line ends `· 1 slot to an agent's checks`

#### Scenario: Empty

- **WHEN** no branch of the focused repository has a queue
- **THEN** the segment prints nothing

#### Scenario: Another repository's document is unreadable

- **WHEN** the focused repository's `main` has that queue and the
  watcher's state also holds a document of another repository that is
  not JSON
- **THEN** the segment prints the focused repository's line, the same as
  without that document
