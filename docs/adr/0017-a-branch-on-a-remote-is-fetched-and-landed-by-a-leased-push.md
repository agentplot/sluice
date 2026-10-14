---
number: 17
title: A branch on a remote is fetched and landed by a leased push
status: accepted
date: 2026-10-14
decision-makers:
- Chuck Swanberg
---

# A branch on a remote is fetched and landed by a leased push

Deciders: the user, planning the change `main-on-github` and ruling on how a read meets rule core.4, 2026-10-14.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 1, 24, 45, 46, 56, C.4, C.6; `design/sluice/proposals/pull-requests-1.md` (C.4 pushed refs, C.7 `remote`); ADRs 0003, 0005, 0009, 0011.

## Context and problem statement

sluice lands on a branch in one host's repository, and the branch a team shares lives on a forge. An operator pushed sluice's `main` by hand after every landing, and a push from anyone else went unseen until that hand push failed. The statement's pushed refs profile of the branch seam (C.4) reads the branch from its remote and lands with a push that succeeds only if the branch has not moved. How does an instance bind it, beside local refs, without the core learning that some branches are remote?

## Considered options

1. A pushed refs profile wrapping local refs, and a router per branch that sends each branch's calls to it when the branch's policy names a `remote`.
2. One profile for every branch an instance serves, chosen when the instance starts.
3. A `sync` method on the branch seam, which the core calls before each round.
4. A mirror: land on the local branch as today and push it after each landing.

## Decision outcome

Option 1. `[branches."<pattern>"] remote` names the git remote a branch lives on. The router reads it from the policy on the local branch's revision, which may lag the remote's; that is enough to choose the profile, and the gate itself is read from the revision the profile reads (rule core.15). A `remote` the repository has no remote of that name for is refused by name.

The pushed refs profile holds a local refs profile and the remote's name. Forming, committing, ancestry and reading files go to local refs, since every object is on the host. `read` fetches the branch into its remote-tracking ref on every call and returns the fetched revision: one set of profiles serves every step of `sluice run`, so a revision kept between calls would outlive its step (rule core.14). Reading never moves the local branch, so the submit operation, which reads the branch, writes no branch (rule core.4). `move_to` pushes with `--force-with-lease` on the expected revision; once the remote accepts, the local branch is set to the landing commit. A lease refusal is reported with what the remote holds, never retried blind (rule core.5). Any other failure is an error, which leaves the landing intent for the next step to settle by fetching. The git command fetches and pushes, since it carries the host's credential helpers and SSH setup; gix still moves the local branch, so the protection hook never sees it (ADR 0009).

Option 2 cannot serve a repository with local bolt branches and a shared `main`. Option 3 teaches the core that some branches are remote, and `read` is already where a round learns the branch. Option 4 lands before the remote agrees: a push from elsewhere is found only after the landing, which is then on no shared branch.

### Consequences

- Only the branch reaches the remote; the state stays in local files (C.6).
- A landing found on the remote by a later round, after a step stopped between the push and recording it, is recorded without moving the local branch; the local branch catches up at the next landing.
- A round that cannot fetch lands nothing and fails; checks already running keep running, since their supervisor is not the step.
- The tests use a bare repository on the host as the remote; git leases, refuses and fetches against it as against a forge.
