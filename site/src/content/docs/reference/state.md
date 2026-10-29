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
  .refused/bolt/concurrent-units.json
                       while this host's rounds can't read the branch's remote:
                       what they couldn't read, since when, for how many rounds,
                       and which of it the log holds yet
  .places/<checkout>.json
                       a checkout's record: its kind, path, branch, ports, the
                       block of ports it holds on the host and, for a
                       worktree, when its last run started and how it ended
  .places/<checkout>/  a candidate's checkout, made for its run and removed after
  .previews/<preview>.json
                       a preview's record: its branch, revision, ports, names,
                       addresses, when it was made and its command's run; for a
                       failed preview, what its command printed and nothing held
  .previews/<preview>/ a preview's checkout, kept until it is released
  .tmp/<code>/         a checkout's scratch, SLUICE_TMP, which its record names:
                       a short code made from the checkout's name, the same
                       length for every checkout; a worktree keeps its own
                       across runs. A checkout an older sluice made keeps its
                       scratch under the checkout's name until it is released.
                       A directory here that no record names is removed by
                       the next round or sluice exec on the host
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
  forge/answers/<h>.json
                       GitHub's last answer at one address, with the ETag
                       and time it came with, so the next read asks GitHub
                       only whether it changed
  forge/pauses/<app>-<installation>-<allowance>.json
                       until when GitHub said the app's allowance of calls
                       on that installation is spent, in GitHub's words
  forge/requests/<owner>/<repository>/<branch>.json
                       when sluice last asked GitHub for the open pull
                       requests into the branch, and what it read and
                       posted of each, so that one that hasn't changed
                       isn't read again
  forge/requests/<owner>/<repository>/<branch>.lock
                       held while that file is written
  forge/requests/<owner>/<repository>/<branch>.moved
                       touched by a submit that opened, updated or marked
                       ready a pull request into the branch, so the next
                       round asks at once
  ports/<n>            a block of ports, naming the checkout that holds it
  readers/<repository>.lock
                       held by the repository's `sluice run`, or else by the
                       first watch or wait of it, for as long as it runs
  readers/<repository>.json
                       the overview it last read, as `sluice overview --json`
                       prints it, for every other watch and wait to follow
  readers/<repository>.driver
                       held by the repository's `sluice run` for as long as
                       it runs
  readers/<repository>.version
                       the version of sluice that holds the lock
  readers/<repository>.lease
                       held by a watch of a newer sluice until it takes over
                       the reading
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

Every sluice on the host reads `forge/` before it calls GitHub. An
answer is used only when GitHub replies that it hasn't changed, and one
no read has used for a day is removed. A pause stops every sluice on
the host from calling GitHub until the time GitHub named. A branch's
file in `requests/` says when its pull requests were last asked for, so
a sluice started again asks nothing until `[forge] discovery` has passed
since any sluice on the host asked. Nothing in `forge/` holds a token,
and any file in it may be removed at any time: sluice then asks GitHub
as if it had kept nothing.

A block of ports is free again when the checkout named in it is gone.
A file in `running/` counts against the bound while its `.hold` is held
or its run is going. One whose run has ended, or whose starter stopped
before the run began, holds nothing: the next run that starts removes
it. A file in `sessions/` is there while its agent's run is going, and
its `sluice exec` rewrites it as each task ends, for `sluice overview`
to show; nothing that decides a result reads it.

