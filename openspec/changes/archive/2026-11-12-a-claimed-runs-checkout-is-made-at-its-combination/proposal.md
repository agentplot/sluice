## Why

A combination's checkout is made with `git worktree add --detach` at the
combination. On a branch whose state is on its remote, the combination
may be formed on another host, and the host that makes the checkout
fetches every open combination of the branch once a round, before it
claims. A pool's job starts its claimed run with no round before it, and
a round's claim reads the state after that fetch, so a combination
opened between the two is claimed but not here.

On 2026-11-12 switchboard-kit's `bolt/a-person-without-the-app-is-told-why`
tried checkouts 20 to 25 at `05b3ce00`, each failing with
`fatal: invalid reference: 05b3ce00…`. Each counted as a run whose
checks could not run (114), so the third in a row read the combination
as not passed (115): #61 and then #62 were blamed with every check not
run, for a commit their host had not fetched. Record 0117 decides it.

## What Changes

- **A claimed run's checkout is made at its combination, however its
  claim reached the host.** Where the host lacks the combination, it
  fetches the combination's own ref,
  `refs/sluice/combinations/<branch>/<id>` (C.6), from the branch's
  remote before it makes the checkout. A host that holds it fetches
  nothing (82).
- **A checkout that cannot be made is a run that never began.** The run
  has no outcome (60): it is open again for any host, nobody is blamed,
  its record names each check with the error's words until its next run
  has a result (16), and it counts toward no runs in a row (115). A
  start that fails once its checkout is made counts as before.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `run-claims`: a claimed run's checkout is made at its combination,
  fetched by its own ref where the host lacks it.
- `run-outcome`: a checkout that cannot be made counts toward no runs in
  a row.

## Out of scope

- **The statement's 114 and 115.** They hold as written: a run whose
  checkout was never made began no check, and is 60's run with no
  outcome. Naming it in 115 is the statement's to do, if the design
  wants it said there.
- **A checkout that can never be made.** It is tried each round until
  the candidate's time limit (66), as a run no host can take is (87).
  A cap of its own waits on a run of that kind being seen.
- **`sluice job run`** of the change `actions-pool`, which starts its
  pool job's run through `start_claimed` and so gets this fetch with it.

## Impact

- `crates/sluice-seams/src/lib.rs`: the state seam's `fetch_candidate`.
- `crates/sluice/src/profiles/remote_refs.rs`,
  `crates/sluice/src/profiles/records.rs`: the remote refs profile's
  fetch of one candidate by its own ref, and its routing.
- `crates/sluice-core/src/step.rs`: `start_run` fetches a combination
  the host lacks before its checkout, and reads a checkout that cannot
  be made as a run that never began (`unmade`, `Started::Unmade`).
- `crates/sluice-fakes/src/lib.rs`: candidates published, fetched one by
  one, and a checkout at a revision the host lacks failing as git does.
- Tests: `crates/sluice-conformance/tests/could_not_start.rs`,
  `crates/sluice/tests/remote_refs.rs`.
- Records and rules: record 0117, rule core.58.
- Sources: clauses 16, 60, 66, 82, 87, 114, 115, 133, C.6.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice/src/profiles/remote_refs.rs
crates/sluice/src/profiles/records.rs
crates/sluice-core/src/step.rs
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/tests/could_not_start.rs
crates/sluice/tests/remote_refs.rs
docs/adr/0117-a-claimed-runs-checkout-is-made-at-its-combination-and-a-checkout-not-made-is-a-run-that-never-began.md
docs/architecture/core.md
site/src/content/docs/architecture/rules.md
openspec/changes/a-claimed-runs-checkout-is-made-at-its-combination/**
```
