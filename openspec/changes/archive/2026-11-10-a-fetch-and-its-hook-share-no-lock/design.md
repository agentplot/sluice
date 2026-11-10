## Context

See proposal.md for why. Record 0110, the statement's clauses 124, 133
and 134 and S64 as written (agentplot/blueprints 60c25ef), and rules
core.28 and core.29 say what holds. This design says how sluice builds
it.

**The family.** Of the Boomdrive family, only sluice installs a git hook
and holds a lock over its own fetches. No other tool's statement names
either, so nothing needs reconciling.

## Decisions

**D1. Judging reads the state as held.** `Sluice::needing` is asked only
by the protection. It reads the branch's state with `State::read_held`,
which fetches nothing on a branch kept on its remote and is the same
read as `State::read` for one in the host's own files. `judge` asks
`pruned` first, so git's packing passes with no state read.

**D2. The lock is named in the environment of the git that runs under
it.** `fetching` gives back the lock and its path. `call` hands the path
to `git_as`, which sets `SLUICE_FETCH_HELD` on that try's git alone. No
process-wide variable is set.

**D3. Under its ancestor's fetch, a fetch is a fetch that brought
nothing.** Before its first try, `call` asks `under_own_fetch`. That is
true where `SLUICE_FETCH_HELD` names this repository's lock, both paths
made canonical, and a lock taken without waiting finds it held. `call`
then gives back a successful output with nothing printed. Every caller
reads the refs as they stand: `fetch_specs` as fetched, `fetch_ref`
reading its ref, and a tick's beats reading the hosts' refs. The case is
logged at debug and on no branch, since nothing was refused (133).

**D4. A lock found free is taken.** Where the variable names the lock
and the lock is free, as after a detached maintenance outlived its
fetch, the fetch takes the lock as any fetch does.

## Risks / Trade-offs

- A hook under sluice's own fetch that records a refusal writes it from
  the state as last fetched. Where the remote moved since, the leased
  write is refused, and the hook says it could not record the refusal.
  The refusal stands either way.
- A process under a fetch that finds the lock held by another fetch than
  its ancestor's reads the refs as they stand that once, as after a race
  lost (record 0086).
