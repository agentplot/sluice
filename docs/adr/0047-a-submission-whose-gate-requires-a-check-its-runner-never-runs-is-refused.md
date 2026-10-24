---
number: 47
title: A submission whose gate requires a check its runner never runs is refused
status: accepted
date: 2026-10-23
decision-makers:
- Chuck Swanberg
---

# A submission whose gate requires a check its runner never runs is refused

Deciders: the operator, planning what a submit says of a gate that can never pass, 2026-10-23.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 2, 15, 27, 56; rules core.1, core.2, core.15, core.20; ADR 0044.

## Context and problem statement

The moon runner runs moon in its CI mode (ADR 0044), so a task a repository marks `runInCI: false` or `runInCI: skip` is never run, and a branch that requires one reads it missing on every combination (15). On 2026-10-21 and 2026-10-22 one kit's `main` required such a task, and every landing on it failed. The submissions were queued, tested and failed one after another. sluice warned only in `sluice status` and a waiting submit, read from the latest run, so after a run had failed, and a submit with `--no-wait` never showed it.

A submit can refuse the submission before it is queued, naming the task. Where does that refusal read whether the runner keeps the task out of CI?

## Considered options

1. Ask the runner in the submitting worktree, which holds exactly the head (2), for the tasks it keeps out of CI among the checks the branch's gate requires, read from the branch's current revision (56).
2. The records: refuse when the branch's latest run said a required task was kept out.
3. A checkout of the branch's revision, made for the purpose, and the runner asked there.
4. Read `moon.yml` from the head with git, without moon.

## Decision outcome

Option 1. The runner's setting is the tree's, and the combination carries the submission's tree, so the submitting worktree is where the setting the run will meet is read. The gate and the line of `.config/sluice.toml` each required entry is on are read from the branch's current revision, as every gate is (56). The worktree gives only the runner's own setting, which nothing the gate counts reads.

The check seam gains one operation: given a worktree, the policy's `[tests]` and the required checks, the tasks the runner keeps out of CI, each with the required entry that names it. The moon profile answers it with `moon query tasks`, set up as a run is: the run's environment and `[tests] toolchain`, its script in a scratch directory of sluice's state. A run's plan and this operation read the tasks through one function, so the two cannot differ. Every other runner keeps nothing out.

The refusal names each task, the tag it is required through when it is, the policy's line, and the way on: let the task run in CI in the worktree, or drop it from the policy in the same submission. It holds for every submission while the policy requires the task, whatever the submission affects: the policy is what is wrong, and a refusal that came and went with the files a submission touches would blame some submissions and not others for the same mistake.

Clause 27 has sluice admit every submission a driver makes, whether work may be submitted at all being decided before it reaches sluice. This refusal is made where a dirty or detached worktree's is (2), before the head is published or an id taken, and on a branch that takes pull requests before anything is pushed.

Option 2 says nothing before the first run, and refuses the submission that repairs the branch, since the latest run predates it. Option 3 is slow on every submit and refuses the repair for the same reason: the branch's revision still keeps the task out. Under either, a change that only drops the requirement from the policy is judged by the policy it changes (56) and refused too, so the branch could never be repaired through sluice. Option 4 reads moon's configuration a second way: task inheritance, tags and the forms of `runInCI` are moon's to resolve, and that reading would drift from the one a run's plan uses.

### Consequences

- The submission that lets the task run in CI is taken, and its combination runs the task.
- When the runner cannot be asked, its toolchain, its install or its query failing, the submission is taken with a warning saying why. The run then reads the task missing, as before. Refusing instead would make every submit hinge on the toolchain working in the session's worktree, for a check that exists to name a mistake early.
- Every submit to a branch whose runner is moon runs the toolchain and `moon query tasks` in the worktree: seconds where the toolchain is already built, what `sluice exec` runs there already. A branch whose runner is a command asks nothing.
- The query writes only what moon and the toolchain write in the worktree, ignored by git. It moves and rewrites nothing of it (rule core.20), and leaves it clean, so the next submit is not refused as dirty (2).
- `sluice status` and a waiting submit still warn from the branch's log. A submission can pass the refusal and still meet a kept-out task when an entry ahead of it changes `moon.yml`.
- Rules core.1 and core.2 hold. Only the moon profile names moon; the core's refusal and the CLI's words say "the task runner". Rule core.15 holds: the gate is read from the branch's revision. Its details name the runner's setting, read from the submitting worktree at submit, as not ruled out.
- A pull request opened by hand is taken in by a round, with no worktree to ask the runner in, and is tested and fails as before. `sluice submit --check` and the forecast form a submission without writing one and do not ask either.
