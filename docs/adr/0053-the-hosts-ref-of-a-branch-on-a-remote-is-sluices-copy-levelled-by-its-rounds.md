---
number: 53
title: The host's ref of a branch on a remote is sluice's copy, levelled by its rounds
status: accepted
date: 2026-10-23
decision-makers:
- Chuck Swanberg
---

# The host's ref of a branch on a remote is sluice's copy, levelled by its rounds

Deciders: the user, after a bolt's landing worktree on deneb could not submit to `main` and every fetch's packing of refs was refused there, 2026-10-23, and a fetch into a bare repository run from outside it aborted on another host, 2026-10-24.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 1, 121, 122, 124, C.4 pushed refs, S58, S64; ADRs 0017, 0024, 0048; rules core.1, core.2, core.4, core.5, core.11, core.14, core.20.

## Context and problem statement

A branch whose policy names a remote is read by a fetch and landed by a leased push (ADR 0017). The host's own `refs/heads/<branch>` was set once, right after this host's landing push, and never again. On 2026-10-23, crew's landing worktree on deneb had a bolt checked out two landings behind GitHub's. Its `wt submit main` pushed the worktree's branch, forced, for the pull request: a submit pushing a branch sluice lands on (1). The protection hook then judged git's own write of that ref, and every `git reset --hard` and `git pack-refs` that left the host's ref where it stood, against GitHub's revision, which it fetched on every ref transaction, and refused them all. On another host, a `git --git-dir=<bare repository> fetch` run from outside the repository started the hook in a folder that was no repository; the hook could not open one, and its failure aborted every ref update of the fetch.

Clause 124 says the remote's revision is the branch and the host's ref is sluice's copy of it: levelled to each landing sluice reads, retried until it holds, written at no other time; a submit from a worktree on a branch sluice lands on pushes nothing and submits the remote's revision; any other hand's write to the copy is refused, judged by the copy's own value; and the protection judges the repository git names and lets through, said once, what it cannot read. Where does the copy's answer live, who writes it and when, where is a copy left behind recorded, what does a submit from such a worktree submit, and how does the hook judge?

## Considered options

1. The branch seam answers for the host's copy apart from the branch and levels it from an expected value; only a round levels it, to the landing its records hold; a copy left behind is in the branch's records per host; the submit takes the remote's revision from a worktree on a branch sluice lands on; the hook reads the copy's own value in the repository git names.
2. The pushed refs profile levels the copy inside its `read`, to the revision it fetched.
3. `move_to` keeps setting the copy after the push, and a failure there is retried by the next landing.

## Decision outcome

Option 1.

**The seam answers for the copy.** The branch seam gains `copy(branch)`, which answers `Itself` when the branch has no copy apart from it on this host, or `At(value)`, the host's ref and its value; and `level(branch, expected, to)`, which sets the copy to `to` only while it holds `expected`, or is absent when `expected` is none. Local refs answer `Itself`, since there the host's ref is the branch, and refuse a levelling. Pushed refs answer `At` from `refs/heads/<branch>` with no fetch, and level it with gix from exactly the value expected, a reflog line `sluice: level <branch> to its landing`; a copy found elsewhere is reported, never forced (rule core.5). The routing binding sends both by the branch's policy, as it sends `read`. The core reaches the copy through the seam alone and has no branch for a profile (rules core.1, core.2). A landing's push no longer sets the copy: an accepted push is a landing, whatever the copy does after it (122).

**Only a round levels, to the landing.** In the round's decide step, once the branch is not held (121; ADR 0048), the round reads the copy and, when it is `At` a value other than the branch's last landing, levels it from that value to the landing. The landing's confirmation does the same for the landing it records, after the write that records it, and a failure there never fails the landing (122). The target is the landing the branch's state holds, not the revision read: a hand's commit on top of a landing, which the remote's protection let through (S8), is no landing, and the copy waits at the landing until sluice lands on top of it. A held branch levels nothing; once the operator accepts a revision, the landing is that revision and the next round levels the copy to it. Nothing else writes the copy: not the submit, not `sluice status`, not the hook. A waiting submit or `sluice wait` levels only through the rounds it runs, as any process may run a round (rule core.4). The levelling runs under the branch's lock the round holds and reads the copy just before it sets it, so a copy moved in between is found elsewhere and tried again by the next round.

