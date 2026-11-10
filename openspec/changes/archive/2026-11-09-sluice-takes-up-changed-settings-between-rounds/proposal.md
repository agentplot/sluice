## Why

sluice reads the host's own settings, `~/.config/sluice/config.toml`,
once, as a process starts, and keeps them until it ends. A command ends
in seconds, but a `sluice run` an operator keeps running holds what it
read for as long as it lives. A change of the bound, the caches, the
forge's app, the signer or the peers' settings therefore waits until
someone restarts it. So hosts restart sluice themselves:
afterthought/swancloud's `modules/sluice-driver-darwin.nix` restarts each
running `sluice run` when the file changes. Boomdrive's set links the
file and leaves that restart to each host (agentplot/boomdrive record
0034, point 5). Every host that runs sluice would keep a restart of its
own, which principle 5 and the user's direction of 2026-11-09 ("we need
to externalize the things that can be and make sense to be
externalized") rule out.

The statement's clause 135 (as amended at agentplot/blueprints c619438),
record 0107 (accepted 2026-11-09) and rule core.54 already say what
holds. A sluice process that outlives a command takes up the host's
settings afresh once their file changes, between rounds and never
within one. A file it cannot read leaves it on what it holds, said once.
A run keeps the settings it started under. This change builds that for
the one such process this change's base has, `sluice run`.

## What Changes

- **sluice run looks at the settings file between passes.** Before each
  pass it compares the file's sign with the one it read: the file's own
  modification time and size, the path it resolves to, and that file's
  modification time and size. This starts no process and reads no git
  (rule core.31). The file is read only once its sign moves. A file
  that Nix links into its store, where every file has one time, is
  noticed when its link moves to another.
- **A changed file is taken up at the next pass.** Settings that differ
  from those held bind the repository afresh before the next pass: the
  bound, the cores, the reserve, the systems, the forge's app and its
  interval, the signer, the peers' settings, the deliveries' span and
  the caches. The pass, its rounds and the reader's document all run
  under one set of settings. The run stays the repository's driver and
  reader throughout, and says on stderr, once, which tables it took up.
  The same settings written again change nothing and say nothing.
- **A file that does not read leaves the run on what it holds, said
  once.** This covers a file that is gone, unreadable or refused by
  sluice's reading, and settings the binding refuses, such as a key
  that does not parse. The run keeps what it holds and says on stderr
  once which file and why. It reads the file again once it changes
  again.
- **A run keeps what it started with.** A run already testing keeps the
  cores, caches and grant it was started with. A new bound holds the
  runs started after it, and no run is stopped or started again for a
  change.
- The configuration reference, the commands reference and the setup
  skill say a kept `sluice run` takes a change up at its next pass,
  replacing "once restarted". Record 0107 records the choices this
  design makes within it.

## Capabilities

### New Capabilities

- `settings-take-up`: a `sluice run` kept running takes up a changed
  host settings file at its next pass, between passes and never within
  one. A file that does not read, or settings that do not bind, leave
  it on what it holds, said once. A run already started keeps what it
  started with.

### Modified Capabilities

None.

## Out of scope

- **The host process (135; record 0093).** It is built on
  `bolt/sluice-runs-one-host-process` and is not on this change's base.
  Once both are on main, its pass loop takes the same look between its
  passes and binds each repository afresh, in a change of its own,
  `the-host-process-takes-up-changed-settings`.
- **`sluice daemon` (137).** No daemon is built yet. It takes the same
  look when it is.
- **A change no process can take up in place, and the hand-over to a
  process started afresh (135).** No setting on this change's base needs
  one: every table the file holds today binds afresh in place.
  `[telemetry]` (record 0106) is the first, brought by
  `sluices-telemetry-is-a-host-setting`. Its exporters are set up as a
  process starts. Whichever of the two changes lands second hands a
  change of `[telemetry]` to a process started afresh, at the one step
  where `sluice run` takes up what changed (design D5).
- **A wait, a waiting submit and `sluice overview --watch`.** Each is a
  command and reads the file as it starts. A wait runs rounds only
  where no driver lives, and record 0093 moves those rounds into the
  host process. A watch runs no round.
- **A key file changed in place at the path the file names.** The
  settings file did not change, so nothing is taken up. The key is read
  when the file next changes or a process starts. The reference says
  so.
- **A local layer, `config.local.toml` (principle 10).** sluice reads
  none yet. Once it does, the layer's sign joins the look.
- **swancloud's restart of its drivers.** That is afterthought/swancloud's
  to remove once a sluice with this change runs there (record 0107).

## Impact

- `crates/sluice/src/host_settings.rs` (new): `HostSettings`, the
  settings a process holds and the sign of the file it read them from,
  and `Look`.
- `crates/sluice/src/instance.rs`: `Instance::changed_tables`, and
  `config_path` reachable from the crate.
- `crates/sluice/src/lib.rs`: `Driving` and `drive`, one pass of
  `sluice run` and its pause, with what a pass carries to the next.
  `run`'s `Command::Run` arm drives through them.
- `crates/sluice/src/main.rs`: `sluice run` holds `HostSettings`, looks
  between passes, binds afresh, and says what it took up or refused.
- Tests: `crates/sluice/src/host_settings.rs` (its own),
  `crates/sluice/src/instance.rs` (its own),
  `crates/sluice/tests/cli.rs`, `crates/sluice/tests/instance.rs`.
- Docs: `site/src/content/docs/reference/configuration.md`,
  `site/src/content/docs/reference/commands.md`,
  `plugins/sluice/skills/setting-up-sluice/SKILL.md`,
  `docs/adr/0107-a-sluice-process-that-keeps-running-takes-up-changed-host-settings-at-its-next-round.md`.
- Sources: clause 135 (as amended at agentplot/blueprints c619438),
  clauses 26, 71, 78, 87, 129, 136; principles 5, 10 and 14; record
  0107 (accepted 2026-11-09), records 0093, 0098 and 0106; rules
  core.14, core.22, core.31, core.51, core.54; agentplot/boomdrive record
  0034, point 5.

## Touches

```
crates/sluice/src/host_settings.rs
crates/sluice/src/instance.rs
crates/sluice/src/lib.rs
crates/sluice/src/main.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/instance.rs
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/commands.md
plugins/sluice/skills/setting-up-sluice/SKILL.md
docs/adr/0107-a-sluice-process-that-keeps-running-takes-up-changed-host-settings-at-its-next-round.md
openspec/changes/sluice-takes-up-changed-settings-between-rounds/**
```
