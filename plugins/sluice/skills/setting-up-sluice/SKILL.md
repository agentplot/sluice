---
name: setting-up-sluice
description: Install sluice and set it up in a repository. Use when the user asks to install, set up, configure or adopt sluice, write or change `.config/sluice.toml`, gate a branch, move from `wt merge` or direct merges to a merge queue, land on GitHub through sluice, give sluice a GitHub App, or take pull requests through it; also when `sluice` is not installed or is too old. Chooses the runner, executor, worktree driver and forge for the tools the repository already uses (moon, worktrunk, devenv, GitHub) and shows a reference example for each.
---

# Setting up sluice

Work through these in order. Show every command before running it, and ask
before anything that installs software, changes host config, commits, or
touches GitHub settings.

## 1. The binary

```sh
command -v sluice && sluice --version
```

This plugin needs **sluice 0.12 or later** (`sluice overview`). If it is
missing or older, install or upgrade it. Pick the first that fits:

| the host has | install | upgrade |
|---|---|---|
| sluice from a flake input or home-manager (path under `/nix/store` or `/etc/profiles`) | bump the input in the user's own flake and rebuild; it is their config, so tell them, don't edit it unasked | same |
| nix | `nix profile install github:agentplot/sluice` | `nix profile upgrade --all` (or the entry's name from `nix profile list`) |
| neither, macOS on Apple silicon or Linux x86-64/ARM | the release tarball, below | the same command again |
| Rust 1.98+ | `cargo install --locked --git https://github.com/agentplot/sluice sluice` | the same with `--force` |

```sh
# targets: aarch64-darwin, x86_64-linux, aarch64-linux
mkdir -p ~/.local/bin
curl -fsSL https://github.com/agentplot/sluice/releases/latest/download/sluice-aarch64-darwin.tar.gz \
  | tar -xz -C ~/.local/bin sluice wt-submit
```

Each way installs `sluice` and `wt-submit` (worktrunk runs it as `wt submit`).
Check `~/.local/bin` is on `PATH`, then `sluice --version` again.

## 2. Survey the repository

Find out what the repository already uses before writing anything:

| look for | means |
|---|---|
| `.moon/workspace.yml`, `moon.yml` | runner `moon` ([profiles/moon.md](references/profiles/moon.md)) |
| a single test command (`make test`, `npm test`, `cargo test`, `pytest`) and no moon | runner `command` ([profiles/command.md](references/profiles/command.md)) |
| `devenv.nix` / `devenv.yaml` | `toolchain = ["devenv", "print-dev-env"]` |
| `flake.nix` with a dev shell, no devenv | `toolchain = ["nix", "print-dev-env"]` |
| `.prototools` | moon and tools pinned by proto; sluice runs `proto install` for you |
| tests that start docker, compose or testcontainers | `[executor] docker = true`, a `[tests] release`, and `[tests] ready = ["docker", "info"]` so a stopped engine leaves those tests unrun, not failed ([profiles/checkout.md](references/profiles/checkout.md)) |
| tests that listen on ports | `[checkout] ports` ([profiles/checkout.md](references/profiles/checkout.md)) |
| `.config/wt.toml`, `wt` on `PATH`, `wt merge` in scripts | worktrunk driver ([profiles/worktrunk.md](references/profiles/worktrunk.md)) |
| `git remote get-url origin` on github.com, and the target branch lives there | `remote = "origin"` ([profiles/remote-branches.md](references/profiles/remote-branches.md)) |
| reviews or other systems' checks must pass before landing | pull requests ([profiles/pull-requests.md](references/profiles/pull-requests.md)) |
| a stack a person should try after each landing | previews ([profiles/previews.md](references/profiles/previews.md)) |
| a bare repo with worktrees, or none yet | the `worktrees` skill |

Then ask the user which branches sluice should gate (`main`; a family like
`bolt/*` for small fast landings) and what must pass on each.

## 3. Choose, per part

| part | choices today | reference |
|---|---|---|
| how tests run (`[tests] runner`) | `moon`, `command` | [moon](references/profiles/moon.md), [command](references/profiles/command.md) |
| what tests may touch (`[executor] kind`) | `open`, `sandbox` (macOS) | [sandbox](references/profiles/sandbox.md) |
| where a combination is tested (`[checkout]`) | a clean git worktree, with ports and caches | [checkout](references/profiles/checkout.md) |
| who submits | `sluice submit`, `wt submit` (worktrunk) | [worktrunk](references/profiles/worktrunk.md) |
| where the branch lives | local, or on a remote (`remote`) | [remote-branches](references/profiles/remote-branches.md) |
| sluice's identity on GitHub (host `[forge]`) | a GitHub App | [github-app](references/profiles/github-app.md) |
| how work reaches the queue | straight from a worktree, or pull requests | [pull-requests](references/profiles/pull-requests.md) |
| running copies of landed revisions (`[preview]`) | a command per repository | [previews](references/profiles/previews.md) |
| more than one host, or more than one system (`[signers]`, `[systems]`) | peers on the git remote, standing hosts signing with SSH keys | [peers](references/profiles/peers.md) |

The rules per branch (`required`, `parallel`, `batch`, `hold`, `flaky`,
`timeout`, `trust`) and `[landing] subject` are in the
[configuration reference](https://agentplot.github.io/sluice/reference/configuration/#branchespattern-rules-per-target-branch).
Leave `flaky = "fail"`.

### Parallel, batch and the host's bound

Never leave `parallel` and `batch` to their defaults unexamined: they decide
how long agents wait and how much the host runs. Find out, for each gated
branch:

1. **How long its gate takes.** Time the required checks once, from CI's
   history, the repository's own test script, or by running them.
2. **How often submissions arrive.** Ask how many agents will submit to it.
3. **What one run costs the host.** Tests that start containers, an
   emulator or a database hold that stack for the whole run.

Then choose, and tell the user why:

| the branch | set |
|---|---|
| a gate of a few minutes | `batch = false`, `parallel = 2` to `4` |
| a gate of 15 minutes or more, with several agents submitting | `batch = true`, `parallel = 4` to `8` |
| failures expected to be common | `batch = false` |

- `batch = false` tests every combination as it arrives, up to `parallel` at
  once: each submission lands about one run after it was submitted, and it
  costs one run per submission.
- `batch = true` tests only the longest combination, one run at a time on the
  branch. Submissions arriving meanwhile wait and form the next batch; no
  timer collects them. A pass lands the whole batch; a failure costs a second
  round, testing the shorter combinations, before the blamed submission is named.
  `parallel` is then the largest batch.
- One branch can't do both. Bolts and `main` can differ: `batch = false` on
  `bolt/*` with a short gate, `batch = true` on `main` with the full one.
- Set `timeout` to about twice the gate's usual time; the default is an hour.
- Set `hold` on a branch whose every move starts a build or a deployment,
  such as `main`, to how far apart submissions may arrive and still land in
  one move, e.g. `"5m"`. Leave it off bolt branches.
- Set the host's `[runs] max` (step 5) to how many runs the host holds at
  once. It caps every branch of every repository on the host together,
  agents' `sluice exec` runs and previews too: every sluice on the host
  reads one count. With no `max`, a quarter of the cores sluice may use is
  shared by every repository on the host. Leave room in it for what else the
  host runs, agents' sessions included; sluice reads nothing of how busy the
  host is. On a Mac, which gives sluice every core, set `[runs] cores` too,
  to the cores sluice may use, so the runs leave the rest to the sessions.
- Every run gets its own checkout and its own ports. Check the tests write
  nothing outside the checkout (a fixed output directory, a lock file, a port
  they choose themselves): two overlapping runs collide there and fail
  unrelated submissions.

After a failure sluice tests fewer at once, halving the number for each
blamed submission and adding one back per pass; `sluice status <branch>`
shows `testing N at once (up to M)`. A branch stuck at one is a branch whose
submissions keep failing.
[Parallel and batch](https://agentplot.github.io/sluice/guides/parallel-and-batch/)
draws all of this.

Complete starting points:
[moon-minimal](references/examples/moon-minimal.sluice.toml),
[moon-worktrunk-github](references/examples/moon-worktrunk-github.sluice.toml),
[moon-peers-github](references/examples/moon-peers-github.sluice.toml),
[command-only](references/examples/command-only.sluice.toml).
Every key: <https://agentplot.github.io/sluice/reference/configuration/>.

## 4. Write and commit the policy

Write `.config/sluice.toml`. sluice reads it **from the branch it gates**,
never from a submission, so it takes effect once it is committed on that
branch. Commit it there the way the repository lands work today, before
protecting the branch. That one change is the last one landed by hand.

## 5. Protect the branch and set up the host

```sh
sluice protect        # git hook: only sluice moves a gated branch (submit, step and run install it too)
```

Host settings live in `~/.config/sluice/config.toml`, not the repository:

```toml
[runs]
max = 2         # runs at once on this host, every repository's; default a quarter of the cores
cores = 12      # the cores sluice may use; default what the host gives it, every core on a Mac
reserved = 1    # of those, kept for submissions; agents' `sluice exec` never takes them
```

Each run is given the cores sluice may use divided by `max` (`SLUICE_JOBS`,
and moon's `--concurrency`), so the runs going at once, whatever their
repositories, use those cores and no more. Without `cores`, sluice may use
what the host gives it: its CPU affinity or cgroup quota on Linux, and every
core where the host sets neither, as on a Mac. sluice refuses a `cores` below
1, above the host's cores, or on a host that already limits it.

A repository whose tests bring up a stack of containers sets `max` to how
many stacks fit in memory at once.

For a branch on GitHub, do these in this order, or a step is refused:

1. Commit the policy, with `remote = "origin"` on the branch, and push the
   branch to GitHub by hand, once ([remote-branches](references/profiles/remote-branches.md)).
2. Add the branch's ruleset with the user's own account in its bypass list.
3. Create and install the GitHub App, and name it in `[forge]`
   ([github-app](references/profiles/github-app.md)).
4. Put the app in the bypass list, then remove every other entry.
5. `sluice forge check <branch>`.

For more than one host or system, follow [peers](references/profiles/peers.md)'s
"Set it up, in order": a signing key per host, `[signing]` in each host's
config, then one landing that adds `[signers]` and rewrites `trust`, then
`sluice run` kept running on each standing executor.

A bolt pattern with `remote = "origin"` needs no push by hand: `sluice branch
create bolt/<b> --from main` makes each bolt there as the app, and its
ruleset restricts creations and deletions as well as updates.

## 6. Check it end to end

1. Make a worktree on a new branch, commit a harmless change.
2. `sluice submit --check <branch>` says no conflict.
3. `sluice exec <branch>` runs the gate's tests in the worktree and passes.
4. `sluice submit <branch>` lands it; `git log -1 <branch>` shows the
   `Sluice-*` trailers.
5. `sluice overview` shows the landing under `recent`.

Then tell the agents: they submit, never merge (the `using-sluice` skill).
Remove any merge step, pre-merge test hook or post-merge check from their
scripts: sluice runs the gate on the exact combination that lands.
