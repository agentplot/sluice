# Spec Delta

## MODIFIED Requirements

### Requirement: The pane opens on /sluice

`/sluice` SHALL open a pane listing, per branch, how many submissions it
holds and how many combinations are under test; under TESTING each
combination under test with its targets and elapsed time and the
combinations formed and not running; under QUEUED the submissions with
their states, each from a pull request linked to it on GitHub when the
repository's `origin` is there; and under RECENT recent landings and
blames with their log paths.

#### Scenario: Opened

- **WHEN** the user runs `/sluice`
- **THEN** a pane opens with those sections for every branch

#### Scenario: Headed as the herdr popup is

- **WHEN** the pane shows `main` testing `#12 + #13` with `#14` queued and a
  recent blame
- **THEN** the pane shows the headings TESTING, QUEUED and RECENT, in that
  order, above those lines

#### Scenario: A pull request's submission

- **WHEN** `origin` is `git@github.com:agentplot/sluice.git` and #14 came
  from `pull/41`
- **THEN** #14's row links `PR 41` to
  `https://github.com/agentplot/sluice/pull/41`
