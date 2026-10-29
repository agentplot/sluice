## Why

The change `github-is-asked-only-when-something-moved` makes every
process on a host ask GitHub only what could have moved: a pause on a
spent allowance, conditional reads, discovery at the host's interval and
a request read again only once it moved. Two costs stay.

Every host that reaches a branch still asks GitHub about its open pull
requests. On the agentplot repositories each host keeps its own records
of a shared `main`, so the lead of clause 72 does not exist there: deneb
and mac-studio each take the same pull request into their own queue and
test it twice, and a wait on deneb runs intake on branches mac-studio's
`sluice run` drives. GitHub's ETags depend on the token (record 0071's
amendment), so a kept answer saves a call only to the process whose
token received it, and the saving across hosts comes only from one host
doing the asking.

A request out of draft that waits on its conditions is still read by
REST once per request at each discovery: its reviews, the other systems'
checks and statuses, its ready marks and sluice's own check runs. One
GraphQL query answers all of them for every open request into a branch.

Clause 137 (added 2026-10-28, blueprints 8bdf474), record 0071 and rules
core.32 and core.33 carry the user's decisions of 2026-10-28. The user
decided the same day that a branch no other host has touched gets no
intake push: on a repository only one host uses, the mark's renewal, a
push about every hundred seconds, buys nothing.

## What Changes

- **One host takes a branch's requests in.** Where the records are on
  the remote, only the instance that keeps the branch (72). Where each
  host keeps its own records, the holder of `refs/sluice/intake/<branch>`
  on the remote, a mark renewed by a leased push, taken where absent,
  taken from a wait by a driver at once, taken by anyone once it stood
  unchanged for the lease by the taker's own observation, and let go
  by a wait that ends. A branch no other host has touched gets no mark
  pushed (the user, 2026-10-28). A round on another host withdraws its
  own entries of the branch's requests, naming the host that holds the
  mark. A wait whose request another host takes in takes nothing in and
  runs no round: it follows the request by GitHub's word and sluice's
  check on its head, read at most once in a quarter of the interval.
- **One query per branch.** At a discovery whose conditional list came
  back changed, or while a request out of draft waits on its conditions,
  one GraphQL query reads every open request into the branch with its
  reviews, its head's checks and statuses, its ready marks and sluice's
  own check runs, in place of the REST reads per request.
- The pull requests guide, the GitHub App guide and the commands
  reference say what sluice now does.

## Capabilities

### New Capabilities

- `request-intake-host`: one host at a time takes a branch's requests
  in, by the lead or by the intake mark; the hand-over of entries; a
  wait that follows its request across hosts.

### Modified Capabilities

- `forge-reads`: one query per branch, and the reads per request when
  it is refused.
- `pull-request-intake`: a ready request is submitted on the host that
  takes the branch's requests in.

## Out of scope

- **On one host, a wait that follows `sluice run`**: built by the bolt
  `the-driver-pushes-outcomes-and-waits-follow-it` (its
  `readers/<repository>.driver` lock and `driver_live`), which lands
  before this change; this change composes with it and changes none of
  it.
- Everything `github-is-asked-only-when-something-moved` built: the
  pause, conditional reads, discovery, the readings, postings on change.

## Impact

- `crates/sluice/src/profiles/pushed_refs.rs`: the mark fetched with the
  branch, pushed and deleted under a lease.
- `crates/sluice/src/profiles/github/kept.rs`: the intake mark's
  observation.
- `crates/sluice/src/profiles/github/rest.rs`, `forge.rs`, `fake.rs`:
  the one GraphQL query and its file-backed fake.
- `crates/sluice/src/profiles/github/pulls.rs`: whether the host takes
  in, the intake mark, the hand-over, the follow across hosts, the
  readings filled from the query.
- `crates/sluice/src/lib.rs`: a wait letting the mark go at its end; a
  wait following across hosts.
- Tests: `crates/sluice/tests/profiles/pull_requests.rs`,
  `profiles/forge/github.rs`, `pushed_refs.rs`.
- Docs: `site/src/content/docs/guides/pull-requests.md`,
  `guides/github-app.md`, `reference/commands.md`.
- Sources: clauses 58, 72, 73, 135, 136 and 137 (added 2026-10-28), C.6
  (blueprints 8bdf474); record 0071; rules core.5, core.29, core.32 and
  core.33 (sluice 9ddfeb3c).

## Touches

```
crates/sluice/src/lib.rs
crates/sluice/src/profiles/pushed_refs.rs
crates/sluice/src/profiles/github/*.rs
crates/sluice/tests/**
site/src/content/docs/reference/commands.md
site/src/content/docs/guides/github-app.md
site/src/content/docs/guides/pull-requests.md
```
