# Design

## Context

The rule is record 0094, accepted 2026-11-07. One watcher per user on a
host serves every herdr server. It reads each repository once and keeps
each document once for every server's views. Between a document's
changes and herdr's events it forks nothing on a timer. The plugin only
reads (0023). Each overview watch follows the host's one reader of the
repository's records (0056), which record 0093's host process becomes
once it is released (129).

What herdr gives a plugin, at herdr 0.9.5:

- Each server has its own socket. Every command herdr runs for the
  plugin, the startup, an event's, an action's or a pane's, has
  `HERDR_SOCKET_PATH` and `HERDR_BIN_PATH` naming the server that ran
  it. The `herdr` command talks to the server whose socket is in its
  environment.
- `HERDR_PLUGIN_STATE_DIR` is one folder for every server of a user:
  deneb's `~/.local/state/herdr/plugins/agentplot.sluice` holds
  eighteen `servers/<sum>` folders.
- The manifest's `[[startup]]` and `[[events]]` (`pane.created`,
  `worktree.created`) run a command in the server that had the event.
  Nothing runs when a server stops: a stopping server removes its
  socket.
- A pane's token reported with `--ttl-ms` lives that long unless it is
  reported again. The sidebar shows it as given, so a clock in it is
  the plugin's to move.

What the plugin runs at 26e131bd, for each herdr server:

| Process | Lives | Forks |
|---|---|---|
| `watch.sh` | while its server answers | every second `sleep`, `head`, `cut`, `ls`; every tick `ls -di`, `awk`, `hd pane list`, `look`, `sh forecast.sh`, `sh tokens.sh --refresh` |
| the renewer subshell | with the watcher | every fifteen seconds `sleep`, `mkdir`, `date`, `mv` |
| a source: outer subshell, the pipe's left side, the command, the reader loop | while the repository is watched | for each line `sh document.sh`, which runs `jq`, `hd` and `sha256sum` some two dozen times |

`look` lists panes, tabs and workspaces, runs `git` twice for each pane
folder, and runs `jq` for each source. `forecast.sh` and `tokens.sh`
each list the panes again and run `jq` for each document, worktree and
token. Signal 2026-11-07-agentplot-ops-ac49fa28/01 counted eleven
servers and about 280 processes on deneb.

## Goals / Non-Goals

**Goals:**

- One watcher process for a user's herdr servers on a host. One
  overview watch, two processes, for each repository a pane of any
  server is in.
- While nothing changes, the watcher waits. It wakes for a document, a
  herdr event, a popup's asking, a worktree's head moving, a server
  stopping, and what time itself changes on screen.
- Each document line is handled whole, with a fixed handful of
  processes.
- Each server's tokens and toasts are its own. They come from the one
  document of each repository.

**Non-Goals:**

- The popup's redraw each second while it is open (`bin/status.sh`).
  That is a person's view while they look, and no watcher's.
- The segment's run every five seconds. herdr's `tab_bar_right`
  interval makes it, as `setup` shows.
- Anything under `crates/` (0023, 0094), and the host process of record
  0093.
- Flume's herdr plugin, which flume's own change takes to no process
  (flume record 0051 as amended 2026-11-06).

## Decisions

### D1. The lease is the host's and names a process

`watch.lease` sits in the plugin's state folder, beside `servers/`. It
names the watcher's pid and its identity as `lib/procs.sh` reads it
(start time from `/proc` or `ps`, else its command line), then the
plugin folder and the version. A lease is held while its process lives
and is still that process (`same_process`). Nothing renews it, so no
renewer runs. Where the system cannot tell a process from a later one
given its pid (no `ps`, no `/proc`), the pid alone is read, and the log
says so once, as the watcher already does for its sources.
`lib/lease.sh`'s lock moves beside the lease. It guards taking and
releasing the lease alone.

An `--ensure` registers its server (D2). Where the lease's process lives
and runs this plugin folder and version, the ensure sends it `USR1` and
exits. Otherwise it starts a watcher. A watcher that finds the lease
held by a live process of its own folder and version exits at once. One
of another folder or version takes the lease. The watcher it displaces
reads the lease by the shell's `read` at its next wake or tick, finds
another pid there, and stops with its sources. A watcher also gives way
at a tick when its own folder's `herdr-plugin.toml` is gone or newer
than its lease (`! -e`, `-nt`). A reinstall or a former version put back
therefore stops it, and the next server's event starts the watcher of
the folder installed.

The path not taken is today's lease, renewed every fifteen seconds by a
subshell. Its renewer forks on a timer, and the identity check gives
what its time ever did: a pid the system gives another process later is
not taken for the watcher.

### D2. A server is registered by its events and told gone by its socket

