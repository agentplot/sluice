---
number: 75
title: A change no required check reads lands at once beside the queue, and the combinations under test keep their runs and land with it
status: accepted
date: 2026-10-30
decision-makers:
- Chuck Swanberg
---

# A change no required check reads lands at once beside the queue, and the combinations under test keep their runs and land with it

Status: accepted by the user on 2026-10-30, on their card (req_1ab76b00feda4d5bbfd0, "Approved").

Deciders: the user, on their card of 2026-10-30 (req_3a9c29d3019847efa507), asked "Write a sluice record: a change no check reads (docs) lands at once, never waiting behind code under test?", answered "Yes, write the record for your approval"; their words before it: "It seems like a problem that we are sending doc changes through merge queues and having to wait for them. People aren't going to like that". An earlier note of theirs declined letting doc-only changes skip batching in general; this record is narrower: only work no required check reads, which never joins a combination.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 6, 7, 8, 10, 11, 12, 13, 15, 17, 22, 23, 24, 36, 50, 56, 58, 70, 71, 78, 117, 122, 124, 125, 137, 140, S68, the C.2 rows (blueprints d6133ad); records 0006, 0044, 0051, 0068, 0070, 0072; rules core.4, core.5, core.6, core.15, core.17, core.22, core.33; a docs-only record of another repository's on a host, tested in one combination with code submissions 399 and 400 and waiting for their checks; Flume's entry 63, docs only, landed 44 seconds after its submit on an idle host.

## Context and problem statement

A submission whose work no required check reads already runs nothing: the verdict is the gate read over the checks the candidate affects (15), and record 0044 lands such a change with nothing run. It still joins the chain. On a busy branch it is formed behind, or into, combinations of code under test, and it lands only when they do: a repository's docs-only record waited out the checks of two code submissions it was combined with, though nothing it changed could change their outcome. On an idle branch the same kind of change landed in 44 seconds. The wait is the queue's order, not anything the change needs.

The user asks that a docs change not wait. Which changes qualify, how such a landing squares with the chain's order (8, 10, 24) and with combinations already under test, what of conflicts, remotes, signers, groups and requests, and what a session is told?

## Considered options

**How "no check reads it" is decided.**

1. **The check's affected reading** (36), asked under the gate at the branch's current revision (56): between the branch's revision and that revision extended by the work, and between each candidate formed and not landed, as it would land now, and that candidate extended by the work; none named in any (chosen).
2. Path patterns in the policy, `docs/**` and the like. Another field to keep, and wrong the day a check starts reading those paths, a link checker or a docs build; the runner already knows what each task reads, and 15 already trusts its reading.
3. The affected reading at the branch's revision alone. Misses a candidate under test whose own work makes a check read the files, a new docs task or a widened input; that candidate's verdict would then not cover the tree it lands as.
4. The keys themselves, computed at both trees. Exact, but needs a place and the runner's hashing per candidate; for the moon profile an unaffected task's key does not change, which is what option 1 asks more cheaply.

**Ordering, and the combinations under test.**

- A. **Land beside the chain; a formed candidate lands carrying the work, untested again** (chosen). Its candidate is the branch's revision extended by it alone (10, 11), and it stands ahead of every entry (8). A candidate formed before it keeps its runs, keys and verdict, and lands from the new revision with its tree plus that work (23, 24), the same candidate on that revision (10, core.17).
- B. Land beside the chain, and form every candidate under test again on the new revision, as 24 does today for a branch found elsewhere. The docs change then costs every combination under test its run: the wait moved from the docs onto the code.
- C. Put it at the head of the chain, landing as soon as the candidates under test land. Still waits behind code under test, which is what the user objected to.
- D. Fold it into the next landing of the chain. Waits the same.

**Conflicts.**

- i. **Work that does not merge cleanly onto the branch's revision, onto each formed candidate and onto the work of every entry ahead of it takes its place in the chain**, saying which entry it waits behind (chosen). An earlier submission is never made to conflict by a later docs change, and the conflict, where real, is found at forming against the later work as 12 and 65 already do.
- ii. Remove it as a conflict at once. It may merge cleanly once the entry ahead lands; removing it costs its session a submit for nothing.
- iii. Land it and let the earlier entry find the conflict at its landing. Displaces an entry under test by one submitted after it (6).

**Signed branches and the remote.**

- **No signer is asked** (chosen): no run is read, and the affected reading is the one every candidate's narrowing already rests on (15, 36, record 0044); a signed claim for it here alone would guard nothing the narrowing leaves open (78, core.22).
- A signer's attested affected reading. A new claim for a reading no other landing signs.

**Groups.**

