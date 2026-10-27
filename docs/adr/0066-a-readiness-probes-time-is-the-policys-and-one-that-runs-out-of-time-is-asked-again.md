---
number: 66
title: A readiness probe's time is the policy's, and one that runs out of time is asked again before the run is refused
status: proposed
date: 2026-10-27
decision-makers:
- Chuck Swanberg
---

# A readiness probe's time is the policy's, and one that runs out of time is asked again before the run is refused

Status: proposed to the user on 2026-10-27, as `briefs.15` has it; accepted only on their yes.

Deciders: madswan-design, on wldn-design's reading of 2026-10-27 relayed by swancloud-design (afterthought/crew-state, swancloud/main, signal 2026-10-27-swancloud-operator-dcf318e8/01); the user's yes is asked on the record's card.
Sources: statement clauses 84, 114, 115, 134; `crates/sluice/src/profiles/checks.rs` on main at 2026-10-27 (`READY_LIMIT`, sixty seconds, a constant; the probe run once after the toolchain, the run refused when it fails or runs out of time); records 0023 and 0062; chuck-herdr-alpha, 2026-10-27 22:15 to 22:38Z: main's combination of 847, 848 and 850 refused three times, "the host is not ready to run the checks: [tests] ready `bun run infra/ministack/host-ready.ts`" past 60 s, under a load of 47 to 152 on 16 cores, the same probe answering in 3 to 5 s by hand in a warm checkout.

## Context and problem statement

A repository may declare a readiness probe, `[tests] ready`, that sluice runs before a candidate's checks so that a host that cannot run them refuses the run rather than run it without what it needs (84). sluice gives the probe sixty seconds, a constant of its own, and treats time run out as the host not ready: the place is released, the run has no outcome and the candidate is formed again (114). On a loaded host the probe answers slowly and not no, so each refusal threw away a run that would have passed, three times in twenty minutes, and a host at its busiest is the one that never gets to run its checks.

## Considered options

1. Sixty seconds, sluice's constant, the probe asked once: as built.
2. The probe's time is the policy's to state beside it, a minute given.
3. A probe that runs out of time is asked again before the run is refused.
4. Both 2 and 3: the time is the policy's, and time run out answers nothing, so the probe is asked again as many times as the policy states, three given, before the run is refused; the refusal that stands names the probe, how long each ask ran and the host's load.

## Decision outcome

Option 4.

**Time run out answers nothing.** The retry policy's principle (134, record 0062) is to try again what was never judged and never what was answered. A probe that said no has answered: the host lacks what the checks need, and the run is rightly refused. A probe that ran out of time has said nothing, and under load that is the common case; asking again, bounded, is how sluice reads a slow answer everywhere else, and the cost of three asks is seconds against a run thrown away.

**The time is the repository's to know.** How long its probe takes is a fact of the repository's stack, as its checks' limits are (66), not of sluice: a probe that starts a container engine's client is not a probe that reads a file. The policy states it beside the probe; a minute stands where it states none, so no policy changes on this record.

**A refusal says what it saw.** When every ask ran out of time, the refusal names the probe, how long each ask ran and the host's load at the time, so an operator reading the branch's log sees a loaded host and not a broken one. The run still has no outcome and blames nobody (114), and the candidate is formed again as before.

**Why not the others.** Option 1 refuses good runs on load alone. Option 2 alone lets a policy buy time it cannot know it needs, and a probe that runs out of a longer time still throws the run away. Option 3 alone leaves the time sluice's, where a repository whose probe takes two minutes unloaded can never run.

### Consequences

- Clause 84 of the statement says the probe's time is the policy's, a minute given; time run out is asked again, three times given; the refusal names the probe, each ask's time and the host's load.
- The policy gains a time beside `[tests] ready`, read as a check's limit is; `READY_LIMIT` becomes its given.
- The refusal's words change: an operator sees the asks and the load, not only "not ready".
