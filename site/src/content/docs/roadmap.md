---
title: Roadmap
description: What is built, and what is designed and not yet built.
---

Each host runs its own queue today. sluice lands its own `main` this
way, gated on every test tier in a sandbox.

## Built

| | macOS | Linux |
|---|---|---|
| The queue, worktrees, checks, branch protection | ✓ | ✓ |
| Release binaries | Apple silicon | x86-64, ARM |
| Sandboxed checks | ✓ Seatbelt | planned: bubblewrap, Landlock |
| moon as the task runner, or one command | ✓ | ✓ |
| A build cache a checkout starts from | ✓ | ✓ |
| Telemetry over OTLP, opt-in | ✓ | ✓ |

## Designed, not built

- **Sandboxed checks on Linux**, with bubblewrap and Landlock, and memory
  and core limits by cgroup
  ([ADR 0012](/sluice/decisions/0012/)).
- **Several hosts sharing one branch's queue**, as peers coordinating
  through the git remote or through a central server:
  [many hosts](/sluice/many-hosts/overview/).
- **Checks on several architectures**, where a branch names the systems
  each test must pass on: [architectures](/sluice/many-hosts/architectures/).
- **A GitHub Actions hybrid** for repositories that stay on GitHub, as an
  adoption path: [GitHub Actions](/sluice/many-hosts/github-actions/).

The many-hosts design is under review in
[agentplot/blueprints](https://github.com/agentplot/blueprints) and
nothing of it binds the code until it is ratified there.
