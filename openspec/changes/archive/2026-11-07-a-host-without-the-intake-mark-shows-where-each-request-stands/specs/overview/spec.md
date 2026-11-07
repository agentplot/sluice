## ADDED Requirements

### Requirement: A branch lists the pull requests another host takes in that this host follows
In `sluice overview`, a branch with pull requests this host follows and
whose submissions its queue does not hold SHALL have `taken_in_by` and
`pull_requests`, as `sluice status` gives them, and the same rows in its
words. A one-shot overview SHALL read them as status does. A reader's
document SHALL carry them as the host last kept them, its tick reading
GitHub for none of them (129, 137; rules core.25, core.31).

#### Scenario: A one-shot overview
- **WHEN** host `b`'s `sluice run` holds main's mark and holds `#12` as submission 1406, and `sluice overview --json` runs on host `a`, which follows `#12`
- **THEN** `main` has `taken_in_by` naming `b` and a `pull_requests` row for `#12` with `queue` naming 1406 and a `read_at`

#### Scenario: A watch reads none of them itself
- **WHEN** `sluice overview --json --watch` is host `a`'s reader for a minute while `a` follows `#12`, and nothing else on `a` runs
- **THEN** its documents carry `#12`'s row, and `a` read neither `#12` nor its check from GitHub in that minute
