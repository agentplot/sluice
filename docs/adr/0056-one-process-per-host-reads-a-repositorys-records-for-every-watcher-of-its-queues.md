---
number: 56
title: One process per host reads a repository's records for every watcher of its queues
status: proposed
date: 2026-10-25
decision-makers:
- Chuck Swanberg
---

# One process per host reads a repository's records for every watcher of its queues

Status: proposed to the user on 2026-10-25, as `briefs.15` has it; accepted only on their yes.

Deciders: the user, who asked through the operator on 2026-10-24 whether the herdr and Claude Code plugins could do without one sluice process per Claude Code tab, and answered the design put to them in madswan-design's pane with "go" on 2026-10-25; the record's other words are madswan-design's, for the user to accept on this record's card.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 26, 71, 80, 129; ADR 0023; rules core.11, core.14; `plugins/sluice-board/hooks/register.tsx` (`feed`), `plugins/herdr/lib/sources.sh` and the herdr-watch spec; `crates/sluice/src/overview.rs` (`document`, `run`); signal `2026-10-24-madswan-design-fe11a4cd/01`.

## Context and problem statement

ADR 0023 gave every reader one overview and a watch form of it. The Claude Code board runs `sluice overview --json --watch` for each session's life, and the herdr plugin runs it once per repository per herdr server. Each watch rebuilds the document once a second from the records, the policy at each branch's revision and the host's places, which runs `git worktree list` twice and a `git cat-file` per branch and per held worktree every tick. On 2026-10-24 deneb ran 48 such watchers, 37 of them the board's and 11 the herdr plugin's across eight herdr servers, several at 16% of a core before their git children, for six repositories. The Claude Code engine gives a plugin no file, socket or network: a tab learns anything outside only from a process it runs, so a tab will always run something. What should that something cost?

## Considered options

1. One reader per repository per host inside sluice: `sluice overview --watch` takes a lease in the host's sluice state, the holder reads and writes each document to a file, and every other watch and `sluice wait` on the host follows that file.
2. The board reads the herdr plugin's state file when it runs under herdr.
3. A daemon per host serving the overview over a socket, which the plugins connect to.
4. Each watch polls a one-shot `sluice overview --json` every few seconds instead of watching.

## Decision outcome

Option 1. A host's first watch or wait of a repository becomes its reader; every later one follows the reader's document, reading no records, policy or places of its own, and prints the same lines it would have read itself, so neither plugin changes a line and `sluice wait` stops polling the records. A reader that stops is replaced by the next watch or wait within a tick, and followers read what it last wrote until then; a reader serves its own host alone, since the records are the one truth across hosts (26, 80) and a document is a host's view of them. On deneb the 48 readers become one per repository, and a follower costs nothing measurable.

Option 2 couples two plugins and leaves a tab outside herdr as it is. Option 3 is the same idea with a service to keep alive, which this problem does not need; the host-wide bound (71) may yet want it, and a reader's lease is a step toward it, not away. Option 4 moves the git storm around without shrinking it.

### Consequences

- Clause 129 of the statement says this (A.8 Hosts); rule core.25 is drawn from it.
- `sluice overview --watch` and `sluice wait` gain the lease and the follower path; a one-shot `sluice overview` reads the records itself as before.
- The herdr plugin's own one-watcher lease becomes redundant and stays harmless.
- The reader's document is written whole on each change, so a follower never reads part of one, as the herdr-watch spec already requires of its state files.
