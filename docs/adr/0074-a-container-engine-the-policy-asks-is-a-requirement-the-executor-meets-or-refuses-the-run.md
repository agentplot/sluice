---
number: 74
title: A container engine the policy asks for is a requirement the executor meets, or it refuses the run and leaves it open
status: proposed
date: 2026-10-30
decision-makers:
- Chuck Swanberg
---

# A container engine the policy asks for is a requirement the executor meets, or it refuses the run and leaves it open

Status: proposed to the user on 2026-10-30, as `briefs.15` has it; accepted only on their yes.

Deciders: madswan-design, on swancloud-planner's and swancloud-ops's reading of 2026-10-30 (swancloud signal 2026-10-30-swancloud-planner-425be3b2/01): mac-studio's gate runs sluice as its own user, which has no container engine it can reach, OrbStack running only under the person's login session, while sluice's own policy says `[executor] docker = true` for its stack test; madswan-planner proposed keeping the stack test off that host by a named system, which the statement's definition of a system rules out. The user's yes is asked on this record's card.
Sources: `agentplot/blueprints:design/sluice/requirements.md` definitions (system), clauses 66, 83, 84, 85, 86, 87, 139, the C.3 sandbox row; `crates/sluice/src/profiles/checks.rs` on origin/main at 2026-10-30 (`docker_sockets`, `sandbox_profile`: `docker = true` opens the engine's socket paths in the profile and checks for no engine); records 0016, 0072; rules core.34, core.35.

## Context and problem statement

A policy may say `[executor] docker = true`: the run may reach a container engine (record 0016), and the sandbox opens the engine's usual sockets to it. Nothing asks whether the executing host has an engine. A host that has none runs the checks anyway; the stack test fails as a check, the candidate is blamed, and nothing tells that failure from the code's own.

On 2026-10-30 mac-studio's gate, where sluice runs as a user with no login session and no engine, held agentplot/sluice main's intake. Every candidate's stack test would have failed there. The hosts that can run it, deneb among them, are the same system, macos-arm64, so no system the gate could name (86) keeps the run off the host that cannot.

## Considered options

1. **A named system for engine-bearing hosts.** A system is the processor architecture and operating system as the toolchain names it (definitions), and both hosts are one system; naming "macos-arm64-docker" makes a capability a system and breaks what a system's pass means (69, 85).
2. **`docker = true` is a requirement the executor meets** (chosen). An executor that finds no engine the run could reach, no socket at `DOCKER_HOST` or at the places the profile would open, refuses the run before claiming it, as it refuses a limit it cannot enforce (84); the run stays open for a host whose executor can, and where none can the candidate's time limit reads it not passed (66, 87's shape). The refusal is the executor's own report (139), never a probe or a check's words.
3. **A host declares it has no engine** in its own configuration, and claims no such run. The same outcome, but a fact the host can read for itself written down by hand, wrong the day the engine is installed or removed.
4. **Leave it**: the stack test fails on the host and the candidate is blamed. The attribution record 0066 forbids: a host's fault read as the code's.

## Decision outcome

Option 2.

**An engine the policy asks for is a requirement.** `[executor] docker = true` says the checks need a container engine; an executor that cannot give one cannot run the checks as the policy states them, which is 84's case: it refuses the run rather than run it without. The refusal is read before the run is claimed, from the host itself, so no place is cut and no check runs for it.

**A refused run stays open.** As a run whose system no live executor is (87), it waits for an executor that can meet the requirement; a wait or a status says what it waits for. Where no executor on any host can within the candidate's time limit, the candidate reads as not passed, as one whose run its time limit ended (66), and the culprit's session is told why.

**How the executor knows.** It looks where it would open the engine's socket for the run, `DOCKER_HOST` and the places the profile names (C.3), and reads a socket that is there and answers as an engine; it reads nothing of the check's words. A host that gains or loses an engine is read as it stands at each run.

**Why not the others.** Option 1 bends what a system is. Option 3 writes down what the host can read. Option 4 blames the code for the host.

### Consequences

- Clause 84 gains the requirement and the C.3 sandbox row names the refusal, in the statement at agentplot/blueprints; rule core.38 is added, marked proposed; `_open.md` names this record until it is built.
- One unit on the yes, citing this record: an executor with no reachable container engine refuses a run whose policy asks one before claiming it, the run stays open, and status and wait say what it waits for.
- swancloud's gate hosts need no system declared for this; a host with an engine takes the run, one without leaves it.
