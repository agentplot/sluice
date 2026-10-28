# herdr-status Specification

## Purpose
sluice's queue on a key in herdr: the status popup for the focused pane's
repository, the same view in its own tab, and a line of it in herdr's tab
bar, all read from the watcher's state.

## Requirements

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

### Requirement: The popup redraws on change

The popup SHALL redraw within a second of the watcher writing a new
document for its repository, whether or not the branch's version moved,
and SHALL keep its clocks current, without a key being pressed and without
running a watch of its own.

#### Scenario: A landing

- **WHEN** the popup is open and the watcher writes a document in which
  `#12` and `#13` landed
- **THEN** within a second TESTING no longer lists them and RECENT shows
  the landing

#### Scenario: A task ends

- **WHEN** the popup is open and the watcher writes a document in which
  only a task of the combination under test has ended
- **THEN** within a second the count, the bar and the task's line show it

### Requirement: The popup's keys

In the popup `q` and Escape SHALL close it, `j` SHALL jump to the culprit's
pane as the `jump` action does, and `a` SHALL switch between the focused
repository and every repository the watcher holds.

#### Scenario: Close

- **WHEN** the user presses Escape in the popup
- **THEN** the popup closes and the terminal is left as it was

#### Scenario: All repositories

- **WHEN** the watcher holds two repositories and the user presses `a`
- **THEN** the popup shows both

### Requirement: Outside a sluice repository

Opened from a pane that is in no git repository, the popup SHALL name
the pane's folder as in no git repository and show every repository the
watcher holds. Opened from a repository without sluice's records, or from
a pane in no git repository while the watcher holds none, it SHALL say
`No sluice records in this repository.` and wait for `q` or Escape; while
the watcher has not yet read a repository it SHALL say it is waiting for
sluice.

#### Scenario: A plain directory

- **WHEN** the user opens the popup from a pane in their home directory
  while the watcher holds no repository
- **THEN** the popup shows `No sluice records in this repository.` and no
  error

#### Scenario: A plain directory, repositories held

- **WHEN** the user opens the popup from a pane in their home directory
  while the watcher holds a repository with `#95` under test
- **THEN** the popup says the home directory is in no git repository and
  shows that repository's queue

### Requirement: The dashboard opens in its own tab

The `dashboard` action SHALL open the popup's view in a tab of its own,
showing every repository the watcher holds.

#### Scenario: Opened

- **WHEN** the user runs the `dashboard` action
- **THEN** a tab opens listing every held repository's branches

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

### Requirement: Setup shows, never edits

The `setup` action SHALL show the snippets to add to herdr's config for the
key, the sidebar rows and the tab-bar segment, and SHALL change no file
outside the plugin's own directories. The segment's command SHALL name
the copy in the plugin's state directory with `$HOME` in place of the
home directory, so the same line serves every host whose herdr keeps its
state at the same place below the home directory, and SHALL name the
absolute path only when the state directory is not under `$HOME`.

#### Scenario: Setup

- **WHEN** the user runs `setup`
- **THEN** a popup shows a `[[keys.command]]` binding `prefix+s` to
  `agentplot.sluice.status`, the sidebar rows, and the segment, and herdr's
  config is unchanged

#### Scenario: One line for every host

- **WHEN** setup runs with the state directory at
  `$HOME/.local/state/herdr/plugins/agentplot.sluice`
