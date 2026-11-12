---
number: 46
title: Status reads a branch where its records last saw it
status: accepted
date: 2026-10-16
decision-makers:
- Chuck Swanberg
---

# Status reads a branch where its records last saw it

Deciders: the user, reviewing how fast sluice's reads are on a busy branch, 2026-10-16.
Sources: `boomdrivedev/blueprints:design/sluice/requirements.md` clauses 26, 49, 61; ADR 0023; rules core.11, core.14, core.15.

Amended 2026-11-06 by the construct stage of `change/a-quiet-branch-reads-its-remotes-policy`, below: the policy is read at the branch's revision as the host holds it.

## Context and problem statement

`sluice status <branch>`, `sluice status <id>`, the text of `sluice submit --status` and every loop of `sluice run` read a branch's queue through the core's `status`, which read the branch's revision from the branch seam. For a branch on a remote, that read is a fetch: on this repository's `main`, about 0.75 s of every `sluice status main`'s 0.9 s. `sluice overview` reads the same branch at the revision its records last saw, `base`, and never fetches (ADR 0023). Must a status read fetch?

## Considered options

1. `status` reads the branch at `base`, the revision its records last saw, and reads the branch itself only for a branch whose records have seen no revision.
2. `status` keeps fetching, and a flag skips the fetch.
3. `status` keeps fetching.

## Decision outcome

Option 1. A status read is a read of the records (26, 49, 61): its state and the verdicts read for it, which a fetch does not change. The revision it names is the one the queue was formed on, the same one `sluice overview` names, so the two never disagree. A commit the branch has taken since is seen by the next round, which reads the branch as it always has; nothing a round, a gate or a landing decides reads `status` (rule core.15). The `parallel` shown is read from the policy at `base`, as the overview's is, and decides nothing.

Option 2 makes the slow read the default for every agent that does not know the flag. Option 3 costs a network request for every status read, on every loop of `sluice run`.

### Consequences

- `sluice status <branch>` on this repository's `main` reads in about 0.03 s where it took 0.9 s.
- A branch with no records yet, or none that have seen a revision, is still read from the branch, which is how `status` tells a branch with no records from no branch at all.
- The revision `status` names can be behind the remote's branch until the next round reads it.

## Amendment, 2026-11-06: the policy is read at the branch's revision as the host holds it

Status: accepted on 2026-11-06 by the construct stage of `change/a-quiet-branch-reads-its-remotes-policy`, as its own decision. Clauses 56, 124 and 129 already read a branch's policy at its revision, so this amendment squares the record with them. It keeps what the user decided here, a status read that fetches nothing, changes only the revision the policy is read at, and is cheap to change.

Deciders: the construct stage of `change/a-quiet-branch-reads-its-remotes-policy`.

Sources: `boomdrivedev/blueprints:design/sluice/requirements.md` clauses 26, 56, 124, 129, 135 and 137, as main words them at b0df70b; record 0093; record 0069; rules core.15, core.27 and core.31; the signal `afterthought/crew-state:agentplot/main signals/2026-11-06-agentplot-ops-706fa293/01-sluice-reads-a-quiet-branchs-policy-at-its-last-round`; deneb's records of agentplot/flywheel-next's `main` in the person's copy, `~/Code/github_agentplot/flywheel-next/flywheel-next.git/sluice/main/`, and the reflog of its `refs/remotes/origin/main`; `crates/sluice-core/src/ops.rs` (`status`) and `crates/sluice-core/src/overview.rs` (`branch_view_of`, `overview`) as main held them at 0b15d250; the change `a-quiet-branch-reads-its-remotes-policy`, its design D1 to D3.

### Context

This record put the policy a status read shows at `base`, the revision the branch's records last saw, on the ground that it "decides nothing". A policy sluice refuses there does decide something: the branch cannot be read. The overview's `branch_view_of` reads the policy at `base` too, and serves the overview, `sluice` with no command, and what a watch or a wait shows. The overview's order reads the default branch's policy at its view's `at`, which is `base`. And `base` moves only at a round, which runs on a branch only while it has work in play (135).

