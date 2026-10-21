---
title: Configuration
description: Every table and key of the policy file, the host's own config, and the environment.
sidebar:
  order: 1
---

A repository's policy is one file, `.config/sluice.toml`, checked in.
sluice reads it from the target branch, never from a submission, so a
submission cannot loosen the rules it is tested under.

```toml
[tests]
runner = "moon"
toolchain = ["devenv", "print-dev-env"]

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
trust = ["open", "sandbox", "container"]

[branches.main]
required = ["#suite", "#live"]
parallel = 2
batch = true
hold = "5m"
trust = ["confined", "sandbox"]
```

## `[tests]`: how tests run

| key | values | meaning |
|---|---|---|
| `runner` | `"moon"` | Run moon tasks. sluice asks moon which tasks the combination affects and runs only those; a task whose inputs did not change replays its earlier result instead of running. |
| | `"command"` | Run one command; exit 0 passes. Every test runs every time. For a repository without moon. |
| `command` | a command | With `runner = "command"`, the command to run, e.g. `"tests/suite"`. |
| `release` | a command | The release command: what removes what the tests handed to another process to keep, a container above all. It runs once a checkout is released, under the same executor, toolchain and environment as the tests, ports included, and for a worktree when `sluice release` runs in it ([ADR 0016](/sluice/decisions/0016/)). A list, the command first, or a string run by `sh -c`. |
| `toolchain` | a command | Prints the shell the tests run in, e.g. `["devenv", "print-dev-env"]`. It runs in the checkout, outside the sandbox; the tests run inside it with what it printed. sluice reads nothing from it. |
| `ready` | a command | Shows that the host can run the tests, e.g. `["docker", "info"]` for tests that need a container engine. It runs in the checkout under the same executor, toolchain and environment as the tests, within 60 seconds. When it fails before a run, the run is refused and nothing is recorded. When it fails after a run in which a test did not pass, those tests could not run: they have no result, and the combination is tested again. A list, the command first, or a string run by `sh -c`. |
| `not_run` | an exit or a list of them | Exits by which a test says it could not run here, e.g. `[77]`. A test that exits with one has no result, as with `ready`. Unset, every exit but `0` is a fail. |

Which tasks must pass is not here: it is set per branch.

A combination whose tests could not run is tested again and blames
nobody. After three runs in a row in which its tests could not run, its
newest submission is blamed, and each test is named with why. A command
given to `sluice exec` after `--` is held to neither `ready` nor
`not_run`.

## `[checkout]`: where a combination is tested

sluice tests each combination in a clean checkout of its own, apart from
every agent's worktree: `git worktree add --detach` at the combination,
`git worktree remove` when its result is read. Nothing runs in it but the
tests: no git or worktrunk hooks. Whatever a test needs before it runs
goes in the toolchain or in a task the test depends on.

There is one way to make a checkout, so no key names it. An older
`method = "git-worktree"` is still read; any other `method` is refused
with the reason ([ADR 0013](/sluice/decisions/0013/)).

| key | values | meaning |
|---|---|---|
| `ports` | e.g. `["web", "db"]` | A port for each name, held for the checkout on this host and checked free, given to the tests as `SLUICE_PORT_WEB`, `SLUICE_PORT_DB` ([ADR 0016](/sluice/decisions/0016/)). An agent's worktree gets its ports on its first `sluice exec` and keeps them until `sluice release`. A preview gets a name for each as well, `SLUICE_NAME_WEB`. |
| `ports_from` | a port, default `20000` | Where the blocks of ports begin. |

### `[[checkout.cache]]`

What a new checkout may start with, the way a CI job restores a cache.

| key | meaning |
|---|---|
| `key` | The files whose content keys the cache. |
| `paths` | The ignored paths it holds. |

A checkout starts with the cache of its combination's key, or else the
one saved last; a checkout whose combination passed saves its `paths`
under its key. sluice's own repository caches `target` keyed by
`Cargo.lock` and `.prototools`: a test build that takes two and a half
minutes cold takes under a second warm. A cache only spares work: it
never decides a result.

## `[executor]`: what the tests may touch

| key | values | meaning |
|---|---|---|
| `kind` | `"open"` | The tests run as a process group, with nothing held back. |
| | `"sandbox"` | The tests may write only their checkout and scratch directory (macOS: Seatbelt; Linux: not built yet). |
| | `"container"` | The tests run in a container from the image the policy names. Planned, for a box that runs no agents ([executors and systems](/sluice/guides/executors-and-systems/)); not built. |
| `network` | `false`, `"localhost"`, `true` | No network; only 127.0.0.1 and ::1, for tests that start a server and talk to it; or any. |
| `memory` | e.g. `"8G"` | Refused where the executor cannot hold it, never run without it. |
| `docker` | `true` | The tests may reach the host's container engine. A container may mount any path, so such a run is never counted as confined. |

