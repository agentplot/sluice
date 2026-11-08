# Design

## Context

Clause 135 is this change's subject, with 129's reader and the count of
71 and 150. Record 0093 holds the decision, and rules core.31, core.25
and core.49 bind it. Record 0087's turns and its `hold` stand as
decided, their count kept in the host process (record 0093's amendment
of it). The family's principles 4 to 6 (agentplot/blueprints
`design/principles.md`, proposed) say why: a tool that needs a
background process starts it and stops it when idle, and each shared
thing is decided in one process.

What sluice holds at the bolt's base, 68d68a9a, that this change
replaces or moves:

- `Command::Run` in `crates/sluice/src/lib.rs`. Its loop takes
  `reader::Driver` for the repository and the reader's place. Each pass
  then asks every remote once (`seams.state.refresh`), releases what is
  abandoned or unusable, takes requests in, runs a round of every
  branch in play, and writes the reader's document.
- `wait_on` and `Reading` in `lib.rs`. A wait reads its submission,
  runs a round of its branch unless `reader::driver_live`, and is the
  reader where none lives.
- `reader.rs`: the reader's lock, document, version, lease and the
  driver's lock under `$XDG_STATE_HOME/sluice/readers/`.
- `host_count.rs`: the `running/` registry, an entry per admitted run
  with a hold its starter keeps locked until the run's supervisor holds
  the run directory. `checks::Checks::admit` admits at `Tests::start`,
  so the round's count (`step.rs` `capacity`, `choose_starts`) and the
  admission are two steps under two locks.
- `exec.rs` and `preview.rs` loop on `session_capacity` and `AtBound`,
  each pausing and asking again.
- Commands are bound to one repository each (`main.rs`,
  `profiles::Bound::open`). `Bound` holds `Rc`s, so one `Bound` lives
  in one thread.
- The tests over the fakes (`tests/cli.rs`) run `sluice::run` in their
  own process against `support::FakeHost`, and their waits run rounds
  there. The tests on disk (`OnDisk`, `reader_tick.rs`) run the binary
  with `SLUICE_ROUND_SLEEP=record`, which skips every pause.

## Goals / Non-Goals

**Goals:**

- One host process for each user on a host, started by a command when
  work arrives and gone when idle, nothing installed (135).
- Every repository's queues moved in it, each apart (26), and found
  again after a restart from an index the commands write (135).
- The host's bound decided in it alone: a combination's room held from
  its count to its start, turns between combinations and sessions, and
  sessions' and previews' runs in the order they came to wait (71, 150;
  core.49).
- Waits and watches that follow it and run no round, and that start it
  again within a tick when it stopped (129).
- Every test that drives rounds by `sluice step`, a wait or `sluice run
  --rounds` keeps driving them, on the fakes and on disk.

**Non-Goals:**

- Pull request intake's removal (the change
  `pull-requests-are-queued-on-the-forge`).
- One forge client shared across repositories (proposal, Out of scope).
- A wake for a host that runs no sessions (59, 111).
- Changing whether `sluice submit` waits by default.

## Decisions

### D1. One host process for each state folder, and where it is found

`sluice host` is a hidden command, as `sluice supervise` is. It takes
`$XDG_STATE_HOME/sluice/host/lock` for its whole life and exits at once
where another holds it. sluice's state folder is the user's, so this is
one process for each user on the host (135). Beside the lock, under
`host/`:

```text
$XDG_STATE_HOME/sluice/host/
  lock           held by the host process while it lives
  start.lock     held by a command starting it, and by the process as
                 it decides to end (D6)
  version        its version, pid and start time, written whole
  log            what its passes print, its warnings and failures; the
                 previous process's kept as log.1
  repositories   the index (D4), written whole
<the user's directory>/host-<digest>.sock
```

