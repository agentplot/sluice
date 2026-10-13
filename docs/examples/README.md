# Setting up sluice: a walkthrough

What a developer or an agent sees when sluice is added to a repository on one
machine. The repository is crew itself: it already runs its tests with moon,
works in worktrunk worktrees, and builds bolts from units. Everything here
runs today on one machine; sluice lands its own `main` this way.

## What sluice does, in one paragraph

Agents finish work in their own worktrees and **submit** it to a target
branch instead of merging it. sluice queues the submissions, tests each one
**combined with everything queued ahead of it**, exactly as it would land,
and merges it only if that combination passes. When a combination fails, it
works out which one submission broke it and tells only that agent. No agent
ever merges, and the target branch only ever moves to code that passed.

## 1. Install

```sh
nix profile install github:agentplot/sluice
```

That installs two executables: `sluice`, and `wt-submit`. worktrunk runs any
`wt-<name>` on your `PATH` as `wt <name>`, so `wt submit` works with no
configuration. (`cargo install --git https://github.com/agentplot/sluice
sluice` does the same without nix.)

## 2. The config: `.config/sluice.toml`

One file, checked in. sluice reads it from the target branch, not from the
submission, so a submission cannot loosen the rules it is tested under.

```toml
[tests]
runner = "moon"
toolchain = ["devenv", "print-dev-env"]

[checkout]
method = "git-worktree"

[[checkout.cache]]
key = ["Cargo.lock", ".prototools"]
paths = ["target"]

[executor]
kind = "sandbox"
network = "localhost"

[landing]
subject = "chore(land): #{id} {from}"

[branches."bolt/*"]
required = ["#suite"]
parallel = 4
batch = false
flaky = "fail"
timeout = "20m"
trust = ["open", "sandbox", "container", "cache"]

[branches.main]
required = ["#suite", "#live"]
parallel = 2
batch = true
trust = ["confined", "sandbox"]
```

### `[tests]`: how tests run

| key | values | meaning |
|---|---|---|
| `runner` | `"moon"` | Run moon tasks. sluice asks moon which tasks the combination affects and runs only those; a task whose inputs did not change replays its earlier result instead of running. |
| | `"command"` | Run one command; exit 0 passes. Every test runs every time. For a repository without moon. |
| `command` | a command | With `runner = "command"`, the command to run, e.g. `"tests/suite"`. |
| `toolchain` | a command | Prints the shell the tests run in, e.g. `["devenv", "print-dev-env"]`. It runs in the checkout, outside the sandbox; the tests run inside it with what it printed. |
| `release` | a command | Run when the checkout is removed, after the tests, however they ended, under the same executor and environment: removes what the tests handed to another process, e.g. `"docker rm -f $(docker ps -aq --filter label=sluice.place=$SLUICE_PLACE)"`. It must be safe to run twice. |

Which tasks must pass is not here: it is set per branch, because that is
the point. A bolt's branch lands units many times a day and wants a fast
gate; `main` lands whole bolts and can demand more.

### `[checkout]`: where a combination is tested

sluice tests each combination in a clean checkout of its own, apart from
every agent's worktree: `git worktree add --detach` at the combination,
`git worktree remove` when its result is read. Nothing runs in it but the
tests: no git or worktrunk hooks. Whatever a test needs before it runs goes
in the toolchain or in a moon task the test depends on.

**`[[checkout.cache]]`**: what a new checkout may start with, the way a CI
job restores `actions/cache`. `key` names the files whose content keys the
cache; `paths` the ignored paths it holds. A checkout starts with the cache
of its combination's key, or else the one saved last; a checkout whose
combination passed saves its `paths` under its key. sluice's own caches
`target` keyed by `Cargo.lock` and `.prototools`: a test build that takes
two and a half minutes cold takes under a second warm. A cache only spares
work: it never decides a result.

**`ports`**: the ports a checkout's tests use, by name, e.g. `["ministack",
"api", "console"]`. Each checkout gets a block of its own, held on the host
across every repository and checked free first: `SLUICE_PORT_MINISTACK`,
`SLUICE_PORT_API`, `SLUICE_PORT_CONSOLE`, in the order written, from
`ports_from` (20000 by default) plus the checkout's slot times the number
of names. No two checkouts alive share a port, and a name's port stays
the same for the checkout's life, so separate systems find each other by
these numbers.

### `[executor]`: what the tests may touch

| key | values | meaning |
|---|---|---|
| `kind` | `"open"` | The tests run as a process group, with nothing held back. |
| | `"sandbox"` | The tests may write only their checkout and scratch directory (macOS: Seatbelt; Linux: not built yet). |
| `network` | `false`, `"localhost"`, `true` | No network; only 127.0.0.1 and ::1, for tests that start a server and talk to it; or any. |
| `memory` | e.g. `"8G"` | Refused where the executor cannot hold it, never run without it. |
| `docker` | `true`, `false` | Whether the tests may reach the container engine's socket. A container may mount any path, so a run that could reach it is recorded as not confined. The engine pulls images itself, whatever `network` says. |

Every result records which executor ran it and what it held.

### `[landing]`

`subject` is the first line of a landing commit: `{id}` the submission,
`{from}` the agent's branch, `{agent}`, `{to}` the branch landed on. The
default is the conventional `chore(land): #{id} {from}`.

