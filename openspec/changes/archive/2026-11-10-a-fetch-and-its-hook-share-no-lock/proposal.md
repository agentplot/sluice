## Why

sluice deadlocked itself in a repository (signal
2026-11-10-agentplot-ops-635d7c76/01). On deneb on 2026-11-10, in
agentplot/sluice, `sluice submit main --no-wait --json` took pull request
#235 in. Its fetch of the request's head held the repository's fetch
lock (record 0086) while its git fetched. That fetch's maintenance ran
`git pack-refs --all --prune --auto`, and each pruning of a loose branch
file ran the `reference-transaction` hook.

For a deletion of a protected branch, the hook read which submissions
still need the branch before it read whether the packed refs hold it.
That read fetched the branch's state from its remote, and the fetch
waited on the lock its own ancestor held. git waited for the hook, and
the submit for git, for more than forty minutes. Every sluice command
that reads the repository queued behind them. It was seen with sluice
0.73.1, and 0.78.1's reads hung behind it the same way.

Clause 124 and S64 let git's packing through, and the branch-protection
spec has the protection fetch nothing to judge a write. The hook broke
both. Nothing stopped any other sluice that a fetch's git runs from
waiting on its ancestor's lock either. Record 0110 decides the fix, and
rule core.29 names the case.

## What Changes

- **The protection judges with nothing fetched.** git's packing passes
  before any of the branch's state is read. What still needs a branch is
  read from its state as the host last fetched it. Only recording a
  refusal reaches the remote.
- **A fetch names its lock to its git.** While a try holds the
  repository's fetch lock, its git runs with `SLUICE_FETCH_HELD`, the
  lock's path, which every process under that git inherits.
- **A sluice under sluice's own fetch fetches nothing.** A fetch in a
  process whose environment names this repository's fetch lock, while
  the lock is held, takes no lock and runs no git. It reads the refs as
  they stand, as after a race lost (record 0086). A hook that records a
  refusal there writes it from the state as last fetched.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `branch-protection`: a requirement added, that judging a write
  fetches nothing on any branch, git's packing passing before the state
  is read.
- `remote-retry`: a requirement added, that a sluice run under sluice's
  own fetch takes no fetch lock and fetches nothing.

## Impact

- `crates/sluice/src/profiles/pushed_refs.rs`: `call`, `fetching`,
  `git_as`, and `under_own_fetch`, new.
- `crates/sluice/src/protect.rs`: `judge` asks whether the packed refs
  hold a deleted branch before it reads the branch's state.
- `crates/sluice-core/src/ops.rs`: `needing` reads the state as held.
- `crates/sluice/tests/protect.rs`: two tests, each of a fetch whose
  reference transaction runs the hook.
- `docs/adr/0110-...`, and rule core.29's details.
