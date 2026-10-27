## Why

sluice reaches a remote two ways, git for every fetch and push and
GitHub's HTTP API for pull requests, checks and the app's tokens, and
today it tries a failed call again in hand-written places, each with its
own figure: a wait reads through GitHub's 5xx answers for five minutes,
and a fetch or push GitHub refused on a token just bought runs once more
on a fresh one. Everything else fails on the first hiccup: a round whose
fetch meets a dropped connection or a 503 fails outright, and a token
GitHub refuses twice in its first seconds fails a submit. Clause 134 and
ADR 0062 put every call under one retry policy, a maintained crate's
backoff, and rule core.29 rules out the loops written at the call sites.

## What Changes

- Every call sluice makes to a remote, a git fetch, listing or push and
  every call to GitHub's API, runs under one retry policy: exponential
  backoff with jitter from the `backon` crate, bounded by what called it.
  A waiting submit or `sluice wait` gives it five minutes; a round run
  any other way, a submit that does not wait and a branch command give
  it thirty seconds.
- What the remote never judged is tried again: a connection not made, a
  server unable to answer (5xx or 429, before a push was sent), and a
  credential refused before any ref was negotiated (not found,
  permission denied, unauthenticated).
- A credential the remote refused is bought afresh for the next try,
  settled as a fresh token is today, never the one refused. The host's
  own credentials, where sluice has no app, are never bought afresh.
- A read (a fetch, a listing, an API `GET` or a GraphQL query) is tried
  again whatever happened, since reading changes nothing.
- A push is tried again only when the remote never judged it, with the
  first's refspec and lease, for every push alike: a landing, a branch
  made or removed, a session's branch or head pushed for a submit, and
  the records' refs. A push the remote refused at the ref, the lease
  stale or the branch protected, is the move refused and is not tried
  again. A push whose answer was not read (the connection cut after the
  push was sent, a server error after it) is never pushed again: sluice
  reads the ref, and what the remote holds decides.
- A write to GitHub's API is tried again by what it does (134). One that
  sets a state, a pull request's draft mark, a check run's or a
  comment's or a pull request's text edited, is tried again as a read
  is, since setting the state twice leaves it as once. One that makes or
  starts something, a check run, a comment, a pull request opened, a
  workflow's run, is tried again blind only while the connection was
  never made. After an answer not read, a server error among them,
  GitHub is read first: a check run, a comment or a pull request it
  already holds is carried on and not made again, and one it lacks is
  made then. A workflow's run, which nothing reads back, is never
  dispatched again: a run's dispatch is reported unanswered in the
  branch's log, in the event that claims the run for the pool's job,
  and a wake's in the warning a wake gives today.
- A refusal that stands when the policy's time is spent is reported in
  the remote's own words, followed by what each refused credential was.
  A round's is its one event in the branch's log, as now (133): never a
  line per try.
- **BREAKING (behaviour):** a push GitHub answered with a server error
  before the push was sent is tried again, where today it is made once;
  a token GitHub refuses is tried on fresh tokens until the policy's
  time is spent, where today there is never a third try.
- The second try on a fresh credential in the pushed refs profile and
  the wait's own run through 5xx answers are gone; both are the policy.
  The wait still says it could not read when its five minutes are spent,
  in the same words.
- The GitHub app guide and the commands reference say what sluice now
  does.

## Capabilities

### New Capabilities

- `remote-retry`: the one retry policy every call to a remote runs
  under: what it tries again, what it never tries again, how a refused
  credential is bought afresh, how long it tries for each caller, and
  what it reports when its time is spent.

### Modified Capabilities

- `forge-identity`: the requirement that a call refused on the app's
  token is made once more on a fresh one gives way to `remote-retry`; a
  push answered with a server error before it was sent is tried again;
  a refusal that stands names every refused token and may follow more
  than two tries.

## Out of scope

- The ready mark a pull-request submit waits to see on GitHub's
  timeline (`ensure_readied` in `crates/sluice/src/profiles/github/pulls.rs`):
  its tries wait for GitHub's clock on a call GitHub answered, not for a
  call that failed, so no clause puts it under this policy.
- `release_retries`, a preview's release run once a round under the
  branch's policy: it is no call to a remote and stays as it is (ADR 0062).
- How the branch's log records a refusal (133): the log's event, its
  once-per-cause rule and its end are built and do not change here.
- Recording a wake in the branch's log: a wake has no event today, and
  one lost delays the branch and loses nothing (59, 111), so a wake
  whose dispatch went unanswered keeps the warning a failed wake gives.
- Retrying a call on the host's own credentials that the remote refused:
  sluice cannot buy those afresh, so a push on them is made once, as now.

## Impact

- New `crates/sluice/src/retry.rs`: the policy, over `backon`.
- `crates/sluice/src/profiles/pushed_refs.rs`: every git call to the
  remote through the policy, its classing of git's failures, the
  hand-written second try gone, a push left unanswered settled by
  reading its ref.
- `crates/sluice/src/profiles/remote_refs.rs`: its pushes name the ref
  and value each sets, so an unanswered push is settled by reading it.
- `crates/sluice/src/profiles/github/app.rs`, `rest.rs`, `pulls.rs`,
  `actions.rs`, `check.rs`, `mod.rs`: every API call through the
  policy, its classing of HTTP failures, a 401 buying a token afresh; a
  check run, a comment and a pull request whose answer was not read
  settled by reading GitHub; a dispatch whose answer was not read
  reported, never sent again.
- `crates/sluice-core/src/step.rs`, `crates/sluice-core/src/records.rs`:
  a pool's run whose dispatch went unanswered is claimed as dispatched,
  its claim's event carrying GitHub's words, so the next round reads
  whether the job runs instead of dispatching it again.
- `crates/sluice/src/lib.rs`, `crates/sluice/src/main.rs`: the wait's
  `Unread` loop gone; each command hands its bound to the policy.
- `Cargo.toml`, `crates/sluice/Cargo.toml`, `Cargo.lock`: `backon`.
- Tests: `crates/sluice/tests/pushed_refs.rs`, `remote_refs.rs`,
  `cli.rs`, `profiles/forge.rs`, `profiles/forge/github.rs`,
  `support/http.rs`; `crates/sluice-conformance/tests/peers.rs`.
- Docs: `site/src/content/docs/guides/github-app.md`,
  `site/src/content/docs/reference/commands.md`.
- Sources: clauses 24, 45, 46, 104, 105, 122, 133, C.5, C.6 and the
  pushed refs row of C.7 (blueprints c457619), and 134 as amended for
  forge writes (blueprints e524de8); ADRs 0017, 0061, 0062; rules
  core.4, core.5 (sluice 9062bac1) and core.29 as amended (sluice
  b33c8ca9).

## Touches

```
Cargo.toml
Cargo.lock
crates/sluice/Cargo.toml
crates/sluice/src/retry.rs
crates/sluice/src/lib.rs
crates/sluice/src/main.rs
crates/sluice/src/instance.rs
crates/sluice/src/profiles/branches.rs
crates/sluice/src/profiles/pushed_refs.rs
crates/sluice/src/profiles/remote_refs.rs
crates/sluice/src/profiles/records.rs
crates/sluice/src/profiles/github/*.rs
crates/sluice-core/src/step.rs
crates/sluice-core/src/records.rs
crates/sluice-seams/src/lib.rs
crates/sluice-fakes/src/lib.rs
crates/sluice/tests/**
crates/sluice-conformance/tests/peers.rs
site/src/content/docs/guides/github-app.md
site/src/content/docs/reference/commands.md
```
