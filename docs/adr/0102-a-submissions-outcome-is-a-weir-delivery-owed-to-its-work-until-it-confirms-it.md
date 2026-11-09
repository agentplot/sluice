---
number: 102
title: A submission's outcome is a weir delivery, owed to its work until it confirms it
status: accepted
date: 2026-11-08
decision-makers:
- Chuck Swanberg
- the construct stage of change/sluices-inbox-is-weirs-delivery-form
---

# A submission's outcome is a weir delivery, owed to its work until it confirms it

Status: accepted. The direction is the user's, on agentplot-design's card of 2026-11-08, in agentplot/weir record 0006: "one shared inbox in weir next", sluice moving onto weir's crate in a change of its own. How sluice takes the form, below, is the construct stage's, under the statement as amended for it, and each part is cheap to change.

Deciders: the user, 2026-11-08, in agentplot-design's pane, among the words record 0006 quotes: "Is it actually tracking what was read?", "Is it done well?", and "So would we move a shared crate under Weir so that it's loose and Flume could use the same inbox system?"; then, on agentplot-design's card that day, "Runner as one more recipient now; one shared inbox in weir next" (agentplot/weir record 0006). The construct stage of `change/sluices-inbox-is-weirs-delivery-form` for the rest.

Sources: `agentplot/blueprints:design/sluice/requirements.md` 50, 51, 124, C.5 and S76, S77 as amended on the branch `design/sluices-outcomes-are-weirs-deliveries`, 3bb8688; `design/weir/requirements.md` 100 to 113, C.7, C.8, I9 and S22 to S25 at 3c92bcb; `design/principles.md` 12 (proposed 2026-11-08, from the user's words that day); agentplot/weir record 0006 and its crates `weir-delivery` and `weir-pack` at 52b9552; agentplot/flume's `crates/flume/src/profiles/merge_queue/sluice/mod.rs` (`inbox`), which reads `sluice inbox --json`; this repository's records 0068, 0070, 0085, 0099 and 0100 and rules core.31, core.48 and core.50; `crates/sluice-core/src/ops.rs` (`Pending`, `pending_on`), `crates/sluice-core/src/step.rs` (`perform_outbox`), `crates/sluice-seams/src/lib.rs` (`Delivery`, `Driver::notify`), `crates/sluice/src/lib.rs` (`inbox`, `delivering`) and `plugins/sluice-board/hooks/register.tsx` (`collect`).

## Context and problem statement

sluice marks a submission's outcome delivered on the word of whatever carrier put it before the session: herdr took the notice into the pane, the pull request driver posted it, `sluice inbox` printed it, or a wait reported it. The mark is then final. An agent whose session stops after the notice lands, or after the inbox printed, and before it acts, never sees the outcome again, since every later inbox answers nothing for it. Nothing marks an outcome nobody collects, so the blame of a worktree removed before its session read it stays pending in the branch's state for good, and no reader shows it. Flume has the same shape for the ends of its work. weir record 0006 gives both one form: a message recorded once under a key with its readers, owed to each until that reader confirms it by its key, a hand-over confirming nothing by itself, and an owed message expiring after the host's span and shown, never dropped. weir's `weir-delivery` crate holds the four records and the calls, `weir-pack` the cases every deliverer's store passes. How does sluice take it?

## Considered options

**Who the reader is.**

1. **The submission's work as sluice records it, `from.worktree`: the worktree's branch, or `pull/<n>` for a request intake took in (chosen).** It is the name sluice already finds a submitter by: a worktree's inbox collects for its branch and for each request opened from it where sluice lands that branch nowhere (record 0085). It survives a restart and another host. A submission by name has no reader and records no delivery (124; core.50).
2. The session, as `delivered_to` names it today. Not taken: a restarted session has another pane, so nothing would ever be owed to it.
3. The agent. Not taken: one agent works in many worktrees, and a worktree's next agent would not collect what its last left.

**Where the deliveries are kept.**

- a. **A file of the branch's live records, `deliveries.jsonl`, C.8's JSON spelling, each record on its own line in the order added (chosen).** It is read and written under the branch's lock as every live record is, so it is on the remote for a branch whose records are (core.11) and a session on another host collects from it.
- b. C.8's recutils spelling. Not taken: sluice's records are JSON throughout (core.41), and C.8 gives JSON for a store on a machine; recutils is weir's shared branch's.
- c. One store for the host. Not taken: a session on another host could not read it (core.11).

**Which hand-overs confirm on receipt.**

- i. **None but the pull request's post and a caller that says so (chosen).** The round's notice, `sluice inbox` and a wait hand over and confirm nothing. The pull request driver's post confirms on receipt, since the request keeps it for whoever reads the request and nothing else collects for it. `sluice inbox --confirm` and `sluice wait --confirm` confirm on receipt for a caller that keeps the outcome for readers of its own, as Flume's merge-queue profile does, and `wt submit` confirms a landing on receipt where it removes the worktree, since the reader goes with it.
- ii. A session's start confirms what its inbox prints, as weir 0006 allows. Not taken as the default: it is the case the user's "Is it actually tracking what was read?" asks about. A caller that wants it says `--confirm`.

**How a reader confirms.** `sluice confirm <key>...`, run where the reader works, confirms each key for the folder's readers, refusing a key no branch holds and one owed to another reader; `--as operator` confirms for the delivery's own reader, for a worktree gone before it confirmed. A submit from a worktree that withdraws its earlier blamed or conflicting submission confirms that outcome for its reader, which has acted on it by submitting again. Not taken: a flag of `sluice inbox` alone, which a notice could not name in one command for one outcome.

**When an outcome is handed over again.** The round pushes its notice at most once, and none for an outcome already handed to its reader or confirmed. `sluice inbox` hands over everything still owed each time it runs; `sluice inbox --new` only what no way has handed its reader, for a harness that starts a turn on each new outcome and must not repeat the notice the round pushed.

**The span.** The host's `[deliveries] expire_after`, a duration as sluice's settings write one, `"168h"` where unset, a week as weir's example has it. It is fixed when the round that decides the outcome records it, by that host's setting.

**The records sluice kept before.** Under principle 12 (proposed, from the user's words of 2026-11-08: "I want the system to be built the way it should be and then have agents fucking fix the data transition as necessary"), sluice reads no `delivered`, `delivered_to`, `told` or `told_to` into the form and writes none, and keeps no code that records a delivery for an outcome decided before it. The log keeps the `delivered` and `pending` lines it holds, since no written line changes (core.41), and reads them as history. The change names the move as an agent's step: on each host, before the upgrade, the decided submissions whose records hold no `delivered` and that are a worktree's or a request's are listed with the sluice that wrote them, and after it, each is recorded as a delivery owed to its reader, recorded at its decision's time, so an old one shows expired.

## Decision outcome

Option 1, a, i, `sluice confirm`, the hand-overs again as above, the host's span and the agent's move.

- sluice takes its first dependency on weir: `weir-delivery` by a git revision of agentplot/weir, and `weir-pack` for the test that holds the branch's store to the pack's delivery cases (weir 113).
- The round records the delivery where it writes the outbox item today: key `<id>:<outcome>`, the notification's id, reader `from.worktree`, message the notice, `recorded_at` the round's time. The notice's words move to sluice-core so the round writes them.
- `Driver::notify` answers a hand-over with its way, where it reached and whether its reader confirmed on receipt; the core records the hand-over, and the confirmation where it was one.
- A done delivery is let go of whole, its hand-overs and confirmations with it, by the write that moves its submission to the finished records, or the first write after it is done where its submission has moved already. A name a reader confirmed under stays known.
- `sluice status <branch>` shows what is owed on the branch, each reader with its deliveries, owed or expired, their hand-overs and whether the name is known; `sluice status <id>` and the submission documents `sluice wait` and `sluice inbox` print hold the submission's `delivery` in place of `delivered` and `delivered_to`.

## Consequences

- An agent confirms each outcome once it has acted on it; one that does not is handed it again by its next `sluice inbox`. The session note, the skill and the notice say how.
- Flume's merge-queue profile reads `sluice inbox --json`, which no longer confirms: until it passes `--confirm`, as Flume's change `flumes-inbox-is-weirs-delivery-form` is to, every settle reads the same outcomes again. sluice's release that carries this change waits on that change being ready to take it.
- The inbox's JSON loses `delivered` and `delivered_to`, which nothing in the family reads, against the inbox's promise that it only gains fields; the spec says so from this change on.
- Rule core.31 and core.50 are amended and rule core.52 added.
