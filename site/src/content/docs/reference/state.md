---
title: State
description: Where sluice keeps its state, and what a landing leaves in history.
sidebar:
  order: 3
---

Everything sluice decides is written to plain files before it is acted
on, in the repository's git directory, never committed and never pushed,
readable with plain tools whether or not sluice is running.

```
.git/sluice/bolt/concurrent-units/
  version              57: bumped on every write, so two rounds never both write
  submissions/12.json
  submissions/13.json
  log.jsonl            every submission, test run, result, blame and landing, in order
.git/sluice/.logs/bolt/concurrent-units/<candidate>/<task>.log
                       what each failed test printed, for the last 20 failed combinations
.git/sluice/caches/    the [[checkout.cache]] entries, the last two keys of each
.git/sluice/results.jsonl
                       every test result of the repository, with its executor
```

## A submission

```json
{
  "schema": "sluice/submission@1",
  "id": 13,
  "branch": "bolt/concurrent-units",
  "head": "8d41b6c2e5",
  "from": { "agent": "crw-1-unit-2", "worktree": "unit/batch-review" },
  "submitted": "2026-10-11T09:16:40Z",
  "after": [],
  "priority": 0,
  "attempt": 1,
  "state": "failed",
  "tested": { "combination": "e20a6f9b48", "on": "c41e8f2d90", "submissions": [12, 13] },
  "result": "failed",
  "failed": [ { "task": "tests:t-batch-cards", "hash": "5c0e97aa21",
               "log": ".git/sluice/.logs/bolt/concurrent-units/c7/tests:t-batch-cards.log" } ],
  "told": "2026-10-11T09:21:55Z"
}
```

`after` lists submissions this one must land after. `attempt` counts how
many times this worktree has submitted to this branch. Submission ids
are numbered in order, and that order is the queue's order.

## The log

One event per line. A test is logged when it runs, a failure exactly
like a pass, and a replayed result is logged as a replay, never as a
run:

```json
{"schema":"sluice/log@1","event":"ran","combination":"9ab3d07c11","task":"tests:t-after","hash":"4d1e22b0c9","outcome":"pass","host":"mac-studio"}
{"schema":"sluice/log@1","event":"replayed","combination":"9ab3d07c11","task":"tests:t-amend","hash":"77aa01f3de","outcome":"pass"}
{"schema":"sluice/log@1","event":"ran","combination":"e20a6f9b48","task":"tests:t-batch-cards","hash":"5c0e97aa21","outcome":"fail","host":"mac-studio"}
{"schema":"sluice/log@1","event":"blamed","id":13,"combination":"e20a6f9b48"}
{"schema":"sluice/log@1","event":"landing","ids":[12],"from":"c41e8f2d90","to":"b7d2e94f03"}
{"schema":"sluice/log@1","event":"landed","ids":[12],"to":"b7d2e94f03"}
```

`landing` is written before the branch moves and `landed` after, so a
round that crashes between the two is finished correctly by the next
one. Full example files are in the repository under
[docs/examples/state](https://github.com/agentplot/sluice/tree/main/docs/examples/state).

## The version

`version` is bumped on every write, and a round writes only if the
version is still the one it read. Two rounds never both write; the one
that loses reads again. The same rule, as a push with an expected old
value, is how [peers](/sluice/many-hosts/peers/) would share one branch's
state through a git remote.

## What a landing looks like in history

One merge commit per submission: first parent the branch, second parent
the agent's head, so the agent's commits stay in history and `wt list`
shows its worktree as merged. What was tested rides along as trailers,
which outlive the state files:

```console
$ git log -1 bolt/concurrent-units
commit b7d2e94f03
Merge: c41e8f2 3f9c2e1
Author: sluice <sluice@localhost>

    chore(land): #12 unit/touches

    Sluice-Submission: 12
    Sluice-From: crw-1-unit-1 (unit/touches)
    Sluice-Tested: 9ab3d07c11
    Sluice-Result: passed (2 run, 31 replayed)
```

The landing commit carries exactly the tree that was tested
([ADR 0006](/sluice/decisions/0006/)), so when the submissions ahead of a
combination land as they were tested, the combination is the same one on
the new revision and is not tested again.
