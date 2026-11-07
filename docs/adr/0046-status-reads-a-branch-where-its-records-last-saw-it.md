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
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 26, 49, 61; ADR 0023; rules core.11, core.14, core.15.

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

Status: accepted on 2026-11-06 by the construct stage of `change/a-quiet-branch-reads-its-remotes-policy`. Clauses 56, 124 and 129 already read a branch's policy at its revision, so this amendment squares the record with them. It keeps what the user decided here, a status read that fetches nothing, and is cheap to change.

Deciders: the construct stage of `change/a-quiet-branch-reads-its-remotes-policy`.

Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 26, 56, 124, 129 and 136, at a4eeec3; rules core.15, core.27 and core.31; record 0069; the signal `afterthought/crew-state:agentplot/main signals/2026-11-06-agentplot-ops-706fa293/01-sluice-reads-a-quiet-branchs-policy-at-its-last-round`; deneb's records of agentplot/flywheel-next's `main` in the person's copy, `~/Code/github_agentplot/flywheel-next/flywheel-next.git/sluice/main/`, and the reflog of its `refs/remotes/origin/main`; `crates/sluice-core/src/ops.rs` (`status`, `in_play`), `crates/sluice-core/src/overview.rs` (`branch_view_of`, `overview`) and `crates/sluice/src/profiles/github/pulls.rs` (`take_in`) as main held them at 0b15d250; the change `a-quiet-branch-reads-its-remotes-policy`.

### Context

This record put the policy a status read shows at `base`, the revision the branch's records last saw, on the ground that it "decides nothing". A policy sluice refuses there does decide something: the branch cannot be read. The overview's `branch_view_of` reads the policy at `base` too, and serves the overview, `sluice` with no command, the reader's document and a wait's view. Intake reads the queue through `status`. And `base` moves only at a round, which `sluice run` and a reader's tick never run on a branch with nothing in play (129).

On deneb, agentplot/flywheel-next's `main` takes pull requests, keeps its records in the host's files and lands on origin. Its `base` was `2924aa2f`, whose policy holds a `[[checkout.cache]]` table that sluice has refused since 0.45. This host's `origin/main` took the user's clean `f2ce8e3f` by a fetch at 2026-10-28T15:12:12Z. Every submit and status on `main` refused until a hand-run `sluice step main` logged `moved` at 2026-11-06T16:46:34Z. Intake fetched `main` and then read the queue at `2924aa2f`, so it turned away every request. Record 0069 has a clean policy pushed to the remote end the refusal "at the next round", and on a quiet branch nothing runs that round.

### Considered options

1. **The policy is read at the branch's revision as the host holds it** (chosen). That is `Branch::held`: the branch's own ref, or for a branch on a remote its tracking ref as last fetched, read with no process. A read uses `base` only where the host holds no revision, and status uses the branch itself only where neither is known. The queue and `at` stay where the records last saw them.
2. As decided before: the policy at `base`. A quiet branch is refused for as long as its last round's revision is, and nothing short of a round clears it.
3. A status read that fetches. This record declined it for its cost, and the reader's tick would pay it too, against 129.
4. The policy at `held` only where `base`'s is refused. That is two readings, and a view whose gate and refusal come from different revisions.
5. A refused branch counted as in play, so a driver runs its round. That needs a driver, against rule core.31, and a fetch every tick, against 129. A status or a watch on a host with no driver still refuses.

### Decision outcome

Option 1, for `sluice status`, a branch's view and the overview's order, and so for every reader. It is a read of what the host already holds, so it fetches nothing and starts no process, as this record wanted. A clean policy pushed to a branch's remote ends sluice's refusal of that branch the next time a sluice command reads the branch once this host holds the clean revision, so submit and status take work again without anyone running a round. Where the branch's records are on its remote, the host holds it after the next tick, whose one ask brings every branch there (129). Otherwise it holds it after the next submit's read of the branch, which fetches it (124), a round, or any other fetch. Intake is no way to it: its own reads of the policy at the revision this host holds refuse before it fetches, and they stay as they are, since record 0093 removes intake from the host.

The option not taken is a tick fetching a refused branch on its own at the discovery interval. Clause 129 fetches no branch on its own until a round forms on it or its intake is due. The one read such a fetch would serve is a submit's, which already reads the branch fetched (124).

### Consequences

- A quiet branch's refusal ends with no round, wherever the host has the clean revision.
- A status read's `parallel` and warnings, and the view's executor and required checks, are those of the revision the next round forms on, which can be ahead of `at` until that round. The overview names both revisions (64).
- A quiet branch whose records are on the host and which nothing has fetched since the push still shows the refusal at the revision it last fetched, and takes no pull request in. Its next submit takes the work.
