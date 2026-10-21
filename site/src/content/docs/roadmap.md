---
title: Roadmap
description: What is built, and what is designed and not yet built.
---

Each host runs its own queue until a policy names signers; then hosts
share one queue on the git remote. sluice lands its own `main` on one
host, gated on every test tier in a sandbox.

## Built

| | macOS | Linux |
|---|---|---|
| The queue, worktrees, checks, branch protection | ✓ | ✓ |
| Release binaries | Apple silicon | x86-64, ARM |
| Sandboxed checks | ✓ Seatbelt | planned: bubblewrap, Landlock |
| moon as the task runner, or one command | ✓ | ✓ |
| A build cache a checkout starts from | ✓ | ✓ |
| An agent's own test runs, under the host's bound | ✓ | ✓ |
| Previews of what a branch landed, held until released | ✓ | ✓ |
| Telemetry over OTLP, opt-in | ✓ | ✓ |
| Many hosts sharing one queue on the git remote, with signed results: [executors and systems](/sluice/guides/executors-and-systems/) | ✓ | ✓ |
| A check required on several systems | ✓ | ✓ |

## Designed, not built

- **GitHub Actions as a pool of machines**: a job per run no host has
  room for, rounds run in a job so a submission lands with every host
  off, results signed by the workflow itself, and a check per system on
  each pull request.
- **Sandboxed checks on Linux**, with bubblewrap and Landlock, and memory
  and core limits by cgroup
  ([ADR 0012](/sluice/decisions/0012/)).
