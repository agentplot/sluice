---
number: 68
title: sluice stays a plain command line that needs no daemon; a host may run one driver, which waits follow; a submission's outcome is pushed to its submitter and kept until told
status: proposed
date: 2026-10-27
decision-makers:
- Chuck Swanberg
---

# sluice stays a plain command line that needs no daemon; a host may run one driver, which waits follow; a submission's outcome is pushed to its submitter and kept until told

Status: proposed to the user on 2026-10-27, as `briefs.15` has it, and revised on 2026-10-28 on their note; accepted only on their yes.

Deciders: the user, on the record's first card (2026-10-28): "We want Sleuth to be easy for new people to adopt and telling them they have to set up a daemon is probably not what I want. It should be able to work as a simple CLI", which settles that nothing of sluice's need run for sluice to work and that no adopter is told to set up a daemon; the user, in swancloud-ops's pane the same night (afterthought/crew-state, swancloud/main, signal 2026-10-28-swancloud-ops-4f509aa5/01-waits-are-pushed-and-reconnect-after-restart): "If our records already know about sluice submissions, crew units, and chores, then if we restarted a Claude Code session, it should be able to come back up and reconnect with those watchers (either through the mod or through some instruction)", which settles that an outcome reaches a session by a push the session picks up again from the records after a restart; madswan-design for the shape; the user's yes is asked on the record's card. swancloud-design holds the design across crew, Flume and sluice (crew record 0046).
Sources: `README.md` on main at 2026-10-27, "No daemon": sluice keeps nothing running itself, a waiting submit drives the queue while it waits, `sluice run` or `sluice step` from cron does the same for a host; statement clauses 20, 26, 50, 51, 71, 129, 131 and the C.5 worktrunk row; rules core.4, core.11, core.25; ADRs 0011, 0041, 0056 (which declined a per-host service for the same reason); `crates/sluice-core/src/records.rs` (`told` on a submission); swancloud's reading of deneb on 2026-10-28 (signal 2026-10-28-swancloud-ops-cc4c8120/01: the host saturated by watchers and waits).

## Context and problem statement

sluice has always kept nothing running: a repository's owner installs a command line, writes a policy, and submits. A branch's rounds, the forming, testing and landing of its candidates, are run by whoever waits, a waiting `sluice submit` or a `sluice wait`, and `sluice run` or `sluice step` from cron is there for a host that wants its queues moved while nobody waits (core.4, ADR 0011, 0041). That is the principle a new adopter meets first, and it stands.

The fleet's hosts strain it. On deneb many agents each poll their own submission every few seconds and each run rounds, and the queue stops moving the moment the last of them stops. The outcome is told by the round that decides it, to the session's pane through the multiplexer (20, 50, C.5), and `sluice wait` reports it as its exit; an agent whose session restarted between its submit and the outcome has lost its wait and learns nothing until it asks again. The user asks that the outcome reach the agent by a push it picks up again after a restart, from the records, which already hold it; and that whatever the fleet does about its waits, sluice itself must work as a simple CLI with no daemon to set up.

The first draft of this record made a standing driver per host the design, started by what the host binds. The user read it as a service every adopter would be told to run, which is what the README has refused from the start.

## Considered options

1. As built: every wait drives the rounds and reads its own outcome; `sluice run` where someone starts it; the outcome told once by the deciding round and otherwise found by asking.
2. A standing driver per host as the design, started by what the host binds, a service on a managed host or a plugin's lease (this record's first draft).
3. sluice stays a plain command line: nothing of sluice's need run, and a host that runs nothing moves its queues by its waiting submits and waits as today. Where a host chooses to run `sluice run`, by hand, from cron, by a fleet's service or by a plugin's lease, it is the host's one reader (129), and a wait or a watch that finds it live follows it and runs no round. The outcome is pushed into the submitter's session through the multiplexer where one exists, recorded as told when delivered, and told again by a command to a session that starts with it untold.
4. The gate box's driver alone (swancloud record 0039), every other host's sessions following the records over the remote.

## Decision outcome

Option 3.

**A plain command line first.** Nothing in a repository's policy or a host's configuration requires anything of sluice's to run. A host with nothing running moves its queues as it always has: a waiting submit or a `sluice wait` runs the rounds (core.4), and a laptop, a new adopter's or anyone's, is as it was. The README's "No daemon" stands as written. This is the principle; what follows is an option a host may take.

**A host may run one driver, and waits follow it.** A host may run `sluice run`, by hand, from cron, by a fleet's service or by a plugin's lease; the fleet's hosts will, as swancloud decides for them. Where one runs it is the host's one reader (129), so a repository has one process on the host reading its records and moving its queues; a wait or a watch that finds it live follows the document it writes and runs no round, and the host's bound on runs (71) is that one process's to keep rather than a count across waiters. A wait that finds none runs rounds as before. Nothing asks the adopter to know which case they are in.

**An outcome is pushed, recorded as told, and told again to a session that comes back.** The round that decides an outcome, whatever process ran it, pushes it into the submitter's session through the host's multiplexer where it has one (20, 50, C.5) and records on the submission that it was told, and when; a `sluice wait` that reports it records the same. An outcome not yet told stands in the records as decided and untold, and a session that starts, after a restart or on a new host, asks by one command for every untold outcome of the submissions it made, found by the submitter the submission recorded, and is told them; so no wait need stand for an outcome to arrive, and an agent that waits does so by choice. `sluice wait` keeps its exits and its `--timeout` (ADR 0041) for a session that chooses to block, and for every host where that is all there is.

**Why not the others.** Option 1 leaves the fleet's load as measured, a queue that stops when its last waiter does, and a restarted session that learns nothing. Option 2 is right for the fleet and wrong as sluice's design: it told every adopter to run a service, which the README refused and record 0056 refused again. Option 4 alone leaves a host that submits with no process to take its intake or read for its sessions.

### Consequences

- Clause 135 of the statement states the principle first, that nothing of sluice's need run, and then the option: a host may run one driver, which is its reader, and waits and watches follow it. Clause 51 says the outcome is pushed, recorded as told and told again to a session that starts. The C.5 worktrunk row says `sluice wait` follows a driver where one lives and runs rounds itself where none does. Rule core.31 is drawn from them.
- The README's "No daemon" paragraph stands and gains one sentence: where `sluice run` is running on a host, waits and watches follow it and run no rounds of their own.
- Binding `sluice run` as a service on the fleet's hosts is swancloud's (crew record 0046); sluice ships nothing that starts it.
- The herdr plugin and the Claude Code board, which already follow the reader's document, carry the push into the session's pane and the telling at a session's start; what crew does with a told outcome is crew's, as the wrapper.
- The submission record's `told` names when and to which session the outcome was delivered; a record without it is an untold outcome.
