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
- A write to GitHub's API (a `POST` or `PATCH`) is tried again only when
  GitHub never judged it; one whose answer was not read is reported.
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
- `crates/sluice/src/profiles/github/app.rs`, `rest.rs`, `mod.rs`:
  every API call through the policy, its classing of HTTP failures, a
  401 buying a token afresh.
- `crates/sluice/src/lib.rs`, `crates/sluice/src/main.rs`: the wait's
  `Unread` loop gone; each command hands its bound to the policy.
- `Cargo.toml`, `crates/sluice/Cargo.toml`, `Cargo.lock`: `backon`.
- Tests: `crates/sluice/tests/pushed_refs.rs`, `remote_refs.rs`,
  `cli.rs`, `profiles/forge.rs`, `profiles/forge/github.rs`,
  `support/http.rs`.
- Docs: `site/src/content/docs/guides/github-app.md`,
  `site/src/content/docs/reference/commands.md`.
- Sources: clauses 24, 45, 46, 104, 105, 122, 133, 134, C.5, C.6 and the
  pushed refs row of C.7 (blueprints c457619); ADRs 0017, 0061, 0062;
  rules core.4, core.5, core.29 (sluice 9062bac1).

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
crates/sluice-seams/src/lib.rs
crates/sluice-fakes/src/lib.rs
crates/sluice/tests/**
site/src/content/docs/guides/github-app.md
site/src/content/docs/reference/commands.md
```