### Services the tests need

sluice starts no services. If the tests need a database, an emulator or a
dev server, the tests bring it up, the way every CI system and merge queue
leaves it to the job: a moon task the tests depend on that starts devenv's
processes and waits until each answers, testcontainers, Playwright's
`webServer`, a pytest fixture. sluice gives them the ports to use.

What sluice owes the tests, and nothing more:

- a clean checkout of exactly the combination under test;
- `SLUICE_CHECKOUT` and `SLUICE_TMP` in the environment: where the checkout
  is, and a scratch directory of its own;
- `SLUICE_PLACE`, a short name of the checkout's own, for its containers,
  deployments and addresses, and a `SLUICE_PORT_<NAME>` for each port
  `[checkout] ports` names;
- a time limit per branch, and a run that hits it recorded as timed out,
  not failed;
- the test command run in its own process group, which sluice kills when the
  command exits, times out, or the checkout is removed, so a server left
  running can never break the next combination and get the wrong submission
  blamed.

- `[tests] release` run whenever the checkout is removed, for what escapes
  the process group (a container, a detached process manager), including
  after a time limit, when the tests' own cleanup never runs.

### `[branches."<pattern>"]`: rules per target branch

**`required`**: the moon targets that must pass for a submission to land on
this branch, run only where the combination affects them. crew tags every
test `suite`, so `#suite` is all of them; a slower tag such as `#live` can be
required on `main` and not on bolts. With `runner = "command"` the command
is the only test and `required` is left out.

**`parallel`**: how many queued submissions are tested at the same time.

