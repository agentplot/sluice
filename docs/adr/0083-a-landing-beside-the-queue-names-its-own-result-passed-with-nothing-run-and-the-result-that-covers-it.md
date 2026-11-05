---
number: 83
title: A landing beside the queue names its own result, passed with nothing run, and the result that covers it
status: proposed
date: 2026-11-05
decision-makers:
- Chuck Swanberg
---

# A landing beside the queue names its own result, passed with nothing run, and the result that covers it

Status: proposed on 2026-11-05 by the construct stage of the change `a-landing-beside-the-queue-names-the-verdict-that-covers-it`, from its brief. The trailer's words go into every branch's history, which is never rewritten (23), so they are the user's to accept. This record is accepted on their yes, and the statement's amendment is pushed then.

Deciders: the construct stage of `a-landing-beside-the-queue-names-the-verdict-that-covers-it`, proposing; the user, accepting or not.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 15, 22, 23, 26, 70, 118, 140, S68, C.5, C.6, and 23, 140 and S68 as amended on the branch `design/a-landing-beside-the-queue-names-the-verdict-that-covers-it` at 1b21406; the drafted landing claims, 142 to 146, proposed 2026-11-01 and not accepted, on the branch `research/sluice-signs-its-landings`; records 0006, 0044, 0075; rules core.11, core.14, core.17, core.39, core.43; `crates/sluice-core/src/text.rs` `beside_message`, `crates/sluice-core/src/step.rs` `choose_beside` and `perform_landing`, `crates/sluice/src/profiles/github/render.rs` `head` and `pulls.rs` `outcome_shown` as main held them at 3f5aed5f; afterthought/crew `plugin/lib/kit.py` `verification` and `plugin/lib/gather.py`'s read of `Sluice-Result`; afterthought/swancloud `packages/swancloud-releaser/releaser.sh`'s `case $result in passed*`; switchboard-kit's #1319 at b01e966.

## Context and problem statement

Record 0075 lands work no required check reads beside the queue, with nothing run, on a revision a result covers (140, 70). Its landing commit says `Sluice-Result: landed beside the queue (no required check reads its changes)` and carries no `Sluice-Tested`. Every other landing's `Sluice-Result` opens with the result it landed on, `passed (3 run, 0 replayed)`.

Two readers read a branch's head as passed where its `Sluice-Result` opens with `passed`. crew reads a bolt's verification from its head's trailer, `result.split()[0] == "passed"`. On switchboard-kit, a bolt's last tested head, e25fea4, passed with 141 checks run. A docs-only take-main, #1319 at b01e966, then landed beside, and crew read the bolt's verification as unknown, so the bolt could not reach its landings card. Every docs-only take-main strands a bolt so. swancloud's releaser builds main's head only where its trailer matches `passed*`, so a docs-only landing on swancloud's main builds no release until a code landing.

sluice knows which result covers a landing beside: the one its branch's revision stands on (70, 140, core.17). How does the landing commit say so, so that each reader reads it one way, sluice's?

## Considered options

**What the trailer opens with.**

1. **The landing's own result, `passed (0 run, 0 replayed)`, then that it landed beside the queue and the result that covers it** (chosen). Clause 15 makes a combination's result the gate read over the checks it affects; 70 leaves the rest to the result that covers the branch's revision. A landing beside's combination is the branch's revision with its work alone (140). That work affects no required check there or at any combination formed. The gate over the checks it affects, none, on a covered revision, is passed, with nothing run. Every landing's `passed` already reads so: `passed (3 run, 0 replayed)` on a gate of 141 checks says 3 ran and 138 stand on the result its revision carried. A landing beside is the case where none ran. The counts are this landing's own, so no reader takes another revision's runs as run here. The drafted landing claims read it the same way: a landing beside's claim says the gate passed and names no check (144, proposed).
2. **The covering result's own trailer, `passed (141 run, 0 replayed)`, with `Sluice-Tested` naming its combination**, as submissions landed together on the deepest combination's result each name that result (23). Read as green unchanged, but gives this landing 141 runs it never made, and names as tested a combination whose tree never held its work.
3. **Keep `landed beside the queue` and add a trailer or field naming the covering result**, `Sluice-Covered-By:` or a JSON field. True, but every reader that reads green from `passed` must learn the new trailer or walk the first parents to the covering landing. crew and swancloud each change, and any other reader stays broken. It also says the landing has no result, when 140 gives it one, read with nothing run (15).

**How the covering result is named.**

- a. **By the revision of the branch it landed as, with its words** (chosen): `covered by the result at <revision>, passed (141 run, 0 replayed)`. That revision is on the branch's first-parent history in every copy and is never rewritten (23); its own trailers name the combination. By core.17 a combination that lands is the same combination on the revision it lands as.
- b. By its combination's revision, as `Sluice-Tested` names one. A combination's commit is sluice's own, which a copy of the repository may not hold and which sluice gives up once its result is read.

**Where sluice reads the covering result.**

- **The branch's state** (chosen). `branch.json` names, beside the revision the combinations form on, the result that covers it, set at each landing under an unchanged gate (55, 70) and carried by each landing beside. State written before this names none: the branch then lands nothing beside until its next landing names one, so the trailer is never written without it.
- The base revision's own commit message. A decision read from a message outside the state (core.11, core.14), parsed from words sluice may change.

**What the forge shows.** The request's `sluice` check and its report say what the trailer says: landed beside the queue with no check run, no required check reads its changes, and the result at the covering revision, with its words. No check per required check is posted for the landing beside: such a check says it was run or replayed on the request's head (118), and none was.

## Decision outcome

Options 1, a, the branch's state, and the forge saying the same (clauses 23, 140 and S68 amended).

A landing beside's commit reads:

```
chore(land): #1319 Take main

Sluice-Submission: 1319
Sluice-From: <agent> (<branch>)
Sluice-Result: passed (0 run, 0 replayed) beside the queue: no required check reads its changes; covered by the result at <revision>, passed (141 run, 0 replayed)
```

where `<revision>` is the branch's revision, in full, that the covering result landed as, and its words are that landing's own. No `Sluice-Tested` is written: the landing's own combination was never run. A landing beside after a landing beside names the same covering result. A landing through the queue writes its trailers as before. sluice lands only on a passed result (22), so the covering result is always a pass.

The request's `sluice` check is titled `landed as <rev> beside the queue`, and its summary reads `Landed on <branch> as <rev> beside the queue, with no check run: no required check reads its changes. Covered by the result at <revision>, passed (141 run, 0 replayed).` Its report's head says the same, with the covering revision linked.

Once this lands, crew reads a landing beside's trailer as green, `sluice's verdict on its last landing, b01e966: passed (0 run, 0 replayed) beside the queue: ...`, and swancloud's releaser builds the head; neither changes. A landing beside written before this keeps its words (23): a bolt whose head is one reads green again at its next landing.

### Consequences

- Clauses 23, 140 and S68 are amended on the blueprints branch at 1b21406, marked proposed, and pushed to agentplot/blueprints main on the user's word before the code lands (core.12). Rule core.39 and its details name the landing commit's result.
- `branch.json` gains the result covering its revision. A submission landed beside, its outbox item and its delivered outcome carry it, so the forge can show it. Each is optional and absent in state written before; an older sluice reading them passes them by.
- On a branch whose state was written before this, the first work no required check reads after the upgrade takes the queue once. Its landing names the result every landing beside after it names.
- A reader that looks for `landed beside the queue` at the start of the trailer no longer finds it there; the words are kept after `passed`.
