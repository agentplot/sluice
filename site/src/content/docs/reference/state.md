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
.git/sluice/
  results.jsonl        every test result of the repository, with its executor
  .next-id             the next submission's id
  .lock                held while an id is given or a branch's records are written
  bolt/concurrent-units/
    version            57: bumped on every write, so two rounds never both write
    branch.json        the queue's own state, one document
    submissions/14.json
                       a submission still in the queue, or blamed or conflicting
    .finished/12.json  a submission landed or withdrawn, moved here unchanged
                       once nothing still waits on it
    finished.jsonl     one line for each submission in .finished/: its id,
                       state and worktree
    log.jsonl          every submission, test run, result, blame and landing, in order
    lock               held by a round of this branch
  .logs/bolt/concurrent-units/<candidate>/<task>.log
                       what each failed test printed, for the last 20 failed combinations
  .caches/<bucket>/<key>/
                       the [[checkout.cache]] entries, the last two keys of each
  .places/<checkout>.json
                       a checkout's record: its kind, path, branch, ports, caches,
                       the block of ports it holds on the host and, for a
                       worktree, when its last run started and how it ended
  .places/<checkout>/  a candidate's checkout, made for its run and removed after
  .previews/<preview>.json
                       a preview's record: its branch, revision, ports, names,
                       addresses, when it was made and its command's run; for a
                       failed preview, what its command printed and nothing held
  .previews/<preview>/ a preview's checkout, kept until it is released
  .tmp/<checkout>/     the checkout's scratch, SLUICE_TMP; a worktree keeps its own across runs
  .runs/<checkout>/    a candidate's run, or a preview's command
  .runs/<worktree>/<n>/
                       each run an agent started in its worktree, numbered;
                       release/ beside them is where sluice release runs
  in each run directory:
    request.json       what to run, where, with which environment and limits
    lock               held by the run's supervisor for as long as the run is going
    pgid               the run's process group, so the whole of it can be ended
    out.log            what the run printed
    out.times          when out.log grew: its length and the time, at most once a second
    result.json        how the run ended, once it has
    release.log        what the release command printed
    released           written once the release command has run
```

What belongs to the host and not to any repository is kept under
`$XDG_STATE_HOME/sluice` (default `~/.local/state/sluice`), shared by
every repository on the host and never pushed:

```
~/.local/state/sluice/
  ports/<n>            a block of ports, naming the checkout that holds it
  running/.lock        held while a run is counted against the bound
  running/<run>.json   each run going on the host, every repository's:
                       a submission's or an agent's, its repository, its
                       checkout, where its output is, the process that
                       started it and when
  running/<run>.hold   held by the process that started the run until the
                       run is under way
  sessions/.lock       held while an agent's run's file is written
  sessions/<worktree>.json
                       each run an agent has going in its worktree: the
                       worktree, its branch, its process, when it started,
                       where its output is, the command it runs and how far
                       its tasks have got
```

A checkout's name is written as one file name, each `/` of it as `%2F`
(or `+2F` for a directory a test works in), so a worktree's whole path
is one name. macOS and Linux allow a file name of 255 bytes, so a name
longer than 246 once written is cut short. It keeps its first 160
bytes, then `%~` (or `+~`) and 16 hex digits of the SHA-1 of the whole
name. Each record carries the whole path or revision it is for,
so nothing needs the file name read back.

A preview's record is kept apart from the other checkouts' so that a
sluice older than previews never reads one. A preview's command counts
in `running/` and is listed in `sessions/` while it runs, as an agent's
run is.

A block of ports is free again when the checkout named in it is gone.
A file in `running/` counts against the bound while its `.hold` is held
or its run is going. One whose run has ended, or whose starter stopped
before the run began, holds nothing: the next run that starts removes
it. A file in `sessions/` is there while its agent's run is going, and
its `sluice exec` rewrites it as each task ends, for `sluice overview`
to show; nothing that decides a result reads it.

## A branch whose state is on its remote

A branch with a `remote`, in a policy with
[`[signers]`](/sluice/reference/configuration/#signersname-who-may-sign-results),
keeps its state on that remote as refs. Every host that submits to the
branch reads the same queue, and none of them has to be running for the
rest to go on:

```
refs/sluice/state/<branch>       one commit: the branch's files above, branch.json,
                                 submissions/, .finished/, log.jsonl and version,
                                 replaced on every write
refs/sluice/runs/<signer>        each signer's signed results, only ever added to:
                                 <combination>/<system>/<n>.json, one run's results
                                 in a signed envelope
refs/sluice/ids                  the next submission's id, for every host
refs/sluice/heads/<commit>       a submitted head, until the submission leaves the queue
refs/sluice/combinations/<branch>/<id>
                                 a combination open to any host with room
