---
number: 66
title: No readiness probe runs before a run; a check makes itself ready, and the probe tells a host's fault from the code's after a failure
status: proposed
date: 2026-10-27
decision-makers:
- Chuck Swanberg
---

# No readiness probe runs before a run; a check makes itself ready, and the probe tells a host's fault from the code's after a failure

Status: proposed to the user on 2026-10-27, as `briefs.15` has it, and revised the same day on their note; accepted only on their yes.

Deciders: the user, on the record's first card (2026-10-27): "this just seems to smell. a test could just make itself ready too", which settles that a check readies itself and no probe stands before a run; madswan-design, on wldn-design's reading of 2026-10-27 relayed by swancloud-design (afterthought/crew-state, swancloud/main, signal 2026-10-27-swancloud-operator-dcf318e8/01), for the shape; the user's yes is asked on the record's card.
Sources: statement clauses 66, 84, 114, 115; `crates/sluice/src/profiles/checks.rs` on main at 2026-10-27 (`READY_LIMIT`, sixty seconds; the probe run before the plan, the run refused when it fails or runs out of time; the probe run again after a failed run for 114); chuck-herdr-alpha, 2026-10-27 22:15 to 22:38Z: main's combination of 847, 848 and 850 refused three times, "the host is not ready to run the checks: [tests] ready `bun run infra/ministack/host-ready.ts`" past 60 s, under a load of 47 to 152 on 16 cores, the same probe answering in 3 to 5 s by hand in a warm checkout.

## Context and problem statement

A repository may declare a readiness probe, `[tests] ready`. sluice runs it twice: before a candidate's checks, refusing the run when it fails or outruns sixty seconds, so that a host that cannot run the checks does not run them; and after a run that did not pass, so that a check which failed because the host lost what it needs is read as the host's fault, the run having no outcome and nobody blamed (114). On a loaded host the pre-run probe answers slowly and not no, so each refusal threw away a run that would have passed, three times in twenty minutes.

The first draft of this record kept the pre-run probe and gave it the policy's time and retries. The user read that as a smell: a test can make itself ready, as it does on a laptop, and a probe with budgets and retries in front of it is machinery around a question the test answers for itself.

## Considered options

1. The pre-run probe as built: sixty seconds, sluice's constant, asked once.
2. The pre-run probe kept, its time the policy's and time run out asked again before the run is refused (this record's first version).
3. No probe before a run: a check brings up what it needs and waits for it within its own limit (66); the probe is asked only after a run that did not pass, to tell the host's fault from the code's (114).

## Decision outcome

Option 3.

**A check readies itself.** What a check needs, a container engine answering, a stand-in up, a stack listening, the check brings up and waits for, as its test harness does wherever it runs; a wait that outlasts the check's limit fails the check as any slow check fails (66). sluice adds no question of its own in front of it, so no run is refused for a host slow to answer a probe, and there is no budget and no retry to tune.

**The probe's one job is attribution.** When a check did not pass, sluice asks the repository's probe whether the host still has what the checks need; a host that lost it makes the run one with no outcome, run again and blamed on nobody (114). That is the only moment the answer changes what sluice records, and it is kept. A probe that runs out of time there is read as the host not ready, which errs toward blaming nobody, the safe side.

**Why not the others.** Option 1 refuses good runs on load alone. Option 2 tunes the refusal rather than asking whether it belongs: a budget the policy states and three asks are machinery in front of a question the check already answers for itself, and a probe that passes a second before the checks start proves nothing about the minutes after.

### Consequences

- Clause 84 of the statement no longer has the host's readiness refuse a run; it says the probe is asked only after a run that did not pass, and never before one.
- `[tests] ready` keeps its meaning for 114 and gains no time or count; `READY_LIMIT` bounds only the after-run ask.
- A repository whose checks assumed the probe had gone first brings up or waits for what it needs in the check, as it does outside sluice.
