---
number: 117
title: The one-process rebuild goes first, and a queue's home is redesigned before more fixes
status: accepted
date: 2026-11-12
decision-makers:
- the user
---

# The one-process rebuild goes first, and a queue's home is redesigned before more fixes

Status: accepted by the user on 2026-11-12, on their card ("Approved").

Sources: the design review of sluice's main at `045cb7d0`, 2026-11-12 (https://claude.ai/artifact/Vz3Q9zgdTaxmxnZJ7pjF5D), which four reviewers wrote without having made any fix it judged, and which agentplot-design checked against the code; record 0093; record 0116; `bolt/sluice-runs-one-host-process` at `e61acba0`.

## Context and problem statement

In the two weeks to 2026-11-12, main took 62 fix commits and 35 fix landings. The review found two things that make more fixes in two areas cost more than they save.

- **The busiest area of fixes is what record 0093 removes.** Record 0093, accepted by the user on 2026-11-06, gives a host one sluice process, with no intake mark, no hand-over between processes and no wait that runs a round. Main still has all of these, and about ten fixes and record 0100 went into them after 0093 was accepted. The bolt that builds 0093, `bolt/sluice-runs-one-host-process`, has 113 commits and has taken none since 2026-11-09, while main moved 330 commits past their merge base.
- **No record says where a branch's queue lives.** Each process works out from the policy it reads whether a branch's records are the host's files or the remote's refs, and it keeps that answer for its whole life. Locks, numbers and results are each kept by one store. So two processes that answer differently each hold their own lock and decide their own queue. On 2026-11-12 this one cause gave switchboard-kit's main a second #52 and #53, stalled every round of a GitHub-held branch with a run under test, and moved bolts between the host and GitHub (record 0116). Record 0116's "comes home" adds another per-host answer rather than a recorded one.

## Considered options

1. **The rebuild goes first, and a queue's home is redesigned before more fixes there** (chosen).
2. **Main keeps being fixed as defects arrive.** Every fix in those areas goes into code that 0093 deletes or that the redesign replaces, and the bolt falls further behind.

## Decision outcome

- **The one-process rebuild is sluice's first work once code goes on.** `bolt/sluice-runs-one-host-process` restarts, taking main in, ahead of every other sluice bolt.
- **Until it lands, no fix of main goes into what it removes.** That means the intake mark, hand-overs and their withdrawals, a wait that runs a round, and a host's bound counted across processes. A defect there gets a workaround that holds, such as a restart, a resubmission or a step by hand, and is told to agentplot-design.
- **A queue's home is redesigned before any more fixes change how a queue's records are routed or moved between the host and the remote.** The redesign is a record of its own. It writes a queue's home in its records and keeps an entry's number across a move. `a-branch-comes-home-when-its-signers-go`, record 0116's unit, is not built.
- **Two fixes go ahead.** `a-remote-held-round-keeps-its-lock`, which the user named. And `a-branchs-numbers-come-from-its-own-records`, which the user approved on 2026-11-12, stops a branch's numbers repeating and gives each branch's numbers from its own records, as the redesign will.

Deciders: the user, on agentplot-design's card of 2026-11-12 (req_d4be2dcc695e41b9b335). To "Make the one-host-process bolt sluice's first work after the pause, and stop fixing the parts it removes?" they answered "Yes, rebuild first and freeze those fixes". To "Redesign where a queue lives (host or GitHub) before more fixes there, holding the 'branch comes home' change?" they answered "Yes, redesign first; the live lock fix still goes ahead". Which fixes the freeze covers, and that the numbers fix goes ahead beside the lock fix, are agentplot-design's.

### Consequences

- Record 0116's half on a branch coming home waits on the redesign; its half on routing by the head's policy stands.
- A defect in a frozen area stays open longer, held by a workaround.
- The review's other findings become design records in turn: one owner of a run's state, a round's effects performed only after its write, a wait that reads recorded links, and contracts for the host and the forge.

## Amendment, 2026-11-12: sluice's main takes no more fixes, and its defects go into the refactor

Status: proposed to the user on 2026-11-12, as `briefs.15` has it; accepted only on their yes. Should sluice's main stop taking fixes until the rebuild and the redesign have landed? Recommended: yes. Most of November's fixes went into code the rebuild deletes or the redesign replaces, so each one is paid for twice. Until then, a sluice problem gets a workaround that holds, such as a restart, a resubmission or a change to the repository's own settings, and its cause is written into the redesign. Work that would land untested, or a submission that would be lost, is still fixed at once. The other way is to keep fixing main as problems arrive, which delays the refactor and keeps adding to what it has to replace.

Deciders: the user, on their card. Where each defect goes in the refactor, and the one exception below, are agentplot-design's.

Sources: agentplot-ops' readings of switchboard-kit's records on chuck-herdr-alpha, 2026-11-12: twelve runs that day ended `unproven` and were opened again, and each waited for one of the host's two runs while its status read "testing 37m of 25m"; and of sluice's main at `5d571ccb`, where a run with an unproven replay is opened again whole with replays off (`crates/sluice-core/src/step.rs`, the unproven branch and the run's request) and status measures a candidate's time from its first testing line (`crates/sluice/src/progress.rs`). The statement's clauses 66, 87, 108 and S42.

- **No fix of sluice's main is launched, in any area, until the refactor has landed.** A defect gets a workaround that holds, such as a restart, a resubmission, a step by hand or a change of the repository's own policy, and goes to agentplot-design, who writes it into the refactor's design. The one exception is a defect that lands a combination that has not passed, or loses a submission, since no workaround holds either; it is fixed on main as before.
- **The refactor is the one-process rebuild, then a queue's home and a run's life redesigned on it.** The rebuild is this record's first point. The redesign is one record that writes a queue's home in its records, as this record's third point says, and a run's life as states written before anything acts on them, one owner of each run's state, as the review's findings name.
- **The two fixes asked for are requirements of the redesign, not fixes.** A run's time limit is the run's own, from its own start: a run that waits for room is not running, is under no limit (66, 87), and reads as waiting for room since when, never as testing. A replay sluice cannot prove is run again alone: the checks that ran keep their outcomes, and only the unproven ones run, each with no replay of its own key (108, S42).
