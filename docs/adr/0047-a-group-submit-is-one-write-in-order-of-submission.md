---
number: 47
title: A group submit is one write, in order of submission
status: accepted
date: 2026-10-23
decision-makers:
- Chuck Swanberg
---

# A group submit is one write, in order of submission

Deciders: the user, ratifying the group submit on 2026-10-23 ("I approve your recommendation"), and building the OpenSpec change `a-group-submit-takes-all-or-none`.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 5, 6, 7, 12, 17, 18, 50, 62, 65, 67, 82, 125, 128, C.5 and S59; `agentplot/blueprints:design/sluice/proposals/batch-submit-1.md`; ADR 0021; rules core.4, core.6, core.14.

## Context and problem statement

Clause 125 lets a submitter hand sluice several heads as one act, in an order: sluice takes all of them or none, "each as its own entry after the one before (6)", and forms no candidate on any of them until every one is taken. Two of its words need a reading before they can be built. What makes each entry "after the one before"? And on a branch that takes pull requests, where `sluice submit` writes no entry and intake later takes each request on its own, what keeps intake from taking the first request of the act before the last is open?

## Considered options

For "after the one before":

1. Each entry follows the one before in order of submission: the act numbers its entries in the order given, gives every one the same `after` and priority, and names none of the act as another's `after`.
2. Each entry names the one before as an after-entry.

For the act's guarantee on the branch's records:

3. One write: the act reads every place first, then inserts every entry, each place's earlier one withdrawn (67), in one compare-and-set of the branch's records (62).
4. A "group pending" mark on each entry, which rounds skip until a second write clears it.

For pull requests:

5. A group mark: each request's body carries `<!-- sluice: group=<token> <k>/<n> head=<oid> -->` beside the agent mark, written before its head reaches the request, and intake takes a marked group only whole, through one group submit in `k` order, once all `n` requests are open, out of draft, at their marked heads and meeting their conditions.
6. Every request opened as a draft, then all marked ready together.
7. Each request taken as it becomes ready, as today.

## Decision outcome

Options 1, 3 and 5.

Option 1 gives what 6 promises and keeps the rest of 125. Under 7, an entry whose after-entry is removed as conflict or culprit, neither landed nor ahead, waits for good. Under option 2 the third bolt of S59 would never land once the second conflicts, against 125's "each thereafter its own entry" and S59's "the other two land". Numbered in order with equal priority, the act's entries keep their order in the chain (6). With the same `after` and priority on each, none is let in ahead of an earlier one because only the earlier waits on an `--after`. A later submission of higher priority may still come between them, as 6 allows of any two; it is tested and lands with them in order.

Option 3 gives 125's guarantee with nothing for a reader to read. A reader of the records sees none of the act or all of it, on this host or, for records on a remote, on any (C.6). So no round, waiting submit or `sluice wait` forms a candidate on part of it (rules core.6, core.14). Every head is published before the write (82), and the pool is woken once after it. Option 4 is a second write and a state every round must read and honour, where one write needs neither.

Option 5 holds the same guarantee where intake, not the act, writes the entries. A request is bound to its group only while its head is the marked head and it has had no submission at that head. Once the group is taken each request is its own: a new head on one withdraws its entry alone and leaves the group's mark behind (125, 5). Until the group is whole, each bound request's state names the requests of its group it waits for. Option 6 needs a call the forge seam does not have: marking a request ready is a GraphQL mutation, `markPullRequestReadyForReview`, with no REST form. And requests marked ready one at a time are still taken one at a time. Option 7 forms a candidate on the first request alone, which 125 forbids.

Clause 128 says what sluice reads of one request, or cannot, decides that request alone. A group's request that intake cannot read does hold the rest of its group, because the group was submitted to be taken all or none (125): the hold is the act's own condition, set by its submitter, and it holds no request outside the group, whose requests are taken, forecast and posted as 128 says.

### Consequences

- `sluice submit <branch> --worktree <a> --worktree <b> …` is one call of `Sluice::submit_group`, and `submit` is that call with one place, so the two cannot drift.
- A push that fails midway through a pull-request group leaves requests marked as a group of `n` with fewer present, and intake takes none of them; the same act run again marks every request with a new token.
- A group's request closed before the group is taken holds the rest, which say so, until a new submit names them, as a group or one by one.
- A plain `sluice submit` of one worktree rewrites its request's body with no group mark, freeing it from any group it was in.
