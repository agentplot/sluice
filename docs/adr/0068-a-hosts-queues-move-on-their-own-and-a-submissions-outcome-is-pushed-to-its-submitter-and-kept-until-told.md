---
number: 68
title: A host's queues move on their own, and a submission's outcome is pushed to its submitter and kept until told
status: proposed
date: 2026-10-27
decision-makers:
- Chuck Swanberg
---

# A host's queues move on their own, and a submission's outcome is pushed to its submitter and kept until told

Status: proposed to the user on 2026-10-27, as `briefs.15` has it; accepted only on their yes.

Deciders: the user, in swancloud-ops's pane on 2026-10-28 (afterthought/crew-state, swancloud/main, signal 2026-10-28-swancloud-ops-4f509aa5/01-waits-are-pushed-and-reconnect-after-restart): "If our records already know about sluice submissions, crew units, and chores, then if we restarted a Claude Code session, it should be able to come back up and reconnect with those watchers (either through the mod or through some instruction)", which settles that an outcome reaches a session by a push the session picks up again from the records after a restart, and that each mechanism lives in the tool it belongs to; madswan-design for sluice's part, the shape of the driver and of the telling; the user's yes is asked on the record's card. swancloud-design holds the design across crew, Flume and sluice.
Sources: statement clauses 20, 26, 50, 51, 71, 129, 131 and the C.5 worktrunk row; rules core.4, core.11, core.25; ADRs 0011, 0041, 0056; `README.md` on main at 2026-10-27 ("No daemon": a waiting submit drives the queue while it waits, `sluice run` or `sluice step` from cron does the same for a host); `crates/sluice-core/src/records.rs` (`told` on a submission); swancloud's reading of deneb on 2026-10-28 (signal 2026-10-28-swancloud-ops-cc4c8120/01: the host saturated by watchers and waits).

## Context and problem statement

sluice keeps nothing running. A branch's rounds, the forming, testing and landing of its candidates, are run by whoever waits: a waiting `sluice submit`, a `sluice wait`, or `sluice run` where an operator started one (core.4, ADR 0011, 0041). On a host with many agents that is many drivers, each polling its own submission every few seconds and each running rounds, and the queue stops moving the moment the last of them stops. The outcome is told by the round that decides it, to the session's pane through the multiplexer (20, 50, C.5), and `sluice wait` reports it as its exit; an agent whose session restarted between its submit and the outcome has lost its wait and learns nothing until it asks again. The user asks that the queue move without the waits and that an outcome reach the agent by a push it picks up again after a restart, from the records, which already hold it.

## Considered options

1. As built: every wait drives the rounds and reads its own outcome; `sluice run` where someone starts it.
2. One standing driver per host: on a host that holds a repository's records, one process runs the rounds the host may run, started and kept by what the host binds to run it; it is the host's one reader (129), so one process per repository reads and drives; a wait or a watch runs no round while it lives and drives as today only where none does. The outcome is pushed into the submitter's session through the multiplexer and recorded as told when delivered; an untold outcome stands in the records, and a session that starts is told every untold outcome of its own submissions.
3. The gate box's driver alone (swancloud record 0039), every other host's sessions following the records over the remote.
4. `sluice step` from cron on every host, waits unchanged.

## Decision outcome

Option 2. Option 3 is the same design on the host whose policy lets it run the gate; a host that submits and reads still moves its own intake and reading.

**The queue is the host's to move, not the waiter's.** A round may be run by any process (core.4), and that stays; what changes is who does. On a host that holds a repository's records, one standing process runs the rounds the host's policies let it run, the reader of 129 extended to drive, so a repository has one process on the host reading its records and moving its queues. It is started and kept by what the host binds to run it, the fleet's service on a managed host, or the plugin's lease where nothing is bound, as the herdr plugin's watcher is kept today. A wait or a watch that finds it live follows the document it writes and runs no round; one that finds none runs rounds as a waiting submit does today, so a laptop with nothing bound is as it was. The host's bound on runs (71) is then one process's to keep, not a count across waiters.

**An outcome is pushed, recorded as told, and told again to a session that comes back.** The round that decides an outcome pushes it into the submitter's session through the host's multiplexer where it has one (20, 50, C.5) and records on the submission that it was told, and when; a `sluice wait` that reports it records the same. An outcome not yet told stands in the records as what it is, decided and untold, and a session that starts, after a restart or on a new host, is told every untold outcome of the submissions it made, found by the submitter the submission recorded, before it asks for anything; so no wait need stand for an outcome to arrive, and an agent that waits does so by choice, following the driver. `sluice wait` keeps its exits and its `--timeout` (ADR 0041) for a session that chooses to block.

**Why not the others.** Option 1 is the load the user measured, and a queue that stops when its last waiter does. Option 3 alone leaves a host that submits with no process to take its intake or read for its sessions. Option 4 keeps every wait polling and driving beside the cron, and tells nothing to a session that restarted.

### Consequences

- Clause 135 of the statement says a host's queues move on their own and how the driver is started; clause 51 says the outcome is pushed, recorded as told and told again to a session that starts; the C.5 worktrunk row says `sluice wait` follows the host's driver and runs rounds only where none lives. Rule core.31 is drawn from them.
- The README's "No daemon" paragraph says instead that sluice keeps one driver per host running where the host binds one, and nothing where it does not.
- The herdr plugin and the Claude Code board, which already follow the reader's document, carry the push into the session's pane and the telling at a session's start; what crew does with a told outcome is crew's, as the wrapper.
- The submission record's `told` names when and to which session the outcome was delivered; a record without it is an untold outcome.
