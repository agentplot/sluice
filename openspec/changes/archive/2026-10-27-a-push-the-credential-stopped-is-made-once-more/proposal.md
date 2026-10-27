## Why

GitHub's git servers now and then refuse an app token sluice has just
bought, reading a private repository as not found and a push as
permission denied, and accept the same token a few seconds later. A
submit, a landing or a bolt's creation then fails at random (signal
2026-10-26-madswan-ops-16f1e125/01, afterthought/paymaster on deneb with
sluice 0.44.0). The chore before this one makes a refused fetch again on
a fresh token, but never a push, so a push still fails on a fresh
token's first use. Clause 134 says a push refused this way is made once
more too, with the lease of the first (ADR 0062).

## What Changes

- A push the remote refused on sluice's credential before it took up any
  ref (not found, permission denied or unauthenticated, and no ref's
  answer in git's report) is made once more, on a token bought afresh
  and five seconds old, with the same refspec and the same lease. This
  covers every push sluice makes: a landing, a branch made or removed by
  `sluice branch`, a session's branch pushed for its pull request, a
  submitted head, and the records' refs on the remote. A read refused
  the same way keeps the second try it already has.
- A push the remote refused at the ref (the lease stale, the branch
  protected, any `[rejected]` or `[remote rejected]` line) is the move
  refused, as now, and is never made again.
- A leased push whose answer git never read (a connection cut, a server
  error) is never made again. sluice reads the branch: if the remote
  holds what the push set, the move is done; otherwise the failure is
  reported with what the remote holds, and a landing is settled by the
  next round as now.
- A refusal that stands after the second try is reported in the remote's
  own words, followed by what each refused token was: the app, the
  repository, the installation, its repositories and permissions, and
  its age. There is no third try.
- The GitHub app guide says what sluice does when GitHub refuses a token
  it has just given.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `forge-identity`: a read or a push refused on sluice's credential is
  made once more on a fresh one; a push refused at the ref, or whose
  answer went unread, is not; a refusal that stands names both tokens.
- `branch-lifecycle`: a branch made or removed whose push went
  unanswered is decided by what the remote holds.

## Out of scope

- Writing a refused read into the branch's log (133): the unit
  `a-refused-read-of-the-remote-is-in-the-branchs-log` does that.
- Accepting ADR 0062 and rule core.29: both are proposed and wait on the
  user's yes on their card. This change builds what they describe and
  cites them; it does not change their status.
- The host's own credentials (no `[forge]` table): a refusal there is
  not sluice's token, and nothing is made again.

## Impact

- `crates/sluice/src/profiles/pushed_refs.rs`: the shared git call for
  the remote makes a refused push once more, and the leased push reads
  the branch after an answer it never got. Every push in the remote
  refs profile goes through the same call, so the records' refs change
  with no edit of their own.
- `crates/sluice/tests/pushed_refs.rs` and the fake HTTP server under
  `crates/sluice/tests/support/`: a fake remote that answers a push.
- `site/src/content/docs/guides/github-app.md`.
- Sources: clauses 24, 45, 46, 104, 105, 122, 134, C.4 pushed refs (the
  row d8b70a9 amended), C.5, C.6; ADRs 0017, 0062; rules core.4, core.5
  (its details as amended by 0062), core.29 (proposed).

## Touches

```
crates/sluice/src/profiles/pushed_refs.rs
crates/sluice/tests/pushed_refs.rs
crates/sluice/tests/support/**
site/src/content/docs/guides/github-app.md
```
