# Proposal

## Why

Two changes still open on sluice's `main` fail `openspec validate
--strict`, the check every archived spec already passes and every open
change is about to be held to (the chore
`open-changes-are-checked-as-strictly-as-archived-ones`, which comes
after this change and cannot land while either fails). The design
agent ruled on both (madswan-design, 2026-10-26, recorded in the plan's
proposal 108):

- `actions-pool` rewrites the requirement "forge check verifies the
  setup" from the text it had before `pull-request-reports` was
  archived. Its rewrite drops the scenario "No permission to post
  checks" and narrows the permissions the app must hold back to
  contents alone. The ruling: the scenario is copied into the rewrite,
  and the permission list keeps checks and workflows beside contents.
- `finished-submissions` has no specs at all, so it has nothing to
  archive. The ruling: it does not skip its specs; it adds a new spec,
  `finished-records`, whose requirements are the state its proposal
  describes, the log kept in parts (its part 2, not yet built)
  included.

## What Changes

- `actions-pool`'s rewrite of "forge check verifies the setup" keeps
  what `main` says today: the app needs write on contents, workflows,
  checks and pull requests, and a check refused for want of checks
  write is named. Its two new scenarios for the pool stay as written.
- `finished-submissions` gains `specs/finished-records/spec.md`: a
  finished submission moves out of its branch's live state; a branch's
  own reads cost its queue; a finished submission stays readable and
  counted; older state moves on the next write; the log is kept in
  parts of 1,000 lines; the live state carries the recent log, and a
  reader of history reads every part.
- `finished-submissions`' proposal names `finished-records` as the
  capability it adds.

Nothing else in either change moves: no task is ticked or unticked, no
design is rewritten, and no code, page or test changes. The text of
both edits is in this change's design, for the code stage to copy.

## Out of scope

- Making the gate validate open changes: that is the chore
  `open-changes-are-checked-as-strictly-as-archived-ones`, which waits
  on this change.
- Building part 2 of `finished-submissions`, the log in parts: its
  tasks 2.1 and 2.2 stay open, for a later unit.
- Writing the statement change `finished-submissions` rests on,
  `agentplot/blueprints:design/sluice/proposals/finished-records-1.md`,
  which blueprints does not hold: it is the design agent's (see the
  design's risks).

## Capabilities

### New Capabilities

None. This change edits two other changes' planning files and changes
no behaviour of sluice's, so it carries no spec of its own
(`skip_specs`).

### Modified Capabilities

None.

## Impact

- `openspec/changes/actions-pool/specs/forge-identity/spec.md`: one
  requirement's text and one scenario.
- `openspec/changes/finished-submissions/`: a new spec file and a
  Capabilities section in its proposal.
- No crate, plugin, site page or test.

## Touches

```
openspec/changes/actions-pool/specs/forge-identity/spec.md
openspec/changes/finished-submissions/proposal.md
openspec/changes/finished-submissions/specs/finished-records/spec.md
```
