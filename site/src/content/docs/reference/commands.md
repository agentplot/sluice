---
title: Commands
description: The sluice command line, and wt submit.
sidebar:
  order: 2
---

| Command | |
|---|---|
| `sluice` | Where this worktree stands: its submission, each branch's queue, its place, and what to run next. `--json` for a status line. |
| `sluice submit <branch>` | Queue this worktree's head and wait for the outcome; to a branch with pull requests, open one for it instead. `--no-wait` returns once queued or opened; `--status` reports where the last submission from this worktree stands; `--check` only reports whether it would conflict; `--after 2,5` lands it after those; `--json` for scripts. |
| `sluice status <branch>` | The queue: each submission still in it, its agent, its worktree and its state, with how many are in it; `--all` for every submission the branch has had. It reads the branch where its records last saw it, as `sluice overview` does, and never fetches the branch. A branch whose state is on its remote is read from there; when the remote can't be reached, it shows the state last fetched and says so. A branch with no records and no such branch exits 1. `sluice status <id>` for one. `sluice status` alone lists every worktree sluice holds ports or runs for. `--json` for tools. |
| `sluice overview` | Every branch's queue at a glance: the combinations under test, the submissions still queued, the latest landings, blames and conflicts, and the worktrees and previews held. `--json` for tools; `--json --watch` prints it again each time it changes. |
| `sluice run` | Work every branch's queue until stopped. On a host whose config names a [`[signing]`](/sluice/reference/configuration/#signing) key, it also takes runs of the branches that trust its signer. |
| `sluice step <branch>` | One round of a branch's work, then exit: for cron, CI or a hook. Says `led` when another host keeps the branch. `--json` for scripts. |
| `sluice amend <id>` | Reorder a queued submission (`--after`, `--priority`) until it is under test. `--json` for scripts. |
| `sluice withdraw <id>` | Take a submission out of the queue. One already withdrawn is said so, and exits `0`; one landed is refused. `--json` for scripts. |
| `sluice protect` | Install the branch-protection hook; `--remove` takes it out. Warns of a worktree that has checked out a protected branch. `--json` for scripts. |
| `sluice exec <branch>` | Run the tests this worktree's head affects against the branch, in the worktree, under the host's bound, with the worktree's ports and the branch's limits. `-- <command>` runs a command instead. `--json` for scripts. |
| `sluice release` | Release this worktree: run the policy's release command, end what its runs left running, and free its ports. The worktree stays. `--json` for scripts. |
| `sluice preview <branch> <revision>` | Bring up a revision the branch has landed in a checkout of its own, at addresses a person opens, until it is released. `--wait` returns once its command has ended; `--json` for scripts. |
| `sluice previews` | Every preview on the host, with its addresses and state. `--json` for scripts. |
| `sluice release <preview>` | Release a preview, by its name or its revision: its release command runs, its checkout goes, its ports and names are free. `--json` for scripts. |
| `sluice branch create <branch> --from <branch>` | Make a branch sluice lands on, such as a bolt, at another such branch's current revision: on its remote when its policy names one. `--json` for scripts. |
| `sluice branch delete <branch>` | Delete it once its work has landed into `main`, or the branch `--into` names, and nothing is queued on it. `--json` for scripts. |
| `sluice forge check [<branch>]` | Whether sluice's own identity on GitHub is set up so only sluice may move the branch there. `--json` for scripts. |

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

`sluice exec <branch> -- <command>` runs any command the same way, with
`SLUICE_CHECKOUT`, `SLUICE_TMP` and the ports in its environment, and
exits with the command's exit code. It records nothing.

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
exec` on the host, or the next round, releases it as `sluice release`
would. The policy's release command runs in the worktree's scratch
directory with its ports, its runs end, and its ports are free. That
command says so first, and the release is logged in the branch's
state beside the releases of combinations' checkouts:

```console
$ sluice exec bolt/concurrent-units
released /home/me/wt/unit-c: the worktree no longer exists; its runs ended, the release command run, its ports free
running the checks 8d41b6c affects against bolt/concurrent-units (c41e8f2)
…
```

Until then, `sluice status` lists it as removed.

## `sluice overview`

`sluice overview` shows every branch of the repository that has a
queue, read from its state alone: it starts nothing, releases nothing
and writes nothing, and it reads with no sluice running.

```console
$ sluice overview
bolt/concurrent-units at c41e8f2 · 2 in the queue · testing 1 at once (up to 4)
  testing bolt/concurrent-units + #13 + #14 3/5 since 2026-10-16T09:16:44Z
  2026-10-16T09:15:02Z  landed #12 as c41e8f2
  2026-10-16T09:11:40Z  blamed #11 on 9a03c1e
