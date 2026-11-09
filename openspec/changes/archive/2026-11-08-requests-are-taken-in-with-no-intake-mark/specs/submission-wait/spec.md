## MODIFIED Requirements

### Requirement: A wait across hosts names where its pull request stands
A `sluice wait` or waiting `sluice submit` whose pull request another
host takes in SHALL, in every `--json` document it prints for the open
pull request, `--status`'s and the one at its timeout among them, give
`taken_in_by` and `queue` as `sluice status` gives them. In its words
it SHALL say which host holds the pull request as which submission, or
that it waits to be taken in by the host that keeps the branch, each
time that changes (137).

#### Scenario: Out of time, queued on the other host
- **WHEN** host `b` keeps main and holds `#12` as submission 1406, and `sluice wait main --timeout 2s --json` runs in `#12`'s worktree on host `a`
- **THEN** it exits 5 and prints `pull_request` 12, `taken_in_by` `{"host":"b"}` and `queue` with `"submission":1406`

#### Scenario: The status a submit reads
- **WHEN** the same, and `sluice submit main --status --json` runs in that worktree
- **THEN** it prints `head`, `draft`, `taken_in_by` and `queue` beside `pull_request`, `branch` and `state`

#### Scenario: Its words
- **WHEN** `sluice wait main` follows `#12` on `a` while `b` holds it as submission 1406
- **THEN** it prints a line naming `#12`, submission 1406 and `b`, once, and again only when one of them changes

#### Scenario: Not yet taken in
- **WHEN** host `b` keeps main and `#13` needs one review, and `sluice wait main --timeout 2s --json` runs in `#13`'s worktree on `a`
- **THEN** it exits 5, its `queue` reads `"taken_in":false` and `"waits_for":["1 review"]`, and its `taken_in_by` reads `{"host":"b"}`