An `--ensure`, the popup and the setup action register their server in
`servers/<sum>/`, where `<sum>` is the checksum of `HERDR_SOCKET_PATH`.
The folder holds `socket`, the path, `herdr`, the server's
`HERDR_BIN_PATH`, and `link`, a hard link to the socket where the
filesystem allows one. The watcher calls herdr for a server with that
server's `HERDR_SOCKET_PATH` and binary. A server is served while its
folder exists.

At each tick the watcher tests each server with builtins: `[ -e
"$socket" ]`, and `[ "$socket" -ef "$link" ]` where a link was made. The
`-e`, `-nt` and `-ef` tests are POSIX.1-2024's, and dash's, bash's and
busybox's. A server that fails is dropped: its folder is removed, and
with it its tokens and panes. So is a server whose herdr calls failed
twice in a row. Once no server remains, the watcher stops, ends its
sources and releases the lease. A server started at a stopped one's path
has another socket file. It runs the startup, which registers it again
with a fresh link.

A server with no socket path is the tests' stand-in, run outside herdr.
It is asked `pane list` at each tick, since nothing else can tell it
gone. herdr always gives a path.

Two paths were not taken. Asking every server `pane list` at every tick,
as today, forks once for each server each tick and grows with the
servers. A long-lived subscription to each server's event API would be
one process for each server, which the record's load target rules out,
and the `herdr` command offers no stream.

### D3. The host's state and each server's

| Kept | Where | Read by |
|---|---|---|
| the lease, its lock | `watch.lease`, `lease.lock` | every `--ensure`, the watcher |
| each repository's latest document, its source's folder, learned worktrees | `docs/<key>.json`, `.dir`, `.paths` | every server's popup, dashboard, segment, tokens, jump, link |
| the events told | `told/<key>.json`, one list | the reader that keeps the document |
| blamed tests | `failed/<key>-<id>` | tokens, toasts |
| forecasts and head stamps | `checks/<sum of worktree>`, `.stamp` | tokens, the tick |
| probes, the popup's asks, the problem | `probe/`, `want/`, `problem` | the watcher, the popup |
| the sources of the running watcher | `run/<pid>/` | the watcher, the next watcher |
| a server's socket, herdr, link | `servers/<sum>/socket`, `herdr`, `link` | the watcher |
| what the watcher last set in a server, its sequence | `servers/<sum>/tokens.json`, `seq` | the watcher |
| a server's panes as last read | `servers/<sum>/panes.json` | the watcher |

`lib/common.sh` names the two: `state`, the host's, and `server`, the
folder of the server in the environment. Every script reads documents
and probes from the host's state.

### D4. The watcher waits on one sleep and is woken by a signal

The watcher's loop starts one `sleep "$tick" &` and blocks in `wait` on
it. `trap 'woke=1' USR1` ends the `wait` at once. POSIX has a trapped
signal end `wait`, and dash, bash and busybox ash do so. The loop then
handles what the markers say changed and waits again on the same
`sleep`, while it still runs. A wake therefore forks no new timer, and
the next `sleep` starts only when one ends.

These wake it:

| Waker | Marker | The watcher then |
|---|---|---|
| a server's `--ensure` | `servers/<sum>/` new or touched | reads that server's panes now and again at the next tick (D6), and looks for repositories (D7) |
| a source's reader that kept a document | `dirty` | sets the tokens of the servers with a pane in that repository (D9), forecasts what moved (D10) |
| a source's reader at its end | `run/<pid>/<source>.ended` | restarts it at the next tick |
| the popup asking for a repository | `want/<sum>` | looks for that repository now |

A signal that arrives while the loop is working leaves its marker, and
the next turn reads it before it waits.

The path not taken is a FIFO the watcher reads. A writer that finds no
reader blocks, so a herdr event's command could hang on a watcher that
had just died. `kill -USR1` to a gone pid fails at once, and the ensure
then starts a watcher.

### D5. A tick runs builtins, and a shown token is set again once a minute

At each tick, fifteen seconds (`SLUICE_HERDR_TICK`), the watcher tests:

- each server's socket (D2);
- each watched worktree's head, by the age of the worktree's git `HEAD`
  and `logs/HEAD` against its stamp (D10);
- each source whose end is marked, which it restarts;
- its own folder's manifest (D1).

These are builtins: `-e`, `-ef`, `-nt`, `read`, and the shell's own
arithmetic. The tick forks nothing more unless one of them found a
change.

Every fourth tick, once a minute, the watcher makes a pass of each
server whose `tokens.json` holds a token. It lists that server's panes
once, computes its tokens once, and reports each token that changed and
each one shown, since every shown token is due within its two minutes.
A server that shows no token has no pass.