- **THEN** the segment's command is `sh
  "$HOME/.local/state/herdr/plugins/agentplot.sluice/segment/bin/segment.sh"`,
  and run through `/bin/sh -lc` under two different home directories,
  each holding the copy, it prints that home's queue

### Requirement: The popup fits its pane

The popup and the dashboard SHALL never print more lines than their
terminal has, read again at each redraw. Where the frame does not fit,
each combination's task list SHALL be cut to its running and failed tasks
with a line counting the rest by state, then RECENT's oldest entries
SHALL go, and only then the bottom SHALL be cut with a line saying how
many lines went. Where the frame fits, every task is listed.

#### Scenario: Thirty tasks in twenty lines

- **WHEN** the popup is twenty lines high and a combination under test
  lists thirty tasks, eighteen passed, one failed, one running and ten
  waiting
- **THEN** the frame is at most twenty lines, keeps the branch's heading,
  the combination, the failed and the running task and the keys, and
  shows `… 18 passed, 10 waiting` in place of the rest

#### Scenario: Shorter still

- **WHEN** the popup is twelve lines high over the same document
- **THEN** the frame is at most twelve lines, keeps the failed task, and
  ends its rows with `… N more lines`

### Requirement: The popup lists agents' checks

The popup SHALL show, after the branches, a CHECKING section with one row
per entry of `sessions[]` with a `run`, or with a `last_result` that
ended in the last 15 minutes, worded as the board's pane words it, and
under it `host: R of B runs · Q queue · S agent(s)` while any run counts,
leaving out a part that is zero.
A check SHALL never be listed as a submission or worded as testing.

#### Scenario: A check under way

- **WHEN** worktree `fix-cards` runs its checks against `main`, 3 of 8
  tasks ended, for 70 s, and `room` is `{bound: 6, queue: 3, sessions: 1,
  reserve: 0}`
- **THEN** the popup shows CHECKING with `fix-cards → main 3/8 ▓▓▓░░░░░░░
  1m10s`, then `host: 4 of 6 runs · 3 queue · 1 agent`

### Requirement: The branches keep the overview's order

The popup and the tab bar SHALL list a repository's branches in the order
of its overview's `branches[]`, and SHALL name no branch in their code;
scrolling and the count of branches with nothing queued SHALL hold in
that order. The popup and the dashboard SHALL show the branch the focused
pane's `sluice_branch` token names first, even when idle, and leave it
out of that count.

#### Scenario: The policy's order

- **WHEN** the overview lists `release`, an idle `bolt/a`, then `main`
- **THEN** the popup shows `release` above `main` and one other branch
  with nothing queued

#### Scenario: The named branch first

- **WHEN** the overview lists `release`, an idle `bolt/a`, then `main`,
  and the focused pane has `sluice_branch=bolt/a`
- **THEN** the popup shows `bolt/a`, then `release`, then `main`, and no
  line counting branches with nothing queued

#### Scenario: A named branch with no records

- **WHEN** the focused pane has `sluice_branch=hotfix` and no document of
  its repository lists `hotfix`
- **THEN** the popup's first branch line is `hotfix · no records`

### Requirement: The segment keeps its last line under load

The segment's entry that `setup` shows SHALL run it every 5 seconds with
a limit of 30 seconds, so a loaded host's slow run leaves the last line
in place rather than clearing it.

#### Scenario: The segment's time limit

- **WHEN** the user runs `setup`
- **THEN** the segment's entry reads `interval_seconds = 5,
  timeout_seconds = 30`

### Requirement: The segment is the server's

The segment SHALL print the queue held in the state of the host it runs
on, for the repository of the directory it runs in, and SHALL need
nothing of its environment but that host's home directory, herdr's
socket path and `PATH`. herdr runs it on the server the panes run on, so
a client attached from another host shows the same line as one on the
server's host.

#### Scenario: Only the server's environment

- **WHEN** the segment is run through `/bin/sh -lc` in the focused
  pane's directory, with an environment holding only the server's
  `HOME`, `HERDR_SOCKET_PATH` and `PATH`, and that server's watcher holds
  a queue for the repository
- **THEN** it prints that queue's line, the same as when run with the
  plugin's whole environment

### Requirement: The popup shows a branch's held places

The popup and the dashboard SHALL show under a branch's heading, headed
RELEASE FAILED, each checkout, worktree and preview of that branch held
because its release failed and tried no more: its name, how many times
it failed, the last line of what failed, and the command to release it,
with `--accept-failure` to free it without, on its host when another's.
They SHALL be worded as the board's `/sluice` pane (123, 26).

#### Scenario: A held checkout

- **WHEN** the repository's document shows `main` with
  `sluice/try/main/48` held after four failed releases ending
  `usage: scripts/preview up`
- **THEN** under `main`'s heading the popup shows RELEASE FAILED, then
  `✗ sluice/try/main/48 held · failed 4 times · usage: scripts/preview up`,
  then `sluice release sluice/try/main/48 · --accept-failure frees it
  without`

#### Scenario: A held worktree

- **WHEN** the document's worktree `/work/sluice/unit-a`, held to `main`,
  has a failed release it no longer tries
- **THEN** under RELEASE FAILED the popup shows `✗ worktree unit-a held`
  with its failure, then `sluice release --worktree /work/sluice/unit-a
  · --accept-failure frees it without`

#### Scenario: Another host's checkout

- **WHEN** the document is `vega`'s and `sluice/try/main/48` is held by
  `deneb`
- **THEN** its row says `held on deneb`, and the command beneath it
  begins `on deneb:`

#### Scenario: An idle branch with a held place

- **WHEN** `main` has nothing queued, no recent event and a held
  checkout
- **THEN** the popup shows `main`'s block with its RELEASE FAILED rows,
  rather than counting it among the branches with nothing queued

#### Scenario: Still being tried

- **WHEN** a checkout's release has failed once and is not yet stopped
- **THEN** the popup shows no RELEASE FAILED row for it

#### Scenario: An older sluice

- **WHEN** a document has no `held_checkouts` and no `release_failed`
- **THEN** the popup shows no RELEASE FAILED heading and reads the
  document as before

### Requirement: The segment counts a branch's held places

The tab-bar segment's line for a branch with held places SHALL end with
`N held: release failed`, after the clause for agents' checks; a branch
with held places and nothing queued SHALL get its line (123).

#### Scenario: Held beside a queue

- **WHEN** the focused repository's `main` has its queue and one held
  checkout
- **THEN** the segment's line for `main` ends `· 1 held: release failed`

#### Scenario: Held with nothing queued

- **WHEN** `main` has nothing queued and two held places
- **THEN** the segment prints `sluice main · 0 submissions · 2 held:
  release failed`

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
