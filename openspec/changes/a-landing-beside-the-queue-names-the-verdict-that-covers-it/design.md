# Design

## Context

A landing commit names the verdict it landed on as `Sluice-*` trailers
(23; record 0006). `landing_message` in `crates/sluice-core/src/text.rs`
writes, for a landing through the chain:

```
Sluice-Submission: 12
Sluice-From: agent-1 (touches)
Sluice-Tested: <combination>
Sluice-Result: passed (2 run, 31 replayed)
```

`beside_message`, for a landing beside the chain (140; record 0075),
writes `Sluice-Result: landed beside the queue (no required check reads
its changes)` and no `Sluice-Tested`.

`choose_beside` in `crates/sluice-core/src/step.rs` lands beside only
where `branch.json`'s `verified` holds: the branch's revision `base` is
covered by a verdict, since sluice landed it under the gate it now
carries (70). `perform_landing` sets `verified` to `same_gate` at every
landing, a landing beside included, and a round that finds the branch
moved by another hand clears it. Nothing records which verdict covers
`base`: `verified` is a flag, and the landed candidate leaves
`branch.json` once its landing is recorded.

A request's `sluice` check for a landing beside is drawn by
`outcome_shown` in `crates/sluice/src/profiles/github/pulls.rs`: success,
"landed as <rev> beside the queue", summary "Landed on <branch> as <rev>
beside the queue, with no check run: no required check reads its
changes." `render::has_report` is false for it, having no row and no
task, so no report is drawn: no text on the check, and a comment left
from an earlier head is never edited.

### Looking across

Who reads the trailer, in the family and beyond:

- **crew** (afterthought/crew `plugin/lib/gather.py`): `git log -1
  --format=%(trailers:key=Sluice-Result,valueonly)` on a bolt's head;
  `kit.verification` reads green where `result.split()[0] == "passed"`,
  "none" where the head has no trailer, and "unknown" otherwise, naming
  the words. Its docstring rests on "sluice lands nothing that did not
  pass".
- **swancloud's releaser** (afterthought/swancloud
  `packages/swancloud-releaser/releaser.sh`): `%(trailers:key=Sluice-Result,
  valueonly,separator=%x20)` on `origin/main`, first line, `case $result
  in passed*`.
- **flume** reads `sluice status <base> --json` and the landing
  commit's `Sluice-Submission` (flume's statement, read-submission row),
  never `Sluice-Result`. **Boomdrive** reads no trailer.
- **The drafted landing claims** (sluice 142 to 146, proposed
  2026-11-01, not accepted; blueprints branch
  `research/sluice-signs-its-landings`, 144): a claim "says the gate
  passed and nothing more ... and the claim of a landing beside the chain
  (140) names no check". The same reading as this design: a landing
  beside's gate passed, over no check.

Both readers read one thing: the first word of `Sluice-Result`, as the
verdict of the landing at the head. That word is sluice's to own, and
both readers already cite it as sluice's. One design serves both: the
first word is always the landing's own verdict, and on a landing it is
always `passed`, since a branch moves only on a satisfied verdict (22).
Neither reader needs a change, and no reader is asked to learn a second
form.

## Goals / Non-Goals

**Goals:**

- A landing beside's `Sluice-Result` opens with the landing's own
  verdict, `passed`, with counts true to what it ran, and names the
  verdict that covers it and the revision that verdict landed as.
- sluice reads the covering verdict from its records, never from a
  commit's message.
- The request's check and report say what the trailer says.
- crew and swancloud read a landing beside as passed with no change of
  theirs.

**Non-Goals:**

- Rewriting a landing beside already in a branch's history (23).
- Changing `sluice submit`, `sluice wait`, the notice or their JSON.
- A trailer of a new key, or a field a reader must learn.
- The landing claims (142 to 146).

## Decisions

### D1. The trailer opens with the landing's own verdict, `passed (0 run, 0 replayed)`

```
chore(land): #1319 Take main

Sluice-Submission: 1319
Sluice-From: <agent> (<branch>)
Sluice-Result: passed (0 run, 0 replayed) beside the queue: no required check reads its changes; covered by the result at <revision>, passed (141 run, 0 replayed)
```

Is `passed` true to what was run? Clause 15 makes a candidate's verdict
the gate read over the checks the candidate affects; 70 leaves the
checks it does not affect to the verdict that covers the branch's
revision, and makes every check affected where none does. A landing
beside's candidate is the branch's revision extended by its work alone
(140), and that work affects no required check at the branch's revision
nor at any candidate formed. The gate read over the checks it affects,
none, on a covered revision, is passed, with nothing run: 140 itself
says "with nothing run (15)".

That is what every landing's `passed` already means. On a gate of 141
checks, `passed (3 run, 0 replayed)` says 3 affected checks ran and
passed; the other 138 stand on the verdict the branch's revision
carried. A landing beside is the case where none is affected. The counts
are the landing's own, `0 run, 0 replayed`, so a reader that sums runs,
or reads the counts as runs on this tree, reads the truth.

