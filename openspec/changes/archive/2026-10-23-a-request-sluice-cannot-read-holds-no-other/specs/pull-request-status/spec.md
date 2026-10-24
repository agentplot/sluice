## MODIFIED Requirements

### Requirement: A waiting pull request shows its forecast
Each round SHALL forecast every waiting pull request against the branch
and everything queued, running no check, and include the result in its
`sluice` check: that it would merge without conflict behind the queued
submissions, or which pull requests and files it conflicts with. A
waiting pull request whose head sluice cannot fetch SHALL get no
forecast: a draft's check SHALL read `waiting: a draft`, and an
out-of-draft one's SHALL name what it is missing, each with no forecast
in its details. A pull request missing nothing whose head sluice cannot
fetch, or one whose conditions sluice cannot read, SHALL have nothing
new posted that round.

#### Scenario: A conflict seen early
- **WHEN** a waiting pull request changes a file that a queued pull request changes in a way git cannot merge
- **THEN** its `sluice` check names the queued pull request and the file, and no check of the branch's gate runs for it

#### Scenario: A draft with no head is only a draft
- **WHEN** a draft's head cannot be fetched
- **THEN** its `sluice` check reads `waiting: a draft`, and its details hold no forecast

#### Scenario: Ready but unreadable, nothing new is posted
- **WHEN** a pull request is out of draft with every condition holding and its head cannot be fetched
- **THEN** sluice posts nothing new on it that round, and its earlier check, if any, stands