The socket is in the user's own short directory, `/tmp/sluice-<uid>/`
as record 0063 makes and checks it, or the directory
`SLUICE_USER_SCRATCH` names. Its name carries a digest of the state
folder's real path, so each state folder has its own process. A socket
path over 104 bytes cannot be bound on macOS. A state folder in a long
temporary path, a test's or a sandbox's, would put one under
`host/` past it (the core.30 incident of 2026-11-03). Record 0093 says
"a socket in sluice's state folder". It is amended to say where the
socket is and why (task 6.4). The path not taken is the socket under
`host/`, with a refusal where its path is too long, which fails every
test whose scratch is deep.

The process binds the socket only while it holds `lock`. It first
removes a socket file a dead process left. The lock goes with its
process however it ends, so a crash leaves nothing to wait out.

### D2. Reaching it and starting it

`host_process::client::reach(repository)` connects to the socket and
says `hello` with its version. The process answers with its own version
and pid. Requests and answers are lines of JSON, one request for each
connection, except a follow or a held room, which keep theirs (D7,
D12). Where nothing answers, the command takes `start.lock`, tries once
more, and otherwise spawns `current_exe() host`. The spawned process is
detached: in a new session (`setsid`), its working directory `/`, stdin
null, stdout and stderr to `host/log`, and the environment of D3. The
command then polls the socket for up to ten seconds and lets
`start.lock` go. Ten commands at once take `start.lock` in turn: the
first starts the process, and the other nine find it answering.

The requests are these:

- `serve {repository}`: the repository has work. The process adds it to
  the index (D4), starts its task where none runs, and runs a pass of it
  at once rather than at its next tick.
- `step {repository, branch}`: one round of the branch, answered with
  the step's report (D10).
- `follow {repository, kind}`: keep the repository read while the
  connection is open. The kind is a watch, a wait or `sluice run`.
  Each pass's report is written down the connection (D7, D11).
- `pass {repository}`: run a pass now and answer once it ends (D13).
- `hold {kind, repository, place}`: a session's or a preview's room
  (D12).
- `room`: the bound, what counts against it, the runs waiting and whose
  turn is next.
- `hand-over`: end, for a newer sluice (D8).

A process that cannot be started within the ten seconds is an error
that names `host/log`. It is never a silent wait.

### D3. The environment it starts with