Say three agents submit A, B and C to `bolt/x`, in that order. sluice tests
three combinations: `bolt/x + A`, `bolt/x + A + B`, and `bolt/x + A + B + C`.
With `parallel = 4`, all three are tested at once, on the bet that they will
pass. If they do, A, B and C land one after another with no waiting. If
`bolt/x + A + B` fails, B is to blame, A lands, and C is tested again as
`bolt/x + A + C`. The higher the number, the faster a queue drains when
things pass, and the more work is thrown away when something fails. After a
failure sluice tests fewer at once, and builds back up to this number as
things pass. (GitHub's merge queue calls this "maximum pull requests to
build"; Mergify calls it "speculative checks".)

**`batch`**: whether to test every combination or only the longest.

| value | sluice tests | good for |
|---|---|---|
| `false` | every combination: `+A`, `+A+B`, `+A+B+C` | small, fast tests; a failure is pinned to one submission at once |
| `true` | only the longest, `+A+B+C`; if it passes, all three land together; if it fails, the shorter ones are tested to find which submission broke it | slow tests, where running fewer combinations matters more than finding the culprit quickly |

**`flaky`**: a test that has both passed and failed on identical code.
`"fail"` treats it as failing, so the submission is blamed; `"pass"` lets it
through.

**`timeout`**: how long one combination's tests may run, for example
`"20m"`. A run that hits it is recorded as timed out.

**`trust`**: which results satisfy this branch: `"confined"` (no agent could
reach the run), an executor (`"open"`, `"sandbox"`, `"container"`), and
`"cache"` for a result moon replayed instead of ran. Without `"cache"`, every
required test runs every time, since the cache holds whatever any shell ran.
A branch without `trust` accepts every run sluice starts (`"open"`,
`"sandbox"`, `"container"`) and no replay; one whose executor can never
satisfy it starts nothing and says why.

There is no key for operator approval: that is the caller's business, and
crew or flywheel submits only once the work is approved.

## 3. What changes in crew's other config

crew's `.config/wt.toml` today gates `wt merge` with a `[pre-merge]` hook that
runs the suite, and verifies after landing with `[post-merge]`. With sluice,
both go: sluice runs `#suite` on the exact combination that will land, which
is what the pre-merge hook was approximating, and nothing lands untested, so
there is nothing to verify after. Agents still run their tests while they
work; they just never merge.

The coder's merge step changes from

```sh
wt merge bolt/concurrent-units --no-squash --no-remove
```

to

```sh
wt submit bolt/concurrent-units
```

## 4. Branch protection

Every branch with a `[branches]` section is protected: only sluice moves
it. `sluice protect` (and every `submit`, `step` and `run`) installs a git
`reference-transaction` hook that refuses any other update and logs the
attempt:

```console
$ git merge unit/touches        # on bolt/concurrent-units
error: bolt/concurrent-units is protected by sluice; submit the change to sluice instead
```

The hook calls the installed `sluice`, never one built from a submission, so
no submission changes the guard it is judged by.

On a forge, the same is the forge's branch protection, with sluice's
identity the only one allowed to push.

## 5. An agent submits

Whatever submits work to sluice and gets the outcome back is a **driver**:
it is the front door agents and tools use, and it decides nothing about the
queue. worktrunk is the driver here because agents already live in `wt`
worktrees and used to finish with `wt merge`; `wt submit` takes its place.
The alternatives are plain `sluice submit` from any shell, crew's merge
stage, or flywheel's effect, all calling the same `sluice submit` underneath.

```console
$ wt submit bolt/concurrent-units
submission 12 to bolt/concurrent-units · nothing queued ahead
testing bolt/concurrent-units + #12 · 2 tasks to run, 31 replayed
passed · landed as b7d2e94
```

`wt submit` waits for the outcome and exits with it: 0 landed, 1 failed, 2
conflict. `--no-wait` returns at once, and `sluice status 12` reads it later.
Once its submission lands, `wt submit` removes the agent's worktree and its
branch, as `wt merge` did, following worktrunk's `[merge] remove`;
`--no-remove` keeps them.

The agent in `unit/batch-review` submitted two minutes later, so its
combination includes #12:

```console
$ wt submit bolt/concurrent-units
submission 13 to bolt/concurrent-units · 1 queued ahead (#12)
testing bolt/concurrent-units + #12 + #13 · 4 tasks to run, 29 replayed
failed: tests:t-batch-cards
  on bolt/concurrent-units + #12 + #13 (e20a6f9)
  #12 passed on its own, so the failure is from this submission
  log of tests:t-batch-cards: .git/sluice/.logs/bolt/concurrent-units/c7/tests:t-batch-cards.log
fix it in your worktree and submit again
```

A conflict is found before any test runs:

```console
$ wt submit bolt/concurrent-units
submission 14 to bolt/concurrent-units · 1 queued ahead (#12)
conflict: plugin/lib/plan.py
  conflicts with #12 (crw-1-unit-1, unit/touches)
merge or rebase onto bolt/concurrent-units in your worktree and submit again
```

And before submitting, an agent can ask whether its work would conflict:

```console
$ wt submit --check bolt/concurrent-units
would conflict: plugin/lib/plan.py with #12 (crw-1-unit-1, unit/touches)
```

## 6. What keeps the queue moving

`wt submit` keeps the queue moving itself while it waits: every few seconds
it runs one round of sluice's work and checks whether its own submission is
decided. A round is: read the queue, start tests if there is room, read
finished tests, blame a failure, land what passed, write the queue back,
exit. Any number of waiting agents can do this at once; a lock lets only one
round run at a time.

Two commands do the same thing without submitting anything:

```sh
sluice step bolt/concurrent-units   # one round, for one branch, then exit
sluice run                          # rounds for every branch with a queue, until stopped
```

`sluice step` is for a tool that already has a loop of its own, such as
crew's conductor. `sluice run` is for anyone who wants the queue to move
when no agent is waiting.

## 7. Where sluice keeps its state

In the repository's git directory, never committed and never pushed, readable
with plain tools whether or not sluice is running:

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

A submission:

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

`after` lists submissions this one must land after (crew's `After`).
`attempt` counts how many times this worktree has submitted to this branch.
Submission ids are numbered in order, and that order is the queue's order.

The log, one event per line. A test is logged when it runs, a failure exactly
like a pass, and a replayed result is logged as a replay, never as a run:

```json
{"schema":"sluice/log@1","event":"ran","combination":"9ab3d07c11","task":"tests:t-after","hash":"4d1e22b0c9","outcome":"pass","host":"mac-studio"}
{"schema":"sluice/log@1","event":"replayed","combination":"9ab3d07c11","task":"tests:t-amend","hash":"77aa01f3de","outcome":"pass"}
{"schema":"sluice/log@1","event":"ran","combination":"e20a6f9b48","task":"tests:t-batch-cards","hash":"5c0e97aa21","outcome":"fail","host":"mac-studio"}
{"schema":"sluice/log@1","event":"blamed","id":13,"combination":"e20a6f9b48"}
{"schema":"sluice/log@1","event":"landing","ids":[12],"from":"c41e8f2d90","to":"b7d2e94f03"}
{"schema":"sluice/log@1","event":"landed","ids":[12],"to":"b7d2e94f03"}
```

`landing` is written before the branch moves and `landed` after, so a round
that crashes between the two is finished correctly by the next one. The full
files are beside this walkthrough under `state/`.

## 8. What a landing looks like in history

One merge commit per submission: first parent the branch, second parent the
agent's head, so the agent's commits stay in history and `wt list` shows its
worktree as merged. What was tested rides along as trailers, which outlive
the state files:

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

## 9. Reading and changing the queue

```console
$ sluice status bolt/concurrent-units
bolt/concurrent-units at b7d2e94 · testing 2 at once (up to 4)
  #12  landed    crw-1-unit-1  unit/touches
  #13  failed    crw-1-unit-2  unit/batch-review   tests:t-batch-cards on e20a6f9
  #14  testing   crw-1-unit-3  unit/forecast       bolt/concurrent-units + #14
```

`--json` gives the same for crew's conductor or any other tool.

A queued submission's order can be changed until it is under test:

```sh
sluice amend 14 --after 12      # #14 lands only after #12
sluice amend 14 --priority 1    # ahead of priority-0 submissions not yet under test
```

Submitting again from the same worktree withdraws that worktree's earlier
submission.
