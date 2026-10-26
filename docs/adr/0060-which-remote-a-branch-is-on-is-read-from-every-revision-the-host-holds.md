---
number: 60
title: Which remote a branch is on is read from every revision of it the host holds
status: proposed
date: 2026-10-26
decision-makers:
- Chuck Swanberg
---

# Which remote a branch is on is read from every revision of it the host holds

Status: proposed to the user on 2026-10-26, as `briefs.15` has it; accepted only on their yes.

Deciders: madswan-design, on swancloud-design's ruling of 2026-10-26 that sluice's half of deneb's levelling is where `sluice branch create` reads a branch's policy, and on madswan-planner's signal 2026-10-26-madswan-planner-ec418792/01; the user's yes is asked on the record's card.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 56, 104, 121, 124, C.7 pushed refs; ADRs 0017, 0026, 0053; rules core.4, core.5, core.15, core.20; `crates/sluice/src/profiles/branches.rs` `remote`, `remote_if_made_from` and `policy_of` as origin/main held them on 2026-10-26 (ef6d414b); crew's rule `plan.18`.

## Context and problem statement

A branch whose policy names a remote is the remote's revision, and the host's `refs/heads/<branch>` is sluice's copy, which sluice reads nothing from (124, ADR 0053). But which remote a branch is on is itself in the policy, at the branch's revision, which sluice cannot fetch before it knows the remote. The routing binding answered this by reading the policy at the host's own ref of the branch, and at each remote's tracking ref only where the host had no ref of that name, taking the copy's lag as enough to choose the profile.

On 2026-10-26, on deneb, `sluice branch create bolt/<bolt> --from main` refused crew's bolt on agentplot-kit and site-theme. The host's `refs/heads/main` in each stood from before the repository had a policy file at all, since no landing of theirs had ever been pushed from deneb, so the routing found no remote named, routed `main` to local refs, read the host's lagging revision as the branch, and found no policy there to gate the bolt by. The tracking ref `refs/remotes/origin/main`, which the fleet's timer fetches within minutes of each landing, held the policy naming origin. swancloud took the half that is its own, moving the bare repository's default branch when it is a fast-forward and no worktree holds it, and named this half sluice's: a bolt cut for a branch whose remote is origin reads the policy at origin's default, as crew's own reads of main do (`plan.18`), never at the host's lagging main.

Where is the remote's name read, so that a copy that lags or has no policy never decides that a branch is local?

## Considered options

1. From the policy at every revision of the branch the host holds, its own ref and each remote's tracking ref as last fetched: the branch is on the remote any of them names; a revision that lags or holds no policy decides nothing against one that names a remote; revisions naming different remotes are refused, naming them; no fetch is made to decide it.
2. From the host's own ref alone, falling to the tracking refs only where the host has no ref of that name, which is what was built.
3. By fetching the branch from every remote the repository names and reading the policy at each remote's revision before routing.

## Decision outcome

Option 1.

**Every revision the host holds is read.** The routing reads the policy at the host's own ref and at each remote's tracking ref of the branch, and the branch is on the remote any of them names. A revision whose policy names no remote, or that has no policy, is a lagging reading and settles nothing against one that does: a branch is local only when no revision the host holds names a remote. Where two revisions name different remotes the routing refuses, naming both, since no reading of the host's can say which the branch is on. The remote's name chosen, the branch's revision is the remote's, fetched (C.7 pushed refs), and the gate and every move are read under the policy at that revision (56, 104, rule core.15), as before.

**No fetch to decide the routing.** Every read of a branch sluice lands on in local refs, `sluice status` and the overview among them, stays off the network; the tracking refs the host holds are what its own fetches, sluice's pushed refs reads and the fleet's timer last wrote. A host that holds no revision of a branch at all reads it as absent, as before.

**The transition into a remote keeps its path.** A branch landed on in local refs whose policy gains `remote` names it on the host's own ref before the remote has it; option 1 routes it to the remote, whose older copy is then read as a branch whose reading and records disagree, held until the revision read contains the last landing or the operator accepts it (121, C.7), as the statement already describes.

Option 2 is what refused the bolt: a host that never lands a repository has a copy that never moves, and a copy from before the policy file names no remote, so the branch reads as local on exactly the hosts that rely on the remote. Option 3 would put a fetch of every named remote in front of every read of every repository that has a remote configured, local refs repositories included, and would make the routing's answer depend on the network being up.

### Consequences

- `sluice branch create <bolt> --from main` on a host whose copy of main lags or predates the policy makes the bolt at origin's revision of main, under origin's policy, with no levelling of the copy first.
- The routing binding's `remote`, `remote_if_made_from` and `policy_of` read every revision the host holds in one order, and the refusal for two remotes named is new, with a test of its own.
- The conformance suite gains a scenario for a host copy from before the policy file (124).
- Rule core.27 is drawn from this record.
