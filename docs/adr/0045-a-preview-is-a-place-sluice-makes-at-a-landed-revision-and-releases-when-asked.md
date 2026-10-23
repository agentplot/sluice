---
number: 45
title: A preview is a place sluice makes at a landed revision and releases when asked
status: accepted
date: 2026-10-14
decision-makers:
- Chuck Swanberg
---

# A preview is a place sluice makes at a landed revision and releases when asked

Deciders: the user, exploring previews for bolt landings and building the OpenSpec change `previews`, 2026-10-14.
Sources: `agentplot/blueprints:design/sluice/proposals/previews-1.md` clauses 98 to 103 and S30 to S35; `agentplot/blueprints:design/sluice/requirements.md` clauses 25, 31, 34, 56, 66, 71; ADRs 0010, 0016, 0018, 0020; the fleet's dev servers under portless, whose `.localhost` names an identity provider's wildcard redirect accepts.

## Context and problem statement

A person checking a bolt's work wants it running. The fleet kept a worktree of each bolt branch for that, checked out and kept current, its stack brought up by hand on whatever ports were free, and means to stop keeping them. The repository already describes its stack for its checks: an emulator, stand-ins and servers on the ports a place is given (ADR 0016), brought up by its own tools. What does sluice owe a caller, such as crew or flywheel, that wants a running copy of what a branch landed, linked to the landing, replaced at the next landing and gone when the bolt lands?

## Considered options

1. Nothing: the caller keeps a worktree per bolt and brings its stack up there.
2. A separate tool on the host that checks out a revision, brings its stack up and tears it down.
3. A preview as a third kind of place: made by sluice, as a candidate's is, at a revision the branch has landed; its stack brought up by the policy's preview command under the executor and handed off; held until its caller asks for its release.

## Decision outcome

Option 3. `sluice preview <branch> <revision>` refuses a revision not in the branch's history, so a person only ever sees what passed the gate (98). The place seam makes the checkout with its caches and a block of ports, and gives each port a name, `SLUICE_NAME_<NAME>`: one DNS label, `<port>-<rev7>--<branch>--<repository>`, cut to 63 characters with a hash, the revision lengthened to twelve on a clash (99). One label because a wildcard certificate and a `*.localhost` redirect each cover one; sluice owns the name's uniqueness as it owns the port's, and the repository's command routes the name to the port with whatever proxy it uses, so sluice names no proxy (rule core.1). The policy's `[preview] addresses` are templates over the names and ports, filled in when the preview is made, so a caller has them before the stack is up.

What the preview is brought up with, `[preview]`, `[checkout]` and `[tests]` with its toolchain and release command, is read from the previewed revision, because it describes that revision's stack. What its command may use and reach, `[executor]` and the time limit, is read from the branch's current revision, because trust is the branch's now (56; rule core.15). The command runs once as a run of the check seam under that executor, never recorded, required or replayed (103), and counts against the host's bound while it runs as a session run does, never in the reserve (102). `sluice preview` waits for room as `sluice exec` does; there is no daemon to start it later (ADR 0010).

A preview's record is kept in `.previews/`, apart from `.places/`, because a sluice that predates previews parses every record in `.places/` and fails on a kind it does not know, which would stop its rounds. The record holds the run's handle, and the state is read from it and the run every time: `waiting`, `starting`, `up` or `failed` (rule core.14). One revision has one preview on the host, whichever branch asked; asking again gives it back unchanged, and takes up one whose asker stopped while it waited (100).

The caller ends a preview with `sluice release <preview>`, which ends the command's run, runs the release command saved in the run's request, removes the checkout and frees the ports and names, and works when the branch is gone. sluice releases a preview on its own only when nobody can use it: its command failed, timed out or was lost, or its checkout was removed by hand. That happens in every round, `sluice exec`, `sluice preview` and `sluice previews`, under the branch's lock taken without waiting, as ADR 0020's release of a removed worktree does. A failed preview's record stays, holding nothing, with the end of what its command printed, until its caller releases it (101).

Option 1 is the upkeep the fleet means to drop, and its stack has no bound, ports or release. Option 2 is a second owner of ports and teardown beside the place seam, which ADR 0018 refused for session runs for the same reasons.

### Consequences

- A preview is cheap: the repository's own stack on its fakes. A deployment into a real environment with production parity is a delivery tool's, never sluice's, which holds no cloud credentials (ADR 0019).
- The place seam gains `preview`, `preview_run`, `previews` and `release_preview`; the core gains `preview_plan`, `make_preview`, `start_preview`, `preview_state` and `release_unusable_previews`, which a round runs beside `release_abandoned`; the policy gains `[preview]`; the CLI gains `sluice preview`, `sluice previews` and `sluice release <preview>`.
- Rule core.16 names who releases a preview; core.10 admits its handed-off stack; core.19 is new.
- A preview whose stack dies after it came up is still listed `up`: sluice does not watch what a command handed off. Its caller releases it and asks again.
- No cap on previews per host and no lapse: the caller owns the lifetime, and reconciles with `sluice previews` after a crash.
- Two clones of one repository on one host previewing the same revision of the same branch would give the same names; each repository checks only its own previews.
