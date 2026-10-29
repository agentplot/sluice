## Why

sluice's app on GitHub (installation 171576254) calls the REST API
9,000 to 12,000 times an hour against an allowance of 5,200, and is
refused for about fifteen minutes of every hour. GitHub warns that an
integration that keeps calling while it is limited may be banned.

The calls are intake's. Every pass of `sluice run`, every three seconds,
on every branch that takes pull requests, lists the open requests; for
each one, drafts included, it reads sluice's own check runs on the head;
for each one out of draft, its reviews and the other systems' checks and
statuses; for one held or sent back, its timeline; for one with a
report, its comments and check runs again. Ten repositories on
mac-studio share the installation. Every `sluice wait`, waiting submit
and `sluice step` runs the same intake every three seconds, and a wait
on deneb runs it on branches mac-studio's `sluice run` drives. A 403
saying the limit is spent is read as GitHub's answer, so a submit or a
wait fails on it.

Clause 134 as amended on 2026-10-28 and clause 136 added the same day
(blueprints 8bdf474), record 0071 and rules core.29, core.31 and core.32
carry the user's decisions of 2026-10-28 that this change builds.

## What Changes

- **A spent allowance pauses the host.** A 403 or 429 whose words or
  headers say a limit is spent, an answer leaving
  `x-ratelimit-remaining` at 0, and a GraphQL `RATE_LIMITED` answer
  start a pause until `retry-after` or `x-ratelimit-reset` (a minute,
  then longer, where GitHub names neither), written in the host's state
  per installation and per allowance. Every process of sluice on the
  host reads it before it calls GitHub and makes no call while it
  stands. A waiting submit, a wait or a status waits it out within its
  own `--timeout`, saying once until when; a submit with `--no-wait`
  fails at once, exit 3, naming when the pause ends (the user,
  2026-10-28); a round of `sluice run` or
  `sluice step` passes intake by, says so once, and lands what has
  passed. This is a class of rule core.29's one retry policy.
- **Every read is conditional.** Every `GET` to GitHub carries the ETag
  and last-modified time of the last answer at its address, kept in the
  host's state for every sluice process there; a 304 is answered from
  the body kept with them, at no cost to the allowance. No token is
  written anywhere.
- **Discovery apart from change.** The open requests into a branch are
  asked for at the host's `[forge] discovery` interval, a minute by
  default, or at once after a submit on the host opened or moved one,
  after the branch moved, or after its records moved. Between, a tick
  asks GitHub nothing. The interval's last asking is in the host's
  state, so a restarted process asks nothing it shows was just asked.
- **A request read again only once it moved.** A request's reviews,
  checks, ready marks and comments are read again only once its head,
  its `updated_at` or its entry's state moved, or, out of draft and
  still waiting on its conditions, at each discovery. A draft is read by
  the list alone. sluice's check and report are posted only when what
  they say changes, compared with what the host kept of its last
  posting, and read back from GitHub only when the host holds none.
- **BREAKING (behaviour):** a request opened by hand is taken up to a
  discovery interval later than today's three seconds; a review or
  another system's check is seen at the next discovery; a waiting submit
  or a wait meeting a spent allowance waits where it failed.
- The host's configuration reference, the GitHub App guide, the pull
  requests guide, the commands reference and the state reference say
  what sluice now does.

## Capabilities

### New Capabilities

- `forge-allowance`: GitHub's word that an allowance is spent, the
  host's pause, and how each kind of caller meets it.
- `forge-reads`: conditional reads with answers kept on the host,
  discovery at the deployment's interval apart from a move on the host,
  a request read again only once it moved, and postings only on change.

### Modified Capabilities

- `remote-retry`: a 429, and a 403 that says a limit is spent, are no
  longer a call the remote never judged or the remote's answer: they
  are the allowance's pause.
- `pull-request-intake`: a ready request is taken at the branch's next
  discovery, not at each round.

## Out of scope

- **mac-studio's gate agent**, which restarts every seventeen seconds
  because docker is missing. It is the host's to fix: docker must be
  running before the agent starts (its container engine started at
  login and ready before the agent is loaded), or the agent unloaded
  until it is. Nothing of sluice's starts the agent or chooses its
  restart pace (135; record 0068). sluice's part is in this change: a
  restarted process asks GitHub nothing the host's state shows was asked
  within the interval, and honours a pause another process recorded.
- **Webhooks.** They need a service GitHub can reach, which clause 135
  refuses for any adopter (record 0071, option 3).
- **Sharing installation tokens between processes.** Tokens stay in
  each process's memory alone (forge-identity); the ETag cache is shared
  and works whichever way GitHub keys its ETags (record 0071).
- **The git fetch each round makes of a branch on a remote**: it costs
  no API allowance; clause 129's idle tick governs it.
- **One host takes a branch's requests in, and one GraphQL query reads
  them** (137; rule core.33): the change
  `one-host-takes-requests-in-and-reads-them-in-one-query`, built on the
  bolt `the-driver-pushes-outcomes-and-waits-follow-it` once it lands.

## Impact

- `crates/sluice/src/retry.rs`: the class `Judged::Limited` with its
  time, and how each policy meets it.
- New `crates/sluice/src/profiles/github/kept.rs`: the host's
  `forge/` state: answers by address, pauses, each branch's readings.
- `crates/sluice/src/profiles/github/app.rs`: the pause read before a
  call and written from GitHub's headers and words; conditional `GET`s;
  `x-ratelimit-*` and `retry-after` read on every answer.
- `crates/sluice/src/profiles/github/rest.rs`, `forge.rs`, `fake.rs`:
  `Pull` gains `updated_at` and `merged`.
- `crates/sluice/src/profiles/github/pulls.rs`: discovery, the readings
  per request, postings on change, a wait's own request read at most
  once in a quarter of the interval.
- `crates/sluice/src/instance.rs`: `[forge] discovery`.
- `crates/sluice/src/lib.rs`, `main.rs`: the policy each command gets,
  a round passing a spent allowance by.
- Tests: `crates/sluice/tests/profiles/forge/github.rs`,
  `profiles/forge.rs`, `profiles/pull_requests.rs`, `cli.rs`,
  `instance.rs`, `support/http.rs`.
- Docs: `site/src/content/docs/reference/configuration.md`,
  `reference/commands.md`, `reference/state.md`,
  `guides/github-app.md`, `guides/pull-requests.md`.
- Sources: clauses 74, 128, 129, 133, 134 (as amended 2026-10-28), 135,
  136 (added 2026-10-28), C.5 pull request, C.7, C.8, S65 (blueprints
  8bdf474); record 0071; rules core.11, core.14, core.29 and core.31 (as
  amended 2026-10-28) and core.32 (sluice 9ddfeb3c).

## Touches

```
crates/sluice/src/retry.rs
crates/sluice/src/lib.rs
crates/sluice/src/main.rs
crates/sluice/src/instance.rs
crates/sluice/src/profiles/github/*.rs
crates/sluice/tests/**
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/state.md
site/src/content/docs/guides/github-app.md
site/src/content/docs/guides/pull-requests.md
```
