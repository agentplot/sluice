## ADDED Requirements

### Requirement: A wait on a submission this host handed over follows its pull request
A `sluice wait <id>` whose submission this host handed over SHALL follow
its pull request as a wait whose pull request another host takes in
does, while it is open and no draft, never reporting the submission
withdrawn (122, 137).

#### Scenario: A wait on a submission handed over follows its pull request
- **WHEN** host A, with no driver, is testing `pull/12`, and `sluice wait <id>` on A, for `pull/12`'s submission, finds B holding main's mark
- **THEN** the wait's pass withdraws `pull/12`, ends its run and releases its checkout, runs no round for `main`, and follows `#12` on GitHub, exiting 5 under `--timeout` with `#12` named as the pull request it follows
