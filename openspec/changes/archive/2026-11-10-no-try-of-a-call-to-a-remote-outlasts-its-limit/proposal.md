## Why

On deneb on 2026-11-10, sluice 0.73.1's `status <branch> --all --json`
and `overview --json` on agentplot/sluice hung 10 to 17 minutes each,
again and again, until they were ended by hand. Every other sluice read
of the repository, and the runner's acts behind them, waited (signal
2026-11-10-agentplot-ops-e19d134d/01).

A stack sample of the same hang, made against a remote that takes the
connection and never answers, shows the cause. The status waits on its
git fetch while it holds the repository's fetch lock,
`sluice-fetch.lock` (record 0086). The fetch's helper,
`git-remote-http`, waits on the socket with no limit. A second read of
the repository waits on the lock with no limit.

Clause 134 bounds every call to a remote by what called it: a wait's
minutes, a command's or a round's seconds. Two things broke that bound.
A try had no limit, so a remote that never answered held the call, and
the lock, for as long as it kept the connection. And the bound covered
only the pauses between tries, so slow tries stretched a command's
thirty seconds to minutes.

Record 0109, written on deneb's `fix/main--a-hung-fetch-holds-no-lock`
(66ad662f, with its code at 27ffa26e and its spec at 5fa9d784, on main
at 69bc9934), decides the fix. This change takes it up onto main as
main stands once fix `a-fetch-and-its-hook-share-no-lock` (record 0110)
has landed, since both change how a fetch holds the repository's fetch
lock.

## What Changes

- **No try of a call to a remote runs past thirty seconds.** Each try
  of a git call runs in a process group of its own and is ended at
  `retry::TRY_FOR`, thirty seconds, with everything it started: the
  remote's helper, sluice's credential helper and any hook git runs.
  The forge's API calls and a host's credential command already end at
  thirty seconds, and now take the same one value.
- **A try stopped at its limit is a call whose answer was not read.** A
  read so stopped is tried again. A push so stopped is never made
  again: the ref it set is read, and what the remote holds decides
  (122). Its words are sluice's own, after anything git printed.
- **A fetch waits for the repository's fetch lock no longer than a try
  may take.** A lock held past that is a try the remote never judged,
  tried again after a pause. Where it is held every time, the call
  fails, naming the lock, as no refusal of the remote's. A sluice under
  sluice's own fetch still takes no lock at all (record 0110).
- **No try starts once the call's time is spent,** counted from its
  first try, or from the end of a spent allowance's pause. A command's
  call to a remote that never answers ends after about thirty seconds,
  not after one slow try per pause.
- **One owner of a process run to a limit.** A new module,
  `crates/sluice/src/bounded.rs`, runs a process in a group of its own
  to a limit. A git call to a remote and a host's credential command
  both use it.
- The GitHub App guide says no single try runs past thirty seconds.
- Record 0109 is written on main, and rule core.29's details name the
  hang beside record 0110's case.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `remote-retry`: a try's limit, a stopped try read as an answer not
  read, the fetch lock waited for no longer than a try may take, and
  the call's time counted from its first try.

## Out of scope

- **Taking main.** This change waits on fix
  `a-fetch-and-its-hook-share-no-lock` (record 0110) landing on main.
  Its first task takes main into this branch once main holds it.
- **Flume's and the Boomdrive runner's own git fetches.** Flume's
  `fetch_branch` and the runner's clone fetch have no limit either.
  Neither holds a lock that a sluice read waits on, so neither causes
  this hang. The family has no shared owner of a git call's limit, and
  record 0109 tells the orchestrator so.
- **git's own stall limits, `http.lowSpeedLimit` and
  `http.lowSpeedTime`.** They cover one transport and one stage of a
  try. sluice's limit covers every transport and every stage. Record
  0109 says why.
- **Passing Ctrl-C on to a try in flight.** A try at Ctrl-C ends by
  itself or at its limit, as a forge call's does. The design says why.

## Impact

- `crates/sluice/src/bounded.rs` (new), and `pub mod bounded` in
  `crates/sluice/src/lib.rs`.
- `crates/sluice/src/retry.rs`: `TRY_FOR`, `Policy::try_for`,
  `Policy::trying_for`, and the call's time counted from its first try.
- `crates/sluice/src/profiles/pushed_refs.rs`: `call`, `fetching`,
  `git_as` and `told`, beside record 0110's `under_own_fetch` and
  `SLUICE_FETCH_HELD`.
- `crates/sluice/src/identity.rs`: the credential command on
  `bounded::finish`.
- `crates/sluice/src/profiles/github/app.rs`: the API client's limit is
  `TRY_FOR`.
- Tests: `crates/sluice/tests/pushed_refs.rs`,
  `crates/sluice/tests/support/http.rs`.
- Docs: `docs/adr/0109-…`, `docs/architecture/core.md` (core.29's
  details), `site/src/content/docs/guides/github-app.md`.
- Sources: clauses 122, 133, 134; records 0017, 0062, 0086, 0108, 0109,
  0110; rules core.10, core.28, core.29.
