---
number: 69
title: a revision's policy decides where its branch is however much else of it this sluice refuses
status: accepted
date: 2026-10-28
decision-makers:
- Chuck Swanberg
---

# a revision's policy decides where its branch is however much else of it this sluice refuses

Status: accepted by the user on 2026-10-28, on their card ("Approved").

Deciders: the user, on the record's card (2026-10-28): "Approved"; madswan-design for the shape, on madswan-planner's question of 2026-10-28 after the user's push ("124 says a lagging revision decides nothing against one that names a remote; a lagging revision whose policy cannot be parsed should decide nothing either, else a hand push can never clear a refusal on any host") and on slc-2-conductor's finding of the same day (afterthought/crew-state, madswan/main, signal 2026-10-28-slc-2-conductor-3f0e396d/01-one-branchs-refused-policy-stops-every-wait).
Sources: statement clauses 26, 56, 124, 133 and S64 in agentplot/blueprints `design/sluice/requirements.md` (as amended 2026-10-28); rules core.11, core.14, core.27; ADRs 0049, 0059, 0060; on main at c76cd3be: `crates/sluice/src/profiles/branches.rs` (`routing`, `policy_at`), `crates/sluice/src/profiles/records.rs` (`route`), `crates/sluice-core/src/lib.rs` (`find`), `crates/sluice-core/src/ops.rs` (`state`, `moved_to`), `crates/sluice/src/lib.rs` (`Command::Run`); deneb on 2026-10-28 under sluice 0.51.3: `sluice overview` and `sluice wait 250` both exiting 3 on `bolt/a-run-compiles-through-a-shared-cache`'s policy at 4f6fe55f.

## Context and problem statement

Where a branch's records are kept is read from its policy (C.6): on its remote when the policy names signers and a remote for the branch, in the host's own files otherwise. Record 0060 settled that this routing reads the policy at every revision of the branch the host holds, its own ref and each remote's tracking ref as last fetched, so a host's own ref from before the policy file decides nothing against a tracking ref that names a remote (124, core.27). As built, the routing parses each revision's policy whole: a policy this sluice refuses anywhere in it, a table it no longer keeps among them, refuses the routing, and with the routing every read, lock and write of that branch's records.

On 2026-10-28 deneb's sluice 0.51.3 refused `bolt/a-run-compiles-through-a-shared-cache`'s policy at 4f6fe55f for its `[[checkout.cache]]` table, which ADR 0049 retired. The refusal did not stay with that branch. `sluice wait 250`, a wait on another branch, looks its submission up by number across every branch's records and exited 3 at once on the refused one; `sluice run` reads every branch's status in turn and ended on it; `sluice overview` failed whole, which slc-1's fix under clause 26 (`overview-reads-past-a-refused-policy`) answers for the view alone, and which clause 26, amended the same day, now answers for every lookup, wait, status read and round: a branch sluice cannot read is refused alone. That amendment decides between no alternatives and is not this record's.

The user then pushed a clean policy to both branches on GitHub, and deneb refused still: the host's own ref of the bolt, and flywheel-next's main checkout, lag the remote; the routing reads their policies too; and sluice levels a host's copy only at a landing it reads, which needs the routing first. A hand push could clear nothing on any host whose copy lagged. This record decides what a revision's refused policy decides of where its branch is.

## Considered options

1. As built: each revision's policy is parsed whole for the routing; one this sluice refuses refuses the branch, and whatever touches the branch's records fails with it, until the host's copy is levelled, which only a landing does.
2. A revision whose policy this sluice cannot parse decides nothing of where the branch is, as a revision that lags or holds no policy does (the planner's proposal).
3. The routing reads each revision's policy only as far as the routing needs, the remote named for the branch and whether signers are named, as the protection already reads it (ADR 0059); what else in the policy this sluice refuses is that branch's refusal, read at the branch's revision on its remote, where a push clears it.

## Decision outcome

Option 3.

**Routing reads what routing needs.** The question 124 asks of a revision is two fields wide: the remote its `[branches]` table names for the branch, and whether it names signers. The protection has read policies that narrowly since ADR 0059, passing every other field over, and the routing reads them the same way: every revision the host holds keeps its vote on the remote, a revision this sluice refuses elsewhere in its policy votes as the others do, and only a revision whose remote line or signers table itself cannot be read refuses the routing, naming it. Once routed, the branch's revision is the remote's, fetched (124), and its policy there is read whole for its gate and every move of it (56): a `[[checkout.cache]]` table on the remote's revision refuses that branch, as it should, and a clean policy pushed there ends the refusal at the next round. The host's lagging copy decides nothing by its other fields, and is levelled by the landing it next reads, as 124 says.

**Why not option 2.** A refused revision that decides nothing is silent. Where it is the one revision naming the remote, a tracking ref fetched before the host's own ref existed, or a bolt cut from a main whose policy this host's sluice is older than, the branch routes to the host's own files: its records are read and written where no other host sees them, and nothing says so. A loud refusal is wrong once and names itself; a quiet misrouting is wrong on every write after and names nothing. Option 3 keeps every revision's vote and confines the refusal to what refused.

**Why not as built.** It cannot clear. A refused copy is levelled only by a landing the host reads, the reading needs the routing, and the routing is what the copy refuses; the operator's only way out is a hand write to the copy, which 124's protection refuses, or standing the protection aside, which record 0056 declined to make a practice. And the refusal reached every other branch's wait, run and view, which clause 26 now forbids on its own.

### Consequences

Good: a policy change that retires a table, as ADR 0049 did, strands no host whose copies lag; a push to the remote clears a refused branch everywhere; the routing and the protection read a policy the same way, one reading with two callers.

Bad: a revision's policy is read in two forms where the branch is routed to a remote, the narrow one for the routing and the whole one at the remote's revision for the gate; the second read is the one 56 always made.

Neutral: rule core.27 is amended with the narrow reading; statement clause 124 and scenario S64 carry it.

## More information

The unit this record calls for, queued on its acceptance: the routing reads each revision's policy only as far as it needs, and a revision this sluice refuses elsewhere decides the remote all the same. The companion unit under clause 26, that a branch sluice cannot read is refused alone, is queued from the clause and waits on no card.
