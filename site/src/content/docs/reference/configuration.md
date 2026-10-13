---
title: Configuration
description: Every table and key of .config/sluice.toml, the host's config, and the environment.
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

## `[tests]`: how tests run

| key | values | meaning |
|---|---|---|
| `runner` | `"moon"` | Run moon tasks. sluice asks moon which tasks the combination affects and runs only those; a task whose inputs did not change replays its earlier result instead of running. |
| | `"command"` | Run one command; exit 0 passes. Every test runs every time. For a repository without moon. |
| `command` | a command | With `runner = "command"`, the command to run, e.g. `"tests/suite"`. |
| `toolchain` | a command | Prints the shell the tests run in, e.g. `["devenv", "print-dev-env"]`. It runs in the checkout, outside the sandbox; the tests run inside it with what it printed. sluice reads nothing from it. |

Which tasks must pass is not here: it is set per branch.

## `[checkout]`: where a combination is tested

sluice tests each combination in a clean checkout of its own, apart from
every agent's worktree: `git worktree add --detach` at the combination,
`git worktree remove` when its result is read. Nothing runs in it but the
tests: no git or worktrunk hooks. Whatever a test needs before it runs
goes in the toolchain or in a task the test depends on.

| key | values | meaning |
|---|---|---|
| `method` | `"git-worktree"` | The one method. Anything else is refused with the reason ([ADR 0013](/sluice/decisions/0013/)). |

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
| | `"container"` | The tests run in a container from the image the policy names. Designed for [many hosts](/sluice/many-hosts/peers/); not built. |
| `network` | `false`, `"localhost"`, `true` | No network; only 127.0.0.1 and ::1, for tests that start a server and talk to it; or any. |
| `memory` | e.g. `"8G"` | Refused where the executor cannot hold it, never run without it. |

Every result records which executor ran it and what it held. An executor
that cannot enforce a limit the policy states refuses the run
([ADR 0012](/sluice/decisions/0012/)).

## `[landing]`

| key | meaning |
|---|---|
| `subject` | The first line of a landing commit: `{id}` the submission, `{from}` the agent's branch, `{agent}`, `{to}` the branch landed on. The default is the conventional `chore(land): #{id} {from}`. |

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
builds back up to this number as things pass. (GitHub's merge queue calls
this "maximum pull requests to build"; Mergify calls it "speculative
checks".)

### `batch`

Whether to test every combination or only the longest.

| value | sluice tests | good for |
|---|---|---|
| `false` | every combination: `+A`, `+A+B`, `+A+B+C` | small, fast tests; a failure is pinned to one submission at once |
| `true` | only the longest, `+A+B+C`; if it passes, all three land together; if it fails, the shorter ones are tested to find which submission broke it | slow tests, where running fewer combinations matters more than finding the culprit quickly |

### `flaky`

A test that has both passed and failed on identical code. `"fail"`
treats it as failing, so the submission is blamed; `"pass"` lets it
through.

### `timeout`

How long one combination's tests may run, for example `"20m"`. A run
that hits it is recorded as timed out.

### `trust`

Which results satisfy this branch: `"confined"` (no agent could reach
the run), an executor (`"open"`, `"sandbox"`, `"container"`), and
`"cache"` for a result moon replayed instead of ran. Without `"cache"`,
every required test runs every time, since the cache holds whatever any
shell ran. A branch without `trust` accepts every run sluice starts
(`"open"`, `"sandbox"`, `"container"`) and no replay
([ADR 0015](/sluice/decisions/0015/)); one whose executor can never
satisfy it starts nothing and says why.

There is no key for operator approval: that is the caller's business, and
a tool submits only once the work is approved.

## The host's config: `~/.config/sluice/config.toml`

How many runs a host tests at once is the host's own setting, not the
repository's:

```toml
[runs]
max = 2    # default: a quarter of the processors, shared equally between runs
```

The bound is the instance's: sluice starts no more runs at once than it,
counted from the state of every branch the instance serves
([ADR 0012](/sluice/decisions/0012/)).

## Environment

| variable | set by | meaning |
|---|---|---|
| `SLUICE_AGENT` | the agent | The agent's name, recorded on its submissions and shown in `sluice status`. |
| `SLUICE_CHECKOUT` | sluice, for a run | The absolute path of the checkout under test. |
| `SLUICE_TMP` | sluice, for a run | A scratch directory of the run's own, for sockets, pid files and data directories, so parallel checkouts never collide. |
| `SLUICE_OTEL` | the host | `1` turns telemetry on; see [telemetry](/sluice/reference/telemetry/). |
