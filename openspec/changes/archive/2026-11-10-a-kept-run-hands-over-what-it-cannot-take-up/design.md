## Context

See proposal.md for why. Clause 135 (as amended at agentplot/blueprints
c619438), record 0107 and rule core.54 say what holds: a change a
process cannot take up in place it takes by handing its repositories to
a process of its own started afresh. This design says how `sluice run`
does it on main, which holds no host process and no daemon.

**The family.** Of the family's processes, only sluice's take up their
own settings while they run (record 0107's design, "The family").
Boomdrive's runner takes its file on its command line, so a changed
file is a changed job that the host's activation restarts (agentplot/
boomdrive record 0034, point 5). Within sluice, the host process on
`bolt/sluice-runs-one-host-process` already hands its repositories to a
newer sluice's: the newer command sends `Request::HandOver` over the
host's socket, the old process ends, and the command starts a process
of its own. That fits a process a command starts and nothing else
follows. A `sluice run` is one an operator keeps, under a terminal, a
pane or a service manager that follows its process. So 135's hand-over
has one owner, the statement's clause, and two forms: the host
process's, ending for a process the next command starts, and `sluice
run`'s, this design's, starting afresh in its own process (D2). The
change `the-host-process-takes-up-changed-settings` gives the host
process its form and cites D4 for what a hand-over carries.

What the code has on main:

- `main` (`crates/sluice/src/main.rs`) reads the host's settings once.
  For `sluice run` it reads them as `HostSettings`, with their file's
  sign. It chooses telemetry's settings from the command and the
  table, and starts telemetry. `Telemetry::start` installs the global
  subscriber and the OpenTelemetry providers, and where the table names
  a collector it removes every `OTEL_EXPORTER_OTLP_*` from the process's
  environment first (151; record 0106, D4 of
  `sluices-telemetry-is-a-host-setting`). `main` then binds the
  repository (`Bound::open_with`) and loops `keep_running`.
- `keep_running` drives one pass at a time (`sluice::drive`, carrying
  `Driving`). After each pass it looks at the file
  (`HostSettings::look`). On `Look::Changed(instance)` it binds afresh,
  checks the new binding's host systems, swaps the binding, accepts the
  settings and says `sluice: took up the host's settings in <path>:
  <tables> changed`, the tables from `Instance::changed_tables`.
- `Instance::changed_tables` names `[runs]`, `[forge]`, `[signing]`,
  `[peers]`, `[deliveries]` and `[[caches]]`, never `[telemetry]`. A
  change of `[telemetry]` alone reads as `Changed`, since `Instance`'s
  equality takes in its `telemetry`, binds afresh, and is said with no
  table named, while the telemetry the process started with goes on.
- `Driving` carries between passes the passes counted toward `--rounds`
  (`n`), the repository's `Driver` lock, its `Reader`, what the reader's
  document asked, `said_beside`, the holds said (`said`), the words each
  branch passed by was said with (`passed`), and the policies read by
  revision.
- `Driver` holds `<repository>.driver` in the host's readers with an
  exclusive `flock` on a file sluice opens, which Rust opens closed on
  exec. A watch or a wait reads a live driver by a shared try
  (`driver_live`), and neither leads nor runs rounds while one lives.
- A run's supervisor is a child of the process that started it, made a
  session leader (`setsid`), and reaped by a thread of that process
  (`Runs::start`). Its check runs are the supervisor's children. A run
  is counted in the host's state, never in the process's memory.
- `Telemetry::flush` flushes traces and logs, and not metrics.

## Goals / Non-Goals

**Goals:**

- A kept `sluice run` takes up a changed `[telemetry]`, and any later
  table no binding takes up in place, from its next pass. Nothing on the
  host restarts it (135; core.54).
- Whatever runs it still follows it: same process, same output.
- Nothing another process reads changes during the hand-over. The
  repository has its driver throughout, so no wait runs rounds where a
  `sluice run` lives (core.31), and no second `sluice run` takes it
  (135).
- A hand-over that cannot start leaves the run as it was, said once
  (core.54).

**Non-Goals:**

- The host process, `sluice daemon`, a newer release under a kept run,
  and comparing what a command would send: see the proposal's Out of
  scope.
- Taking `[telemetry]` up in place.

## Decisions

### D1. What no binding takes up in place: `[telemetry]`, compared whole

`Instance::changed_tables` names `[telemetry]` where the two readings'
`telemetry` differ, between `[deliveries]` and `[[caches]]`, the
configuration reference's order. `instance::AFRESH`, a list beside it,
names the tables a binding cannot take up in place: `[telemetry]`
alone. A change whose tables include one of `AFRESH` is handed over
(D5). Every other change is taken up in place as before.

The table is compared whole, as every other table is. A change of a key
the command's own flags shadow, such as `logs` under `--otel-logs`,
still starts the run afresh.

*Why not in place.* `tracing::subscriber::set_global_default` and
OpenTelemetry's global providers are set once in a process. A process
whose table named a collector removed the caller's OTLP variables from
its own environment, and a table that names none needs them back (151;
core.53).

*Not taken.* Comparing the telemetry settings the command would choose
under each table (`Settings::choose`). It saves a hand-over only where a
command's flags shadow every key that changed, and it brings
telemetry's choosing into the take-up. Rebuilding the providers in
place behind a reloadable layer. The environment's variables are gone
all the same, and every signal's pipeline would be built for a case
that comes with a change of the host's file.

### D2. Start afresh in the same process

The run hands over by replacing its own program image (`execve`, with
the environment given whole). The process keeps its id, its parent, its
terminal, its standard output and error, its folder and its session, so
a terminal, a pane, launchd or systemd go on following the same
process. Everything the old image held in memory is gone, telemetry's
subscriber and providers among it. A start that fails leaves the
process as it was: no variable of its environment and no signal's
disposition changes.

*Not taken.* std's `CommandExt::exec`. While it tries the start it sets
the process's own environment to the command's, so another thread
reading a variable meanwhile reads the command's, or memory freed once
the start failed, and it sets `SIGPIPE` back to its default, which a
run whose start failed would keep. Starting a new process and ending
this one. A service
manager would see its process end. launchd's `KeepAlive` would start a
second `sluice run`, and systemd's default stop would end the new one
with the unit's other processes. A terminal would get its prompt back
while the new process wrote over it. Ending with a code that asks
whatever runs it to restart it: core.54 rules out a host restarting
sluice for its settings, and a plain terminal restarts nothing.

### D3. What it starts: the run as it started

The image started afresh is this run as it was started:

- **The executable it started as.** `main` resolves
  `std::env::current_exe()` through every link as it starts
  (`std::fs::canonicalize`). A host whose switch put another release on
  the PATH meanwhile still hands over to this release, which reads the
  document it writes (D4).
- **The command line it was given**, `std::env::args_os()`, so its flags
  choose for it as before (151), `--rounds` among them.
- **The environment it started with.** `main` takes
  `std::env::vars_os()` first, before telemetry takes the caller's OTLP
  variables out, and without the hand-over's own variable (D4). The new
  image gets that environment, cleared and set whole, and the variable.

*Not taken.* The `sluice` the PATH names, or `current_exe()` read at
the hand-over. Either may name a newer release after a host's switch,
for which this document is no form. The environment as it stands at
the hand-over. It lacks the caller's OTLP variables wherever the old
table named a collector, and a table that names none reads them (151).

### D4. What it carries, and how

The old image writes a `HandOver` (`crates/sluice/src/hand_over.rs`)
as JSON into `SLUICE_HANDED_OVER`:

- `settings`: the text of the settings file that the run read and bound
  (D5), and `tables`: the tables that changed, as `changed_tables`
  names them;
- `passes`: the passes counted toward `--rounds`;
- `driver`: where the run drives, the number of a descriptor that
  duplicates the driver's lock, open across the exec;
- `supervisors`: the process ids of the run supervisors this process
  started and has not reaped (D7);
- `said`, `passed`, `said_beside`: the holds and refusals said, as
  `Driving` keeps them.

`main` reads the variable first, before any thread or child starts, and
removes it from its environment, so no child inherits it. In the same
step it makes the driver's descriptor one closed on exec: a duplicate
closed on exec, then the inherited one closed. That way no run
supervisor the new image starts later holds the lock past this run.
At the first pass, `Driver::adopt` takes that descriptor as the driver
only where it is the repository's driver file, the same device and
inode as `<repository>.driver`, and holds its lock. Otherwise it drops
it, and the pass takes the lock as any start does.

The new image then:

- holds the settings handed over, `HostSettings::handed`, read from the
  text with `Instance::from_text`, under no sign. Its first look reads
  the file once: the settings handed over read as `Same`, a change made
  meanwhile as `Changed`, and a file that does not read as `Refused`,
  said once with the settings handed over kept (core.54);
- binds them and starts its telemetry as any start does;
- builds its `Driving` from what was carried (`Driving::handed`), the
  driver already held;
- reaps the supervisors handed over (D7);
- says `sluice: took up the host's settings in <path>: <tables> changed,
  in this sluice run started afresh` once it is bound, before its first
  pass, so the line is said only once it holds.

The form needs no version: one executable writes it and reads it (D3).

*Not taken.* The settings read afresh from the file by the new image. A
file that does not read at that moment would end the run, since a start
refuses the settings it cannot read, which core.54 rules out. The
driver's lock let go and taken again. Between the exec and the take,
`driver_live` reads no driver, so a wait on the host runs rounds where a
`sluice run` lives (core.31), and another `sluice run` may take the
repository (135). A file in the host's state for the document. A
crashed hand-over would leave it behind, and another process could read
it. Carrying the policies read by revision and what the reader's
document asked. Each is read once more, as record 0107's D7 reads each
policy once after a take-up.

### D5. The order: bind, flush, hand over

After a pass, where the look answers `Changed` and the tables include
one of `AFRESH`:

1. The run binds the new settings (`Bound::open_with`) and checks the
   new binding's host systems, as an in-place take-up does (record
   0107, D5), and drops the new binding. Where either fails, it keeps
   what it holds, nothing is handed over, and it says on stderr once,
   through `telemetry::warn`: `sluice: warning: <path> changed <tables>,
   which this sluice run takes up only by starting afresh, and its
   settings do not bind, so it keeps the settings it holds until the
   file changes: <why>`.
2. It flushes its telemetry, `Telemetry::flush`, which flushes metrics
   as well as traces and logs. What the old image held is sent to the
   collector it sent to. The providers stay up.
3. It duplicates the driver's lock open across the exec, writes the
   `HandOver`, and starts the image afresh (D2, D3).
4. Where the start fails, which returns only on an error, it closes the
   duplicate and keeps its settings, binding and telemetry. It says on
   stderr once, through `telemetry::warn`: `sluice: warning: <path>
   changed <tables>, which this sluice run takes up only by starting
   afresh, and it could not start afresh, so it keeps the settings it
   holds until the file changes: <why>`. The sign is kept, as for any
   refusal, so this is said once for this change.

`Look::Changed` carries the text it read beside the `Instance`, so the
document holds exactly what was bound.

*Why a failed start keeps everything.* Accepting the change in part,
its in-place tables bound and its telemetry not, would hold settings
the file names and the process does not run under, and the next look
would read them as `Same`. All or nothing keeps record 0107's D4:
settings that do not take leave the run on what it holds.

### D6. The run started afresh is the run it was

The repository's driver is held across the exec by the descriptor
handed over (D4). Every wait and watch on the host follows the run
throughout. Its first pass takes the reader's place, which no follower
took while the driver lived (`Reader::step` with `Drives::Another`). It
counts the passes driven toward `--rounds`, so `sluice run --rounds 12`
drives twelve passes across any hand-over. It does not say again what
the old image said. It checks the host's systems at its first pass, as
any start does (87).

### D7. Runs going keep what they started with, and are reaped

A run's request and record hold its cores, caches and grant (record
0107, D6). The supervisor and its checks are processes of their own,
and the exec touches neither. The thread that reaped a supervisor is
gone with the old image, though. `runs.rs` keeps the process ids of the
supervisors this process started and has not reaped, in a set the
reaping threads add to and take from. The new image reaps each one
handed over on a thread of its own (`waitpid`) and keeps it in the same
set, so a second hand-over carries it again. A supervisor that ended
during the exec is reaped at once.

*Not taken.* Reaping every child (`waitpid(-1)`), which takes the end of
a child another part of sluice waits on: a supervisor still starting,
or git. A supervisor started so that it is no child of `sluice run`, by
a double fork, which loses the start's watch on a supervisor that exits
at once (`Runs::start`).

## Risks / Trade-offs

- **A key file removed between the bind and the new image's bind.** The
  new image's start fails, as any start does on a key that does not
  read, and the run ends. The window is the exec's milliseconds against
  an operator's or an apply's edit. The bind in D5 catches every key
  that already does not read.
- **A signal in the exec's window.** Until the new image sets its
  handler, `SIGINT` and `SIGTERM` take their default and end the process
  at once, which is what they asked. The driver's lock goes with the
  process, as on any end.
- **What the old image held in memory is read again**: each branch's
  policy once, the reader's document's question once, and the forge's
  discovery at its next interval from the ETags the host's state keeps
  (core.31, core.32).
- **The document is in the process's environment** until the new image
  removes it, a moment visible to the host's user alone. It holds the
  settings' text, in which no secret is written (core.51: keys by path,
  a credential by its command).
- **A test's skipped clock** (`SLUICE_ROUND_SLEEP=record`) starts again
  at the real time in the new image. No test reads the clock across a
  hand-over.

## Migration Plan

Nothing to migrate: the file, the state and the records keep their
form. A host that restarts sluice when the file changes keeps working,
and may drop its restart once its sluice has this change (record 0107).

## Open Questions

None.