refs/sluice/logs/<branch>/<id>   what a failed combination's tests printed, for
                                 the last 20
refs/sluice/hosts/<host>         a host's sign of life and the room it has, rewritten
                                 by that host alone
refs/sluice/set-aside            the results `sluice set-aside` set aside, one file
                                 each, only ever added to
```

A write to a branch's state is a push that the remote takes only if the
ref is still the commit the round read, so the version rule below holds
across hosts. A signer's results are added on top of whatever its ref
holds, so two hosts that sign as one signer never lose each other's.
What only this host keeps, its locks, its runs' tasks and copies of the
logs it kept, is in `.git/sluice-remote/<remote>/`. A result moon
replayed is kept there too: a replay is nobody's observation, so nobody
signs it.

Read any of it with git, on any host:

```sh
git fetch origin '+refs/sluice/*:refs/sluice-fetched/origin/*'
git show refs/sluice-fetched/origin/state/main:branch.json
git ls-tree -r refs/sluice-fetched/origin/runs/box
```

A signed result is an [in-toto](https://in-toto.io) statement in a
[DSSE](https://github.com/secure-systems-lab/dsse) envelope. Each test
and system is one subject, named `<test>@<system>`, its digest the
test's key. The predicate holds each outcome, the branch, the
combination, and what the signer claims of the run: its executor,
whether it was confined, and its system. A host signs with the SSH key
its [`[signing]`](/sluice/reference/configuration/#signing) names. sluice counts a result
only once the signature checks against a signer the branch's policy
names.

## A submission

```json
{
  "schema": "sluice/submission@1",
  "id": 13,
  "branch": "bolt/concurrent-units",
  "head": "8d41b6c2e5",
  "from": { "agent": "crw-1-unit-2", "worktree": "unit/batch-review",
            "path": "/home/me/wt/unit-batch-review" },
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

`state` is one of `queued`, `waiting` (behind the submissions in
`after`), `testing`, `passed`, `landing`, `landed`, `failed`,
`conflict` and `withdrawn`. These nine are stable: under
`sluice/submission@1` none of them is renamed or removed, and none is
added, so a script that reads `state`, such as an agent looping on
`sluice wait --json`, can match all of them.

`after` lists submissions this one must land after. `attempt` counts how
many times this worktree has submitted to this branch. `from.worktree`
is the branch the work was submitted from; `from.path` is the worktree
itself, for a reader to match a submission to the session that made it.
A submission written by an older sluice has no `path`.

A passed submission waiting under its branch's [`hold`](/sluice/reference/configuration/#hold)
has `"held": { "by": [12, 13, 14], "until": "2026-10-11T09:26:40Z" }`:
the longer combination it waits for, by its submissions, and when the
hold runs out. Nothing reads it to decide; the round decides from the
branch's record, where each combination that passed has `passed_at`.

A submission that has landed or been withdrawn moves, unchanged, from
`submissions/` to `.finished/` in the same write that tells its outcome,
once no combination or landing still names it. A round, `sluice
overview`, `sluice` alone and `sluice status <branch>` read only the
branch's own record, `submissions/` and `finished.jsonl`, so they cost
the size of the queue, not of the branch's history. `sluice status <id>`
reads a finished submission's one file, and `sluice status <branch>
--all` reads them all. Records written by an older sluice move on the
branch's next write.

A pull request's submission comes from `pull/<n>`, its number on
GitHub, in place of a worktree: `"from": { "agent": "crw-1-unit-2",
"worktree": "pull/41", "head": "unit/cards" }`, with no `path`; `head`
is the branch the pull request was opened from. Its agent is the one the pull request's
description names, in the comment `sluice submit` writes there, or
else whoever opened it. Each new commit on the pull request is a new
submission from the same `pull/<n>`. A pull request still waiting for
review or checks has no submission at all: its state is read from GitHub
each round, and shown there as the `sluice` check. Submission ids
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
one.

`released` is logged when a combination's checkout is released, and
when a run or round releases the checkout of a worktree that no longer
exists, which also names the worktree:

```json
{"schema":"sluice/log@1","event":"released","place":"session//home/me/wt/unit-c","worktree":"/home/me/wt/unit-c"}
```

Full example files are in the repository under
[docs/examples/state](https://github.com/agentplot/sluice/tree/main/docs/examples/state).

## The version

`version` is bumped on every write, and a round writes only if the
version is still the one it read. Two rounds never both write; the one
that loses reads again. On a branch whose state is on its remote, the
same rule is a push with an expected old value, and it holds between
hosts as it does between rounds on one
([above](#a-branch-whose-state-is-on-its-remote)).

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
