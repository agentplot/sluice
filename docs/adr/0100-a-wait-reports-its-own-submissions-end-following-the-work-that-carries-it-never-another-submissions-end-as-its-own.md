---
number: 100
title: A wait reports its own submission's end, following the work that carries it, never another submission's end as its own
status: accepted
date: 2026-11-07
decision-makers:
- the construct stage of change/a-submit-reports-the-end-of-its-own-entry
---

# A wait reports its own submission's end, following the work that carries it, never another submission's end as its own

Status: accepted, a decision of the construct stage of `change/a-submit-reports-the-end-of-its-own-entry` under the statement as written (5, 50, 51, 67, 122, 137, C.5's pull request row), which no clause of it changes; cheap to change later. It amends record 0085's choice of what a wait reports of a submission withdrawn whose work goes on, and keeps the rest of record 0085.

Deciders: the construct stage of `change/a-submit-reports-the-end-of-its-own-entry`, on agentplot-planner's unit of the bolt `sluice-tells-each-submitter-its-own-end`: "A waiting submit and sluice wait report the end of the very entry the submit made, its landing as landed with exit 0, and never another entry's end as theirs."
Sources: `boomdrivedev/blueprints:design/sluice/requirements.md` clauses 5, 50, 51, 67, 122, 135 and 137 and C.5's pull request row, at cd8fd73; records 0068, 0070, 0085, 0093; rule core.48; the signal `agentplot/crew-state:agentplot/main signals/2026-11-07-agentplot-ops-d4947942/01-sluice-exits-4-after-its-own-landing`; deneb's records of boomdrivedev/sluice's `main` on 2026-11-07, the runner's copy, `~/.local/share/flywheel/repositories/agentplot/sluice/sluice.git/sluice/main/` (`log.jsonl`, `finished.jsonl`), and the person's, `~/Code/github_agentplot/sluice/sluice.git/sluice/main/` (`log.jsonl` lines 7835 to 7846, `.finished/399.json`); `crates/sluice/src/lib.rs` (`carried`, `wait_on`, `await_submission`, `submit_pull`) and `crates/sluice/src/profiles/github/pulls.rs` (`Pulls::follow`) as the bolt holds them at 68d68a9a.

## Context and problem statement

At 03:52:46Z on 2026-11-07, crew's landing of `bolt/sluice-tasks-check-only-what-they-touch` printed: "sluice exited 4 after its push: #2 withdrawn by agentplot-ops at 2026-11-07T03:52:10Z". deneb keeps two copies of boomdrivedev/sluice. Both took pull request #204 in: the runner's copy as #2 at 03:50:35Z, the person's as #399 at 03:50:36Z. The person's copy landed #399 at 03:52:03Z as `ed8474b`, a merge whose second parent is #204's head `daeaf98`. At 03:52:04Z the runner's copy formed `[2]` again on `ed8474b`. At 03:52:10Z its intake read #204 gone from GitHub's open requests and withdrew #2 in its submitter's name. crew's waiting submit, run in the runner's bare clone on sluice 0.68.1, read #2 withdrawn and exited 4.

The chain is record 0085's, which sluice 0.70.0 builds. The round lands work the branch already holds. Intake withdraws no request whose head the branch holds. A wait that reads its submission withdrawn while the branch holds its head reports it landed. deneb's runner was still on 0.68.1 that night.

Two things record 0085 left still go against the unit's words.

- **A wait ends with another submission's end.** Record 0085 D3 has a wait whose submission was withdrawn follow the later submission, or the pull request, that carries its work, "reporting `#<b>`'s record". Its own risks name it: "`sluice wait <id>` may report another submission's record". The wait's exit is then `#<b>`'s. A submit's wait can exit 1 for a later submission's failed check, or print a record whose `id` is not the one it submitted, with nothing under `--json` saying so. A waiting submit to a branch that takes pull requests reports whichever submission of its request was decided last, on whatever head the request has by then.
- **A landing read as a closing.** A waiting submit and `sluice wait <branch>` read a closed request through `Pulls::follow`. That checks whether the branch holds the request's head against the host's own copy of the branch (`Branch::held`, no fetch). When another queue's landing closes the request before this host has fetched the branch, the wait says `closed without landing` and exits 1, for work that landed. That happens beside a live driver, whose tick is the only fetch, and in the race record 0085 D5 names.

By 122 a submission's outcome is landed once its branch holds its work. By 51 a notification names its entry and its outcome. What does a wait report as its end when its submission was withdrawn and its work goes on in another submission or in its request?

## Considered options

**What a wait reports when another submission or its request carries its work.**

1. **Its own submission's end, once the work has one** (chosen). The wait follows the carrier, as record 0085 has it. Once the branch holds the submission's head, the wait reports the submission landed, as the commit that landed it. Once the carrier is blamed, in conflict, withdrawn with nothing carrying it on, or its request closed, the wait reports the submission withdrawn, exit 4. Either way the document is the submission's own record, with `carried_by` naming the carrier and how it stands, and the words say which carrier ended how.
2. Record 0085 as built: the carrier's record and exit, saying in words which carrier it follows. A submit that waited on #12 can exit 1 for #13's failed check. Under `--json` it prints #13's record with nothing saying so, against the unit's words.
3. Stop at the withdrawal and exit 4, following nothing. This is what crew's waiter read on deneb: a landing read as a failure whenever the carrier lands. Against 122.

**Which submission a waiting submit to a branch that takes pull requests calls its own.**

- a. **The first submission this host's records hold of its request, as the wait first reads it** (chosen). The submit's first pass takes the request in, which is the submit's own act (135), so this is the submission the submit made. From then on the wait waits on that submission as `sluice wait <id>` does, with option 1 for a later head.
- b. Whichever submission of the request is decided, as built. When the request moves on to a head the submit never sent, a branch landed onto under its landing among them, the wait reports that head's submission as its own.
- c. The submission at the head the submit sent, read by head alone. A head can be submitted twice, by name and from a worktree, and a request's new head that carries the old one would then have no submission the wait calls its own.

**How a wait reads a closed request.**

- i. **Against the branch as a round reads it** (chosen). A request read closed, whose submission is not landed, is read against `Branch::read`, which fetches a branch on its remote, before it is called closed without landing. One read per closing, as `carried` already reads the branch. A branch that cannot be read leaves the wait as one that could not read, exit 3, never closed.
- ii. GitHub's `merged` read as landed. sluice lands only by its push, and only the branch holding the head says the work landed (122, core.45). A request merged by GitHub's own button would read landed.
- iii. Keep following until the host's copy holds the head. A closing with no driver and no further intake would wait out its time, and `--status` cannot wait.

## Decision outcome

Options 1, a and i.

### Consequences

- A wait's exit and document are always those of the submission it was given: the id, the submission the submit made, or the worktree's last. Where it followed a carrier, `carried_by` names it, `{"id", "state"}` for a submission and `{"pull_request", "state"}` for a request with none here. The document only gains a field (`sluice/submission@1`).
- A wait marks delivered only the outcome it reports as its own. The carrier's outcome stays pending for its own carrier, the round's notice or `sluice inbox` (51; records 0068, 0070), and is told to its session as its own.
- A submission withdrawn by its session's next submit, followed to that submit's landing, reports `landed as <rev>`, `landed_elsewhere`, by the carrier's landing. One followed to that submit's failure reports withdrawn and exits 4, naming the failure.
- Where the carrier is a submission that landed, `<rev>` is the carrier's landing commit, its `landed_as`. The code stage chose it, cheap to change: the oldest commit of the branch's first-parent history holding the submission's head, as record 0085 reads a landing found, names that head itself where the branch landed the carrier by fast-forward, a commit no landing made. Where no submission carried the work, or the carrier is the request, `<rev>` is read as record 0085 reads it.
- A waiting submit by name of a branch sluice lands on, crew's landing of a bolt, reports the submission it made even when the bolt moves under its request.
- Rule core.48 rules out a wait ending with another submission's record or exit, and a closed request read against the host's copy of the branch alone. It no longer lists the carrier's record as not ruled out.
- Record 0093's host process changes how a wait follows, not what it reports. The end of option 1 is read in `carried` and the request's follow, which a wait following the host process keeps. Its forge queue leaves no host taking requests in, so the request's follow across hosts becomes every host's, and option a's own submission is then the one sluice's check on the request's head carries (137; record 0095).
