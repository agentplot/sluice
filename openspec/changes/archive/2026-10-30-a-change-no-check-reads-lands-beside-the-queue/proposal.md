# Proposal

## Why

A submission whose work no required check reads already runs nothing
(15; record 0044), yet it joins the chain and lands only when the
combinations of code it was formed into land: a docs-only record waited
out the checks of two code submissions. Record 0075, accepted, has it
land at once beside the chain, and the combinations under test keep
their runs and land with it (140, S68; 8, 10, 23 and 24 as amended;
rule core.39).

## What Changes

- A round, before forming, asks the check's affected reading of each
  queued entry in no candidate, under the gate at the branch's current
  revision: between that revision and the revision extended by the work,
  and between each formed candidate as it would land now and that
  candidate extended by the work. When none names a required check, the
  work merges cleanly onto all of them and onto the work of every entry
  ahead, changes neither the policy nor the check profile's own
  settings, and the branch's revision is covered by a verdict, the entry
  lands beside the chain in that round, with nothing run.
- A candidate formed before such a landing keeps its runs, keys and
  verdict, moves onto the revision the landing made, and lands as its
  tested tree with that work; it is never formed or tested again for it.
- Work a formed candidate's checks would read, or that conflicts with an
  entry ahead, takes the chain and says which entry it waits behind and
  why; work read at the branch's revision takes the chain as before.
- The leading heads of a group submit that each qualify land beside the
  chain in the act's order.
- The check seam gains the affected reading as a query; the moon runner
  answers it in a scratch tree, the command runner answers every check.
- The outcome is `landed`, said as "landed as <rev> beside the queue: no
  required check reads its changes"; JSON carries `beside: true`, and a
  waiting entry `beside: {behind, why}`. A request's report and its
  `sluice` check say it landed with no check run.
- The site's pages on the queue and on outcomes say so.

Out of scope: nothing waits on a later change; a signer's attested
reading is not built (the record chose none).

## Capabilities

### New Capabilities

- `landing-beside`: which work lands beside the chain, when, what the
  combinations under test do, what a waiting entry is told, and what the
  outcome says.

### Modified Capabilities

None: the existing specs' requirements stand; where work no check reads
lands beside them is the new capability's.

## Impact

- `sluice-seams`: `Tests::reads` and its types; `Decided::Landed` and
  `Standing::Landed` carry whether it landed beside.
- `sluice-core`: the round's beside landing (`step.rs`), the candidate's
  carried work, the submission's `beside`, the landing message.
- `sluice`: the moon and command runners' reading (`profiles/checks.rs`,
  `profiles/moon.rs`), submit, wait and status words, the notice, the
  GitHub report and check.
- `sluice-fakes`, `sluice-conformance`: the reading in the fakes and the
  scenarios.
- `site/`: the queue and outcome pages.
