<div align="center">

# sluice

**A merge queue for agents working in parallel worktrees.**

Agents submit. sluice tests each submission exactly as it would land,<br>
names the one that broke the build, and moves the branch only to code that passed.

[![ci](https://github.com/agentplot/sluice/actions/workflows/ci.yml/badge.svg)](https://github.com/agentplot/sluice/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/agentplot/sluice?sort=semver)](https://github.com/agentplot/sluice/releases/latest)
[![license](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

</div>

<p align="center">
  <img src="docs/diagrams/sluice.svg" alt="Agents submit and never merge; each candidate is tested as it would land; a failure names one culprit and tells only its agent; the branch moves only to what passed" width="820">
</p>

## Why

Run ten coding agents in ten worktrees and each one's tests pass on its
own. Merge them and the branch breaks, because nobody tested the
combination. Let the agents merge for themselves and they race, rebase
over each other's work, and can't tell whose change broke the build.

sluice takes merging away from the agents. Each one finishes its work and
**submits** it. sluice queues the submissions, tests each one **on top of
everything queued ahead of it**, and lands it only if that exact
combination passes.

- **Tested as it lands.** A candidate is the branch, plus every
  submission ahead, plus this one, checked out fresh in its own worktree.
  The commit that lands carries exactly the tree that passed.
- **One culprit, one message.** When a combination fails, sluice works
  out the one submission at fault and tells only that agent, with the
  failing tasks and their logs. Everyone else's work keeps moving.
- **The branch only moves to green.** A git hook refuses every update to
  a protected branch except sluice's, and sluice moves it only from the
  revision it tested on.
- **Parallel, and frugal.** Several candidates are tested at once, each
  starting from the build cache its policy declares. With
  [moon](https://moonrepo.dev), only the tasks a change affects run, and
  a branch can accept results moon already has.
- **Confined checks.** A run can be held in a sandbox: it writes only its
  own worktree and reaches only the network its branch allows.
- **No daemon.** sluice starts no services. A waiting `sluice submit`
  drives the queue while it waits; `sluice run`, or `sluice step` from
  cron, does the same for a host. Every decision is written to plain
  files before it is acted on, so a crash at any point resumes cleanly.

## Install

```sh
nix profile install github:agentplot/sluice
```

<details>
<summary>Other ways: a release binary, cargo, a flake input</summary>

**A release binary**, for macOS on Apple silicon and Linux on x86-64 and
ARM:

```sh
curl -fsSL https://github.com/agentplot/sluice/releases/latest/download/sluice-aarch64-darwin.tar.gz \
  | tar -xz -C ~/.local/bin sluice wt-submit
```

Linux: `sluice-x86_64-linux.tar.gz` or `sluice-aarch64-linux.tar.gz`.

**With cargo** (Rust 1.98 or later):

```sh
cargo install --locked --git https://github.com/agentplot/sluice sluice
```

**As a flake input**, for NixOS, home-manager or a devenv shell:

```nix
inputs.sluice.url = "github:agentplot/sluice";
# then: inputs.sluice.packages.${system}.default
```

</details>

Each installs two executables: `sluice`, and `wt-submit`, which
[worktrunk](https://github.com/max-sixty/worktrunk) runs as `wt submit`.

## Quick start

**1. Say what gates the branch.** Commit `.config/sluice.toml`:

```toml
[tests]
runner = "command"          # or "moon", to run only what a change affects
command = "make test"

[checkout]
method = "git-worktree"

[branches.main]
parallel = 4                # candidates under test at once
```

sluice reads this file from the branch itself, never from a submission,
so no submission can loosen the rules it is judged by.

**2. Protect the branch**, once per repository:

```sh
sluice protect
```

**3. Submit** from an agent's worktree, with its work committed:

```sh
sluice submit main          # or: wt submit main
```

It waits, and says what happened:

```
submission 3 to main · 1 queued ahead (#2)
testing main + #2 + #3 · 4 tasks to run, 2 replayed
passed · landed as 9f2c1e4
```

or, when this submission broke the combination:

```
failed: tests:t-cards
  on main + #2 + #3 (5b81d07)
  #2 passed on its own, so the failure is from this submission
  log of tests:t-cards: /…/sluice/.logs/main/c4/tests:t-cards.log
fix it in your worktree and submit again
```

The exit code says the same: `0` landed, `1` failed, `2` a conflict or a
refusal, `130` stopped with Ctrl-C (the submission stays queued).

A complete setup, with moon, a sandbox, a build cache and two tiers of
branches, is walked through in [docs/examples](docs/examples/README.md).

## Commands

| Command | |
|---|---|
| `sluice submit <branch>` | Queue this worktree's head and wait for the outcome. `--no-wait` returns once queued; `--check` only reports whether it would conflict; `--after 2,5` lands it after those; `--json` for scripts. |
| `sluice status <branch>` | The queue: each submission, its agent, its worktree and its state. `sluice status <id>` for one. |
| `sluice run` | Work every branch's queue until stopped. |
| `sluice step <branch>` | One round of a branch's work, then exit: for cron, CI or a hook. |
| `sluice amend <id>` | Reorder a queued submission (`--after`, `--priority`) until it is under test. |
| `sluice withdraw <id>` | Take a submission out of the queue. |
| `sluice protect` | Install the branch-protection hook; `--remove` takes it out. |

An agent names itself with `SLUICE_AGENT`. How many runs a host tests at
once is the host's own setting, in `~/.config/sluice/config.toml`:

```toml
[runs]
max = 2    # default: a quarter of the processors, shared equally between runs
```

## Telemetry

With `SLUICE_OTEL=1`, sluice sends traces and metrics over OTLP to
`localhost:4318`: a trace per round, with each candidate's forming,
testing, verdict and landing, and metrics for queue depth, time to land,
failures and cache use. moon's traces from inside each run carry the
round's trace id. Nothing reads them back: the queue never depends on
its telemetry.

```sh
nix run nixpkgs#otel-tui    # watch it in a terminal
```

Setting up a host to receive it: [docs/telemetry.md](docs/telemetry.md).

## Platforms

| | macOS | Linux |
|---|---|---|
| The queue, worktrees, checks, branch protection | ✓ | ✓ |
| Release binaries | Apple silicon | x86-64, ARM |
| Sandboxed checks | ✓ Seatbelt | planned: bubblewrap, Landlock |

Each host runs its own queue today. Several hosts sharing one branch's
queue, as peers coordinating through the git remote or through a central
server, is designed and not yet built.

## How it is built

A small core that knows git, and five seams it reaches everything else
through: where a candidate is checked out, how its checks run, how the
branch moves, how agents submit, and where the records live. Each seam
is bound by a profile, so one core works with moon or a plain command, a
sandbox or none, local refs or a forge.

The design is a statement of invariants in
[agentplot/blueprints](https://github.com/agentplot/blueprints)
(`design/sluice/requirements.md`). This repository cites its clauses by
number, keeps the rules a change must respect in
[docs/architecture](docs/architecture/core.md), and records each decision
in [docs/adr](docs/adr).

sluice lands its own changes: `main` here moves only through sluice,
gated on every test tier in a sandbox.

## Development

```sh
devenv shell
moon run ':#check-fast' ':#check-tools' ':#check-e2e'
```

Commits follow [Conventional Commits](https://www.conventionalcommits.org),
checked by a commit hook. Each push to `main` with a feature, a fix or a
docs change is tagged and released, with notes by
[git-cliff](https://git-cliff.org); `release-preview` shows the next one.

## License

[MIT](LICENSE)