The host process is started with the host's own environment, never the
starting command's place (135): what `checks::KEEP` keeps for a run
(`PATH`, `HOME`, `USER`, `LOGNAME`, `SHELL`, `TMPDIR`, `LANG`, `TERM`,
`PROTO_HOME`, `MOON_HOME`) and `LC_*`, with `XDG_*`, `SSH_AUTH_SOCK`,
`OTEL_*`, `DOCKER_*` and the `SLUICE_*` settings, `SLUICE_AGENT` left
out. Of git's own it keeps the variables that set where its
configuration comes from and whether it may prompt (`GIT_CONFIG_GLOBAL`,
`GIT_CONFIG_NOSYSTEM`, `GIT_TERMINAL_PROMPT`, `GCM_INTERACTIVE`). It
keeps no variable that names a repository, worktree, index or object
store (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_COMMON_DIR`,
`GIT_OBJECT_DIRECTORY` and their kind), no multiplexer session's
(`HERDR_*`, `TMUX`, `TMUX_PANE`) and no development shell's. The list
is one function, `host_process::environment::kept`, and the check
profile names the shell variables it leaves out, so no tool's name is
in the core (core.1).

`PATH` is the starter's, as every run's already is. A run's equipping
enters its policy's toolchain first, which sets its own `PATH` (147;
core.46). The starter's `PATH` only finds the host's tools: git, the
multiplexer, the toolchain's command. The path not taken is the login
shell's `PATH`, read by running `$SHELL -lc`. It costs a shell's start
at each start, and finds nothing on a host whose login shell differs
from the one its sessions run.

### D4. The index of repositories

`host/repositories` is a JSON list of common git directories, written
whole under `host/start.lock`. A command adds its repository once it
has read a policy at a revision the host holds or found records there:
`submit`, `wait`, `step`, `status`, `overview`, `exec`, `preview`,
`equip`, `branch`, `amend`, `withdraw`, `set-aside`, `release` and the
home view (135; "Index, on any sluice use", the user's choice). A
command that finds its repository already listed writes nothing. The
host process reads the index at its start and again when a `serve`
names a repository it does not hold. At its start it looks at each
listed repository as an idle tick does (129), with no process started,
and runs a task for each with work in play. While it runs, its tick
looks at each listed repository the same way, so work another host
wrote to a remote's records is found while the process lives. Each
look at a repository runs on a thread of its own under the round's
retry policy (134), so a remote slow to answer holds neither the
thread that accepts commands' connections nor another repository's
look, and a spent allowance is passed by, never waited out (rule
core.29). A look under way keeps the process from ending, as a task
does. A listed repository whose directory is gone, or which has no
sluice (no records and no policy at any revision the host holds), is
taken out of the index by the host process.

### D5. A task for each repository

Each repository with work in play or a follower has a thread of its
own in the host process. The thread opens its own `Bound` for the
repository, since `Bound` holds `Rc`s, with the round's retry policy
(134). The pass `Command::Run` runs today moves into
`host_process::pass::pass`, unchanged in what it does: the remote's
one ask, the releases, the intake, a round of every branch in play,
and the reader's document. One repository's slow fetch or refused
policy holds only its own thread (26). A panic in a task is caught,
logged with the repository, and the task is started again at the next
tick. The other tasks go on. A task ends once its repository has
nothing in play and nothing follows it. A task ticks every
second while a watch follows its repository, a watch's tick today, and
every three seconds otherwise, `sluice run`'s and a wait's tick today
(`sluice::tick`). The document names the tick it was read at (64).

`check_host_systems` runs once at the process's start. A host whose
config names a system it is not claims no run and says so in its log.
`sluice run` still refuses to start on it, naming the system, as the
standing executor's scenario has it.

### D6. When it ends

After each tick, the process ends where no task runs, no connection is
open and no room is held or asked for. It takes `start.lock`, accepts
every connection already queued on the socket without blocking, and
ends only where none came. It then removes the socket, lets `lock` go
and exits. A command that met no answer meanwhile waited on
`start.lock`, and finds nothing answering once it has it, so it starts
a new process. None is lost between the two. The process also ends at
its next tick once `host/` or its state folder is gone, removing the
socket. A test's scratch removed at its end ends the process it
started.

It keeps nothing the records and the host's state do not hold: the
entries and the runs (`running/`, the branches' records), the index and
the readers' documents. Runs are supervised in their own process groups
and outlive it (core.10). A process killed mid-round leaves what a
stopped `sluice run` leaves, and the next process's rounds carry it on
from the records (S10; 122, `perform_landing`).

### D7. The reader is the host process

For each repository it holds, the task takes the reader's lock and
writes the document as the driver does today. It also takes
`readers/<repository>.driver`, so an older sluice's wait or watch on
the host finds a driver live and follows rather than running rounds.
Where an older `sluice run` holds that lock, the task runs no round of
the repository while it lives and follows its document, as a second
`sluice run` does today. The other repositories go on.

A watch or a wait finds the document as followers do today
(`Reader::Follow`). It also opens a `follow` connection, which keeps
the repository's task alive while it is open and tells the process it
is followed. A follower whose connection drops, because the process
ended or was killed, reaches it again at its next tick, starting it
where none answers (129). It goes on reading the last document until
the new process writes one.

A reader of an older sluice, a watch or a wait of a release before
this one, can still hold a repository's reader lock. The task takes
its lease as a newer watch does today (`Reader::step(true, ..)`). It
follows until the older reader lets go, then reads, and runs no round
before it reads. The lease rules of `overview-reader` stand, with the
host process in the newer watch's place.

### D8. A newer sluice takes over

`hello` answers the process's version. A command whose version is
newer, compared as release numbers (`reader::older`), sends
`hand-over`. The process then stops its ticks. Each task finishes the
round it is in, lets its locks go and ends. The process ends as in D6,
without waiting for idleness. Connections held for room or a follow
close, and their commands reach the new process. The newer command
polls until the socket is gone, for up to the ten seconds of D2, then
starts its own. An older command meeting a newer process uses it as it
is. A request the process does not know is answered with an error
naming both versions, and the command says it cannot use this host
process. A build from a checkout is 0.0.0, older than any release, so
a test's binary never displaces a host's installed sluice. Each runs
in its own state folder anyway.

### D9. Waits and watches follow, and run no round

`wait_on` loses its round. At each tick it reads as a follower and
runs nothing, where today it calls `intake` and `s.round`. Where the
host process takes passes only when asked (D13), it sends `pass` first.
`Reading::driven` and `driven_by` go, since a driver is always there to
follow. A round's failure on the wait's branch is written to the
document as the branch's failure (133). The wait says it once on
stderr in those words and goes on following to its timeout. A wait
reports a landing once its record says landed, or once the branch holds
its work (core.48). What a round failed after the landing is in the
host process's log and the branch's state, never in the wait's exit.
Exits 3 for a round that failed before the landing go with the round.
Exit 3 stays for a read the wait itself could not make.

A submit writes its entry as today, or opens its request and takes it
in until the sibling change. It then sends `serve`, and returns with
`--no-wait` or waits as `sluice wait` does. `wt submit` is a waiting
submit. `sluice overview --json --watch` follows as a wait does, with
the `follow` connection D7 gives it.

### D10. `sluice step` is the host process's round

`Command::Step` sends `step`. The repository's task runs `s.round` on
the branch, between its own passes, and answers with the step's
`--json` document today's step prints: `outcome`, `released`,
`previews_released`, `releases` and `held`. The command prints that
document under `--json`, and otherwise the lines today's step prints
from it. The exits are as before. A step's round is the host process's
round of the branch, so `sluice step` and a pass behave alike (rule
core.14).

### D11. `sluice run` follows

`sluice run` stays a command so a host's unit that runs it keeps
working. It is hidden from the help and the skills. It checks the
host's systems, says once on stderr "sluice starts its own host
process; nothing needs `sluice run`", and opens a `follow` of kind run
for its repository. Each pass's report comes down the connection: the
lines a pass of `sluice run` printed on stdout, and each branch it
passed by with its words. The command prints them as the pass did.
`--rounds n` ends after n passes, exiting as the last pass did (3
where it passed a branch by). Where passes are taken only when asked
(D13), it sends `pass` for each. A unit's `sluice run` therefore keeps
its repository's task alive while it runs, and costs one connection.
The path not taken is removing the command, which would turn each
host's unit into a restart loop until it is retired.

### D12. The host's room in one place

**The seam.** `Tests` gains `hold(&self, kind: RunKind) ->
Result<Option<Part>>`, and `start` takes the part: `start(&self,
request, part: Part)`. `Part` is opaque to the core, and dropping one
gives its room back (record 0087, option 1). `choose_starts` holds a
part for each run it would start, a combination's of its window or one
opened again, before it writes the decision that marks the run
starting. Where `hold` gives none, it marks no further run starting.
The round keeps its parts through its landings, releases and outcomes
and hands each to its run's `start_run`. A run marked starting with no
part, a stopped round's, is opened again by the next round as today. A
start the check seam refuses at the bound opens again each run decided
behind it, its part dropped and no place made, as 71 has it: the host
holds none of the room the round counted on. A run a lead marks
starting under the claim it gives a pool's job is no stopped round's:
the job's round holds its part as it starts it, as `start_claimed`
does, and opens it again where its host has none (88, S19).
`capacity` and `starting_here` go from the decision, since a part is
the count. `perform_starts`'s second reading of `running()` goes too.
`start_claimed` takes the part its claim's start holds.
`Started::AtBound` stays for a start whose part was refused by the
host process after a hand-over.

**The room.** `host_process::room::Room` is one value in the host
process behind a mutex. It counts the registry's live entries (D12,
registry), the parts it holds, and the waiting runs with their kinds
and times. Its admission is record 0087's:

- A combination's part is held where a run of the bound is free,
  unless that run is one a session may take, a session's or a
  preview's run waits, and the turn is the sessions'. Held in room a
  waiting session could have taken, the turn becomes the sessions'.
- A session's or a preview's part is held where a run of the bound
  less `[runs] reserved` is free and is owed to no one. That is, no
  session's or preview's run waits ahead of it, and either the turn is
  the sessions' or the combinations waiting are fewer than the free
  runs. Held, the turn becomes the queue's.
- A refusal records or renews the run's wait, and a hold removes it. A
  combination's wait counts within thirty seconds of its round's last
  ask. A session's or a preview's counts while its connection is open.

**The registry.** The host process admits through `HostCount` under
`running/.lock` as starters do today, so an older sluice's `sluice
exec` on the host still counts against it and is counted. Each part
writes its entry with a hold the host process keeps locked. `start`
hands the hold to the run's supervisor as an admission does today. A
part dropped unstarted, by a start that failed, a refused write or a
connection that closed, unlocks its hold at once. A host process that
starts counts the entries whose supervisors still hold their runs,
which outlived the process before it.

**Sessions and previews.** `sluice exec` and `sluice preview` send
`hold` and keep the connection open while they wait. The process
answers `waiting` with why: "the host is at its bound", or "a
combination's run starts first" while a free run is owed to a waiting
combination. It answers `held` once the room is theirs. The command
prints the reason once and starts its run with the part. Its `started`
line, with the run directory, ends the connection's hold. Ctrl-C closes
the connection and gives back the part or the wait. The check
profile's `Tests` in a command process holds through this connection.
In the host process it holds through `Room` itself.

**What a person reads.** The overview's `room` gains `waiting`, by
kind, and `turn`, `queue` or `sessions`, read from the host process
where one answers. Its `host_process` is `{version, pid, since}`, or
null where none answers, beside `host`, which stays the host's name that
the tools watching the host read. `sluice status` with no target prints
the same room line.

### D13. Passes when asked: the tests' host

The tests drive rounds one command at a time. A host process that took
passes on its own would race them. Under `SLUICE_ROUND_SLEEP=record`,
the value the tests already set to skip pauses, the host process takes
no pass of its own. It runs a pass only for a `pass` or a `step`, and
answers each once the pass ends; a `serve` holds the repository for the
next ask, so a `--no-wait` submit leaves the queue for the test's next
command. Each follower, a wait, a watch or `sluice run`, sends `pass` at
each of its ticks. A test's
rounds are therefore those its commands asked for, in its own order.
The process still checks at each real second whether it should end
(D6), so a test's scratch removed ends it.

`exec::Host` gains `host_process(&self) -> &dyn HostProcess`, the
requests of D2 as methods. `Bound` answers through the socket.
`support::FakeHost` answers in the test's own process: `pass` and
`step` run the pass or the round on the fakes' seams there, a `serve`
holds the repository for the next ask, `hold` asks a `Room` it keeps,
and every follower asks a pass, since the fake takes passes only when
asked. The tests over the fakes
therefore keep their commands. Their rounds run through the same
`host_process::pass` the host process runs.

The path not taken is a setting that keeps the host process from
starting, with waits running rounds as before. It keeps the code path
135 removes alive under a flag (core.12 rules out a behaviour the
statement forbids kept behind a flag).

### D14. Which commands reach it

- **`serve`, starting it where none answers:** a submit, waiting or
  not, by worktree, by name or by request; `withdraw`, `amend`,
  `set-aside`, `branch create`, `branch accept`, and a `release` of a
  checkout or preview.
- **`follow`:** `sluice wait`, a waiting submit, `wt submit`, `sluice
  overview --watch` and `sluice run`.
- **`step`:** `sluice step`.
- **`hold`:** `sluice exec` and `sluice preview`.
- **Reach only where work is in play:** `sluice status`, `sluice
  overview` and the home view. Where none answers they read whether
  anything is in play as an idle tick reads it, with no process
  started, and start the process only where it is. This is how a host
  whose process crashed, or slept, is moved again by the next command
  of sluice's (135).
- **Never:** `hook`, `credential`, `supervise`, `protect`, `forge`,
  `inbox`, `equip` beyond the index, and `--check` forecasts. They move
  no queue.

## Looking across the family

- **flume.** Its C.6 step is nothing under record 0093. A settle reads
  each place's outcome, and its read through `sluice status` or `wait`
  starts the host process where work waits and none runs (D14). flume's
  rooms (70, 130, 145, 181) and sluice's (150) agree where they meet:
  a wait is the host's, equals start in the order they came, and the
  count and the choice are one step. Each bounds its own thing, agents
  and checks, and neither calls the other.
- **The herdr plugin.** Record 0094 keeps one `sluice overview --json
  --watch` per repository a pane is in. Each is a follower of the host
  process (D7), so the host process lives while panes watch, as 129
  has a reader live while it is followed.
- **The runner.** Boomdrive's core.13 and record 0003 name sluice's
  driver among the services a host keeps. Under this design nothing of
  sluice's is kept, and the runner's pass, which settles flume, is the
  host's clock. That repository's change is its own (principle 11), and
  this change names it in the proposal.
- **One decider in one process** (principle 6). The queue's rounds,
  the bound and the readers are each decided in the host process. The
  registry is kept so an older sluice on the host still counts and is
  counted, and so a process that starts counts the runs that outlived
  the last. The allowance's pause stays in the host's state, which
  every task of the process reads (proposal, Out of scope).

## Risks / Trade-offs

- **One process holds every repository.** A fault that ends it stops
  every queue on the host until the next command. Tasks run in threads
  of their own with panics caught (D5), and any command restarts it
  (D14). A crash loses nothing (D6).
- **A long-lived process.** What a pass keeps by revision is forgotten
  each pass (`next_pass`), as `sluice run`'s already is. A task ends
  with its repository's work, so nothing of an idle repository stays
  in memory.
- **Followers keep it alive.** A watch per repository with open panes
  keeps the process up all day. It idles as 129 has an idle reader
  idle, with no process started and nothing read but what tells it
  whether anything changed.
- **Mixed versions during a rollout.** An older wait finds the driver
  lock held and follows. An older `sluice run` keeps its repository's
  rounds and the task follows it (D7). An older `sluice exec` admits
  through the registry the host process counts (D12). An older reader
  gives way by the lease.
- **The tests' host differs in what triggers a pass** (D13). What a
  pass does is the same code, so a test's round is the host's.
- **A socket in `/tmp`.** It lives in the user's own directory, which
  record 0063 checks is theirs and closed to others. A directory that
  fails those checks refuses the start, naming why.

## Migration Plan

1. The bolt lands with the host process. Hosts install the release.
2. A host's `sluice run` units keep running as followers (D11) until
   afterthought/swancloud removes them, which record 0093 names as the
   per-repository services that retire.
3. Agents' instructions that tell them to wait so the queue moves, or
   to keep `sluice run` going, change in this repository with the code
   (tasks 6.1 to 6.3). Those elsewhere change when their host moves
   (principle 11).

Rollback is installing the previous release. Its waits find no driver
lock held once the host process has ended, and run rounds as they did.

## Open Questions

None.