This is the design's reading of record 0094's "forks no process on a
timer". Three promises the plugin keeps need time, and only time.
herdr-sidebar's "Tokens fade without the watcher" gives a token two
minutes unless it is set again. The sidebar's clocks (`#95 testing
3m/15m`, `#94 landed 12m` and its end at thirty minutes) move with the
minute and with no document. And herdr-watch's stop "within two ticks"
of the servers stopping needs a tick to stop at. The watcher's timed
work is held to those three. One `sleep` a tick times them, and builtins
make the test. The minute's pass runs only where a token shows, since
it changes what the sidebar shows or keeps it from lapsing. A host whose
panes show nothing forks one `sleep` every fifteen seconds and nothing
else. If agentplot-design reads the record otherwise, this decision is
where the change moves.

Two paths were not taken:

- Tokens with no time to live, cleared when the watcher stops. A
  watcher killed outright would leave its tokens until the next watcher
  started, against herdr-sidebar's fade requirement.
- Clocks left to the next document. On an idle queue, `#94 landed 12m`
  would stand still and never reach its end.

### D6. Each server's panes are read at its events, and kept between them

The watcher reads a server's panes, with their folders, their
`sluice_branch` tokens, and the git worktree and repository of each
folder new to it. It does so at the server's ensure, and again at the
next tick, so a token or a folder given just after a pane opens is
seen. It also reads them at a popup's asking and at the minute's pass
(D5). It keeps them in `servers/<sum>/panes.json`. It runs `git` only
for a folder it has not met. A document's change uses the panes as kept.
Only a landing's toast lists tabs and workspaces too, to tell the pane
in front, and only in the servers with a pane of it.

A pane closed, or a pane's folder or named branch changed with no event,
is therefore seen at the server's next event, or within a minute on a
server showing a token. A named pane always shows a token (`release ·
nothing queued`), so after its first sighting its server has a pass
each minute.

### D7. One source for each repository, two processes each