- **The leading heads of an act that each qualify land beside the chain in the act's order; from the first that does not, every head takes the chain** (chosen). This is 6 applied: each head lands after the one before it (125), so a docs head behind a code head waits for it, and by then it is in a candidate and lands with its chain.
- Only a whole act of qualifying heads. Holds a group's leading docs behind its own later code for no reason the order gives.
- Any qualifying head of an act at once. Lands a head before one it is after (6).

## Decision outcome

Options 1, A, i, no signer, and leading heads (clause 140; clauses 8, 10, 23 and 24 amended; S68).

**Which work qualifies.** A submission's work qualifies when the check's affected reading, under the gate read at the branch's current revision (56, core.15), names no required check between the branch's revision and that revision with the work, nor between each candidate formed and not landed, as it would land now, and that candidate with the work. The work must also merge cleanly onto all of these and onto the work of every entry ahead, change neither the branch's policy nor what the check's or the place's profile names as its own setting (the runner's configuration, the toolchain the place enters), and stand on a revision covered by a verdict: a revision covered by none makes every required check affected (70), so nothing qualifies there. A runner whose affected reading is every check, the script profile's, qualifies nothing. Work some required check reads takes the chain whole, whatever else it changes: a mixed submission is never split. An entry whose after-entries have not landed waits as any does (6, 7), and an entry already in a candidate lands with its chain.

**When it lands.** At the first round that reads it, before forming, under the branch's lock (core.14), with nothing run. It takes no place, no run and no room in the window or the host's bound (13, 71); priority does not order it, since it waits on nothing.

**The combinations under test.** A candidate formed before the landing beside keeps its runs, its keys and its verdict. What its tree lacks is work no required check reads at that candidate, by the same reading that qualified the work, so every required check's key is what it was. When its verdict lets it land (22, 117), it lands from the revision the landing beside made, one commit per entry with that revision as the first parent of the first, its tree the tested tree with the work landed beside it (23). It is named as that revision extended by its entries (core.17). It is never formed or tested again for the work. A landing that finds the branch moved by anything else is refused and the chain formed again (24, S8), as today.

**Conflicts and checks that would read it.** Work that a formed candidate's checks would read, or that does not merge cleanly with an entry ahead, takes its place in the chain by priority and submission (6), and the entry says which entry it waits behind and why. It is then formed on top of that entry, conflicting or not, as any submission.

**Remotes, requests and intake.** The landing beside is a landing: on a branch on its remote, a leased push from the revision read (24, 124, core.5), refused and reported, never retried blind; its revision becomes the landing every later read must contain (121, record 0051). Only the instance advancing the branch (58) lands it, and a request's entry only the host that takes the branch's requests in (137, core.33): a submit on any other host writes the entry and lands nothing (core.4), and a wait there follows its request on the forge. No signer is asked (78, core.22).

**What is said.** The outcome is `landed`, pushed to the submitter's session as any outcome is and kept pending until delivered (50, 122; records 0068, 0070). `sluice submit`, `sluice wait` and the notice say "landed as <revision> beside the queue: no required check reads its changes"; their JSON carries the outcome `landed` and `beside: true`. An entry that waits instead says so in `sluice status` and a waiting submit's text, "waits behind #<n>: its checks read this change" or "conflicts with #<n>'s work", and in JSON `beside: {behind: <n>, why: "read" | "conflict"}`; work some required check reads at the branch's revision carries no `beside`. A request's report and its `sluice` check say it landed with no check run.

### What could go wrong, and what holds it

- **A formed candidate's own work makes a check read the docs.** The affected reading is asked at each formed candidate as it would land, so such work takes the chain and is tested with what reads it.
- **Two landings beside in a row.** Each is asked against each candidate as it would land now, so with every earlier landing beside carried; a candidate lands with all of them.
- **The affected reading is wrong.** Then every candidate's narrowing is already wrong (15); this record leans on no reading the gate does not already trust.
- **A policy change slipped in as docs.** The policy is excluded by name: it changes the gate, which a landing beside would change under candidates judged by the old one (56).
- **A docs landing displaces an earlier submission.** It cannot: work that does not merge cleanly with an entry ahead takes the chain.

### Consequences

- Clause 140 and S68 are added, and clauses 8, 10, 23 and 24 amended, in the statement at blueprints d6133ad, each marked proposed; rule core.39 is added, and the details of core.6 and core.17 name what it allows; `_open.md` names this record until it is built.
- One unit on the yes, citing this record: the round's qualifying test and landing beside before forming, the landing of a formed candidate from a revision landings beside alone moved, the entry's `beside`, and the words in submit, wait, status and a request's report.
- A branch whose required checks read everything, or whose runner reads every check as affected, behaves as before.
