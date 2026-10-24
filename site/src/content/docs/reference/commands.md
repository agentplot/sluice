---
title: Commands
description: The sluice command line, and wt submit.
sidebar:
  order: 2
---

| Command | |
|---|---|
| `sluice` | Where this worktree stands: its submission, each branch's queue, its place, and what to run next. `--json` for a status line. |
| `sluice submit <branch>` | Queue this worktree's head and wait for the outcome; to a branch with pull requests, open one for it instead. `--no-wait` returns once queued or opened; `--status` reports where the last submission from this worktree stands; `--check` only reports whether it would conflict, and `--check --head <branch>` whether a branch no worktree holds would; `--after 2,5` lands it after those; `--json` for scripts. |
| `sluice wait <id>` | Wait for a submission's outcome, running sluice's rounds meanwhile, as a waiting submit does; `sluice wait <branch>` for this worktree's last submission to it. `--timeout 9m` returns after that long with the submission as it stands, and exits `5`; `--json` prints its record. |
| `sluice status <branch>` | The queue: each submission still in it, its agent, its worktree and its state, with how many are in it. Each combination under test says how far it has got. `--all` for every submission the branch has had. It reads the branch where its records last saw it, as `sluice overview` does, and never fetches the branch. A branch whose state is on its remote is read from there; when the remote can't be reached, it shows the state last fetched and says so. A branch with no records and no such branch exits 1. It lists the checkouts held because their release failed, each with what failed and how many times it was tried. `sluice status <id>` for one. `sluice status` alone lists every worktree sluice holds ports or runs for. `--json` for tools. |
| `sluice overview` | Every branch's queue at a glance: the combinations under test, the submissions still queued, the latest landings, blames and conflicts, and the worktrees and previews held. `--json` for tools; `--json --watch` prints it again each time it changes. |
| `sluice run` | Work every branch's queue until stopped. On a host whose config names a [`[signing]`](/sluice/reference/configuration/#signing) key, it also takes runs of the branches that trust its signer. |
| `sluice step <branch>` | One round of a branch's work, then exit: for cron, CI or a hook. Says `led` when another host keeps the branch, and `held` when the revision it reads lacks the branch's last landing ([below](#sluice-branch)). `--json` for scripts. |
| `sluice amend <id>` | Reorder a queued submission (`--after`, `--priority`) until it is under test. `--json` for scripts. |
| `sluice withdraw <id>` | Take a submission out of the queue. One already withdrawn is said so, and exits `0`; one landed is refused. `--json` for scripts. |
| `sluice protect` | Install the branch-protection hook; `--remove` takes it out. Warns of a worktree that has checked out a protected branch. `--json` for scripts. |
| `sluice exec <branch>` | Run the tests this worktree's head affects against the branch, in the worktree, under the host's bound, with the worktree's ports and the branch's limits. `-- <command>` runs a command instead. `--json` for scripts. |
| `sluice release` | Release this worktree: run the policy's release command, end what its runs left running, and free its ports. The worktree stays. `--accept-failure` frees its ports and ends its runs without running the release command. `--json` for scripts. |
| `sluice preview <branch> <revision>` | Bring up a revision the branch has landed in a checkout of its own, at addresses a person opens, until it is released. `--wait` returns once its command has ended; `--json` for scripts. |
| `sluice previews` | Every preview on the host, with its addresses and state. `--json` for scripts. |
| `sluice release <name>` | Release a preview, by its name or its revision: its release command runs, its checkout goes, its ports and names are free. Given a checkout held because its release failed, `sluice/try/<branch>/<n>`, run its release once more. `--accept-failure` frees either without running the release command. `--json` for scripts. |
| `sluice branch create <branch> --from <branch>` | Make a branch sluice lands on, such as a bolt, at another such branch's current revision: on its remote when its policy names one. `--json` for scripts. |
| `sluice branch delete <branch>` | Delete it once its work has landed into `main`, or the branch `--into` names, and nothing is queued on it. `--json` for scripts. |
| `sluice branch accept <branch> <revision>` | Accept the revision a held branch is read at, which lacks its last landing, as the one it stands on: the next round forms the queue on it. The operator's to do, never an agent's. `--json` for scripts. |
| `sluice forge check [<branch>]` | Whether sluice's own identity on GitHub is set up so only sluice may move the branch there. `--json` for scripts. |

## Forecasting a conflict

`sluice submit <branch> --check` forms the combination the worktree's
head would land in were it submitted now, behind every submission
queued on the branch, and says whether it would conflict. It queues
nothing and changes nothing. A bolt's branch, which no worktree holds,
is forecast with `--head`; the submissions made from that branch are
left out, as submitting it again would withdraw them:

```console
$ sluice submit main --check --head bolt/runner
would conflict: docs/architecture/builds.md with #217 (wldn-design, design/runner-credential)
```

It exits `0` with no conflict, `2` with one or on a refusal. `--json`
prints `conflict`, null or `{paths, with}`, and what the forecast was
formed from: `head`, the revision forecast; `onto`, the branch's
revision; and `ahead`, the submissions formed ahead of it, in order. A
tool forecasts again only when one of them has moved. A forecast takes
the combinations the queue has already formed on the branch's revision,
so it costs one merge behind them.

```json
{"conflict":{"paths":["docs/architecture/builds.md"],"with":[{"id":217,"from":{"agent":"wldn-design","worktree":"design/runner-credential"}}]},"head":"4f0c2a1…","onto":"9b7e3d0…","ahead":[217]}
```

## `sluice exec`

An agent runs its tests while it works with `sluice exec`, in its own
worktree. The run counts against the host's bound for as long as it runs,
waiting while the bound is full, and never uses the part
`[runs] reserved` keeps for submissions. It gets the worktree's ports,
the same on every run, runs under the branch's executor, memory,
network and time limit, and ends with its process group. The tests are
the ones the head affects against the branch, read from the branch's own
policy, never the worktree's.

```console
$ sluice exec bolt/concurrent-units
running the checks 8d41b6c affects against bolt/concurrent-units (c41e8f2)
ports: SLUICE_PORT_DB=20001 SLUICE_PORT_WEB=20000
…what the tests print…
passed: 2 tasks
recorded 2 results under their keys for 8d41b6c
```

When the worktree holds exactly its head, with nothing modified, staged
or untracked, each test's result is recorded under its key as an open
run. A run that passed also saves the caches `[[checkout.cache]]`
declares, as a passed combination's checkout does. The worktree keeps
its own copy. A later submission's combination with the same key reads
the result on a branch that trusts open runs, and replays the test when
the policy declares the cache, the same condition as between
combinations. A worktree holding anything more runs the same tests,
records nothing, and says `not recorded: worktree is not clean`.

Before it runs, exec forecasts the head against the branch, as `sluice
submit --check` does, and says when it would conflict: `would conflict:
<files> with <what>`. The run goes on either way, its exit is the
run's, and nothing of the forecast is recorded. `--json` carries it as
`forecast`, in `--check --json`'s shape, or null for a detached
worktree.

A test that could not run is listed as `could not run: <test>: <why>`
and has no result, so nothing is recorded under its key. A test could
not run when its program could not be started, when it exits with a code
`[tests] not_run` names, or when the host failed `[tests] ready` after a
run in which it did not pass. A host that fails `ready` before the run
is refused it, and the command's output says why.

`sluice exec <branch> -- <command>` runs any command the same way, with
`SLUICE_CHECKOUT`, `SLUICE_TMP` and the ports in its environment, and
exits with the command's exit code. It records nothing, and is held to
neither `ready` nor `not_run`.

Run suites through `sluice exec` so the agents on one host take turns
under its bound rather than all running at once. A quick, narrow call,
such as a type check or one unit test, can run directly.

While it runs, `sluice overview` shows it as a check on the worktree,
never as a submission: how many of its tasks have ended under the moon
runner, or the command it runs, and when it started. Each check takes a
run of the host's bound, so when the bound is full and a submission
waits, the branch's counts line says how many runs the checks hold:
`testing 2 at once (up to 3) · 1 slot to an agent's checks`.

What a run starts and hands to another process, a container above all,
stays up between runs, so the next run finds it. `sluice release` ends
it. `wt submit` releases the worktree before worktrunk removes it; on
plain git, run `sluice release` before `git worktree remove`.

## Worktrees sluice holds

A worktree keeps its ports and what its runs left running until it is
released, however long since its last run: sluice cannot tell an agent
that is thinking from one that is gone. `sluice status` with no argument
lists every worktree it holds in the repository, with its branch, its
last run, how that run ended and its ports, so a person or a crew can
release one with `sluice release --worktree <path>`. `--json` gives the
same as one document, a `worktrees` list with `worktree`, `branch`,
`last_run`, `ports`, `removed` and, once a run there has ended,
`last_result`.

```console
$ sluice status
worktrees held until released:
  /home/me/wt/unit-a  bolt/concurrent-units  last run 2026-10-14T09:12:40Z  last result passed on 1a2b3c4, recorded  SLUICE_PORT_DB=20001 SLUICE_PORT_WEB=20000
  /home/me/wt/unit-b  bolt/concurrent-units  last run 2026-10-11T16:03:05Z  last result command failed on 5d6e7f8  SLUICE_PORT_DB=20003 SLUICE_PORT_WEB=20002
release one with: sluice release --worktree <worktree>
```

`last_result` says how the last `sluice exec` in the worktree ended:

| field | |
|---|---|
| `at` | when it ended; `last_run` is when it started. A run under way, or one whose `sluice exec` was killed, has no `last_result` |
| `head` | the worktree's commit it ran on, in full |
| `kind` | `checks`, or `command` for `sluice exec <branch> -- <command>` |
| `outcome` | `pass`, `fail`, `timed_out`, `lost` (the run stopped without saying how it ended) or `stopped` (ended by its caller) |
| `recorded` | its results are kept for their keys, so a submission of `head` reuses them. Never true for a command, or for a run with files changed and not committed. Never true when a commit or an edit was made while it ran: it names the commit it started on. Never true for a run that timed out, was lost or was stopped |
| `tasks` | for a run of the checks that reported, each task as it ended: `target` and `state`, `passed`, `failed` or `replayed` |
| `reused` | for a run of the checks that reported: true when `recorded` is, every check passed, and the branch's gate trusts the executor it ran under. A submission of `head` then replays its results rather than running them again |

The next run replaces it, and `sluice release` removes it with the rest.
A worktree's head is ready to submit when `kind` is `checks`, `outcome`
is `pass`, `recorded` is true and `head` is the worktree's head now. Its
files may have changed since; that is for the reader to check, since
sluice reads nothing from the worktree to list it.

A worktree that has checked out a branch sluice lands on is named
too, whether sluice holds anything for it or not. sluice moves that
branch under it and leaves its files as they were, so `git status`
there shows each landing undone. `sluice status`, `sluice overview` and
`sluice protect` warn of it, and in `--json` its entry carries
`holds_queued_branch`, the branch. Nothing else changes: the landing
goes through, and the worktree is left as it is. Look at the branch from
a detached worktree instead; see
[worktrees beside sluice](/sluice/guides/worktrees/).

```console
$ sluice status
no worktree holds ports or runs here
warning: /home/me/repo/main has main checked out: sluice moves main under it and leaves its files behind; look at it from a worktree switched with git switch --detach
```

A worktree removed without a release is no agent's. The next `sluice
exec` on the host, or the next round, releases it from what sluice
holds for it. The release command its last run stored runs once, in its
scratch directory, with its ports and the environment its runs had.
Neither the toolchain nor the policy is read again, since the directory
they would run in is gone ([ADR 0047](/sluice/decisions/0047/)).
Whatever that command does, the worktree's runs end, its ports are free,
and it is never tried again. A release that tears a stack down by name
or port still does. One that reads files from the worktree finds none
and fails, and what it would have removed is left for a person to find.
That command says so first. The branch's state logs it `gone`, with how
its release went, once:

```console
$ sluice exec bolt/concurrent-units
freed /home/me/wt/unit-c: it no longer exists; its release ran once from its scratch directory; its runs ended, its ports free
running the checks 8d41b6c affects against bolt/concurrent-units (c41e8f2)
…
```

When its release fails, sluice says `its release failed once from its
scratch directory:` and what it printed; when its runs stored none, `it
had no release to run`.

Until then, `sluice status` lists it as removed.

A worktree's `sluice release` whose release command fails exits `1`
with what the command printed. The worktree keeps its ports, and
`sluice status` lists it with what failed until a `sluice release`
succeeds. No round releases it: `sluice release --accept-failure` frees
its ports and ends its runs without the release command, and leaves the
worktree. In `--json` its entry carries `release_failed`: `failed`,
`tries`, `at` and `stopped`.

## Checkouts held because their release failed

When a combination's checkout can't be released, its release command
failing, the checkout stays, held. The branch's state names it with what
failed and how many times it was tried. The next rounds try it again, one
try a round, as many times as the branch's
[`release_retries`](/sluice/reference/configuration/#release_retries)
allows, then stop and say it is held. A held checkout never holds its
branch: the same rounds land what passed, tell outcomes and test the
next combinations in other checkouts. A waiting submit, `sluice step` and
`sluice run` say each try that failed, and the hold once:

```console
could not release sluice/try/main/48 (try 2 of 4): the release command of /home/me/repo/.git/sluice/.places/sluice+2Ftry+2Fmain+2F48 ended exit status: 2: usage: scripts/preview up
sluice/try/main/48 is held: its release failed 4 times: … `sluice release sluice/try/main/48` runs it again; `sluice release sluice/try/main/48 --accept-failure` frees it without
```

`sluice status <branch>` lists the branch's held checkouts, and
`--json` as `held_checkouts`, each with `place`, `failed`, `tries`, `at`,
`stopped` (tried no more) and, on a branch whose state is on its
remote, the `host` that made it:

```console
$ sluice status main
main at b7d2e94 · 0 in the queue · testing 0 at once (up to 2)
held checkouts, whose release failed:
  sluice/try/main/48  4 tries, held, last at 2026-10-23T09:14:02Z
    | the release command of /home/me/repo/.git/sluice/.places/sluice+2Ftry+2Fmain+2F48 ended exit status: 2: usage: scripts/preview up
release one with: sluice release <checkout>, or without its release command: sluice release <checkout> --accept-failure
```

Once you have fixed what the release needed, `sluice release
sluice/try/main/48` runs it once more, whatever its tries; it exits `1`
and stays held if it fails again. `sluice release sluice/try/main/48
--accept-failure` gives up on it: its runs end, the checkout sluice made
is removed and its ports are free, and the release command is not run, so
whatever it would have removed is left. A checkout is released on the
host that made it. A held checkout doesn't keep its host from claiming:
the host claims runs of its branch as any other does, in other
checkouts. The held one keeps its ports and disk until it is released.

A checkout whose directory someone deleted, held or not, has the release
its run stored run once from its scratch directory by the next round, is
freed whatever that did, and is logged `gone`.

## `sluice overview`

`sluice overview` shows every branch of the repository that has a
queue, read from its state alone: it starts nothing, releases nothing
and writes nothing, and it reads with no sluice running.

```console
$ sluice overview
bolt/concurrent-units at c41e8f2 · 2 in the queue · testing 1 at once (up to 4)
  testing bolt/concurrent-units + #13 + #14 · 3/5 tasks · running tests:t-cards 2m10s · 6m02s of 15m
  2026-10-16T09:15:02Z  landed #12 as c41e8f2
  2026-10-16T09:11:40Z  blamed #11 on 9a03c1e
checking /home/me/wt/unit-a → bolt/concurrent-units 3/8 since 2026-10-16T09:17:02Z
host: 2 of 4 runs · 1 queue · 1 agent
2 worktrees held · 0 previews
```

`--json` prints one document, `sluice/overview@1`, for a pane, a status
line or a script. Fields are only ever added to it.

| field | |
|---|---|
| `schema`, `host`, `repository`, `at` | the schema, the host's name, the repository's git directory, and when it was read |
| `branches[]` | each branch with a queue: `branch`, `at` (its revision as its state last saw it), `version` (moves on every write), `window`, `parallel` and `depth` (submissions still queued, waiting, testing, passed or landing). They come in the order the default branch's policy names them |
| `branches[].candidates[]` | each combination: `id`, `submissions`, `revision`, `state`, `started`, `limit` (seconds), `targets` (the tests the branch requires), `executor`, `ran`, `replayed`, and `failed`, each failed test with where its output is kept |
| `branches[].candidates[].tasks[]` | while a combination is under test with the moon runner, each of its tasks: `target`, `state` (`waiting`, `running`, `passed`, `failed` or `replayed`), and, once known, `started`, `ended` and `exit`; left out when nothing has been said of them |
| `branches[].submissions[]` | each submission still queued, as `sluice status <id> --json` prints it |
| `branches[].recent[]` | the branch's last ten `landed`, `blamed` and `conflict` lines of its log, oldest first, exactly as written |
| `branches[].held` | while the branch is held ([`sluice branch`](#sluice-branch)): `read`, the revision it is read at, `lacks`, the landing that revision lacks, and `since`; left out otherwise |
| `sessions[]` | the worktrees held, each with how its last run ended (`last_result`), and those that have checked out a branch sluice lands on (`holds_queued_branch`), as `sluice status --json` lists them |
| `sessions[].run` | while a `sluice exec` runs in the worktree: `started`, `command` for a run of a command, and `tasks`, as a combination's, while the moon runner says them |
| `previews[]` | the previews on the host, as `sluice previews --json` lists them |
| `room` | the host's runs: `bound`, `queue` (this repository's combinations under test), `sessions` (every `sluice exec` and preview command running on the host) and `reserve` (the part of the bound `[runs] reserved` keeps from `sluice exec`) |

A combination's `tasks` say how far its tests have got and decide
nothing: its result is still read from the finished run. Each round
updates them, about every 3 seconds while a submission waits, from
what moon writes as each task ends, and they go once the combination
is no longer under test. They are kept on the host that runs the
combination, so another host shows none. The `command` runner has one
test, the run itself, and gives none.

`sluice status` and a waiting submit say a combination's progress as
the text form does. It counts the tasks that have ended and names up to
three running ones, longest first, with how long each has run. Then it
gives the combination's time against its limit. A running task's time
starts when moon printed that it had started, which the run notes as
its output arrives. Where no tasks are kept, the text gives the
combination's time alone.

A check's `run` comes from the host's own state, written by its `sluice
exec` as each task ends, and goes when the run ends; its `last_result`
then says what it is worth. Like a combination's tasks, it decides
nothing.

The branches come in the order of the `[branches]` tables of the
policy on the default branch, the branch the repository's HEAD names.
A pattern's branches stand together where the pattern does, in name
order, and branches no table names come last. Each branch's gate is
still its own policy's.

`--json --watch` prints the document as one line at once, and again
each time anything in it but `at` changes, reading about once a second
until stopped. A reader finds what happened since its last line by
comparing `recent`. A branch's revision is the one its state last saw,
never fetched, so a branch on a remote costs it nothing to watch.

A submission's `from.path` names the worktree it came from, so a pane
or a session can find its own submissions in the document.

## `sluice` with no command

`sluice` alone shows where this worktree stands, cut from the overview
to the worktree by the path it submits from, or, for a pull request, by
the branch it was opened from: its latest submission
still queued, or else the latest the branches' recent lines name; each
branch's counts; its place; and the commands that follow. Like the
overview it reads the state alone and writes nothing, so a status line
may run it every few seconds.

```console
$ sluice
worktree unit/cards (crw-1-unit-2) at /home/me/shop.unit-cards
#14 to bolt/concurrent-units · testing
bolt/concurrent-units: 2 in the queue · testing 1 at once (up to 4)
place: SLUICE_PORT_WEB=20000 · last run 2026-10-16T09:16:44Z
next: sluice status 14
next: sluice withdraw 14 --as crw-1-unit-2
```

`--json` prints one line, `sluice/home@1`, whose fields are only ever
added to: `worktree` (`path`, `branch`, `agent`), `submission` (`id`,
`branch`, `state`, `failed` with each test's `log`, `landed_as`),
`branches[]` (`branch`, `depth`, `testing`, `parallel`, the overview's
counts), `place` (`ports`, `last_run`) and `next`. Outside a worktree
sluice can submit from, `worktree`, `submission` and `place` are null.
A submission from before `from.path` was recorded names no worktree and
is not shown. A pull request still waiting for review or checks has no
submission yet; `sluice submit <branch> --status` follows it.

## `sluice preview`

A preview is a running copy of a revision a branch has landed, for a
person to try: the repository's own stack, brought up by the policy's
[`[preview]` command](/sluice/reference/configuration/#preview-a-running-copy-of-a-landed-revision)
in a checkout of its own, with a port and a name for each of
`[checkout] ports`. It is held until something asks for its release. The
[previews guide](/sluice/guides/previews/) says what it is for and how a
crew drives it.

```console
$ sluice preview bolt/concurrent-units 2eb676c
preview/2eb676c8e45d of bolt/concurrent-units at 2eb676c: starting
  api  http://localhost:20001/docs
  web  https://web-2eb676c--bolt-concurrent-units--shop.localhost
  ports: SLUICE_PORT_API=20001 SLUICE_PORT_WEB=20000
```

Only a revision in the branch's history is previewed: the head of a
submission still queued, or a combination under test, is refused. The
addresses are printed as soon as the command starts, filled in from the
policy's templates. The command counts against the host's bound while it
runs, waiting while the bound is full and never using the part
`[runs] reserved` keeps for submissions; once it has ended, the preview
counts against nothing. Ctrl-C while it waits leaves the preview to be
started by the next `sluice preview` of that revision.

Asking again for a revision that has a preview gives that preview, its
names, ports and state unchanged, and starts nothing, so a caller that
cannot tell whether it asked can ask. A revision has one preview on the
host, whichever branch it was asked for through.

A preview is `waiting` for room, `starting` while its command runs,
`up` once the command exited 0, or `failed`. A failed preview is
released at once, with what its command printed kept, and stays listed
until it is released. `--wait` returns once the command has ended and
exits 1 for a failure. `--json` prints one document:

```console
$ sluice preview bolt/concurrent-units 2eb676c --wait --json
{"name":"preview/2eb676c8e45d","branch":"bolt/concurrent-units","revision":"2eb676c8e45d…","state":"up","made":"2026-10-14T09:12:40Z","checkout":"/home/me/shop/.git/sluice/.previews/preview+2F2eb676c8e45d","addresses":{"api":"http://localhost:20001/docs","web":"https://web-2eb676c--bolt-concurrent-units--shop.localhost"},"names":{"SLUICE_NAME_API":"api-2eb676c--bolt-concurrent-units--shop","SLUICE_NAME_WEB":"web-2eb676c--bolt-concurrent-units--shop"},"ports":{"SLUICE_PORT_API":"20001","SLUICE_PORT_WEB":"20000"},"exit":0}
```

A failed preview's document carries `output`, the end of what its
command printed. Nothing of a preview is recorded as a test result, and
no submission waits on it.

`sluice previews` lists the host's previews the same way, as a list
with `--json`. It first releases any preview whose command failed, and
any whose checkout someone removed by hand. For the latter, the release
its command's run stored runs once in its scratch directory, with its
ports and names; the preview is freed whatever that did, and never
tried again. `sluice release <preview>` releases one: its command's run
ends, the release command runs in the checkout, the checkout is removed
and its ports and names are free. No round, step or landing ever
releases a preview that is up. A preview whose release command fails is
tried again as a combination's checkout is, then held; `sluice previews`
shows what failed, and `--json` carries it as `release_failed`.

## `wt submit`

`wt-submit` is installed beside `sluice`, and
[worktrunk](https://github.com/max-sixty/worktrunk) runs any `wt-<name>`
on the `PATH` as `wt <name>`. It is a driver: it calls `sluice submit`
for the worktree it is run in, waits, and on landing removes the
worktree and its branch as `wt merge` did, following worktrunk's
`[merge] remove`. `--no-remove` keeps them. [sluice with
worktrunk](/sluice/guides/worktrunk/) has the move from `wt merge`.

```console
$ wt submit bolt/concurrent-units
submission 12 to bolt/concurrent-units · nothing queued ahead
testing bolt/concurrent-units + #12 · 4s of 15m
testing bolt/concurrent-units + #12 · 31/33 tasks · running tests:t-cards 1m10s, tests:e2e 40s · 1m52s of 15m
testing bolt/concurrent-units + #12 · 32/33 tasks · running tests:t-cards 1m31s · 2m13s of 15m
testing bolt/concurrent-units + #12 · 2 tasks to run, 31 replayed
passed · landed as b7d2e94
```

While it waits, it says its combination's progress again each time the
count of ended tasks or the running tasks change, as `sluice overview`
shows them.

Submitting again from the same worktree withdraws that worktree's earlier
submission.

### To a branch with pull requests

When the branch's policy has a
[`pull_requests` table](/sluice/reference/configuration/#branchespatternpull_requests),
`sluice submit` and `wt submit` don't queue the worktree's head. They
push the worktree's branch to the branch's remote and open a pull
request from it, ready for review, or update the one already open from
that branch. The pull request waits for its conditions like any other,
and sluice submits it once they hold:

```console
$ wt submit main
pushed unit/batch-review · opened #41: Batch review cards
waiting: needs 1 review, qa/approved
  forecast: merges without conflict behind #38, #40
queued behind #38, #40
testing main + #38 + #40 + #41
testing main + #38 + #40 + #41 · 2 tasks to run, 0 replayed
passed · landed as b7d2e94
```

A waiting submit prints each state the pull request reaches, runs
sluice's rounds while it waits, and exits as any submit does. On a pull
request, `#41` is the pull request's number. `--no-wait` returns once
the pull request is open, and `wt submit main --status` reports its
state later. `wt submit` removes the worktree only when the pull request
lands; sent back, the worktree stays for the fix.

A pull request a submit opened is the agent's. When its submission
leaves the queue without landing, blamed, in conflict, withdrawn, or on
commits no submit pushed, sluice marks it a draft, and no round takes it
again in the agent's name. The next submit marks it ready for review at
the commits it pushes, the same ones included, and sluice takes them
like any others. After a run the time limit ended, with no new commits:

```console
$ wt submit main
pushed unit/batch-review · updated #41: Batch review cards · marked ready for review
queued behind #38
testing main + #38 + #41
testing main + #38 + #41 · 2 tasks to run, 0 replayed
passed · landed as c41f0a2
```

A submit of new commits while the pull request is still in the queue
withdraws its submission there, and the new commits wait for their
conditions; the submit says `withdrew its submission 12`.

## `sluice branch`

sluice makes and deletes the branches it lands on, so nothing else has
to write them. Each takes the branch's lock, so no round acts on it
meanwhile, and each is one move from what it read, refused when the
branch was elsewhere.

`sluice branch create <branch> --from <other>` makes the branch at the
other branch's current revision, fetched from its remote when it has
one. Both must be branches sluice lands on: the policy at that revision
gives each a gate, and the new branch lives where that policy's
`remote` says. A branch already there is refused, and so is a name
whose records still queue submissions.

`sluice branch delete <branch>` deletes it, only when:

- no submission on it is still queued or under test;
- no landing on it is unfinished;
- its revision is in `main`'s history, or the history of the branch
  `--into` names.

The deletion is leased on the revision it read, so a landing since,
from any host, refuses it. Its records stay, and `sluice status
<branch>` still reads them.

```console
$ sluice branch delete bolt/payments
refused: #12 still queued on bolt/payments; wait for them to land, or withdraw them
```

A branch is held when the revision a round reads it at lacks the last
landing sluice recorded there. A hand that moved it back does that, and
so does a branch that landed in this host's own refs gaining a `remote`
whose copy is older. While it is held, no combination is formed and
nothing lands; tests already running go on, and their results are read
once the hold ends. `sluice step`, `sluice run`, `sluice status
<branch>`, `sluice overview` and a waiting submit or `sluice wait` say
so, naming both revisions:

```console
$ sluice step bolt/payments
held: bolt/payments reads 10e4791, which lacks the landing c43609b; put the landing back on bolt/payments, or run sluice branch accept bolt/payments 10e4791
```

The hold ends by itself on the first round that reads the branch at a
revision holding the landing again, as when the landing is pushed back
to the remote. A commit on top of the landing never holds the branch:
the queue is formed on it as it stands.

`sluice branch accept <branch> <revision>` is the other way out: the
operator accepts the revision read as the one the branch stands on,
naming it as `held` names it, whole or by seven characters or more. It
is logged, every later read must contain it, and the next round forms
the queue on it. It moves no branch, and is refused unless the branch
is held at that revision and still read there. A branch sluice makes
stands on the revision it was made at, so a branch deleted and made
again owes nothing to the landings of the one deleted.

```console
$ sluice branch accept bolt/payments 10e4791
accepted bolt/payments at 10e4791a2b5c6d7e8f90a1b2c3d4e5f6a7b8c9d0, which lacks the landing c43609b8e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6; the next round forms the queue on it
```

All three exit `0` when done, `1` when a branch they name doesn't exist,
and `2` on a refusal. A round that holds a branch is no failure:
`sluice step` exits `0`, saying `held`.

## `sluice forge check`

Checks what the host's
[`[forge]`](/sluice/reference/configuration/#forge) table needs for a
branch whose `remote` is on GitHub. The key reads and the app makes a
token for the repository. The app may write the repository's contents
and its workflows.
Every ruleset on the branch lets the app through and nobody else. The branch is the one checked out where it
runs unless it is named. It says each thing it checked, and exits `1`
when any fails, naming which.

GitHub shows who may bypass a ruleset only to the ruleset's admins, so
the check reads the rulesets with your own token: `GH_TOKEN`,
`GITHUB_TOKEN`, or the one `gh` is signed in with. Everything else it
reads as the app.

```console
$ sluice forge check main
ok      [forge] key /home/me/.config/sluice/app.pem reads
ok      app 123456 makes a token for acme/widgets
ok      the app may write acme/widgets's contents
ok      the app may write acme/widgets's workflows
failed  ruleset "main" on main lets more than the app through: RepositoryRole 5
```

[Protecting main on GitHub](/sluice/guides/github-app/) sets
it up.

## `sluice set-aside`

sluice records nothing under a test's key for four failures it tells
apart on its own, so none of them needs setting aside:

- a run whose program never started;
- a host that failed `[tests] ready`;
- an exit `[tests] not_run` names;
- a test the time limit ended.

A host failure none of these catches still sits under the test's
key, and a later pass makes the key read as flaky. On a branch whose
`flaky` is `"fail"`, the same inputs then never pass. The operator who
finds the cause on the host sets the fail aside:

```console
$ sluice set-aside cli:tools --at 2026-10-17T13:05:32Z --reason "OrbStack was down"
set aside cli:tools 7272335f33f8 recorded at 2026-10-17T13:05:32Z on main 79b832d: OrbStack was down
no gate reads it from now; its line stays in the results
```

`--at` is when the result was recorded, as a flaky failure names it.
`--hash` gives the start of its key when two results of the test were
recorded at once. The result's line stays in `results.jsonl`. The
set-aside, with who made it and why, is kept in `set-aside.jsonl` beside
it, and no branch reads the result again. An agent never sets aside a
result; the Claude Code plugin refuses it, and tells the agent to bring
the operator what it found instead. Under `--json`, the set-aside's
record is printed.

Each set-aside marks a result sluice recorded when it should not have.
Its reason says what the host was doing, so a failure sluice could have
told apart on its own can be taught to it.

## `sluice wait`

`sluice wait <id>` waits for a submission's outcome as a waiting submit
does. Between reads of the submission's state it runs sluice's rounds,
so the queue moves while it waits, and it ends with the same outcome
and exit. A submission already decided is reported at once, and nothing
runs. `sluice wait <branch>` waits for this worktree's last submission
to that branch, the one `sluice submit <branch> --status` reports.

With `--timeout`, as `90s`, `9m` or `1h30m`, it returns once that long
has passed and exits `5`, with the submission as it stands. The
submission stays in the queue. An agent whose tool calls end before a
result comes submits without waiting, then waits in steps, for as long
as `sluice wait` exits `5`:

```console
$ sluice submit main --no-wait --json
{"id":12,"branch":"main","ahead":[]}
$ sluice wait 12 --timeout 9m --json
```

Under `--json`, it prints one document however it ends, out of time
and stopped with Ctrl-C included: the submission's record, as `sluice
status <id> --json` prints it. Read the outcome from its `state` and
the exit. A landed or withdrawn submission leaves `sluice status
<branch>`'s listing, so searching that listing for an outcome never
finds one.

Without `--json`, it prints the submission's state as it changes and,
while its combination is under test, how far it has got:

```console
$ sluice wait 12
#12 queued
testing main + #12 · 3s of 45m
testing main + #12 · 7/12 tasks · running sluice:e2e 25m03s, site:prose 40s · 26m10s of 45m
testing main + #12 · 12 tasks to run, 0 replayed
passed · landed as b7d2e94
```

To a branch with pull requests, `sluice wait <branch>` follows the
worktree's pull request as a waiting `sluice submit` does. While the
pull request is open, its document under `--json` is the one `sluice
submit --status --json` prints, `{"pull_request","branch","state"}`;
once its submission is decided, the submission's record.

## Exit codes

| exit | meaning |
|---|---|
| `0` | landed, whatever failed after the landing: what failed is on stderr, after `sluice: after the landing:` |
| `1` | failed: this submission was blamed |
| `2` | a conflict, or a refusal |
| `4` | withdrawn while it was waited on |
| `5` | `sluice wait --timeout` ran out; the submission stays queued |
| `130` | stopped with Ctrl-C; the submission stays queued |

A submission has landed once its branch holds its work, and nothing that
fails after that changes it. A waiting submit or `sluice wait` whose
round moves the branch and then fails, releasing the combination's
checkout or telling the outcome, prints the landing and exits `0`. What
failed is on stderr, on its own line:

```console
passed · landed as b7d2e94
sluice: after the landing: the release command of … ended exit status: 1: usage: scripts/preview up
```

Under `--json`, stdout holds the submission's record alone, its `state`
`landed`. When the round stopped before recording the landing, the
record is the one the next round writes.

Every command, not only submit, exits `3` when sluice itself fails (a
repository it cannot open, a record it cannot read), with the cause on
stderr. A waiting submit or `sluice wait` does so only while the
submission has not landed.

A waiting submit or `sluice wait` whose read the forge answers with a 5xx,
GitHub unable to answer just then, reads again, a pause apart, for five
minutes. Past them it exits `3` as well, saying it could not read, apart
from any outcome:

```console
could not read #77: sluice wait main
sluice: could not read #77 for 5m03s, so whether it landed is not known: GET /repos/o/r/pulls/77: 503 Service Unavailable
```

Under `--json`, stdout holds the error document, with that command in its
`next`. Whether the submission landed is not known then: only its
record says, and the command in `next` reads it again. An outcome is only
ever reported from the records. A command exits `64` when it cannot read its
command line: an unknown command, flag or value, or a missing argument. `64` is kept apart from
`2`, so a mistyped flag is never taken for a conflict. `--help` and
`--version` exit `0`.

With `--json`, a command that cannot do what it was asked still prints
JSON on stdout, one document in place of the one it would have printed:

```json
{"error":{"kind":"refused","message":"no branch bolt/none","next":["sluice branch create bolt/none --from main","sluice overview"]}}
```

`kind` is `refused` (exit `2`), `not_found` (exit `1`) or `internal`
(exit `3`, with the cause on stderr as well). `next` holds the commands
that move it on, and is empty when there are none.

The smaller commands print one line each under `--json`, and anything
else they say goes to stderr:

| Command | Document |
|---|---|
| `sluice step` | `{"branch","outcome","released","previews_released","releases"}`: `outcome` is `stepped`, `busy`, `led`, `contended` or `held`; `released` and `previews_released` name the worktrees and previews the round freed because they were gone or released because their command failed, and `releases` each combination's checkout whose release failed, was held or found gone, with `release` (`failed`, `held` or `gone`), `failed` and `tries`. A worktree or checkout found gone carries `ran`, whether its stored release ran, and `failed`, what it printed when it failed; a preview found gone carries them under `gone`. A held round adds `held`, with `read`, `lacks` and `since` |
| `sluice branch` | `{"branch","created","from"}`, `{"branch","deleted","into"}` or `{"branch","accepted","lacked"}` |
| `sluice amend` | `{"id","after","priority"}`, as the submission now stands |
| `sluice withdraw` | `{"id","state":"withdrawn"}`, with `"already":true` when it was withdrawn before |
| `sluice set-aside` | `{"schema","at","task","hash","run_at","branch","combination","reason","by"}`, the record kept |
| `sluice release` | `{"worktree","released"}`, or `{"preview","released"}` for a preview; `released` is `false` when there was nothing to release. With `--accept-failure` it adds `accepted`, and a release that failed adds `failed`. For a held checkout, `{"place","released","accepted","failed"}` |
| `sluice protect` | `{"protection":"installed","changed","held"}`: `changed` is `false` when the hook was in place already, and `held` lists each worktree that has checked out a protected branch; with `--remove`, `{"protection":"removed","changed"}` |
| `sluice forge check` | `{"branch","ok","checks"}`, each check with its `ok` and `message` |

A submit following a pull request exits `1` as well when the pull
request is closed without landing.

`sluice preview` exits `0` once its command has started, or with
`--wait` once the preview is up; `1` with `--wait` when its command
failed; `2` when the revision or the branch is refused; `130` when
stopped while it waits.

`sluice release` exits `1` when the release it ran failed, saying what
failed, and `0` otherwise, nothing to release included.

`sluice exec` exits `0` when every test passed, `1` when one failed, was
not run, could not run or timed out, `2` on a refusal, and `130` when stopped with
Ctrl-C, which ends the run. With a command, it exits with the command's
code.

## Who runs the rounds

A round is one pass of sluice's work for a branch: read the queue, start
tests if there is room, read finished tests, blame a failure, land what
passed, write the queue back. Any process may run one, and a lock lets
only one run at a time for a branch.

- **On one machine**, a waiting `sluice submit`, `wt submit` or
  `sluice wait` runs the rounds itself between reads of its own
  submission's state, so nothing else needs to be running
  ([ADR 0011](/sluice/decisions/0011/)).
  `sluice run` and `sluice step` do the same for a host when no agent is
  waiting. For a branch with pull requests, each round first reads its
  open pull requests on GitHub, and `sluice run` serves that branch
  before it has any submission.
- **On many hosts**, one host at a time keeps a branch whose state is on
  its remote: whichever host's round wrote its state last. A waiting
  submit, `sluice run` or `sluice step` on another host reads `led` and
  leaves the branch alone. Once the state has stood unchanged for that
  host's [`[peers] lease`](/sluice/reference/configuration/#peers) with
  work to do, the next round there takes the branch, and the former host
  writes nothing more for it. Each host measures the lease by its own
  clock, never by comparing clocks. A submission written to the remote
  goes on with its submitter's host switched off: any other host lands
  it.
  [Executors and systems](/sluice/guides/executors-and-systems/) sets
  one up.