**A copy left behind is in the branch's records, per host.** `branch.json` gains `copies`, keyed by the instance's name, absent when empty: for each host whose levelling failed or found the copy elsewhere, the value its copy holds, the landing it lacks, why, and since when. The round writes this host's entry in its own write and logs `copy-behind` once per distinct value, landing and why, as `held` is logged once per revision read. A round that finds the copy at the landing, or levels it, takes this host's entry out and logs `copy-levelled` when there was one. Each host writes only its own entry, so a branch whose records are on its remote carries every host's lagging copy and no host overwrites another's (rules core.11, core.14). `sluice status <branch>` shows this host's entry above the queue and under `copy` in JSON, and nothing once it is gone.

**A submit from a worktree on a branch sluice lands on submits the remote's revision.** A worktree's branch is one sluice lands on when the policy at the branch's revision names it. When that branch's copy is `At`, the submission's head is the remote's revision, and where the worktree's head, the host's copy, differs, the submit says so with both revisions and whether the copy is behind, ahead of or apart from the remote, in text and under `copy` in JSON. A copy behind or ahead refuses no submission (124). The pull request driver pushes nothing for such a branch: it finds or opens the request from that branch as the remote holds it, and uses the remote's revision for the new request's title, the withdrawal of an older head's submission and the ready mark. A worktree on a branch sluice lands on nowhere is pushed and submitted at its head, as before. A name the driver gives that is no branch, such as `pull/7`, reads as none and keeps the session's head. The submit writes no ref of a branch sluice lands on, on the host or the remote (rule core.4).

**The hook judges the copy's own value, in the repository git names.** The hook reads each protected branch's value on the host: the copy's for `At`, and the branch's own for `Itself`, which for local refs is the host's ref. Neither fetches. A transaction that leaves every protected branch at its value passes, git's packing and its prune of a loose file it has just packed, and a `git reset --hard` to where the branch stands, among them; one that changes a protected branch's value, or deletes it while a submission still needs it, is refused, logged `protected` with the value it held, the one the write would have set and the command, as before. Whoever ran the command is told, before git's own `fatal:`, that the branch is protected by sluice, the command and both revisions, that sluice levels the host's copy at each landing for a branch with a copy, and how to bring the worktree level. The hook opens the repository at `GIT_DIR` when git sets it, resolved against the folder the command ran in when relative, and at that folder only when git set none, and takes the common directory from the branch seam, not from git run in that folder. A transaction the hook cannot read for a reason of its own, opening the repository or reading a branch's value, policy or what still needs it, passes with one warning, `warning: sluice's branch protection let this through unread: <why>`, whatever the number of refs it holds.

Option 2 needs the branch's last landing in the profile, or levels the copy to a hand's commit; and it writes a ref on every read, the submit's among them (rule core.4). Option 3 leaves every host but the one that landed with a copy that never moves, and a landing that failed to set it with a copy behind until the next landing, which 124 rules out.

### Consequences

- A worktree that has checked out a bolt has its branch moved under it by each levelling, as on the landing host before, and `git status` there shows the landings as staged reverts; sluice names the worktree and moves nothing in it (ADR 0024, rule core.20).
- A copy carrying commits a hand made before the protection, ahead of the landing but holding it, is moved back to the landing. Those commits stay in the reflog and in any branch holding them. A copy ahead with landings the remote lacks is held before any levelling (121, S57).
- The submit from a worktree whose copy is ahead submits the remote's revision, leaving the copy's own commits out, and says so with both revisions.
- A policy the hook cannot parse leaves its branches unguarded on that host until it is fixed; the warning says why on every such write, and the forge's rules still guard a branch on a remote.
- git prints its own `fatal:` after a hook refuses; sluice's lines come first.
- Each host writes its copy's lag into the branch's records only on a failure and once per distinct cause; a host whose levellings succeed writes nothing new.
- `sluice/branch@1` is unchanged: `copies` is absent by default, and an older sluice ignores it.
- C.4's pushed refs row still says a submit from a worktree on that branch pushes the worktree's head for its request; this follows 124, and the row is the statement's to restate.