On deneb, agentplot/flywheel-next's `main` keeps its records in the host's files and lands on origin. Its `base` was `2924aa2f`, whose policy holds a `[[checkout.cache]]` table that sluice has refused since 0.45. This host's `origin/main` took the user's clean `f2ce8e3f` by a fetch at 2026-10-28T15:12:12Z. Every submit and status on `main` refused until a hand-run `sluice step main` logged `moved` at 2026-11-06T16:46:34Z. Record 0069 has a clean policy pushed to the remote end the refusal "at the next round", and on a quiet branch nothing runs that round.

### Considered options

Where a read outside a round reads the policy (the change's design D1, with D3 for the overview's order):

1. **At the branch's revision as the host holds it** (chosen). That is `Branch::held`: the branch's own ref, or for a branch on a remote its tracking ref as last fetched, read with no process. A read uses `base` only where the host holds no revision, and status uses the branch itself only where neither is known. The queue and `at` stay where the records last saw them. The overview's order reads the default branch's policy at the revision its view reads.
2. **At `base`, as decided before.** A quiet branch is refused for as long as its last round's revision is, and nothing short of a round clears it, against 124's refusal "read at the branch's revision on its remote, where a push clears it".
3. **A status read that fetches.** This record declined it, at 0.75 s of a 0.9 s status on this repository's `main`, and a view would pay it too.
4. **At `held` only where `base`'s is refused.** That is two readings, and a view whose gate comes from one revision while its refusal comes from another.
5. **A refused branch counted as in play, so sluice's host process runs its round.** The process would then never end while the refusal stood, fetching the branch at every tick, against 129 and 135. And a status on a host where no process runs would still refuse.

What brings the clean revision to the host (design D2), where nothing is added and each sluice command reads the branch where it reads it today:

6. **Status fetching where the policy at `held` is refused.** It would end the refusal at a status read even where nothing on the host has fetched. It departs from this record's ruling, the user's, that a status read fetches nothing, and the one read that takes work, a submit's, already fetches.
7. **Something of sluice's fetching a refused quiet branch on its own.** Under 129 no branch is fetched on its own until a round forms on it, and under 135 nothing of sluice's runs while no queue has work in play. Either would need a clause amended, for a read a submit already makes.

### Decision outcome

Option 1, for `sluice status`, a branch's view and the overview's order, and so for every reader; options 6 and 7 are not taken. Status reads the queue where its state last saw it, and the policy at the branch's revision as this host holds it, fetching nothing. It is a read of what the host already holds, so it starts no process, as this record wanted.

A clean policy pushed to a branch's remote ends sluice's refusal of that branch the next time a sluice command reads the branch, so submit and status take work again without anyone running a round. A submit reads the branch fetched (124), so the next submit after the push takes the work, whatever the host held before it. Status, the overview and what a watch or a wait shows read the branch whole once any fetch on the host has brought the clean revision: a submit's, a round's, another tool's, or the person's own `git fetch`. Where the host held the clean revision already, as deneb did for nine days, the next read is whole.

This asks nothing of sluice's to run for a quiet branch. Under record 0093 sluice's host process ends once no queue it holds has work in play (135), and a branch's requests are taken in only where its records are on its remote, by the forge's jobs (137).

### Consequences

- A quiet branch's refusal ends with no round: at the next read once the host holds the clean revision, and at the next submit in any case.
- A status read's `parallel` and warnings, and the view's executor and required checks, are those of the revision the next round forms on, which can be ahead of `at` until that round. The overview names both revisions (64).
- A quiet branch that nothing on the host has fetched since the push still shows the refusal at the revision it last fetched, naming it, until the next fetch. A submit to it is never refused for that revision.
