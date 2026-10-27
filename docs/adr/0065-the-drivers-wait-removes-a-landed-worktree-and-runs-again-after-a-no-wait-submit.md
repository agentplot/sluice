---
number: 65
title: The driver's wait removes a landed worktree and runs again after a no-wait submit; the core never removes one
status: proposed
date: 2026-10-27
decision-makers:
- Chuck Swanberg
---

# The driver's wait removes a landed worktree and runs again after a no-wait submit; the core never removes one

Status: proposed to the user on 2026-10-27, as `briefs.15` has it; accepted only on their yes.

Deciders: madswan-design, on madswan-ops's signal of 2026-10-27 (afterthought/crew-state, madswan/main, 2026-10-27-madswan-ops-24b162ca/01-sluice-no-wait-leaves-landed-worktree) carrying the user's words, "The WLDN design agent, for some reason, is manually removing work trees. I think it's incorrectly using sluice"; the user's yes is asked on the record's card.
Sources: statement clauses 25, 26, 50 and the C.5 worktrunk row; rules core.1, core.16, core.20; `crates/sluice/src/profiles/worktrunk/wt_submit.rs` on main at 2026-10-27, which releases and removes the worktree only when its own waiting submit exits landed; records 0018, 0024, 0041 (`sluice wait`).

## Context and problem statement

A waiting `wt submit` removes the session's worktree once its head landed: it releases the place and removes it as `wt merge` would, so the repository's hooks run and nothing of the place stays. Record 0041 gave sessions `sluice submit --no-wait` and `sluice wait`, so a submission never holds an agent's session: the agent waits from its harness's background and keeps taking messages. But `sluice wait` is the core's, and the core never removes a session's worktree (clause 25, core.16): after a no-wait submit nothing of the driver's is there at the landing, so every agent that follows the briefs removes its landed worktree by hand with `wt remove`, which the user read as misuse. The statement's worktrunk row says both things, "on landing, worktrunk removes the session's place" and "with `--no-wait` the session goes on", and names nothing that joins them.

## Considered options

1. `sluice wait` removes the session's worktree on landing, as a waiting `wt submit` does.
2. The driver's wait runs again: `wt submit` run in a place whose submission stands, queued or landed and not yet removed, follows it as its waiting submit would, with the timeout and exit `sluice wait` has, and on landing releases and removes the place; `sluice wait` removes nothing.
3. Agents remove a landed worktree by hand after `sluice wait`, as the briefs have it today.
4. A round removes a session's place whose submission landed.

## Decision outcome

Option 2.

**Removing a worktree is the driver's act.** A worktree is the worktree tool's: its removal runs the repository's pre-remove and post-remove hooks and frees what the tool holds for it, and the core names no worktree tool (core.1). So the core releases a session's place only at its session's or driver's asking and never removes one (25, core.16), and `sluice wait`, a core command, keeps to that. Option 1 would put a worktree tool's work into the core, and option 4 would have a round act on a session's worktree, which core.16 refuses for the same reason record 0018 refused it for session runs.

**The driver's wait need not be the one that submitted.** What a waiting `wt submit` does after `sluice submit` returns landed is read whole from sluice's record: whether this worktree's head landed, and whether worktrunk removes on landing. Nothing in it needs the submit to have been the same process. So `wt submit` run again in a place whose submission stands does the waiting half alone: it follows the entry as `sluice wait <branch>` follows this worktree's last submission to it, takes the same `--timeout` and exits 5 while waiting so an agent runs it from the background in rounds as record 0041 has it run `sluice wait`, and on landing releases and removes the place. Run in a place whose head already landed, it removes at once. A session that submitted with `--no-wait` therefore waits and tidies through the driver, and removes nothing by hand; `sluice wait` stays what it is, the core's wait, for a session that keeps its worktree.

**Why not by hand.** Option 3 is what the briefs say today and what the user read as misuse: a step every agent repeats, a worktree left wherever one forgets, and a `wt remove` whose hooks run under the agent's environment rather than the driver's.

### Consequences

- The statement's C.5 worktrunk row says a `wt submit` run again in a place whose submission stands follows it as its waiting submit would, with `sluice wait`'s timeout and exit, and on landing releases and removes the place; `sluice wait` removes no worktree (25).
- `wt-submit` gains the resumed wait: in a place with a standing submission and no new head, it waits rather than submitting again, under `--timeout`, exiting 5 while the submission is undecided.
- The briefs that have an agent run `sluice wait` and then `wt remove` by hand say `wt submit` in rounds instead, once this is built; that is crew's and the fleet's to change, told by this record's acceptance.
