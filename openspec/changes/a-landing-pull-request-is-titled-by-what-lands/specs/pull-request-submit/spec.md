## ADDED Requirements

### Requirement: A pull request a submit opens is titled by what lands
A pull request `sluice submit` or `wt submit` opens SHALL be titled by
the subject the submit gives with `--subject`, exactly as written, or,
where it gives none, by the name of the branch submitted. Its title
SHALL never be a commit's message, the first commit's or any other's
(118, C.5 pull request).

#### Scenario: A bolt submitted with no subject
- **WHEN** a worktree on `bolt/review-cards`, whose first commit is `docs(openspec): propose review-cards`, runs `wt submit main --no-wait` and `main` has `pull_requests`
- **THEN** the pull request opened is titled `bolt/review-cards`, and the output says `opened #<n>: bolt/review-cards`

#### Scenario: A session submitted with a subject
- **WHEN** an agent on `unit/review` runs `wt submit main --no-wait --subject "Batch review cards"`
- **THEN** the pull request opened is titled `Batch review cards`, and the output says `pushed unit/review · opened #<n>: Batch review cards`

#### Scenario: The description still carries the commits
- **WHEN** a submit with `--subject "Batch review cards"` opens a pull request from a branch holding two commits
- **THEN** the description holds both commits' messages, whole and oldest first, and the title is not either message's first line

### Requirement: A submit with a subject retitles a pull request sluice opened
A submit that updates an open pull request sluice opened SHALL set its
title to the subject the submit gives, when it gives one that differs
from the title. A submit that gives no subject SHALL leave the title as
it is (C.5 pull request, 120).

#### Scenario: A new head with a subject
- **WHEN** sluice opened `unit/review`'s pull request titled `unit/review`, the agent commits again and runs `wt submit main --no-wait --subject "Batch review cards"`
- **THEN** the same pull request, at the new head, is titled `Batch review cards`

#### Scenario: A new head with no subject
- **WHEN** sluice opened a pull request titled `Batch review cards`, the agent commits again and runs `wt submit main --no-wait`
- **THEN** the pull request at the new head is still titled `Batch review cards`

#### Scenario: A request opened before titles changed
- **WHEN** a pull request sluice opened is titled by its first commit, `docs(openspec): propose review`, and the agent runs `wt submit main --no-wait --subject "Batch review cards"`
- **THEN** the pull request is titled `Batch review cards`

### Requirement: A pull request a person opened keeps its title
A submit that updates a pull request a person opened SHALL NOT change
its title, whether or not the submit gives a subject; it writes the
description and marks the request ready as it does for any request
(120, C.5 pull request).

#### Scenario: A subject on a request opened by hand
- **WHEN** a person opened a pull request from `unit/review` titled `Review cards, first pass`, and an agent on `unit/review` runs `wt submit main --no-wait --subject "Batch review cards"`
- **THEN** the pull request is still titled `Review cards, first pass`, its description holds the branch's commits, and it is ready for review at the head submitted

### Requirement: A subject is one line
`--subject` SHALL take one line of text. A subject that is empty or
holds a line break, or `--subject` given with `--check` or `--status`,
SHALL be refused as a command line sluice cannot read, exit 64, before
anything is pushed or opened.

#### Scenario: Two lines
- **WHEN** an agent runs `sluice submit main --subject "$(printf 'one\ntwo')"`
- **THEN** it exits 64, saying a subject is one line, and nothing is pushed or opened

#### Scenario: An empty subject
- **WHEN** an agent runs `sluice submit main --subject ""`
- **THEN** it exits 64, saying the subject is empty, and nothing is pushed or opened

#### Scenario: With a check
- **WHEN** an agent runs `sluice submit main --check --subject "Batch review cards"`
- **THEN** it exits 64 and nothing is forecast, pushed or opened
