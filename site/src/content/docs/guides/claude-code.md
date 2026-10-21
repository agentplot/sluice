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
/plugin marketplace add agentplot/sluice
/plugin install sluice@sluice
```

The first line adds the repository's marketplace, named `sluice`; the
second installs the plugin from it. Choose the user scope to have it in
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
- on a conflict, merges the branch in and submits again, and, told by
  the board that its work would come to conflict, merges the branch in
  while the conflict is small;
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
/plugin install sluice-board@sluice
```

<a class="screenshot-light" href="/sluice/screenshots/board-light.webp"><img src="/sluice/screenshots/board-light.webp" alt="Claude Code with the /sluice pane docked on the right: two combinations under test at 3/5 with their tasks, one submission queued, a landing and a blame; the band above the prompt and the status line below it." width="1248" height="754" /></a>
<a class="screenshot-dark" href="/sluice/screenshots/board-dark.webp"><img src="/sluice/screenshots/board-dark.webp" alt="Claude Code with the /sluice pane docked on the right: two combinations under test at 3/5 with their tasks, one submission queued, a landing and a blame; the band above the prompt and the status line below it." width="1248" height="754" /></a>

While a branch has a queue, the band above the prompt reads like
`sluice main · 3 submissions · testing main+#12+#13 2m10s/15m · #14 waiting · last #11 landed as 9ecc34a`,
and the status line says where your own submissions stand. `/sluice` opens
a pane with each branch's combinations under test, its queue with your
submissions marked and each pull request linked to GitHub, and its recent landings, blames and conflicts, with the
blamed tests' logs. A branch with nothing queued is left out once a
quarter hour has passed since its last event.

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

Your agents' own runs of `sluice exec` are checks, not submissions, and
the board shows them as load on the host, never in a branch's queue. A
check takes one of the host's runs, the same runs combinations are tested
in, so a busy host can test fewer combinations than a branch allows. When
it does, the band ends that branch's line with `· 1 slot to an agent's
checks`, and the status line says it once. The pane lists each check
under CHECKING, one row per checkout and the branch it checks against:
`fix-cards → main 3/8 ▓▓▓░░░░░░░ 1m10s` while it runs, or the command it
runs. A check that ended stays a quarter hour. `14 passed · reused when
submitted` means the gate takes those passes when that commit is
submitted, so its combination doesn't run them again; `not kept:
uncommitted changes` means the checkout held more than its commit and
nothing was kept. Under the rows, `host: 4 of 6 runs · 3 queue · 1 agent`
says where the host's runs go.

When one of the session's submissions lands, fails or conflicts, a toast
says so, and an idle session gets a turn telling it the outcome, which tests
were blamed and where their logs are, so the agent can fix its work without
being asked. The `wake` option in `/config` turns that turn off.

Before it submits, the board forecasts the session's worktree whenever its
commit, its branch or the submissions queued there move, as `sluice submit
--check` does. The work can come to conflict with what the branch has
landed since the worktree was cut, or with a submission ahead. The session
then gets a turn naming the files and telling it to merge the branch in
now, while the conflict is small. A conflict it was told of can also go
away without its commit moving, when a submission ahead fails or is
withdrawn. The session is told that too, and that a submission removed for
the conflict can go again. The `forecast` option in `/config` turns this
off; so does `SLUICE_FORECAST=0` in the session's environment, for a tool
that forecasts for its agents itself.

It reads everything from `sluice overview --json --watch`, so it needs
sluice 0.12 or later, and stays silent in a repository with no queue. It is a
Claude Code mod, a newer and still-changing interface, which is why it is a
plugin of its own.

### Naming a session's branch

The status line follows the session's worktree. When several agents share
one checkout, each working for a different branch whose submissions come
from other worktrees, the checkout can't say which branch is whose.
Whatever starts the session names it in the session's environment:

```sh
SLUICE_BRANCH=release claude
```

With no submission of its own, the status line then shows that branch:
`sluice: release · 3 queued · #12 testing 3/8 · 2 checking`. That is its
submissions in no combination under test, its oldest combination under
test, and the agents' checks held to it. An idle branch reads `release ·
nothing queued`, and one sluice has no records of `release · no
records`. The band and the
pane show that branch first, and the pane shows it even with nothing
queued. The variable is read when the session starts. In herdr the pane
token `sluice_branch` does the same, as
[the herdr guide](/sluice/guides/herdr/#naming-a-panes-branch) says.

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
