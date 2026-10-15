---
number: 12
title: The runner caps runs and the executor holds them
status: accepted
date: 2026-10-12
decision-makers:
- Chuck Swanberg
---

# The runner caps runs and the executor holds them

Deciders: the user, reviewing `docs/plans/phase-1-integrations.md`, 2026-10-12.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 28, 29, 71, 83, 84 and C.8; ADRs 0005, 0010, 0011; `docs/plans/phase-1-questions.md`.

## Context and problem statement

Three crew teams on three bolt branches, each with a window of four, can start twelve test runs on one machine at once, each branch within its own limit. sluice keeps nothing in memory between steps (ADR 0005) and any process may run a round (ADR 0011), so no one process sees every branch's runs. Separately, the shared branch's gate trusts only confined runs (29), and a run's memory and network must be held to what its branch allows. How does sluice cap concurrent runs, and how does it hold a run to its limits and know that it did?

## Considered options

1. A machine-wide semaphore of lock files under the user's state directory, taken by every process that starts a run; limits and sandboxing left to an opaque shell prefix in the policy.
2. Kernel ceilings alone (a cgroup over every run) with no count of runs.
3. The instance that starts runs caps them, counted from the records; a machine is capped by running one `sluice run` that every submission goes to. Limits and confinement belong to an executor that is part of the check seam, enforces what the policy states and reports what it enforced.

## Decision outcome

Option 3. The cap belongs to whoever starts the runs, which is the deployment the statement already describes for a shared runner (C.8): one `sluice run`, reached by `wt submit --remote`, on the same machine or another. Nothing is built to make independent processes agree on a count. Where each session runs its own rounds, the branches' windows are the only limit, and that is the choice of running that way.

Option 1 builds machinery to work around having no runner, and its opaque shell hides confinement from the gate that must trust it. Option 2 lets every run start under a ceiling sized for a few, so each runs slower, hits its time limit or its memory ceiling, and the failure is read as the newest entry's fault.

### Consequences

- Clause 71 binds an instance: "An instance of sluice starts no more checks at once than its own bound." `Tests::bound` is the bound; the core starts no more than it less the candidates under test in the records of the branches the instance serves.
- Clauses 83 and 84 add `execute` to the check seam: a run is held to the cores its instance gives it and the memory, network and writable paths its policy states; an executor that cannot enforce a limit refuses the run; every run records what was enforced.
- The executor profiles are `open` (a process group), `sandbox` (Seatbelt on macOS, bubblewrap and Landlock on Linux, with memory and core limits on Linux by cgroup) and `container`. The policy names one under `[executor]`.
- `[tests] shell` only enters the repository's toolchain; sluice reads nothing from it.