The look takes the repositories in every server's kept panes, with the
popup's asks. It probes each new one with `sluice overview --json` as
today. It starts the default source once for each repository with
records, in a worktree of it. A repository that a look finds no pane
of any server in has its source stopped. Its document, learned paths,
blamed tests and told list are removed with it, so that its return is a
first sight and tells nothing of what happened while it was away
(herdr-toasts' first sight). A source of the user's (`each_repository =
false`) runs once and is never stopped for want of panes.

A source is one pipeline started from the watcher: `( cd "$where" &&
exec <command> ) | reader &`. Its two members are the watcher's
children. The command's pid and identity are kept in
`run/<pid>/<source>.pids` as today, so the next watcher can end what a
killed one left (the existing tests of a killed watcher's sources
stand). The short helper that settles the command's identity after its
first second runs once, at the start. The reader marks its source ended
at its input's end and signals the watcher. The watcher restarts the
source at the next tick, not at once, so a source that fails as it
starts runs once a tick.

### D8. One `jq` for each document line

The reader is a `while read` loop in the pipeline's right side. It hands
each line to one `jq`, with the repository's told list, learned paths
and blamed tests as its inputs. That `jq` validates and normalizes the
document as `document.sh` does today, and prints what to keep and what
to tell: the document, the learned paths, the told list and the events
new to it, with each event's branch, kind, submissions and paths. The
reader writes each file with the shell's redirection to a temporary file
and one `mv`. It runs `sluice status <id> --json` for a blame or a
conflict whose worktree or tests it lacks, as `learn` does. It calls
herdr once for each toast in each server (D9).

The events told are one JSON list for each repository,
`told/<key>.json`, the events' keys kept while they are among the recent
events. A missing list is a first sight: every event in the document is
told without a toast. This replaces a folder for each event, named by a
SHA-256 that forked a digest tool for each event of each document.
`mkdir` no longer has to stop two watchers telling one event, since
there is one watcher. A list that cannot be written still tells
nothing, and the log says it once, as today.

The path not taken is one long-lived `jq --unbuffered` for each source,
reading its lines with `foreach inputs`. It is a third process for every
source, kept while the queue is idle, where one `jq` for each line costs
nothing while nothing changes.

### D9. Each server gets its own tokens and toasts

`tokens.sh` runs for one server at a time. It takes that server's kept
panes and the host's documents, forecasts and blamed tests, and keeps
what it set in `servers/<sum>/tokens.json` with its own `seq`. When a
document is kept, the watcher sets the tokens of each server with a pane
in that repository, which is where its pane and workspace tokens can
change.

A `blamed` or `conflict` event is told once on the host. It raises a
toast in each server with a pane in its repository, or in every server
when none has, as a source of the user's may print a repository no pane
is in. A `landed` event raises a toast for each landed submission in
each server where its worktree's pane is open and not in front, front
being read in that server alone.

### D10. A worktree is forecast once, and again when its head moves

`forecast.sh` takes the worktrees in every server's kept panes, each
once. A worktree is left out only when all its panes, in every server,
have a branch named. The forecast runs as its key says, as today. At
each look the watcher keeps each worktree's git folder, and at each tick
it tests the age of `HEAD` and `logs/HEAD` there against
`checks/<sum>.stamp` with `-nt`. A time equal to the stamp's counts as
moved, and the forecast's own key stops a repeat. A head that moved
gets its count and forecast at that tick. The stamp is renewed by the
shell's `: >`.

### D11. A former version's watchers give way, and their folders go

A former version keeps a lease in each `servers/<sum>/watch.lease`, and
its watcher stops within its second once that lease no longer names it.
At its start, the host's watcher removes each such lease. Once each
former holder has ended, or after two ticks, it ends any pid a former
watcher killed outright listed under `servers/<sum>/run/`, by its
identity (`end_listed`, as today). It then removes the former folders of
each server: `docs`, `told`, `told.d`, `checks`, `probe`, `want`, `run`,
`failed`, `problem`, `nudge`, `dirty` and `no-sha256`. Each server's
`watch.log` stays for the person to read. The watcher's own log is the
host's `watch.log`.

### D12. The segment is the host's

`segment.sh` sources `common.sh` from its copy and reads `docs/` in the
host's state. It needs no socket, so `HOME` and `PATH` are all it reads
of its environment. The watcher refreshes the copy in `segment/` at its
start. An ensure no longer copies it at every event.

### D13. The popup, the dashboard, jump and link read the host's state

They read the host's documents, learned paths and probes, and call herdr
in the server that ran them, through the environment herdr gave them.
The popup's ask for a repository writes `want/<sum>` and wakes the
watcher through `--ensure`. Jump focuses a pane in its own server, the
one server it can focus. Where the culprit's pane is open only in
another, it says no pane is open, as it does today for a pane open
nowhere.

## Looking across the family

- **Flume's herdr plugin** runs the same per-server lease and watcher
  (`plugins/herdr/bin/watch.sh` on flume main at 5668b5d5). Flume record
  0051, as amended 2026-11-06 on the user's words, ends it: that plugin
  keeps no process, its popup reading status when a person asks. No
  shared owner is needed. Once flume's change lands, sluice's plugin is
  the only one of the family's herdr plugins that keeps a process. A
  later plugin that needs a watcher takes this design: a host lease
  naming its process, servers registered by their events, and a tick of
  builtins.
- **Timber's herdr plugin** keeps no process.
- **sluice's overview reader** (0056) and its host process (0093): this
  change makes the plugin's watches one for each repository, the
  reader's followers, and changes nothing in them.

## Risks / Trade-offs

- **A named branch or a folder change on a quiet server.** A server
  that shows no token reads its panes only at its own events and a
  popup's asking. A pane named long after it opened, in such a server,
  waits for the next. Its first sighting gives the server a token and a
  pass each minute.
- **The last pane leaving** is seen at the server's next reading of its
  panes. The manifest subscribes to no `pane.closed`, since herdr
  0.9.3's plugin events are what the plugin is known to have. Until
  then, a repository's watch runs on.
- **A server killed with its socket left.** A crash that leaves the
  socket file looks alive to the tick. Its calls failing twice drop it.
  On an idle host with no token shown, nothing calls it, and the
  watcher runs on until a server's event or the next document.
- **A socket on another filesystem from the state** gets no hard link,
  so `-e` alone tests it. A server started at the same path registers
  itself again at its startup, which is what tells it from the stopped
  one.
- **Two sources printing one repository's documents** under one host
  and repository key can race on its told list and tell an event twice.
  A source of the user's beside the default for the same repository is
  the one way to it.
- **A trapped `USR1` in another shell.** The watcher is run by `sh`,
  which herdr names. The suite runs under the system's `sh` and under
  dash (`test-sh`, `test-dash`), the two herdr meets.

## Migration Plan

1. `herdr plugin update` installs this version's files in the plugin
   folder.
2. The next startup, pane or worktree event of any server runs this
   version's `--ensure`. It registers that server and starts the host's
   watcher. Each other server registers at its own next event.
3. The host's watcher removes the former leases. Each former watcher
   stops within its second, with its sources. The former folders of
   each server are removed (D11).
4. Putting a former version back stops the host's watcher at its next
   tick (D1). The former version's per-server watchers start at their
   servers' next events.

## Open Questions

- Whether herdr's plugin events include `pane.closed` from 0.9.3. If
  they do, a later change can subscribe to it and stop a repository's
  watch as its last pane closes.
