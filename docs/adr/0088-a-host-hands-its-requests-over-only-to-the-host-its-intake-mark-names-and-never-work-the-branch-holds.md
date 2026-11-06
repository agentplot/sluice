---
number: 88
title: A host hands its requests over only to the host its intake mark names, and never work the branch holds
status: accepted
date: 2026-11-06
decision-makers:
- the code stage of fix/a-hosts-intake-never-withdraws-anothers-submission
---

# A host hands its requests over only to the host its intake mark names, and never work the branch holds

Status: accepted, a decision of the code stage of `fix/a-hosts-intake-never-withdraws-anothers-submission` under the statement as written (122, 137, C.5) and rules core.33 and core.48, which it keeps; cheap to change later. How a host's several waits hold one mark is record 0089's.

Deciders: the code stage of `fix/a-hosts-intake-never-withdraws-anothers-submission`.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 5, 45, 122, 137 and C.5's pull request row, at c94c496; rules core.5, core.33, core.48; record 0085; the signal `afterthought/crew-state:swancloud/main signals/2026-11-06-swancloud-ops-012fa188/01-wldn-gate-box-offline-leaves-switchboard-kit-intake-to-waits`; chuck-herdr-alpha's records of WilldanGroup/switchboard-kit's `main`, `/workspace/switchboard-kit/switchboard-kit.git/sluice/main/log.jsonl`, for 2026-11-06; GitHub's timelines of switchboard-kit's pull requests 241 to 243 and its `refs/sluice/intake/main`; afterthought/crew 23b3b3dc's `plugin/lib/plan.py` `pull_hold`; `crates/sluice/src/profiles/github/pulls.rs` (`takes`, `put_mark`, `hand_over`) as main held it at b2263f63.

## Context and problem statement

switchboard-kit's policy names no signers, so each host keeps its own records of `main` (C.6 local files), and the holder of `refs/sluice/intake/main` on GitHub takes main's pull requests in (137). With the Willdan gate box gone, chuck-herdr-alpha was the one host doing so, by crew's waits, `sluice wait <id> --timeout 9m` run again while they time out, several at once, each with sluice 0.68.1. Its log says:

- 14:56:23 to 14:58:47, pull requests 241, 242 and 243 taken in as #1406, #1407 and #1408; 14:59:31, `[1405, 1406, 1407]` under test.
- 15:07:04, `withdrawn` #1406, #1407 and #1408, each `by: another host takes main's pull requests in`.
- 15:07:16 to 15:07:18, GitHub's timelines: each request converted to a draft, by the person's own account, not sluice's App, whose drafts all read `sluice-for-willdan[bot]`. That is crew's landing holding a bolt's request when its landing did not land (`pull_hold`, `gh pr ready --undo`), on sluice 0.68.1's wait reading the withdrawal as withdrawn, exit 4; record 0085 has a wait follow a handed-over request since 0.70.0. 15:09, made ready again by the same account.
- 15:12:12 to 15:12:18, taken in again as #1411 to #1413; their combination tested again, landed at 15:55:09.

No other host took the mark: "another host" is sluice's own words where a push of the mark, refused at its lease, is followed by a read finding no mark at all (`put_mark`). One of the host's waits, ending, had let the mark go between another wait's fetch of it and that wait's renewal; the renewal was refused at its lease; the read found none; and intake withdrew every submission the host's records hold of a request, naming no host. Read the same way, a mark another process of the host moved hands the host's submissions over to the host itself.

137 has a host withdraw its own submissions on finding the mark another's, naming the host that holds it. The host found no mark, and none another's. How does a host settle a push of the mark refused at its lease? And separately: when it does find another host's mark, as a host whose driver came back finds the one that took over while it was gone, what of a submission whose work that other host has landed?

## Considered options

**A push of the mark refused at its lease.**

1. **Decided again on the mark the branch's fetch then brings, once** (chosen). This host's own, which another of its processes moved, is the host's: it takes the requests in. None standing, which the process that held it let go, is pushed again leased on its absence, as 137 has a host finding none do. Another host's names that host, which the host hands over to. A second refusal leaves the pass taking nothing in and handing nothing over; the next pass decides again.
2. Read as another host's take whatever the read finds, as before. A hand-over to no host, or to itself, against 137's "the mark another's ... naming the host that holds it".
3. Take nothing in on any refusal, deciding nothing until the next pass. A host whose mark another host did take holds its submissions one pass longer, and a host whose own process moved it sits out a pass for nothing.

**A submission another host landed.**

- a. **Never handed over** (chosen): a submission whose head the branch, as the pass fetched it, holds has landed, whichever host landed it; the hand-over passes it by, and the round lands it as the commit that landed it (122; record 0085's D1; rule core.48).
- b. Withdrawn as handed over. A withdrawal of work the branch holds, which core.48 rules out in any name.

**What the new holder does with requests already in another host's queue.** Nothing of that host's: each host's records are its own (C.6), and no host writes another's. It takes each open request in as its conditions hold, as 137 has it. The host that held them withdraws its own submissions at its next pass that finds the mark another's, naming the holder, none landing and none whose head the branch holds; a submission it was landing lands, and the holder's submission of the same head is then found landed (122). A host that is gone advances none of its submissions; they wait in its own files until it returns and settles them so.

## Decision outcome

Options 1 and a. Rule core.33 holds them.

### Consequences

- A refused push costs one more fetch and push before a pass decides. A host whose waits come and go, as crew's do, keeps its submissions through a mark let go under its renewal, and nothing is tested twice for it.
- "another host takes <branch>'s pull requests in" is no longer written; an older record holding it reads as before.
- crew's draft of a withdrawn landing's request is crew's, by its own design; with this record and 0085 no hand-over is reported to it as withdrawn, so it drafts none for one.
