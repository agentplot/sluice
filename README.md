# sluice

A merge queue for agents in worktrees. A session builds in its place and
submits its head; sluice orders what was submitted, tests each candidate
exactly as it would land, names the one submission at fault when a
candidate fails, and moves the branch only to a revision that passed.

![sluice: agents submit and never merge; each candidate is tested as it would land; a failure names one culprit and tells only its agent; the branch moves only to what passed](docs/diagrams/sluice.svg)

It runs on one machine today, macOS first, and lands its own changes:
`main` here moves only through sluice, gated on every tier in a sandbox
(`.config/sluice.toml`). The design is a statement of invariants in
[agentplot/blueprints](https://github.com/agentplot/blueprints) under
`design/sluice/requirements.md`, with its book under `books/sluice/`.
This repository holds the code, the rules a change here must keep
(`docs/architecture/core.md`) and the decisions behind them
(`docs/adr/`).

## Install

Each way installs two executables: `sluice`, and `wt-submit`, which
worktrunk runs as `wt submit`.

**With nix**, from the flake:

```sh
nix profile install github:agentplot/sluice   # into your profile
nix run github:agentplot/sluice -- --help      # once, without installing
```

or as a flake input, for NixOS, home-manager or a devenv shell:

```nix
inputs.sluice.url = "github:agentplot/sluice";
# then: inputs.sluice.packages.${system}.default
```

**A release binary**, for macOS on Apple silicon and Linux on x86-64 and
ARM, from the [releases](https://github.com/agentplot/sluice/releases):

```sh
curl -fsSL https://github.com/agentplot/sluice/releases/latest/download/sluice-aarch64-darwin.tar.gz \
  | tar -xz -C ~/.local/bin sluice wt-submit
```

**With cargo**, from the repository (Rust 1.98 or later):

```sh
cargo install --locked --git https://github.com/agentplot/sluice sluice
```

**From a checkout**, to work on sluice: `devenv shell`, then `cargo build`.
Its tiers are `moon run ':#check-fast' ':#check-tools' ':#check-e2e'`.
Commits are conventional, which a commit-msg hook checks: each push to
`main` with a `feat`, a `fix` or another released change is tagged and
released, its version and notes worked out by git-cliff (`cliff.toml`).
`release-preview` shows the next release before anything lands.


## Usage

A repository names the branches sluice keeps and the checks that gate
them in `.config/sluice.toml`, read from each branch's own revision. The
smallest one runs one command in a fresh worktree of each candidate:

```toml
[tests]
runner = "command"          # or "moon", which runs the tasks a gate names
command = "make test"

[checkout]
method = "git-worktree"

[branches."bolt/*"]
parallel = 4                # candidates under test at once
# with runner = "moon": required = ["#check-fast"], tasks or tags
```

Commit it to the branches it names, then, once per repository:

```sh
sluice protect    # a git hook refuses any update to those branches but sluice's
```

From a session's worktree, with its work committed:

```sh
wt submit bolt/x          # through worktrunk, or the same as:
sluice submit bolt/x      # queue this worktree's head and wait
```

It waits, running sluice's rounds while it does, and prints what
happened:

```
submission 3 to bolt/x · 1 queued ahead (#2)
testing bolt/x + #2 + #3 · 4 tasks to run, 2 replayed
passed · landed as 9f2c1e4
```

or, when the combination fails and this submission is the cause:

```
failed: tests:t-cards
  on bolt/x + #2 + #3 (5b81d07)
  #2 passed on its own, so the failure is from this submission
  log of tests:t-cards: /…/sluice/.logs/bolt/x/c4/tests:t-cards.log
fix it in your worktree and submit again
```

The exit says the same: 0 landed, 1 failed, 2 a conflict or a refusal,
130 stopped by Ctrl-C (the submission stays queued), 3 an error.

| Command | What it does |
|---|---|
| `sluice submit <branch>` | Queue this worktree's head and wait. `--no-wait` returns once queued, `--check` only says whether it would conflict, `--after 2,5` lands it after those, `--json` prints the record. |
| `sluice status <branch>` | The queue: each submission, its agent, its worktree and its state. `sluice status <id>` is one submission. |
| `sluice step <branch>` | One round of the branch's work, then exit: for cron, CI or a hook. |
| `sluice run` | Rounds for every branch with a queue until stopped, holding the host to its bound of runs. |
| `sluice amend <id>` | Change a queued submission's `--after` or `--priority`, until it is under test. |
| `sluice withdraw <id>` | Take a submission out of the queue. |
| `sluice protect` | Install the branch-protection hook; `--remove` takes it out. |

A session names itself with `SLUICE_AGENT`; sluice tells only that
session about its own submission. How many runs a host has under test at
once is the host's, in `~/.config/sluice/config.toml`:

```toml
[runs]
max = 2    # default: a quarter of the processors; each run gets an equal share
```

With `SLUICE_OTEL=1`, sluice exports traces and metrics over OTLP/HTTP to
where the `OTEL_EXPORTER_OTLP_*` variables say (by default
`localhost:4318`), and the checks moon runs for it export beside them
([ADR 0014](docs/adr/0014-telemetry-is-opt-in-otlp-and-decides-nothing.md)).
`nix run nixpkgs#otel-tui` shows them in a terminal.

A whole repository's setup, with moon, a sandbox, a build cache and two
tiers of branches, is walked through in
[docs/examples/](docs/examples/README.md).

## Design

One queue design, two instances:

```
   units' places  --submit-->  [queue of the bolt branch]   --lands-->  bolt branch
   bolt branches  --submit-->  [queue of the shared branch] --lands-->  shared branch
```

The core knows git and five seams: a place, a check, a branch, a driver
and the records. Each is bound by a profile: a git worktree or a
container for the place; moon or a command for the check, run open or in
the host's sandbox; local refs or refs pushed to the forge for the
branch; `wt submit`, crew, flywheel or a remote submit for the driver;
local files, refs on the shared remote or a central server for the
records. sluice starts no services: a repository's tests bring up what
they need. What setting it up looks like is in
[docs/examples/](docs/examples/README.md).

## Many hosts

Sessions on many hosts may submit to one branch, a bolt's branch as much
as the shared branch. Two deployments are designed and neither is built
yet: peers, where sluice runs on every host and they coordinate through
records on the shared remote, and a central server, which takes the
queue and the execution off every other host. Both are choices of
profiles over one core.

## Built

The bolt and shared branches' queues on one host: the core and its
conformance suite, a git worktree per candidate with the build cache its
policy declares, the moon and command checks under the open or sandbox
executor (Seatbelt on macOS), local refs and records, branch protection,
and `wt submit`, which waits, exits with the outcome, and removes the
session's worktree once it lands. The plan and its status are in
`docs/plans/phase-1-pass-2.md`.

MIT licensed.
