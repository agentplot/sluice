## Why

sluice lands work no required check reads beside its queue, with nothing
run, and only on a revision a verdict covers (140, 70; record 0075; rule
core.39). Its landing commit then says `Sluice-Result: landed beside the
queue (no required check reads its changes)`, with no `Sluice-Tested`.

Two readers read a branch's head as passed only where that trailer opens
with `passed`, so a landing beside reads to them as no pass:

- **crew** reads a bolt's verification from its head's trailer
  (afterthought/crew `plugin/lib/kit.py` `verification`,
  `result.split()[0] == "passed"`). On switchboard-kit, a bolt's last
  tested head, e25fea4, passed with 141 checks run; a docs-only
  take-main, #1319 at b01e966, then landed beside, and crew reads
  "verification unknown", so the bolt cannot reach its landings card.
  Every docs-only take-main strands a bolt the same way.
- **swancloud's releaser** builds main's head only where its trailer
  matches `passed*` (afterthought/swancloud
  `packages/swancloud-releaser/releaser.sh`), so a docs-only landing on
  swancloud's main builds no release until a code landing.

sluice knows which verdict covers a landing beside: the one on the
revision it landed on, which the branch's gate stands on (70, 140,
core.17). This change makes the landing commit, and the request's check
on the forge, say so, in one form sluice owns, so each reader reads it
one way.

## What Changes

- **The landing commit's trailer.** A landing beside's `Sluice-Result`
  opens with its own verdict, `passed (0 run, 0 replayed)`, then says it
  landed beside the queue and names the verdict that covers it, by that
  verdict's words and the revision of the branch it landed as:

  ```
  Sluice-Result: passed (0 run, 0 replayed) beside the queue: no required check reads its changes; covered by the result at <revision>, passed (141 run, 0 replayed)
  ```

  `passed` is the verdict 15 gives a candidate that affects no required
  check on a covered revision (70), the same reading every landing's
  `passed` makes for the checks its combination does not affect; the
  counts are this landing's own. No `Sluice-Tested` is written, since
  its candidate was never run. A landing through the queue writes its
  trailers as before.
- **The verdict that covers the branch's revision is kept in
  `branch.json`.** Each landing under an unchanged gate names it; each
  landing beside carries it. A branch whose state says its revision is
  covered but names no verdict, state an older sluice wrote, lands
  nothing beside until its next landing names one.
- **The forge says the same.** A request landed beside has its `sluice`
  check's summary name the covering result, its words and its revision,
  and gets a report as any result does, the check's text and the
  request's one comment, saying the same with the revision linked; a
  comment an earlier head left is edited to it. No check per required
  check is posted for it, since none was run or replayed on its head
  (118).
- **The words for people.** The site's state and walkthrough pages, the
  pull requests guide, and the plugin's submit reference show the new
  trailer; `branch.json` and a submission's document gain `covered_by`.
- **The statement and the record.** Clauses 23, 140 and S68 are amended
  on agentplot/blueprints main at eb0d3eb, pushed on the user's word of
  2026-11-06 before the code lands (core.12). Record 0083 is accepted
  on that word, and rule core.39 with its details names the landing
  commit's verdict.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `landing-beside`: "What never lands beside" adds a revision whose
  covering result the branch's state does not name; "The outcome says it
  landed beside the queue" titles the request's `sluice` check; two new
  requirements, "A landing beside's result reads as passed with nothing
  run" and "A landing beside names the result that covers it", set the
  trailer's words; and a third, "A request landed beside names the
  result that covers it", has the request's check and report name the
  covering result, with no row and no check per required check (118).

## What each reader reads once this lands

- **crew**: `git log -1 --format=%(trailers:key=Sluice-Result,valueonly)`
  on a bolt's head that landed beside gives `passed (0 run, 0 replayed)
  beside the queue: no required check reads its changes; covered by the
  result at <revision>, passed (141 run, 0 replayed)`. `verification`
  splits it, finds `passed` first, and reads the bolt green: "sluice's
  verdict on its last landing, <head>: passed (0 run, 0 replayed) beside
  the queue: ...". No change in crew.
- **swancloud's releaser**: the same trailer, first line, matches
  `passed*`, so it builds main's head. No change in swancloud.
- **History landed before this** keeps its words (23). A bolt whose
  head is a landing beside written before this, b01e966 among them,
  reads unknown until its next landing, which writes a trailer crew
  reads green. Reading the old words as green would be crew's own
  change, walking the first parents to the covering landing; this
  change does not ask it.

## Out of scope

- **`sluice submit`, `sluice wait`, the notice and their JSON.** They
  keep "landed as <rev> beside the queue: no required check reads its
  changes" and `beside: true`. The session that submitted needs to know
  it landed; naming the covering result there is a later change if a
  reader asks for it.
- **The landing claims** (142 to 146, proposed 2026-11-01, not
  accepted). Where they are built, a landing beside's claim says the
  gate passed and names no check, which this trailer agrees with; this
  change builds none of them.
- **Rewriting history.** Commits already landed beside keep their words
  (23); nothing is amended or re-signed.
- **crew's reading of the old words.** As above, crew's own work, not
  asked by this change.
