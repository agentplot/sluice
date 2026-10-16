---
number: 23
title: A reader sees the queue through one overview, and plugins live outside the core
status: accepted
date: 2026-10-16
decision-makers:
- Chuck Swanberg
---

# A reader sees the queue through one overview, and plugins live outside the core

Deciders: the user, settling the shared contract between the Claude Code plugin plan and the herdr plugin plan, and building the OpenSpec change `overview`, 2026-10-16.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 26, 49, 52, 61 to 63; ADRs 0003, 0008, 0018, 0020, 0021; rules core.1, core.11, core.14, core.15.

## Context and problem statement

Two plugins want to show sluice's queues as a CI server shows its tests: a Claude Code plugin, in a band above the prompt and a pane, and a herdr plugin, in its panes. Each needs, for every branch of a repository, the combinations under test, the submissions still queued, the latest landings, blames and conflicts, and the worktrees and previews held. `sluice status <branch> --json` gives a branch's submissions alone; the rest is in the records, in files whose layout is the records profile's. A plugin that reads those files bypasses the records seam (rule core.11) and breaks under a records profile that is not files. Neither plugin can tell its own session's submissions apart either: a submission's `from.worktree` is the branch it came from, not the worktree.

The plugins themselves name moon, worktrunk, herdr and GitHub, which rule core.1 forbids outside a profile. Where do they live, and what do they read?

## Considered options

1. Each plugin reads `<common-dir>/sluice/` itself.
2. Extend `sluice status` with more fields and a `--branches` form.
3. One new reader, `sluice overview`, printing one document with a watch form, and the plugins in this repository under `plugins/`.

## Decision outcome

Option 3. `sluice overview` reads every branch with records through the records seam, and the worktrees and previews held through the place seam, and prints them as text or, with `--json`, as one document, `sluice/overview@1`, whose fields are only ever added. `--json --watch` prints the document as one line at once and again whenever anything in it but the time it was read changes, reading about once a second. A reader finds what happened by comparing each branch's `recent`, the last ten `landed`, `blamed` and `conflict` lines of its log, kept exactly as written, so the log stays the one place an event's shape is defined.

It is a reader only. It takes no lock, holds no decision and writes nothing (rule core.14), and it releases nothing, which is why it lists previews through the core rather than through `sluice previews`, which tidies as it lists. A branch is read at the revision its records last saw, `base`, never fetched: a watch that read a remote branch's tip would fetch every second. The gate's figures it shows, `parallel`, the time limit and the required tests, are read from the policy at that revision (rule core.15), and decide nothing. The computation from a branch's records is a pure function in the core, `overview::branch_view`, tested over the fakes.

A submission's `from` gains `path`, the absolute path of the worktree it came from, written by the worktrunk driver at submit and left out by the pull-request driver, which has no worktree. A record without it reads as before.

`status` keeps its three forms unchanged. Option 2 would grow one command into four documents with different shapes, and its branch form fetches a remote branch on every read. Option 1 couples every plugin to the files profile and copies the schema twice.

The plugins live in this repository under `plugins/`: `plugins/sluice` and `plugins/sluice-board` for Claude Code, with the marketplace file at the repository's root, and `plugins/herdr` for herdr. They version with the commands they describe. No crate imports anything under `plugins/`, so rule core.1 does not reach it, and nothing under `crates/` names their tools.

### Consequences

- Rule core.1's details name `plugins/` as not ruled out.
- The CLI gains `sluice overview`; the host interface a command runs against gains the host's name and the repository's path.
- The submission record's `from` gains an optional `path`; an older sluice ignores it.
- A combination's `targets` are the tests its branch requires, not the subset a run found affected, and its `executor` the one the policy names, not the one that held the run: both are read from the policy, not from the run.
- The repository index behind a host-wide `--all`, and a preview's `host`, wait on the herdr previews panel.