Rejected: the covering verdict's own words as this landing's,
`passed (141 run, 0 replayed)` with `Sluice-Tested` naming its
combination, as submissions landed together on the deepest candidate's
verdict each name it (23). Read green unchanged, but it gives the
landing 141 runs it never made, and names as tested a combination whose
tree never held the work. Rejected: `landed beside the queue` kept as
the first words, with a new trailer or field for the covering verdict.
True, but crew and swancloud would each change to learn it, any other
reader of `passed` would stay broken, and it would say the landing has
no verdict, which 140 and 15 give it.

`Sluice-Tested` is not written on a landing beside: its candidate was
never run. The words "beside the queue: no required check reads its
changes" are the same the submit, wait and notice already say (140;
record 0075).

### D2. The covering verdict is named by the revision it landed as, with its words

`covered by the result at <revision>, <words>`: `<revision>` is the
branch's revision, in full, whose landing named the covering verdict,
and `<words>` is that landing's own `Sluice-Result`, `passed (141 run, 0
replayed)`. A landing beside after a landing beside names the same
verdict and revision, never the landing beside before it.

The revision, not the combination: the landing's revision is on the
branch's first-parent history in every copy of the repository, never
rewritten (23), and its own `Sluice-Tested` names the combination, one
hop away. The combination's commit is sluice's own, which another copy
may not hold and which sluice gives up once its verdict is read. By
core.17 a combination that lands is the same combination on the revision
it lands as, so that revision is where the verdict stands on the branch.
On switchboard-kit's bolt the line would have read `covered by the
result at e25fea4…, passed (141 run, 0 replayed)`.

"Result" is the word a person sees for a verdict (the repository's two
vocabularies), as `Sluice-Result` is.

### D3. The covering verdict is kept in `branch.json`

A new type in `crates/sluice-seams/src/lib.rs`, beside `Decided`, which
carries it (D4):

```rust
/// 23, 70, 140: the verdict that covers a branch's revision: the
/// revision of the branch whose landing named it, and its words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoveringVerdict {
    /// The branch's revision that landed on it (core.17).
    pub at: Rev,
    /// Its result word, `passed` (22).
    pub result: String,
    pub ran: u64,
    pub replayed: u64,
}
```

and one function in `crates/sluice-core/src/text.rs` that writes a
verdict's words, `"{result} ({ran} run, {replayed} replayed)"`, which
`landing_message` and `beside_message` both call, so the covering words
are always the covering landing's own.

`BranchDoc` gains `covered_by: Option<CoveringVerdict>`
(`serde(default, skip_serializing_if = "Option::is_none")`), the verdict
covering `base`. It holds only beside `verified`, which stays as it is
for an older sluice on another host to read (C.6):

- `LandingIntent` gains `covering: Option<CoveringVerdict>`, the verdict
  that covers `to` once the branch is there, written with the intent:
  for a landing through the chain, the verdict the last commit names
  (the deepest candidate's, passed), `at` the last commit, which is
  `to`; for a landing beside, `branch.json`'s `covered_by` as it stood.
- `perform_landing` sets `covered_by` to the intent's `covering` where
  it sets `verified` to `same_gate` and `same_gate` holds, and to `None`
  otherwise. A round that finds the branch moved by another hand sets it
  to `None` where it clears `verified`.
- `choose_beside` lands nothing beside unless `covered_by` is set, as it
  lands nothing unless `verified` holds; each landing beside writes
  `beside_message` with it.

So the covering verdict is decided in the records, under the branch's
lock (core.14), and read by any reader of `branch.json` with no sluice
running (core.11). It is never read back from a commit's message.

State written before this change has `verified` and no `covered_by`:
the branch lands nothing beside until its next landing through the
chain names one (D6). An intent written by an older sluice has no
`covering`, and its landing leaves `covered_by` unset the same way.

### D4. What landed beside carries its covering verdict to the forge

The forge's check is drawn from a submission's delivered outcome and its
report from its record (record 0044), so each carries the covering
verdict, optional and absent in state written before:

- `Submission` gains `covered_by: Option<CoveringVerdict>`, set by
  `choose_beside` for each submission it lands beside, with the state
  `Landing`.
- `DecidedRecord::Landed` and `Decided::Landed` gain `covered_by`, set
  by `perform_landing` from the submission for a landing beside, `None`
  for a landing through the chain.
- `Standing::Landed` gains `covered_by`, read by `verdict_view::standing`
  from the submission.

An older sluice reading any of these passes the unknown field by, as it
passes every field written after it.

### D5. The forge says what the trailer says

`outcome_shown`, for `Outcome::Landed { beside: true, covered_by:
Some(c), .. }`: success, title unchanged, "landed as <rev> beside the
queue", summary:

```
Landed on main as `0cef58d` beside the queue, with no check run: no required check reads its changes. Covered by the result at `e25fea4`, passed (141 run, 0 replayed).
```

With no `covered_by`, an outcome recorded before this change, the
summary is today's.

`render::has_report` is true for `Standing::Landed { beside: true,
covered_by: Some(_), .. }`, so the report is drawn and posted as any
result's is (118; record 0044): the check's text, and the request's one
comment, created or edited in place. `draw` writes the head as today,
`## sluice: landed on main as 0cef58d with no check run: no required
check reads its changes`, then:

```
Covered by the result at [`e25fea4`](https://github.com/<repo>/commit/<at>), passed (141 run, 0 replayed).
```

and no row. No check per required check is posted for it: such a check
says a check was run or replayed on the request's head (118), and none
was. A request whose earlier head left a report comment, blamed or
formed again, has that comment edited to the landing beside's, never a
second one posted.

### D6. Upgrading

A branch's first work no required check reads after a host takes this
sluice takes the queue once, since its state names no covering verdict,
and lands through it as any submission does, its trailers naming its own
combination and verdict. That landing is the covering verdict every
landing beside after it names. The cost is one combination's checkout
and runner call per branch, once.

On a branch whose records are on its remote, an older sluice that writes
`branch.json` passes `covered_by` by and drops it on its write; the next
landing by a sluice of this change names it again. Only the instance
advancing the branch lands beside (58, 137), so a landing beside's words
are those of that instance's sluice.

### D7. What people are shown

The site's pages and the plugin's reference show the trailer of D1 where
they show a landing commit's trailers, in the user words, naming no team
or tool of ours (rule core.26):

- `site/src/content/docs/reference/state.md`: after the trailer example,
  a landing beside's trailer and what its words mean; `covered_by` on
  `branch.json` and on a submission's document.
- `site/src/content/docs/guides/walkthrough.md`: where it shows the
  landing beside (`landed as 4e1a07c beside the queue`), the commit's
  trailer.
- `site/src/content/docs/guides/pull-requests.md`: the
  `landed as 4e1a07c beside the queue` row says the check names the
  result that covers it.
- `plugins/sluice/skills/using-sluice/references/submit.md`: beside its
  trailer example, a landing beside's, and that a tool that reads a
  branch's head as passed reads the first word of `Sluice-Result`.

## Tests, and where each scenario is held

| Scenario | Test |
|---|---|
| Docs landed beside after a tested landing | `crates/sluice-conformance/tests/beside.rs` `a_landing_beside_names_its_result_and_the_result_that_covers_it`; `crates/sluice-core/src/text.rs` `a_landing_beside_names_passed_and_what_covers_it` |
| A reader of the head reads it as passed | `crates/sluice/tests/local_refs.rs` `a_landing_beside_reads_as_passed_to_git`, against real git |
| Two landings beside in a row | `beside.rs` `two_landings_beside_in_a_row_both_land_with_the_combination`, extended |
| A combination lands after a landing beside | `beside.rs` `docs_land_beside_two_code_submissions_under_test_and_they_land_carrying_them`, extended |
| State that names no covering result | `beside.rs` `state_that_names_no_covering_result_takes_the_queue_once` |
| A revision no result covers | `beside.rs` `nothing_lands_beside_on_a_revision_no_verdict_covers`, asserting `covered_by` unset |
| The request's check names the covering result | `pulls.rs` `a_landing_beside_the_queue_says_no_check_ran`, extended; `render.rs` `landed_testing_and_formed_again`, extended |
| A request's earlier report is brought up to date | `pulls.rs` `a_landing_beside_edits_the_requests_report`, over a `FileForge`; the pull request suite end to end runs the command runner, which lands nothing beside |
| Records written before | `crates/sluice-core/src/records.rs` `a_branch_and_an_outcome_from_before_covered_by_still_read` |

## Risks / Trade-offs

- **A reader that keys on `landed beside the queue` at the start of the
  trailer.** None is known in the family; the words stay, after
  `passed`. The site's state page says the first word is the landing's
  verdict.
- **A long trailer.** One line of about 180 characters with a full
  revision. git's trailer parsing reads it whole
  (`a_landing_beside_reads_as_passed_to_git`); both readers read the
  first word or the first line.
- **`passed` read as "checks ran on this tree".** The counts say `0
  run`, and the words after say why, and which result it stands on.
- **The one-time queue after upgrade** (D6). Bounded to one landing per
  branch; the alternative, reading the covering verdict from the base
  revision's commit message, decides from words outside the records.

## Migration Plan

Nothing to migrate: `covered_by` and `covering` are optional, set at the
next landing. A landing beside already in a history keeps its words
(23). The statement's amendment is on agentplot/blueprints main at
eb0d3eb, pushed on the user's word of 2026-11-06 before the code lands
(core.12).

## Open Questions

None. The user accepted record 0083, and with it the trailer's words,
on 2026-11-06.
