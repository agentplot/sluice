---
number: 94
title: The herdr plugin keeps one watcher per user on a host for every herdr server, and forks nothing while nothing changes
status: accepted
date: 2026-11-07
decision-makers:
- agentplot-design
---

# The herdr plugin keeps one watcher per user on a host for every herdr server, and forks nothing while nothing changes

Status: accepted on 2026-11-07, agentplot-design's ruling from the records already written (0023, 0056, 0093), on the user's word that the herdr plugins create too many processes.

Amends record 0023 in part, where its plugin's watcher is one per herdr server.

Deciders: agentplot-design, ruling for agentplot-planner on signal 2026-11-07-agentplot-ops-ac49fa28/01, whose words are the user's: "what about the herdr/plugin's creating many/many processes".

Sources: records 0023 (a reader sees the queue through one overview, and plugins live outside the core), 0056 (one process per host reads a repository's records for every watcher of its queues) and 0093 (sluice's own host process); `plugins/herdr/bin/watch.sh`, `bin/document.sh` and `lib/common.sh` and `openspec/specs/herdr-watch/spec.md` at 26e131bd; signal 2026-11-07-agentplot-ops-ac49fa28/01 on afterthought/crew-state agentplot/main (deneb, 2026-11-07: eleven herdr servers, eleven watchers over the same repositories, about 280 processes); flume record 0051 as amended 2026-11-06, which ends flume's own plugin's settle per herdr server.

## Context and problem statement

The herdr plugin keeps its state per herdr server, in a folder named by the server's socket (`lib/common.sh`), and its lease names the socket, so each herdr server runs a watcher of its own. Each watcher wakes every second between its fifteen-second ticks, runs `hd`, its forecast and its tokens each tick, keeps one `sluice overview --watch --json` per repository a pane of its server is in, and hands every line of each to a fresh `sh bin/document.sh`, which runs `jq` and `hd` some two dozen times. Record 0056 made every watch of a repository on a host follow one reader, so the queues are read once however many watch them. The plugin's own processes still grow with each herdr server and each repository: on deneb, with eleven servers, they held about 280 processes and loaded the host. Record 0093 changes who reads the queues, not the plugin.

## Considered options

1. **One watcher per user on a host** serves every herdr server that has the plugin: one `sluice overview --watch` per repository a pane of any server is in, each document kept once for every server's views, and each server told of what changes. Between changes it forks nothing on a timer.
2. A watcher per herdr server, made cheaper: no fork each second, one long-lived document handler per repository. The count still grows with the servers.
3. Nothing in the plugin: wait for record 0093's host process. The queues are already read once per host (0056), so the host process removes none of the plugin's own processes.

## Decision outcome

Option 1.

- **One watcher per user on a host.** However many herdr servers run the plugin, one watcher holds the host's lease and serves them all; a server's startup, pane or worktree event that finds it running starts nothing. A watcher whose servers have all stopped answering stops within two ticks, and the next server's event starts one again.
- **One read of each repository.** The watcher follows one `sluice overview --watch` for each repository a pane of any server is in, and keeps each repository's latest document once, which every server's popup, dashboard, segments and toasts read. A toast of a landing or a blame is told once in each server that has a pane in that repository.
- **Nothing forked while nothing changes.** Between a document's changes and herdr's events, the watcher forks no process on a timer. Each document line is handled without a new process for each of its fields.
- **The plugin still only reads** (0023): it reads sluice through `sluice overview` alone and submits, withdraws, lands and releases nothing; nothing under `crates/` changes for it.

### Consequences

- The herdr-watch spec's "One watcher per herdr server" becomes one watcher per user on a host, its lease kept in the plugin's state for the host and not for a server.
- A host's plugin load is one watcher and one overview watch per repository, whatever the number of herdr servers.
- Record 0093's host process, once released, is what each overview watch follows (129); the plugin needs no change for it.
