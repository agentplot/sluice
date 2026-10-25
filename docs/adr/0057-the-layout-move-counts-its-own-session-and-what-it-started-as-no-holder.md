---
number: 57
title: The layout move counts its own session, and what it started, as no holder of the folder
status: proposed
date: 2026-10-25
decision-makers:
- Chuck Swanberg
---

# The layout move counts its own session, and what it started, as no holder of the folder

Status: proposed to the user on 2026-10-25, as `briefs.15` has it; accepted only on their yes.

Deciders: madswan-design, 2026-10-25, on the question of slc-2's unit `setup-offers-the-layout-in-a-plain-clone` (bolt sluice-onboards-a-newcomer), whose coder found the move refusing on the session's own children; for the user to accept on this record's card.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clause 126 as amended 2026-10-25; rule core.1; the unit's design ("What holds the folder") and the plugin-setup and plugin-worktrees specs; the coder's reading of `lsof` on deneb: an MCP server, a sluice watcher and `caffeinate`, each a child of the session process and a sibling of the command's shell, with their working directory in the worktree.

## Context and problem statement

Where this comes from: sluice is set up in a repository whose working copies stand apart from its git directory, one per branch. When someone runs the setup in an ordinary one-folder clone, the setup offers to move the folder into that layout, and before moving anything it checks that no other program has the folder open, so it never moves a folder out from under an editor or another session (clause 126). The slc-2 team, building that check, found it refusing every time it ran from a coding agent's session: the agent's own helper programs, such as a tool server and a queue watcher, sit in the folder too, and the check counted them as outsiders. The question put to the design agent on 2026-10-25 was small: do a session's own helpers count as the session, and how does the setup tell which process the session is without naming any particular coding harness in sluice's core (core.1)?

## Considered options

1. The session's descendants are the session, so not holders; the session is the process the caller names, or the caller's parent when none is named.
2. The session's descendants are the session; the session is found as the nearest ancestor whose command is the harness's.
3. Only the caller and its ancestors are not holders, as designed: the session's helpers must be stopped before the move.

## Decision outcome

Option 1. What the refusal protects is a process the move would surprise: an editor, another session, a build in flight. The session that runs the move is told the new path, and its history is carried there (126); what it started is its own, started from it and ending with it, and moves with it as its shell does. So a holder is a process outside the session, its ancestors and its descendants. Option 3 asks a session to stop its own harness's helpers, which it cannot, and would keep the move from ever running where it is offered. Option 2 names a harness in the core, which core.1 forbids, and breaks for a person in a terminal, whose session has no harness.

The core finds the session as its caller says: a process the caller names, and when none is named, the caller's parent. A command an agent's harness runs is a shell whose parent is the session process, and a command a person types is a child of their interactive shell, so the default fits both without a name. A plugin may name its harness and pass the session explicitly where the default does not hold, as a skill run through a wrapper script would.

### Consequences

- Clause 126 says it; the unit's design and the plugin-setup and plugin-worktrees specs are reworded to it by a construct pass, which changes no outcome the unit was approved for.
- The move's refusal names the holders it found, as before, and never names the session's own.