One process on the host reads a repository's queues for every watch and
wait of it. Where `sluice run` is running in the repository, it is that
process. It holds the `.driver` file's lock and the `readers/` lock, and
writes the overview file beside them after each of its passes, whether
or not anything is queued. Every wait then follows it and runs no rounds
of its own, and a watch or wait that held the lock when `sluice run`
started lets it go within its next look. A second `sluice run` of the
repository holds neither lock and runs nothing until the first stops.
Where no `sluice run` is running, the first `sluice overview --json
--watch`, `sluice wait` or waiting `sluice submit` of the repository
holds the `readers/` lock. The reader keeps the overview in the file
beside it, rewritten whole each time it changes, so nothing reads half
of one. A read of the queues it couldn't make goes in the file in its
place, as the error document `sluice overview --json` prints when it
fails. It holds the words the read was refused with, and the reader
reads again at its next look. Where the repository has no sluice, the
file holds the watch's last document, the overview with `"set_up":
false`, and the watch ends. The next watch or wait to hold the lock
removes that file before it reads, so nothing that starts once sluice is
set up ends on it. Every other watch of the repository on the host
follows that file and reads no state of its own. Every other wait takes
its submission's progress from that file and, where no `sluice run` is
running, still runs sluice's rounds, which read the branch's state as
they always do. It reads its submission's own file once, when the
submission leaves the queue, to say how it ended. When the one holding
the lock stops, however it stops, the lock goes with it: the next watch
or wait takes over within its next look, a second for a watch, three for
a wait. A watch of a newer sluice than the watch or wait holding the
lock takes over from it within a few seconds. It holds the lease, and
the older one lets go at its next look and follows it from then on; the
newer one reads from its own next look. A sluice from before the lease
takes no part: it names no version and never looks at the lease, so it
reads until its watch or wait ends, and newer watches follow it
meanwhile. The file names take the repository's git directory, which all
its worktrees share, written as one file name as a checkout's is.
`sluice overview` alone always reads the state itself.

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
signs it. Beside them, `refused/<branch>.json` holds what this host's
rounds couldn't read of the remote while that lasts, as `.refused/` does
for a branch whose state is on the host.

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

## A branch's own record

`branch.json` holds the queue's own state: the revision its
combinations are formed on, the combinations, a landing under way and
the outcomes still to deliver. Two fields say what a round reads the
branch against:

- `landed`: the revision every later read of the branch must contain.
  A landing sets it to the revision it landed, `sluice branch accept`
  to the revision accepted, and `sluice branch create` to the revision
  it made the branch at. A record an older sluice wrote has none. The
  next round takes the log's last `landed` line, or the revision the
  queue was last formed on when that one lacks the landing, as for a
  branch deleted and made again.
- `held`: while the revision the branch was last read at lacks
  `landed`, `{"read": "10e4791a2b", "lacks": "c43609b8e1", "since":
  "2026-10-23T09:16:40Z"}`. Nothing is formed or landed on the branch
  meanwhile. It goes on the first round that reads a revision holding
  `landed`, or with `sluice branch accept`.
- `copies`: for a branch kept on a remote, each host whose own branch
  of that name a round could not set to `landed`, by the host's name:
  `{"build-2": {"at": "10e4791a2b", "lacks": "c43609b8e1", "why": "git
  holds the lock on refs/heads/bolt/x", "since":
  "2026-10-23T09:16:40Z"}}`. `at` is the revision that host's branch
  holds, `null` when it has none. Each host writes only its own entry,
  and takes it out on the round that sets its branch to `landed`.

All three are left out when there is nothing to say, so
`sluice/branch@1` is unchanged, and an older sluice reading them passes
over them.

## A submission

```json
{
  "schema": "sluice/submission@1",
  "id": 13,
  "branch": "bolt/concurrent-units",
  "head": "8d41b6c2e5",
  "from": { "agent": "crw-1-unit-2", "worktree": "unit/batch-review",
            "path": "/home/me/wt/unit-batch-review", "host": "build-1",
            "session": "herdr:w1:p2@/tmp/herdr/herdr.sock" },
  "submitted": "2026-10-11T09:16:40Z",
  "after": [],
  "priority": 0,
  "attempt": 1,
  "state": "failed",
  "tested": { "combination": "e20a6f9b48", "on": "c41e8f2d90", "submissions": [12, 13] },
  "result": "failed",
  "failed": [ { "task": "tests:t-batch-cards", "hash": "5c0e97aa21",
               "log": ".git/sluice/.logs/bolt/concurrent-units/c7/tests:t-batch-cards.log" } ],
  "delivered": "2026-10-11T09:21:55Z",
  "delivered_to": "herdr:w1:p2@/tmp/herdr/herdr.sock"
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

`from.host` is the host the submit ran on, by the name its peers know
it by, and `from.session` is the session it ran in. Under herdr that's
`herdr:<pane>@<socket>`: the pane and the herdr server it's on. A
submit run in no herdr pane has no `session`. The round that decides
the outcome types a notice into that pane, one line to the agent there,
when the round runs on the same host:

```text
[sluice] #13 failed on bolt/concurrent-units: tests:t-batch-cards. sluice wait 13 shows why.
```

`delivered` is when the outcome was delivered, and `delivered_to` is
who it was delivered to. An outcome is delivered only when something
put it before the session that submitted it. herdr took the notice into
the pane, and `delivered_to` is the session. The pull request was
posted on, and it's `pull/41`. `sluice inbox` printed it, or a wait on
the submission reported it, and it's the session's herdr pane, as
`herdr:<pane>@<socket>`, or the worktree's path where the session runs
in no herdr pane.

Until then the outcome is pending: the submission stands decided with
neither field. The log has one `pending` event saying why it wasn't
delivered: the submit ran in no pane or on another host, or herdr
refused the notice, in herdr's words. No round pushes it again;
`sluice inbox` or a wait delivers it.

A submission written by an older sluice may say `told` and `told_to`
for the same two fields, or `told` alone; sluice reads them as
`delivered` and `delivered_to`, and writes the new names. An outcome
the round has yet to push waits in `branch.json`'s `outbox`, each item
holding it under the key `told`, which an older sluice on another host
still reads.

A passed submission waiting under its branch's [`hold`](/sluice/reference/configuration/#hold)
has `"held": { "by": [12, 13, 14], "until": "2026-10-11T09:26:40Z" }`:
the longer combination it waits for, by its submissions, and when the
hold runs out. Nothing reads it to decide; the round decides from the
branch's record, where each combination that passed has `passed_at`.

A submission that has landed or been withdrawn moves, unchanged, from
`submissions/` to `.finished/` in the same write that records its outcome,
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
{"schema":"sluice/log@1","event":"ran","combination":"9ab3d07c11","task":"tests:t-after","hash":"4d1e22b0c9","outcome":"pass","host":"build-1"}
{"schema":"sluice/log@1","event":"replayed","combination":"9ab3d07c11","task":"tests:t-amend","hash":"77aa01f3de","outcome":"pass"}
{"schema":"sluice/log@1","event":"ran","combination":"e20a6f9b48","task":"tests:t-batch-cards","hash":"5c0e97aa21","outcome":"fail","host":"build-1"}
{"schema":"sluice/log@1","event":"blamed","id":13,"combination":"e20a6f9b48"}
{"schema":"sluice/log@1","event":"landing","ids":[12],"from":"c41e8f2d90","to":"b7d2e94f03"}
{"schema":"sluice/log@1","event":"landed","ids":[12],"to":"b7d2e94f03"}
```

`landing` is written before the branch moves and `landed` after, so a
round that crashes between the two is finished correctly by the next
one.

`delivered` is logged when a submission's outcome is delivered.
`pending` is logged once when it isn't, with `why` in the words of whatever didn't
deliver it, and no round tries again:

```json
{"schema":"sluice/log@1","event":"delivered","id":12,"notification":"12:landed"}
{"schema":"sluice/log@1","event":"pending","id":13,"notification":"13:failed","why":"agent is blocked"}
```

`held` is logged once for each revision a held branch is read at, with
the landing it lacks, and `accepted` when the operator accepts that
revision with `sluice branch accept`:

```json
{"schema":"sluice/log@1","event":"held","read":"10e4791a2b","lacks":"c43609b8e1"}
{"schema":"sluice/log@1","event":"accepted","revision":"10e4791a2b","lacked":"c43609b8e1"}
```

`copy-behind` is logged when a round can't set this host's own branch
of a branch kept on a remote to its landing. It's logged once for each
revision that branch holds, landing and reason. `copy-levelled` is
logged when a round sets one that was behind. `copy` is the revision
this host's branch holds:

```json
{"schema":"sluice/log@1","event":"copy-behind","host":"build-2","copy":"10e4791a2b","lacks":"c43609b8e1","why":"git holds the lock on refs/heads/bolt/x"}
{"schema":"sluice/log@1","event":"copy-levelled","host":"build-2","copy":"c43609b8e1"}
```

`read-refused` is logged when a round can't read the branch, or its
state, from the branch's remote. It names the host, the remote, the ref
the round asked for and what git said, whole. It's logged once for each
ref and words while that host's rounds are refused, never once a round,
and each host logs its own. The first round on that host that reads the
remote again logs `read-again`. `since` is when the first round was
refused, `stood` how many seconds since then, and `rounds` how many of
that host's rounds it refused:

```json
{"schema":"sluice/log@1","event":"read-refused","host":"build-1","remote":"origin","asked":"refs/heads/main","said":"remote: Repository not found.\nfatal: repository 'https://github.com/acme/app.git/' not found"}
{"schema":"sluice/log@1","event":"read-again","host":"build-1","remote":"origin","since":"2026-10-26T09:16:40Z","stood":1800,"rounds":12}
```

A round that can't read the branch or its state forms, starts and lands
nothing, and says why to whoever ran it, as before. When the state is on
the remote that refused it, the round can't write these events either.
The host keeps them, and its first write to the state after writes them,
each with the time it happened. So an event written late can read
earlier than the one before it.

`released` is logged when a combination's checkout is released. One
freed at the operator's `--accept-failure` carries `accepted`, what had
failed of its release:

```json
{"schema":"sluice/log@1","event":"released","place":"sluice/try/main/48","accepted":"the release command of /home/me/repo/.git/sluice/.places/sluice+2Ftry+2Fmain+2F48 ended exit status: 2: usage: scripts/preview up"}
```

`release_failed` is logged for each try of a release that failed, with
what failed and how many tries so far; `held` is true on the try after
which it is tried no more. `gone` is logged when a run or round frees a
checkout whose directory no longer exists, once it has run the release
command its last run stored, once, from its scratch directory. `ran`
says whether its runs stored one to run, and `failed` what it printed
when it failed; the checkout is freed either way and never tried again.
For a worktree it also names the worktree:

```json
{"schema":"sluice/log@1","event":"release_failed","place":"sluice/try/main/48","failed":"…: usage: scripts/preview up","tries":4,"held":true}
{"schema":"sluice/log@1","event":"gone","place":"session//home/me/wt/unit-c","worktree":"/home/me/wt/unit-c","ran":true}
```

While a checkout's release is failing, `branch.json` keeps it in
`failed_releases`: its `place`, what `failed`, its `tries`, when it was
last tried (`at`), `stopped` once it is held, and on a branch whose
state is on its remote the `host` that made it. It leaves the list once
it is released, freed or found gone. A worktree's or a preview's own
release that failed is kept on its record in `.places/` or `.previews/`
as `release_failed`, since those are the host's alone.

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