Every result records which executor ran it and what it held. An executor
that cannot enforce a limit the policy states refuses the run
([ADR 0012](/sluice/decisions/0012/)).

## `[preview]`: a running copy of a landed revision

What brings a [preview](/sluice/guides/previews/) up. Read from the
revision being previewed, since it describes that revision's stack;
what its command may use and reach, `[executor]` and the time limit,
comes from the branch's current revision. With no `[preview]` table,
`sluice preview` refuses the revision.

| key | values | meaning |
|---|---|---|
| `command` | a command | Brings the stack up in the preview's checkout, hands what must stay running to the tools that keep it (a container engine, a process manager), and exits; exit 0 is `up`. A list, the command first, or a string run by `sh -c`. It runs once, under the executor, with `[tests] toolchain`, and ends with its process group. |
| `addresses` | e.g. `{ web = "https://{name.web}.localhost" }` | What a person opens, by label. `{name.<port>}` is the preview's name for a port of `[checkout] ports`, `{port.<port>}` its number. A port not listed is refused. sluice fills them in and never contacts them. |
| `timeout` | e.g. `"10m"` | How long the command may run; the branch's `timeout` when unset. |

```toml
[checkout]
ports = ["web", "api"]

[preview]
command = "moon run app:preview"
addresses = { web = "https://{name.web}.localhost", api = "https://{name.api}.localhost/docs" }
```

`[tests] release` is a preview's release command too: it runs when the
preview is released and removes what the command handed off, routes
included. Each name is one DNS label of at most 63 characters,
`<port>-<revision>--<branch>--<repository>`, cut with a hash when it is
longer, unique among the repository's previews.

## `[landing]`

| key | meaning |
|---|---|
| `subject` | The first line of a landing commit: `{id}` the submission, `{from}` the agent's branch, `{agent}`, `{to}` the branch landed on. The default is the conventional `chore(land): #{id} {from}`. |

## `[systems]`: what a test runs on

The systems a branch can require a test to pass on, each a name for an
operating system and a processor architecture as the toolchain names
them (`linux` or `macos`; `x86_64` or `aarch64`). A result records the
system it ran on, and a pass on one system never stands for another.

```toml
[systems]
linux-x64   = { os = "linux", arch = "x86_64" }
linux-arm64 = { os = "linux", arch = "aarch64" }
macos-arm64 = { os = "macos", arch = "aarch64" }
```

Every other key that names a system, a branch's `systems` and
`systems_for` and a signer's `systems`, uses these
names, and sluice refuses a name the table lacks.

## `[signers."<name>"]`: who may sign results

A policy with signers keeps the state of every branch with a `remote`
on that remote, so every host that submits to it reads and writes the
same queue ([Executors and systems](/sluice/guides/executors-and-systems/)).
A result counts there only when a signer the branch trusts signed it.
A policy with no `[signers]` keeps every branch's state on the host,
as before.

```toml
[signers.box]
ssh = "exec@build-1 ssh-ed25519 AAAA..."
confined = true
systems = ["linux-x64"]

[signers.sessions]
ssh = ".config/sluice.sessions"
```

