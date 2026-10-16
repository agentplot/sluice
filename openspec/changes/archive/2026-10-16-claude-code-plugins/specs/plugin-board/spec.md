# Spec Delta

## Purpose

The queue shown inside a Claude Code session, and a session told its own
submissions' outcomes.

## ADDED Requirements

### Requirement: The band shows the queue

In a repository with records, the board SHALL show one line above the
prompt with each branch's depth, the combination under test and its elapsed
time against its limit, and the latest landing or blame, and SHALL show
nothing while no branch has a queue.

#### Scenario: Testing

- **WHEN** the overview shows `main` with 3 in the queue and `#12 + #13`
  testing for 130 s of a 900 s limit
- **THEN** the band reads `sluice main · 3 queued · testing #12+#13 2m10s/15m`

### Requirement: The pane opens on /sluice

`/sluice` SHALL open a pane listing, per branch, the queue with states,
each combination under test with its submissions, targets and elapsed time,
and recent landings and blames with their log paths.

#### Scenario: Opened

- **WHEN** the user runs `/sluice`
- **THEN** a pane opens with those sections for every branch

### Requirement: A session is told its submission's outcome

When a submission whose `from.path` is the session's worktree lands, fails
or conflicts, the board SHALL raise a toast, and, while waking is on, SHALL
start a turn telling the session the outcome, the blamed tests and their log
paths, once per outcome.

#### Scenario: Blamed while idle

- **WHEN** the session's submission #13 is blamed for `tests:t-cards`
- **THEN** one turn starts saying #13 failed on `tests:t-cards` with its log
  path, and a reload of the mod does not start another
