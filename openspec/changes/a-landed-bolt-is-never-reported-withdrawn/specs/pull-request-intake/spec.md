## MODIFIED Requirements

### Requirement: A changed pull request leaves the queue
A submitted pull request SHALL be withdrawn when its head changes, when
it is moved back to draft, or when it is closed, merged by hand without
its head reaching the branch included. One that leaves the open pull
requests because the branch, as that pass fetched it, holds its head
SHALL NOT be withdrawn: its work landed, by whichever landing took it,
and the round lands its submission as that landing (122, 5). After a
new head, a pull request opened by hand SHALL wait for its conditions
on that head; one a submit opened SHALL be marked a draft and wait for
its submitter's act.

#### Scenario: A new commit while queued
- **WHEN** a commit is pushed to a submitted pull request a person opened by hand
- **THEN** its submission is withdrawn, the submissions behind it are tested without it, and it is submitted again only once its conditions hold on the new head

#### Scenario: A new commit on a pull request a submit opened
- **WHEN** a commit that no submit pushed reaches a submitted pull request that `wt submit` opened, such as a fix another submission landed on its branch
- **THEN** its submission is withdrawn, sluice marks the pull request a draft, and no later round submits it, whatever its conditions

#### Scenario: Closed while queued
- **WHEN** a submitted pull request is closed
- **THEN** its submission is withdrawn

#### Scenario: Landed by another queue while queued
- **WHEN** a submitted pull request is under test, and another queue of the branch, a second copy of the repository, lands its head, so GitHub calls it merged
- **THEN** its submission is not withdrawn: it is landed as that queue's landing commit, its run ended, nothing more lands of it, and `sluice wait` on it exits 0
