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

Amended 2026-11-09 by the change `sluice-takes-up-changed-settings-between-rounds`, below: what the build of `sluice run` chose within the decision.

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

## Amendment, 2026-11-09: what the build of `sluice run` chose

Status: accepted on 2026-11-09 as the choices of the change `sluice-takes-up-changed-settings-between-rounds`, its design D1 to D7 and its code stage, each within the decision above and cheap to change.

Deciders: the design and the code stage of the change `sluice-takes-up-changed-settings-between-rounds`.

Sources: the statement's clause 135, as amended at agentplot/blueprints c619438; clauses 26, 71, 78, 87, 129 and 136; rules core.14, core.22, core.31, core.51 and core.54; the change's design D1 to D7; `crates/sluice/src/host_settings.rs`, `crates/sluice/src/instance.rs` (`Instance::changed_tables`), `crates/sluice/src/lib.rs` (`Driving`, `drive`) and `crates/sluice/src/main.rs` (`keep_running`).

### Context

This change's base holds one process that outlives a command, `sluice run`. It builds the decision for that process alone. The host process (135; record 0093) is built on `bolt/sluice-runs-one-host-process`, and once both are on main its pass loop takes the same look, in the change `the-host-process-takes-up-changed-settings`. `sluice daemon` (137) takes it when it is built. The hand-over of a change no process can take up in place to a process started afresh is built by whichever of this change and `sluices-telemetry-is-a-host-setting` lands second, since `[telemetry]` (record 0106) is the first setting that needs one; it goes at the one step where `sluice run` takes up what changed.

### Decisions

1. **The sign is the link's own entry, the path it resolves to and the file there.** The file's own entry by `symlink_metadata`, its modification time and size; the path `canonicalize` resolves it to; and the file found there, its modification time and size. sluice's flake module links the file into Nix's store (core.51), where every file's time is one second past the epoch, and the next configuration may link one of the same size. *Not taken:* the target's time and size alone, as this record first words the sign, which misses that case; the operating system's notifications, a thread or descriptor per process and code per platform; a digest of the contents, a read of the file every pass.
2. **A read counts only where the sign stood through it.** A look whose sign moved reads the file and takes the sign again; where the two differ, a writer was still at work, and the look finds nothing and reads again at the next. Settings equal to those held are taken up silently. *Not taken:* taking whatever a read gives, which could leave a pass under half a file written in place.
3. **`sluice run` drives one pass at a time, carrying its driver.** `drive` runs one pass and its pause; `Driving` carries the passes counted toward `--rounds`, the repository's driver's lock and its reader, what the reader's document asked, the holds and refusals said, whether the host's systems were checked, and the policies read by revision. `main` binds afresh only between two passes, so a pass runs wholly under one binding (core.14, core.54). *Not taken:* swapping each profile's parts inside one binding, which spreads the take-up across every profile; returning from the run to rebind, which lets go of the driver's lock, so that another `sluice run` could take the repository while this one lives.
4. **A gone file keeps what is held.** A file that is gone, cannot be read, or that sluice refuses leaves the run on the settings it holds, said once on stderr with the file and why: `it is gone; an empty file gives sluice's defaults` for a gone file, the reader's words otherwise. The sign is kept for a file that did not read, so the refusal is said once while the file stands. A command that starts with no file still takes the defaults. *Not taken:* a gone file read as the defaults, which in an apply's moment between two links drops the forge's app and the signer for a pass; a refusal said at every look, where core.28 and core.47 say each cause once.
5. **The take-up is bound, then accepted, then said.** Settings that read are bound afresh, and the new binding's host systems checked (87), before the old binding is let go; where either fails, the old stays and the failure is said as a refusal. A take-up says on stderr `sluice: took up the host's settings in <path>: <tables> changed`, the tables named in the configuration reference's order. *Not taken:* a take-up that says nothing, which leaves an operator no way to tell whether a change counted.
6. **A run's result is signed by the round that records it.** A run keeps the cores, caches and grant its request and record hold from its start; nothing stops it or starts it again. A later round records its result and signs it with the signer that round's binding holds, as after a restart. A `[peers] name` changed while runs go reads as a rename of the host, its claims lapsing as a quiet host's do (77). *Not taken:* keeping each run's signer and key to sign its result with, which needs the old key readable after the operator replaced it, while a result signed with a key the branch no longer lists reads as missing either way (core.22).
7. **Each policy is read once more after a take-up, by the profiles alone.** A new binding starts with none of the profiles' reads by revision, so its first pass reads what they read once more, as a process started afresh would; a take-up is a move on the host, and nothing repeats while the file and the revisions stand (core.31). The core's policies by revision are carried in `Driving` across passes and bindings, as the run's one sluice kept them before: a policy at a revision is that revision's bytes, which no binding changes. *Not taken:* carrying the profiles' reads into the new binding, which couples every profile to the take-up to save one read per branch per change; dropping the core's policies at each pass, which would read every policy again at every pass and break core.31.

### Consequences

- A key file changed in place at the path the file names is read when the file next changes or a process starts; the configuration reference says so.
- A local layer, `config.local.toml` (principle 10), joins the sign once sluice reads one.
