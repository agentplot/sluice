---
number: 78
title: A finished submission leaves the live records for an index that names its times, and the log is kept in parts
status: accepted
date: 2026-11-02
decision-makers:
- Chuck Swanberg
---

# A finished submission leaves the live records for an index that names its times, and the log is kept in parts

Status: accepted. The operator approved the change `finished-submissions` on 2026-10-16, and clauses 61 and 130 to 132 carry it in the statement from batch 18's rulings (agentplot/blueprints 53f6d44, 2026-10-25); clause 131 gained the index's times on 2026-10-28 (agentplot/blueprints f23cee4). This record, written 2026-11-02, sets down the decision those carry, and moves no clause.

Deciders: the operator, approving the change's proposal on 2026-10-16; batch 18's rulings of 2026-10-25; for the index's times, the statement's answer of 2026-10-28 to a planner's question of which document carries a decided submission's submitted time and the start of its checks.
Sources: statement clauses 26, 53, 61, 63, 129, 130, 131 (as amended 2026-10-28), 132 and C.6 in agentplot/blueprints `design/sluice/requirements.md`; rules core.11, core.14; the change `finished-submissions` (`openspec/changes/finished-submissions/`, its proposal and design); `crates/sluice-core/src/records.rs` (`FinishedLine`) on main at 2026-11-02.

## Context and problem statement

Every read of a branch's state took the whole of it: the records seam's `read` (61) returned the branch's document, every submission and the whole log, and the core parsed every submission to answer any question about the branch, one submission's state included. Nothing ever left that set. This repository's `main`, one day after its first submission, held 86 submission files and a log of 1,058 lines; at that rate a month leaves about 2,500 files and 6 MB of log, so every round, every status line and the overview pay for the branch's whole history rather than its queue.

A reader of the overview who wants to split a decided submission's time between its wait, its checks and its landing finds the times only in that submission's own record, read by a status of it alone.

## Considered options

1. Keep every submission and the whole log among the live records.
2. Move a finished submission out of the live records unchanged, keep an index of the finished among them, naming each one's times, and keep the log in parts.

## Decision outcome

Option 2.

**Finished.** A submission that has landed or been withdrawn is finished. It moves from `submissions/<id>.json` to `.finished/<id>.json`, unchanged in form (53), in the first write that changes something once no combination, landing or outbox item names it, its put, its removal and its index line one write (63). A blamed or conflicting submission stays live, since a withdrawal or a resubmission still changes it. `.finished` starts with a dot, which no component of a branch's name can, so it never meets a nested branch's records.

**The index.** `finished.jsonl` keeps one line per finished submission among the live records: its id, state and worktree, for ordering an `after` and counting attempts (130); its subject, which a person is shown it by (26); and when it was submitted, when its decisive run's checks began and ended, and when it was decided (131). The overview's recent rows carry those times, so a reader splits a submission's time from the one document the host's reader writes (129), with no status read of each finished entry.

**The reads.** A round, the overview, a branch's status and a wait read the live records and the index, never a finished record; a status of one submission reads the live records and at most that submission's own finished record; only a reader that asks for history reads the finished records (131). A finished submission is past amending and withdrawing, refused from its own record.

**The log in parts.** A write that would take `log.jsonl` past a thousand lines closes it as `log/<n>.jsonl`, under the branch's lock as every write is (core.14), and starts the next. A line never changes once written; the live records carry the current part and the newest closed one, so the overview's recent lines are always in them; and a reader that asks for the whole log reads every part in order (132).

Option 1 makes every read pay for the branch's whole history, and what a round needs is its queue.

### Consequences

- Rule core.41.
- Built: finished submissions, the index's id, state, worktree and subject, and the reads (part 1 of the change `finished-submissions`). Not built: the index's times and the overview's rows carrying them, and the log in parts (that change's part 2). `docs/architecture/_open.md` lists them.
- A reader of an older sluice's records sees every submission under `submissions/`, and a newer sluice moves the finished ones on its next write. A script that globbed `submissions/*.json` for history sees only the queue; `sluice status <branch> --all` reads the history.
