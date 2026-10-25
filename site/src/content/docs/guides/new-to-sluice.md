---
title: New to sluice
description: What sluice is, what it leaves to other tools, where your agents' worktrees come from, and what batch and parallel do.
sidebar:
  order: -3
---

Four things to know before setting sluice up. Each part names the guide
that says the rest.

## What sluice is

sluice is a CI server for merging agents' work, with no server to keep
running. An agent finishes its work in a worktree and submits it to a
branch. sluice tests it on top of everything queued ahead of it, and
lands it only when that combination passes.

Each branch's checks are written in `.config/sluice.toml`, under
`[branches]`, much as a CI workflow names what runs on a branch. sluice
reads that file from the branch itself, never from a submission, so no
submission can loosen the rules it is tested under.
[Setting up sluice](/sluice/guides/walkthrough/) shows it on one
machine.

## What sluice leaves to you

sluice is no agent orchestrator: it starts no agents, gives them no
work, and makes no worktree for an agent. Something else runs the
agents, such as Claude Code, herdr or a crew of your own, and tells each
one to submit through sluice when its work is done. With
[the Claude Code plugin](/sluice/guides/claude-code/) installed, Claude
submits through sluice in any repository with a `.config/sluice.toml`.

## Where worktrees come from

Your agents' worktrees come from Claude Code itself, in three ways
([Claude Code's worktrees page](https://code.claude.com/docs/en/worktrees)):

- `claude --worktree <name>` starts a session in a new worktree under
  `.claude/worktrees/`.
- Asked to work in a worktree during a session, Claude makes one.
- Subagents run in worktrees of their own when you ask for it, or when a
  custom subagent sets `isolation: worktree`.

[worktrunk](/sluice/guides/worktrunk/) and `git worktree add` work too.
The only checkouts sluice makes are its own: one for each combination
under test, and one for each preview.
[Worktrees beside sluice](/sluice/guides/worktrees/) says how your
worktrees keep up with what landed.

## Batch and parallel

- `parallel`: how many combinations a branch tests at once; with `batch = false`, sluice tests each submission as it arrives, on the bet that those ahead pass.
- `batch = true`: one combination of everything queued, landed whole when it passes, with a blamed submission looked for only on a failure; for a suite of a quarter hour or more.

[Parallel and batch](/sluice/guides/parallel-and-batch/) shows both
over time, and the host's `[runs] max`, which keeps several agents'
suites from swamping one machine.