2 worktrees held · 0 previews
```

`--json` prints one document, `sluice/overview@1`, for a pane, a status
line or a script. Fields are only ever added to it.

| field | |
|---|---|
| `schema`, `host`, `repository`, `at` | the schema, the host's name, the repository's git directory, and when it was read |
| `branches[]` | each branch with a queue: `branch`, `at` (its revision as its state last saw it), `version` (moves on every write), `window`, `parallel` and `depth` (submissions still queued, waiting, testing, passed or landing) |
| `branches[].candidates[]` | each combination: `id`, `submissions`, `revision`, `state`, `started`, `limit` (seconds), `targets` (the tests the branch requires), `executor`, `ran`, `replayed`, and `failed`, each failed test with where its output is kept |
| `branches[].candidates[].tasks[]` | while a combination is under test with the moon runner, each of its tasks: `target`, `state` (`waiting`, `running`, `passed`, `failed` or `replayed`), and, once known, `started`, `ended` and `exit`; left out when nothing has been said of them |
| `branches[].submissions[]` | each submission still queued, as `sluice status <id> --json` prints it |
| `branches[].recent[]` | the branch's last ten `landed`, `blamed` and `conflict` lines of its log, oldest first, exactly as written |
| `sessions[]` | the worktrees held, each with how its last run ended (`last_result`), and those that have checked out a branch sluice lands on (`holds_queued_branch`), as `sluice status --json` lists them |
| `previews[]` | the previews on the host, as `sluice previews --json` lists them |

A combination's `tasks` say how far its tests have got and decide
nothing: its result is still read from the finished run. Each round
updates them, about every 3 seconds while a submission waits, from
what moon writes as each task ends, and they go once the combination
is no longer under test. The text line's `3/5` counts the tasks that
have ended. They are kept on the host that runs the combination, so
another host shows none. The `command` runner has one test, the run
itself, and gives none.

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
with `--json`. It first releases any preview whose command failed or
whose checkout someone removed by hand. `sluice release <preview>` releases
one: its command's run ends, the release command runs in the checkout,
the checkout is removed and its ports and names are free. No round, step
or landing ever releases a preview that is up.

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
testing bolt/concurrent-units + #12 · 2 tasks to run, 31 replayed
passed · landed as b7d2e94
```

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

A pull request sent back runs nothing more when submitted again with no
new commits. sluice tests the same commits again only once the pull
request is marked a draft and then ready for review. The submit says
so, with the failure it was sent back for:

```console
$ wt submit main
pushed unit/batch-review · updated #41: Batch review cards
#41's head 8d41b6c was tested and blamed at 2026-10-11T09:21:50Z: nothing was queued or run again
failed: tests:t-batch-cards
  on main + #38 + #41 (e20a6f9)
  #38 passed on its own, so the failure is from this submission
  log of tests:t-batch-cards: .git/sluice/.logs/main/c7/tests:t-batch-cards.log
fix it in your worktree and submit again, or mark #41 a draft and then ready for review to test the same commits again
```

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

Both exit `0` when done, `1` when a branch they name doesn't exist, and
`2` on a refusal.

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

## Exit codes

| exit | meaning |
|---|---|
| `0` | landed |
| `1` | failed: this submission was blamed |
| `2` | a conflict, or a refusal |
| `130` | stopped with Ctrl-C; the submission stays queued |

Every command, not only submit, exits `3` when sluice itself fails (a
repository it cannot open, a record it cannot read), with the cause on
stderr, and `64` when it cannot read its command line: an unknown
command, flag or value, or a missing argument. `64` is kept apart from
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
| `sluice step` | `{"branch","outcome","released","previews_released"}`: `outcome` is `stepped`, `busy` or `contended`; the two lists name the worktrees and previews the round released because they were gone or their command failed |
| `sluice amend` | `{"id","after","priority"}`, as the submission now stands |
| `sluice withdraw` | `{"id","state":"withdrawn"}`, with `"already":true` when it was withdrawn before |
| `sluice release` | `{"worktree","released"}`, or `{"preview","released"}` for a preview; `released` is `false` when there was nothing to release |
| `sluice protect` | `{"protection":"installed","changed","held"}`: `changed` is `false` when the hook was in place already, and `held` lists each worktree that has checked out a protected branch; with `--remove`, `{"protection":"removed","changed"}` |
| `sluice forge check` | `{"branch","ok","checks"}`, each check with its `ok` and `message` |

A submit following a pull request exits `1` as well when the pull
request is closed without landing.

`sluice preview` exits `0` once its command has started, or with
`--wait` once the preview is up; `1` with `--wait` when its command
failed; `2` when the revision or the branch is refused; `130` when
stopped while it waits.

`sluice exec` exits `0` when every test passed, `1` when one failed, was
not run or timed out, `2` on a refusal, and `130` when stopped with
Ctrl-C, which ends the run. With a command, it exits with the command's
code.

## Who runs the rounds

A round is one pass of sluice's work for a branch: read the queue, start
tests if there is room, read finished tests, blame a failure, land what
passed, write the queue back. Any process may run one, and a lock lets
only one run at a time for a branch.

- **On one machine**, a waiting `sluice submit` or `wt submit` runs the
  rounds itself between reads of its own submission's state, so nothing
  else needs to be running ([ADR 0011](/sluice/decisions/0011/)).
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
