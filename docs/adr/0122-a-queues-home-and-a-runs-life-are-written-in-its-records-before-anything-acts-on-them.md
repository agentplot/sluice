---
number: 122
title: A queue's home and a run's life are written in its records before anything acts on them
status: accepted
date: 2026-11-13
decision-makers:
- agentplot-design
---

# A queue's home and a run's life are written in its records before anything acts on them

Status: accepted, agentplot-design's, as the redesign record 0117 calls for. The user's word that sluice is refactored, not patched, is record 0117's amendment of 2026-11-12: "We need to do a refactor".

Sources: the design review of sluice's main at `045cb7d0`, 2026-11-12 (https://claude.ai/artifact/Vz3Q9zgdTaxmxnZJ7pjF5D), on a queue's home decided by each process, a run's state with no one owner, and effects performed before the write that names them; records 0037, 0093, 0114, 0116 and 0117 with its amendment; agentplot-ops' readings of switchboard-kit's records on chuck-herdr-alpha, 2026-11-12 (a second #52 and #53, a round stalled on its lock, bolts moved between the host and GitHub, twelve unproven runs opened again whole and read as "testing 37m of 25m"); sluice's main at `49527985`: `crates/sluice-core/src/records.rs` (`RunState` is open, starting, testing, passed, failed or timed out; `no_replay` reopens a whole run) and `crates/sluice-core/src/step.rs`; the statement in agentplot/blueprints, `design/sluice/requirements.md`, clauses 53, 60, 62, 66, 71, 72 to 77, 87, 88, 108, 109, 114, 129, 135 and C.6.

## Context and problem statement

Most of sluice's fixes in November went into two places with no design of their own.

- **Where a queue lives is worked out, not written.** Each process works out from the policy it reads whether a branch's records are the host's files or the remote's refs, and keeps that answer. Two processes that answer differently each hold a lock and number entries of their own. Record 0116 routes by the head's policy, one answer per read, but a move between homes is still a side effect of whichever round first reads the change.
- **A run's state has no one owner, and acts come before the write.** A run is made, started, released or reopened by whichever step reaches it, and some of those acts happen before the records say so. An unproven replay reopens the whole run, and status measures a candidate from its first testing line, so a run waiting for room reads as testing past its limit.

## Considered options

1. **Write both in the branch's records: the queue's home as a record every reader follows, and a run's life as states that are written before any act (chosen).**
2. **Keep working them out, and fix each defect as it comes.** This is what November did; record 0117's amendment ends it.
3. **One coordinator process per branch that holds both in memory.** It gives one owner, but a host that stops loses the queue's state, and 111 has a submission reach its outcome with no host of its submitter running.

## Decision outcome

### A queue's home is a record

- **One home, written down.** A branch's queue has one home, the host's files or its remote's refs, written in the branch's records with the revision of the policy that chose it. Every process reads the home from the records and never works it out from a policy it read. The lock, the numbers and the entries live in the home and nowhere else.
- **The policy says where it should live; a move takes it there.** When a round reads a policy at the branch's head whose home differs from the recorded one (record 0116's route), it writes a move in the current home and opens no new candidate. Once no run of the branch is under test, it carries every open entry to the new home with its number and its state, writes the new home's record, and closes the old home with a pointer to the new one. Each step is written before the next begins, so a round that stops halfway leaves the move written and the next round finishes it.
- **Numbers survive a move.** An entry keeps the number it was given, which comes from its branch's own records (record 0114), and a waiter on that number finds it in the new home through the old home's pointer.
- **A wait reads the entry where it lives.** A wait follows the entry's home, and a closed home's pointer, to the entry's recorded state. It never infers what carried the entry.
- **The other direction is the same act.** A branch whose policy stops naming signers comes home by the same move, which is the half of record 0116 that record 0117 held.

### A run's life is states written before any act

- **One owner.** A run's state is written only in its branch's home, and only by the instance whose claim holds it or the lead that opened it. Nothing acts on a run except from a state written there.
- **The states.**

  ```
  open ──claim──▶ claimed ──start──▶ running ──report──▶ ended ──release──▶ closed
    ▲  waiting for an executor            the limit       each check's outcome:
    │  or for room (87, 71)               counts here     passed, failed, none,
    │                                     (66)            or unproven (108)
    └──────────── a follow-up run holding only the unproven checks
  ```

- **Write, then act, and every act can be done twice.** Making a place, starting the checks, releasing the place, telling a session and landing each happen only after the state that names them is written, and each leaves a proof the next reader checks, so an act found done is not done again. A host that stops between the write and the act leaves a state the next reader finishes.
- **The limit is the run's.** A run's time limit counts from its own `running` state. An `open` run is under no limit while a live executor of its system may take it (87). Status reads the state: an open run waits for room since when; a running run is testing so long of its limit.
- **An unproven replay reruns alone.** A run that ends with unproven checks keeps every other check's outcome. The lead opens a follow-up run holding only those checks, each run with no replay of its own key, and the candidate's verdict is read over both runs (108, S42).
- **A pool's job is a run like any other.** Its start writes the claim and its end the report, through the same states (88), so the pools of record 0119 are built on this.

### How it is built

After the one-process rebuild (record 0093), as record 0117 orders. The statement's C.6 and the clauses of the run's life are amended to match on agentplot/blueprints. A host's queues are drained before the release that brings it, so no record of the older shape is ever read and no code maps one shape to the other (principle 12).

### Consequences

- The November defects in these areas have nowhere to happen. Two processes can't hold two locks or two numbers for one branch, and a move can't strand an entry. A run waiting for room can't read as timed out, and an unproven replay can't cost a whole gate.
- A move between homes waits for the branch's runs under test to end, and opens no candidate meanwhile.
- The GitHub Actions and CodeBuild pools (record 0119) come after this, and their jobs report through the same states.
