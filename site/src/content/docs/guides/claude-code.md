---
title: The Claude Code plugin
description: Install sluice's Claude Code plugin, have Claude install sluice and set up a repository, and let your agents submit, read results and run tests through it.
sidebar:
  order: -2
---

The plugin teaches Claude Code to use sluice: to install it, to set a
repository up for it, and to land work through it instead of merging. It
lives in sluice's own repository, so it changes with the commands it
describes.

## Install

At the prompt of a Claude Code session in a terminal:

```
/plugin install sluice --marketplace agentplot/sluice
```

Answer `y` to add the marketplace, then choose the user scope to have it in
every session.

## Set up a repository

In the repository, ask:

```
set up sluice here
```

Claude checks for the `sluice` binary first. If it is missing or older than
the plugin needs, Claude shows how it would install it and asks first:
through nix where nix is present, else the release binary for your platform,
else `cargo install`. A sluice that comes from your own flake or
home-manager is yours to update, and Claude says so rather than touching it.

Then Claude reads what the repository already uses and proposes a
`.config/sluice.toml` to match:

- moon, or one test command;
- a sandbox for the tests, on macOS;
- ports and build caches for the checkouts;
- worktrunk's `wt submit` in place of `wt merge`;
- a branch on GitHub, sluice's own GitHub App, and pull requests.

It asks which branches to protect and what must pass on each, shows the file,
and walks through `sluice protect`, the host's `[runs]` settings and, for
GitHub, the app and `sluice forge check`. It ends with one change landed
through sluice to prove the setup.

## What your agents do with it

In a repository with a `.config/sluice.toml`, Claude:

- submits with `sluice submit` or `wt submit` and never merges or pushes to
  a protected branch;
- commits everything first, and checks for a conflict with `--check`;
- on a failure, reads the blamed tests and their logs, reproduces them with
  `sluice exec`, fixes and submits again;
- on a conflict, merges the branch in and submits again;
- runs its tests while it works with `sluice exec`, and cleans up with
  `sluice release`;
- brings up a landed revision with `sluice preview` when you ask to try it.

## Kept off protected branches

Two hooks come with the skills.

At the start of a session in a repository whose policy protects branches,
Claude is told which ones, and that work lands on them only through
`sluice submit`.

Before Claude runs a shell command, a guard checks it. A `git push` to a
protected branch, a `git merge` or `git update-ref` onto one, or a
`gh pr merge` into one is refused, with the reason naming the
`sluice submit` command to run instead. Every other command, its own
branch's push included, goes through. The guard reads which branches are
protected from the policy committed in the repository. sluice's own git
hook already refuses a local update; the guard says why before git does,
and covers a push to GitHub and a pull request merge, which that hook
never sees. It needs `jq`, and lets everything through without it.

## The queue in your session

A second plugin, `sluice-board`, shows the queue inside the session:

```
/plugin install sluice-board --marketplace agentplot/sluice
```

While a branch has a queue, the band above the prompt reads like
`sluice main · 3 submissions · testing main+#12+#13 2m10s/15m · #14 waiting · last #11 landed as 9ecc34a`,
and the status line says where your own submissions stand. `/sluice` opens
a pane with each branch's combinations under test, its queue with your
submissions marked and each pull request linked to GitHub, and its recent landings, blames and conflicts, with the
blamed tests' logs.

Press a blame, or a failed test's `log of` row, to open it under its row:
each failed test, its log path (press it to copy the path) and the log's
last lines. Press it again to close it. The pane only shows: it never
moves to another session or another agent's pane.

With the moon runner, a combination under test also shows how many of
its tasks have ended and a bar of the same, as in
`testing main+#12+#13 3/5 ▓▓▓▓▓▓░░░░ 2m10s/15m`. The pane lists each task
below it: `✓` passed, `✗` failed, `…` running and `·` waiting, with how
long each ended task took. Each round updates it, every few seconds
while a submission waits, and it counts tasks, not the tests inside one.
It shows only for combinations tested on this host.

When one of the session's submissions lands, fails or conflicts, a toast
says so, and an idle session gets a turn telling it the outcome, which tests
were blamed and where their logs are, so the agent can fix its work without
being asked. The `wake` option in `/config` turns that turn off.

It reads everything from `sluice overview --json --watch`, so it needs
sluice 0.12 or later, and stays silent in a repository with no queue. It is a
Claude Code mod, a newer and still-changing interface, which is why it is a
plugin of its own.

## Worktrees

Ask about worktrees or a bare repository and the plugin gives the layout
sluice suits best: the repository's git directory kept bare, with `main` and
every branch as worktrees beside it. It converts a normal clone or a `.bare`
directory to it step by step, showing the plan first, moving nothing that an
agent or an editor has open, and deleting nothing.

## What it includes

| skill | when Claude uses it |
|---|---|
| `setting-up-sluice` | installing sluice, writing or changing `.config/sluice.toml`, GitHub, pull requests |
| `using-sluice` | submitting, waiting, reading a failure or conflict, `sluice exec`, previews |
| `worktrees` | the bare repository and worktrees layout, conversions, repairs |

and two hooks: the session note and the guard on shell commands.

Each skill points at the pages of this site for the full story:
[configuration](/sluice/reference/configuration/),
[commands](/sluice/reference/commands/) and the guides.

Without the plugin, the [walkthrough](/sluice/guides/walkthrough/) sets a
repository up by hand.
