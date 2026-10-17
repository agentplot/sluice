---
number: 26
title: sluice makes and deletes the branches it lands on
status: accepted
date: 2026-10-16
decision-makers:
- Chuck Swanberg
---

# sluice makes and deletes the branches it lands on

Deciders: the user, settling with crew's design who writes a bolt's branch on GitHub, 2026-10-16.
Sources: `agentplot/blueprints:design/sluice/proposals/branches-1.md` clauses 104 to 106 and its rewrite of 45; clauses 1, 23, 24, 26, 46, 56; ADRs 0009, 0017, 0019, 0024; rules core.4, core.5, core.14, core.15.

## Context and problem statement

A bolt is a branch sluice lands on, cut from `main` for one piece of work and retired once that work has landed on `main`. With bolts on GitHub, so that agents on several hosts can share one (ADR 0017), the bolt has to exist on GitHub before anything is submitted to it. Its branch then goes once it is done. If the bolt's caller pushes and deletes it, `bolt/*` has two writers on GitHub. The rulesets must then let the caller's credentials through for creation and deletion. On a Mac those credentials go through an agent that asks a person, which an unattended caller can't wait on. And the caller can't see whether anything is still queued on the bolt. Who makes and deletes the branch, and with what guard?

## Considered options

1. The caller pushes the bolt and deletes it; `forge check` admits a second ruleset for creation and deletion, bypassed by the caller's credentials.
2. sluice makes and deletes the branches it lands on, at its caller's asking, as one more move of the branch seam under the lease it already lands with.
3. Bolts stay on the host that made them, and nothing on GitHub changes.

## Decision outcome

Option 2. `sluice branch create <branch> --from <other>` and `sluice branch delete <branch> [--into <other>]` are the caller's two commands. The branch seam gains `make` and `remove` beside `move_to`: clause 45 with an expectation of absent, or a removal. The local refs profile writes with gix, with `MustNotExist` or an expected value. The pushed refs profile pushes leased on absent (`--force-with-lease=<ref>:`) or on the revision read, and the router chooses between them. A branch not yet made has no revision of its own, so the router reads the policy at the revision it is made at, which is the policy the branch will start with (rule core.15).

`make_branch` takes the branch's lock (rule core.14). It refuses when:

- the other branch is missing, or is no branch sluice lands on;
- the policy at the other branch's revision gives the new branch no gate (56);
- the new name's records still queue submissions;
- the branch is already present.

The presence check comes before the move, because git and gix both report making a branch at the revision it already holds as success.

`remove_branch` takes the lock too. It refuses when:

- a submission is still in a chain on the branch;
- a landing is unfinished;
- the branch's revision isn't in the other branch's history, which by 23 is where a landed bolt's revision ends up.

The removal is leased on that revision (46). A pushed branch's local branch and remote-tracking ref go with it. Its records stay (26).

Neither move is reachable from the submit operation (rule core.4). Each is one expectation, reported and never retried when refused (rule core.5). With no `remote`, both work in local refs under the same rules.

Option 1 widens who may write `bolt/*` on GitHub, which the app-only ruleset exists to prevent (ADR 0019). It also leaves the caller guessing whether the bolt is still in use. Option 3 keeps bolts off every other host.

### Consequences

- `bolt/*` on GitHub has one writer for creating, updating and deleting: sluice's app, under one ruleset, as `main` does. `forge check` admits no other bypass.
- A submission to a branch its remote lacks is refused with the `sluice branch create` to run.
- crew makes and retires bolts through these commands and pushes to no branch.
- Records stay per host (C.6): a delete sees only this host's queue. A submission queued on another host meets a missing branch at its next round and is refused there, and its head is untouched.
- The statement's change is drafted as `branches-1.md` in blueprints, for the operator's ruling (rule core.12).
