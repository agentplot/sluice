---
number: 107
title: A sluice process that keeps running takes up changed host settings at its next round
status: accepted
date: 2026-11-09
decision-makers:
- Chuck Swanberg
- the design agent
---

# A sluice process that keeps running takes up changed host settings at its next round

Status: accepted by the user on 2026-11-09 in agentplot-design's pane, for the direction, in their words under *Deciders*. The form below is agentplot-design's own choice, made on the statement, principles 5 and 14 and record 0093, and cheap to change.

Deciders: the user, 2026-11-09, reading deneb's Boomdrive configuration, where a host keeps its own restart of sluice's drivers because sluice reads its settings only as it starts: "we need to externalize the things that can be and make sense to be externalized".

Sources: the statement's clause 135 and principle 5; records 0093 and 0098; rules core.31 and core.51; `crates/sluice/src/instance.rs`, `Instance::read`, and `crates/sluice/src/profiles/mod.rs`, which reads it once as a process opens the host's bound; agentplot/blueprints `design/principles.md` 14 (553bb8c); agentplot/boomdrive record 0034, point 5; afterthought/swancloud `modules/sluice-driver-darwin.nix`, the restart of each running `sluice run` on a change of the file it links.

## Context and problem statement

A sluice process reads the host's own settings, `~/.config/sluice/config.toml`, once, as it starts, and keeps them until it ends (core.51 leaves that standing). A command ends in seconds. sluice's host process ends once no queue has work in play (135), but while work is in play it keeps what it read, and so do `sluice run` and a daemon an operator keeps (137). A change of the bound, the caches, the forge's app, the signer or the telemetry (record 0106) therefore waits for every such process to end. So a host that changes the file restarts sluice's processes itself: swancloud's home-manager restarts each running `sluice run` when the file changes. Boomdrive's set links the file in its activation, with no such restart, and left the restart to the host (agentplot/boomdrive record 0034, point 5). Each host that runs sluice would keep a restart of its own.

## Considered options

1. **A sluice process that keeps running takes up changed settings itself, at its next round (chosen).**
2. **Each tool's module restarts its processes when its file changes.** Every host that writes the file by hand, or by a control plane's local layer (principle 10), would still restart by hand, and the module would need to find processes sluice starts itself (principle 5).
3. **Read the file afresh at every step of a round.** A round would act under two settings at once.

## Decision outcome

Option 1. A sluice process that outlives a command, its host process, `sluice run` or a daemon, notices between rounds that the file changed since it read it, by its modification time and size, which costs no process and no read of git (core.31). It reads the file then, never within a round. A change it can take up in place, such as the bound, the caches or the forge's interval, takes effect from its next round. A change it cannot, such as telemetry, whose exporters are set up as the process starts, it takes by handing its repositories to a process of its own started afresh and ending, as an older host process hands them to a newer sluice's (135). A file it cannot read, or refuses, leaves it on the settings it holds, said once, naming the file and why, until the file reads again. Runs already started keep the settings they were started under.

Not taken: options 2 and 3, above.

## Consequences

- The statement's clause 135 says it, and rule core.54 holds it. The unit `sluice-takes-up-changed-settings-between-rounds` builds it.
- No host keeps a restart of sluice for its settings: swancloud's restart of its drivers, and any like it, goes once a sluice with this runs there (agentplot/boomdrive record 0034, point 5, as amended the same day).
- The runner needs no such change: Boomdrive's set gives it its file on its command line (the bolt `the-boomdrive-flake-configures-the-whole-host`), so a changed file is a changed job, which nix-darwin and the set's activation restart.
