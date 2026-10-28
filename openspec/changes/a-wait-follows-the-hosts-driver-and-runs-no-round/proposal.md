# A wait follows the host's driver and runs no round

## Why

Today every `sluice wait`, waiting `sluice submit` and waiting `wt
submit` runs sluice's rounds itself, even where `sluice run` is already
running on the host and doing the same work. On a host with many agents
that is dozens of processes each reading the records and starting rounds
at once, and the host's bound on checks is kept by whichever of them
counts first. `sluice run` is not even the host's reader: a watch or a
wait holds that, and `sluice run` reads for itself beside it.

Clause 135 settles it (ADR 0068): sluice stays a plain command line
that needs nothing running, and where a host chooses to run `sluice
run`, that one process reads and drives the repository on the host. A
wait or a watch that finds it live follows its document and runs no
round of its own; one that finds none runs rounds as before (129, 135,
C.5 worktrunk row; rule core.31).

## What Changes

- `sluice run` marks itself as its repository's driver on the host for
  as long as it runs, so every other sluice command on the host can tell
  a live driver from none.
- `sluice run` becomes the host's reader of its repository: it writes
  the overview document every watch and wait follows, each pass. A watch
  or a wait that was the reader when `sluice run` started lets the
  reader's place go within its next tick and follows; while the driver
  lives, no watch or wait takes the reader's place.
- A `sluice wait`, a waiting `sluice submit` (one worktree or several),
  and a wait on a pull request run no round, and take no pull request in,
  while the driver is live. They read their submission from the
  driver's document, and say and exit exactly as they do today.
- A second `sluice run` of the same repository on the host runs no round
  while the first lives. It says so once, and takes over at its next
  pass once the first stops.
- A wait or watch that finds no driver is unchanged: a wait runs rounds
  between reads, whether it reads the records itself or follows a watch.
  When a driver stops, waits run rounds again within their next tick.
- With waits running no rounds, only the driver starts checks on such a
  host, so the bound on checks (71) is that one process's to keep.
  Nothing in how the bound is counted changes.
- The site's command and state references say who runs the rounds when
  `sluice run` is live, and name the driver's file in the host's state.

Out of scope:

- The README's sentence that waits follow a running `sluice run` is the
  chore `the-readme-says-waits-follow-a-running-driver`.
- Recording the submitting session, pushing an outcome into the
  submitter's pane, and telling untold outcomes at a session's start
  (51) are `an-entry-records-its-submitting-session-and-the-round-pushes-its-outcome`.
- A newer watch taking an older reader's lease is
  `a-newer-watch-replaces-an-older-reader`, built on its own bolt. This
  change gives way to the driver the same way that one gives way to the
  lease holder. Whichever lands second composes the two, as described in
  the design.
- A wait on a pull request still reads its request from the forge each
  tick, as today. Moving that read to the reader is not this change's
  work.
- `sluice step` is a single round, not a driver. A wait does not follow
  it.
- Starting `sluice run` as a service on any host is the host's choice
  (135). sluice ships nothing that starts it.

## Capabilities

### New Capabilities

- `host-driver`: `sluice run` as a repository's one driver on a host.
  It is the host's reader of that repository. A second run drives
  nothing while it lives. Waits and watches that find it follow it and
  run no round. Where none is live, waits run rounds as before.

### Modified Capabilities

- `overview-reader`: a watch or wait no longer becomes the reader while
  the driver lives, and gives the reader's place to the driver when it
  starts. A follower wait runs rounds only where no driver is live.
- `submission-wait`: a wait runs rounds between reads only where no
  driver is live. Where one is live, it follows the driver's document
  to the outcome.

## Impact

- `crates/sluice/src/reader.rs`: the driver's lock beside the reader's
  files. A reader gives way to a live driver, and a follower leaves a
  free lock to it.
- `crates/sluice/src/lib.rs`: `sluice run` holds the driver's lock and
  writes the reader's document each pass. Every wait loop skips its
  round and its repeated pull-request intake while the driver lives.
  These are `wait_on`, `submit_pull` and `submit_pull_group`.
- `crates/sluice/src/overview.rs`: the watch's reader gives way to the
  driver. The document a pass writes is the one a watch's reader writes.
- Tests in `crates/sluice/tests/cli.rs` and unit tests in `reader.rs`.
- Site: `site/src/content/docs/reference/commands.md` and
  `site/src/content/docs/reference/state.md`.
- No policy key, configuration key or exit code changes. A plain install
  with no `sluice run` behaves as it does today.

## Touches

```
crates/sluice/src/reader.rs
crates/sluice/src/lib.rs
crates/sluice/src/overview.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/support/mod.rs
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/state.md
openspec/specs/host-driver/spec.md
openspec/specs/overview-reader/spec.md
openspec/specs/submission-wait/spec.md
```
