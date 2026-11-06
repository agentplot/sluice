## MODIFIED Requirements

### Requirement: A wait whose pull request another host takes in runs nothing for it
A `sluice wait` or waiting `sluice submit` whose pull request another
host takes in SHALL take nothing in and run no round for it. It SHALL
follow the pull request by GitHub's word, open, merged or closed, and
sluice's check on its head, each read conditionally and no more often
than once in a quarter of the host's `discovery` (137). A `sluice wait
<id>` whose submission this host handed over SHALL follow its pull
request so while it is open and no draft, never reporting the
submission withdrawn (122).

#### Scenario: Read at its pace
- **WHEN** a wait on host A follows `#12`, which host B takes in, for one minute with `discovery` at sixty seconds
- **THEN** it read `#12` and its check from GitHub no more than four times each, and ran no round and no intake

#### Scenario: A wait on a submission handed over follows its pull request
- **WHEN** host A, with no driver, is testing `pull/12`, and `sluice wait <id>` on A, for `pull/12`'s submission, finds B holding main's mark
- **THEN** the wait's pass withdraws `pull/12`, ends its run and releases its checkout, runs no round for `main`, and follows `#12` on GitHub, exiting 5 under `--timeout` with `#12` named as the pull request it follows
