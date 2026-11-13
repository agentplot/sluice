## Why

The change `a-claimed-runs-checkout-is-made-at-its-combination` wrote
its decision as record 0117. Before it landed, main landed a record 0117
of its own, the one-process rebuild going first (`e512b40`), and the
gate's `root:adrs` refuses two records with one number. The checkout's
record takes the next free number, 0118, and the requirements that cite
it cite 0118.

## What Changes

- `run-claims`'s requirement that a claimed run's checkout is made at
  its combination, and `run-outcome`'s requirements that a run whose
  tests could not be started is recorded with why and that a run whose
  checkout could not be made counts toward nothing, cite record 0118 in
  place of 0117. Nothing they require changes.

Out of scope: any change to what the requirements say, which waits on
no later change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `run-claims`: the checkout's requirement cites record 0118.
- `run-outcome`: the two requirements that cite the checkout's record
  cite record 0118.

## Impact

- Specs: `openspec/specs/run-claims/spec.md` and
  `openspec/specs/run-outcome/spec.md`, through this change's archive.
- Records and rules: record 0118,
  `docs/adr/0118-a-claimed-runs-checkout-is-made-at-its-combination-and-a-checkout-not-made-is-a-run-that-never-began.md`,
  and rule core.58, renumbered in their own commit.