| key | meaning |
|---|---|
| `ssh` | An SSH public key in `allowed_signers` form, or the path in the repository of a file of them, one per host. A host signs with the private half its own [`[signing]`](#signing) names. |
| `confined` | Whether results it signs may count as confined: no agent could reach the checkout or what the test wrote. Say it only of a host that runs no agents; sluice never works it out. The default is `false`. |
| `systems` | The systems its results may say they ran on. The default is every system. |

A signer has exactly one `ssh`.

### `ssh`: one key, or a file of them

`ssh` takes one of two forms:

- **A line**, for a signer with one host: `ssh = "build-1 ssh-ed25519
  AAAA..."`.
- **A path** in the repository, for a signer with many hosts:
  `ssh = ".config/sluice.sessions"`. sluice reads the file from the
  branch's current revision, as it reads the policy, so adding a host
  is a landing that adds its line, and a submission can't trust its own
  key. A path the revision doesn't hold stops the branch with an error
  naming it.

Either way, each line is in `ssh-keygen`'s allowed-signers form:

```text
# .config/sluice.sessions: one line per session host
laptop-chuck ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA... chuck's laptop
mini-2       ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA...
```

The first word is a label: sluice shows it and never checks it. Then
the key type and key, and an optional comment. Blank lines and `#`
lines are skipped. Options between the label and the key, such as
`namespaces=` or `valid-after=`, are skipped too, and restrict nothing:
remove a key's line to stop trusting it. Any key on the list signs as
the signer. A host prints its own line with
`echo "$(hostname -s) $(cat ~/.config/sluice/signing.pub)"`.

## `[branches."<pattern>"]`: rules per target branch

One table per pattern of branch names. Every branch that matches one is
protected: only sluice moves it.

### `required`

The moon targets that must pass for a submission to land on this branch,
run only where the combination affects them. A repository that tags
every test `suite` requires `#suite`; a slower tag such as `#live` can be
required on `main` and not on bolts. With `runner = "command"` the
command is the only test and `required` is left out.

### `parallel`

How many queued submissions are tested at the same time.

Say three agents submit A, B and C to `bolt/x`, in that order. sluice
tests three combinations: `bolt/x + A`, `bolt/x + A + B`, and
`bolt/x + A + B + C`. With `parallel = 4`, all three are tested at once,
on the bet that they will pass. If they do, A, B and C land one after
another with no waiting. If `bolt/x + A + B` fails, B is blamed, A lands,
and C is tested again as `bolt/x + A + C`. The higher the number, the
faster a queue drains when things pass, and the more work is thrown away
when something fails. After a failure sluice tests fewer at once, and
builds back up to this number as things pass. With `batch = true`,
`parallel` is the largest batch instead. (GitHub's merge queue calls
this "maximum pull requests to build"; Mergify calls it "speculative
checks".) [Parallel and batch](/sluice/guides/parallel-and-batch/) shows
both over time and says which to choose.

### `batch`

Whether to test every combination or only the longest.

| value | sluice tests | good for |
|---|---|---|
| `false` | every combination: `+A`, `+A+B`, `+A+B+C`, up to `parallel` at once | fast tests, and a host with room for several runs; each submission lands about one run after it was submitted, and a failure names its submission at once |
| `true` | only the longest, `+A+B+C`, one run at a time; if it passes, all three land together; if it fails, the shorter ones are tested to find which submission broke it | slow tests, where running fewer combinations matters more than landing or blaming quickly |

With `true`, the branch tests nothing else while a batch is under test,
unless it has a [`hold`](#hold). Submissions that arrive meanwhile wait,
and the next batch is whatever queued up: no timer collects one. A branch can't test several batches
at once; several branches can, sharing the host's
[`[runs] max`](#runs).

### `hold`

How long a combination that passed may wait to land while a longer one
holding it is still under test, for example `"5m"`. Unset or `"0s"`,
nothing waits: what passed lands at once.

Say A is under test when B and C arrive ten seconds later. Without a
hold, A lands alone when it passes and B and C land in a second move of
the branch. With `hold = "5m"`, A waits for `+A+B+C`; when that passes,
all three land in one move. If it fails, A lands at once with whatever
else passed ahead of the failure. A never waits longer than the hold
from its own pass. Set it on a branch whose every move starts a build or
a deployment, such as `main`, to how far apart submissions may arrive
and still land together. With `batch = true`, a branch with a hold also
tests the longest combination beside one already under test, so B and
C are not left waiting behind A. `sluice status` names what a held
submission waits for and when its hold runs out.

### `flaky`

A test that has both passed and failed on identical code. `"fail"`
treats it as failing, so the submission is blamed; `"pass"` lets it
through.

Under `"fail"`, a submit names such a test `failed: <test> (flaky)`. It
says whether the test passed or failed in this run, when and on which
combination the same inputs did otherwise, and where that run's log is
kept. The same inputs fail whenever they're tested, so the way on is a
change to what the test reads. When the fail was the host's, not the
test's, the operator can instead set it aside with `sluice set-aside`,
and the same inputs read passed from their passes.

### `timeout`

How long one combination's tests may run, for example `"20m"`. A run
that hits it is recorded as timed out.

### `trust`

Which results satisfy this branch.

On a branch whose state is on its remote, in a policy with `[signers]`,
`trust` lists signer names: a result counts only when one of them signed
it. Without `trust`, such a branch trusts no signer, and its policy's
author names them.

On a branch whose state is on this host, `trust` lists `"confined"` (no
agent could reach the run) and executors (`"open"`, `"sandbox"`,
`"container"`), and the results sluice starts here count unsigned. A
branch without `trust` accepts every run sluice starts
([ADR 0015](/sluice/decisions/0015/)); one whose executor can never
satisfy it starts nothing and says why.

A result moon replays from its cache counts only when a run the branch
trusts recorded a pass for the same inputs on the same system. When
none has, the test runs again without the cache. `"cache"`, which once
let any replay count, is ignored, and `sluice status` warns of it.

### `confined`

On a branch whose state is on its remote, `confined = true` counts only
results whose signer has `confined = true`. On a branch whose state is
on this host, list `"confined"` in `trust` instead; sluice refuses
`confined` there.

### `systems`

The systems every required test must pass on, by the names of
[`[systems]`](#systems-what-a-test-runs-on). Each system is one more run
per combination. Without `systems`, a test takes one pass from any
system.

### `[branches."<pattern>".systems_for]`

The systems one test must pass on, in place of `systems`:

```toml
[branches.main]
systems = ["linux-x64"]

[branches.main.systems_for]
"cli:build" = ["linux-x64", "linux-arm64", "macos-arm64"]
```

A combination of `main` then passes only when `cli:build` passed on all
three and every other required test passed on `linux-x64`.
[Parallel and batch](/sluice/guides/parallel-and-batch/#systems-multiply-runs)
counts the runs this costs.

### `remote`

The git remote the branch lives on, such as `"origin"`. Every round
fetches the branch from it and tests on what the remote holds. A
landing is a push that the remote takes only if its branch has not moved
since the combination was formed. The local branch is then moved to
match. A push the remote refuses because someone else pushed is
reported, and the next round tests again on the new revision. Only the
branch is pushed, and the state stays on the host, unless the policy
has [`[signers]`](#signersname-who-may-sign-results): then the state is
on the remote too, under `refs/sluice/`. A branch without
`remote` is a local branch, and a `remote` the repository has no remote
of that name for lands nothing and says so. A branch must be on its
remote before anything is submitted to it: `sluice branch create
<branch> --from main` makes one there, such as a bolt, and a submission
to a branch the remote lacks is refused saying so. A host with no local
copy of the branch, only a fetched `origin/<branch>`, reads `remote` from
the policy there. The guide [sluice on GitHub](/sluice/guides/github/)
sets one up.

### `[branches."<pattern>".pull_requests]`

Turns pull requests on for the branch: work reaches it as pull requests
on its `remote`. A pull request joins the queue only once it is out of
draft and meets these conditions on its latest commit.

```toml
[branches.main]
remote = "origin"

[branches.main.pull_requests]
reviews = 1                  # approving reviews on the current head
checks = ["qa/approved"]     # checks other systems post, by name
```

| key | meaning |
|---|---|
| `reviews` | How many approving reviews the pull request needs on its latest commit; a review given on an earlier commit doesn't count. The default is 0. |
| `checks` | Checks other systems post on the pull request's latest commit, by the name GitHub shows, each of which must have passed. The default is none. |

The conditions are read from the branch itself, so a pull request that
changes `.config/sluice.toml` can't loosen its own. They decide when
sluice takes a pull request, never what it tests: that is `required`.
The table needs `remote`, and sluice refuses a policy that has it
without one. Without the table, `sluice submit` puts work straight into
the queue. The guide [Pull requests](/sluice/guides/pull-requests/) sets
it up.

No key covers operator approval: that is the caller's business, and
a tool submits only once the work is approved.

## The host's config: `~/.config/sluice/config.toml`

How many runs a host tests at once, and who sluice pushes as, are the
host's own settings, not the repository's:

```toml
[runs]
max = 2        # default: a quarter of the cores
reserved = 1   # default: 0; runs of the bound agents' own runs never use

[forge]
app_id = 123456                  # the GitHub App sluice pushes as
key = "~/.config/sluice/app.pem" # its private key on this host

[signing]
signer = "box"                   # the policy's name for this host's key
key = "~/.config/sluice/signing" # an SSH private key on this host

[peers]
lease = "5m"   # how long a quiet host keeps a branch
alive = "2m"   # how often this host shows it is alive
```

### `[runs]`

`max` is the host's bound: how many runs share the host's cores at once.
It counts the runs of submissions and the runs agents start in their own
worktrees with `sluice exec` together, from the state of every branch
the instance serves ([ADR 0012](/sluice/decisions/0012/)). A run waits
while the bound is full.

The cores are the logical CPUs the host gives sluice. With no `max`,
the bound is a quarter of them, and never less than one. Each run gets
the cores divided by `max`, and never less than one: sluice hands that
number to the run as `SLUICE_JOBS`, and to moon as `--concurrency`.

`max` is the one setting for memory as well. Nothing but moon keeps to
the share of cores, and an executor's `memory` limit holds the run's own
processes, never the containers they ask a container engine for. A
repository whose tests bring up a stack of containers sets `max` to how
many of those stacks the host holds in memory at once, whatever its
count of cores.

`reserved` splits the bound. It keeps that many runs of the bound for
submissions: an agent's run in its worktree waits rather than take one,
and a submission's run starts whenever the bound has room. With the
default, 0, agents' runs may hold the whole bound. A `reserved` at or
above `max` leaves agents no run, and sluice refuses to start with the
setting named. `sluice overview --json` shows the bound, what holds it
and the reserve as `room`, and its counts line says when agents' runs
hold back a submission that would otherwise start. What the host records
of its runs and ports is on the [state page](/sluice/reference/state/).

### `[forge]`

The app sluice pushes as, for every branch whose `remote` is on GitHub.
sluice makes a token from the key for each repository when it needs
one, keeps it in memory, and gives it only to its own fetches and
pushes. An agent's own git gets nothing from it, so a ruleset whose only
bypass is the app refuses an agent's push to the branch. `key` is a
path on the host, with `~` for the home directory; sluice refuses a key
file that anyone but its owner may read, and names the mode it needs,
`0600`. Without `[forge]`, sluice pushes with the host's own git
credentials, and `sluice status` says the branch has no identity of its
own. The guide [Give sluice its own GitHub
identity](/sluice/guides/github-app/) sets it up, and
[`sluice forge check`](/sluice/reference/commands/#sluice-forge-check)
says whether it is set up right.

### `[runs] systems`

The systems this host runs, by the policy's
[`[systems]`](#systems-what-a-test-runs-on) names. Without it, the host
runs the one system it is. `sluice run` refuses a system whose operating
system and architecture the host isn't.

### `[signing]`

The signer this host signs its results as, `signer`, one of the
policy's [`[signers]`](#signersname-who-may-sign-results), and the path
of its SSH private key, `key`, whose public half the policy lists.
sluice refuses a key file anyone but its owner may read, as it does
`[forge] key`. A host with no `[signing]` records no result on a branch
whose state is on its remote, and says so on every run. Make the key
with `ssh-keygen -t ed25519 -f ~/.config/sluice/signing -N ""`.

### `[peers]`

How long a branch whose state is on its remote stays with the host
that last wrote it, and how often this host shows it is alive. The same
for every branch the host serves, and never a branch's: a policy that
sets either is refused.

| key | meaning |
|---|---|
| `lease` | How long this host waits, having seen a branch's state stand unchanged, before it takes the branch from a host that has gone quiet. It counts by its own clock, never by the time another host wrote. The default is `"5m"`. |
| `alive` | How often this host rewrites its sign of life. A claim on a run lapses once its host's sign of life has stood unchanged this long, and another host runs it. It must be shorter than `lease`. The default is `"2m"`. |
| `name` | What this host is called among the hosts that share a remote: in a branch's state as its lead, and on the runs it claims. The default is the host's own name; give two instances on one machine a name each. |

## Environment

| variable | set by | meaning |
|---|---|---|
| `SLUICE_AGENT` | the agent | The agent's name, recorded on its submissions and shown in `sluice status`. |
| `SLUICE_CHECKOUT` | sluice, for a run | The absolute path of the checkout under test. |
| `SLUICE_TMP` | sluice, for a run | A scratch directory of the run's own, for sockets, pid files and data directories, so parallel checkouts never collide. An agent's worktree keeps one across its runs. |
| `SLUICE_PLACE` | sluice, for a run | A short name of the checkout's own, for labelling what the tests start, so the release command finds it. |
| `SLUICE_PORT_<NAME>` | sluice, for a run | The port given for each name in `[checkout] ports`, upper-cased, `-` written `_`. |
| `SLUICE_NAME_<NAME>` | sluice, for a preview | The preview's name for each port: one DNS label, unique among the repository's previews, for a proxy to route to the port. |
| `SLUICE_BRANCH` | sluice, for a preview | The branch the preview was asked for through. |
| `SLUICE_REVISION` | sluice, for a preview | The full revision the preview holds. |
| `SLUICE_JOBS` | sluice, for a run | The cores the run may use: the host's cores divided by `[runs] max`. |
| `SLUICE_OTEL` | the host | `1` turns telemetry on; see [telemetry](/sluice/reference/telemetry/). |
| `SLUICE_FORGE_API` | tests | Where the `[forge]` app's API calls go instead of `https://api.github.com`: a fake one. |
