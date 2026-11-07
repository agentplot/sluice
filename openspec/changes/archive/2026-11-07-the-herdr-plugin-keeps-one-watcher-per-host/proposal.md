## Why

Record 0094, accepted 2026-11-07, answers the user's words on signal
2026-11-07-agentplot-ops-ac49fa28/01: "what about the herdr/plugin's
creating many/many processes". The herdr plugin will keep one watcher
for each user on a host, serving every herdr server. Each repository
gets one overview watch, and each document is kept once for every
server's views. While nothing changes, the watcher forks nothing. The
record amends record 0023 in part and builds on record 0056, under
which every watch of a repository on a host follows one reader.

What the plugin does at 26e131bd (`plugins/herdr`):

- `lib/common.sh` puts the plugin's state in a folder named by a
  checksum of the server's socket (`servers/<sum>`). Every herdr server
  therefore has its own lease, documents, told events, forecasts and
  tokens.
- `bin/watch.sh` takes a lease that names the server's socket. It keeps
  a renewer subshell that forks `sleep` and the lock's commands every
  fifteen seconds. Between ticks it wakes every second, forking `sleep`,
  `head`, `cut` and `ls`. At each fifteen-second tick it runs
  `hd pane list` to see that its server answers. It then runs `look`,
  with three `hd` lists, `git` twice for each pane folder and a `jq` for
  each source, and `forecast.sh` and `tokens.sh --refresh`, each of
  which lists the panes again and runs `jq` for each document and
  worktree.
- Each source is four processes: an outer subshell, the left side of a
  pipe, the source's command and a reader loop. The reader forks a
  fresh `sh bin/document.sh` for every line. `document.sh` runs `jq`,
  `hd` and `sha256sum` some two dozen times, one call for each field and
  each event.

On deneb, eleven herdr servers ran eleven watchers over the same
repositories, about 280 processes, and they loaded the host. Record
0093's host process changes who reads the queues. It does not change the
plugin's processes.

## What Changes

- **One watcher per user on a host.** The lease moves from a server's
  folder to the plugin's state for the host. It names the watcher's
  process by its pid and the identity `lib/procs.sh` reads, with the
  plugin folder and version. It is never renewed, so no renewer runs.
  Each server's startup, pane and worktree events register that server
  with the host's watcher and wake it. They start a watcher only when
  the lease's process has ended, or when it runs another plugin folder
  or version.
- **One watcher serves every server.** The watcher calls herdr through
  each registered server's own socket. It stops serving a server whose
  socket is gone or has been replaced. It also stops serving one that
  has not answered two calls in a row. Once it serves none, it stops
  with its sources. A former version's watcher, one per server, gives
  way to it, and that version's folders for each server are cleared.
- **One read of each repository.** The watcher runs one
  `sluice overview --watch --json` for each repository with sluice's
  records that a pane of any server is in. It stops a repository's
  watch once no pane of any server is in that repository. A source is
  its command and one reader of its lines, two processes, and nothing
  more.
- **Each document kept once.** Documents, the events already told,
  blamed tests, learned worktrees, forecasts and probes live in the
  host's state, which every server's popup, dashboard, segment, sidebar
  and toasts read. Each server's own folder keeps only what is that
  server's: its socket, the tokens last reported to it and its panes as
  last read.
- **Nothing forked while nothing changes.** The watcher waits on its
  tick's one `sleep`. A signal wakes it at once when a herdr event
  registers a server, a source's reader keeps a new document, or a
  popup asks for a repository. At a tick it uses shell builtins alone:
  whether each server's socket is still the same file, and whether any
  worktree's head has moved. It forks nothing further unless one of them
  changed. On a server that shows sluice's tokens, it reads the panes and
  sets the tokens again once a minute. That keeps each token within the
  two minutes it lives, and moves its clock.
- **Each line handled whole.** A source's reader handles each document
  line with one `jq`. That call reads the document whole and says what
  to keep and what to tell. The reader then forks only to replace files
  and to call herdr for each new toast. The events told are kept as one
  list per repository, in place of a folder named by a SHA-256 for each
  event.
- **Each server's views are its own.** Tokens are set on each server's
  panes and workspaces from the one document. A blame or a conflict is
  told in each server with a pane in its repository. One whose
  repository no pane is in, as a source of the user's may print, is told
  in every server. A landing is told in each server where its pane is
  open and not in front. A worktree is forecast once, whichever servers'
  panes are in it.
- **The segment is the host's.** It reads the documents in the host's
  state. It needs nothing of its environment but `HOME` and `PATH`.
- **The README and the site's herdr guide** say what the plugin keeps
  running, and how soon it sees a pane's named branch and a token's
  clock.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `herdr-watch`: one watcher per user on a host serving every herdr
  server. One overview watch for each repository a pane of any server is
  in, stopped when none is. Each document kept once in the host's state,
  read whole in one pass. Nothing forked while nothing changes.
- `herdr-sidebar`: every server's panes and workspaces get their tokens
  from the one document. A worktree is forecast once across the servers,
  and again within a tick of its head moving.
- `herdr-toasts`: an event is told once on the host, in each server
  with a pane in its repository. A landing is told in each server where
  its pane is open and not in front.
- `herdr-status`: the segment reads the host's documents and needs only
  `HOME` and `PATH`.

## Impact

- `plugins/herdr` alone. Most of the change is in `bin/watch.sh`,
  `bin/document.sh`, `bin/tokens.sh`, `bin/forecast.sh`,
  `lib/common.sh`, `lib/lease.sh` and `lib/panes.sh`. `bin/status.sh`,
  `bin/segment.sh`, `bin/jump.sh` and `bin/link.sh` read the host's
  state. `herdr-plugin.toml` keeps its startup and events. The
  stand-in `tests/fake/herdr` answers each socket from its own files and
  logs the socket with each call. `tests/run.sh` gains the tests named
  in the tasks.
- Nothing under `crates/`, as records 0023 and 0094 say: the plugin
  reads sluice through `sluice overview` alone (`status <id> --json` and
  `submit <branch> --check --json` as before) and submits, withdraws,
  lands and releases nothing.
- State: the plugin's state folder gains host-wide `docs/`, `told/`,
  `failed/`, `checks/`, `probe/`, `want/` and `run/`, with
  `watch.lease`. `servers/<sum>/` keeps a server's socket, tokens and
  panes. The first watcher of this version removes a former version's
  folders for each server once their watchers have given way.
- Docs: `plugins/herdr/README.md` and
  `site/src/content/docs/guides/herdr.md`. Records 0023, 0056 and 0094
  stand as written. No rule changes. The plugin may name herdr, since
  rule core.1 reaches no crate's code under `plugins/`, and it already
  has its own moon project (`plugin-herdr`) for core.21.
- Out of scope: the popup's redraw each second while it is open,
  which is a person's view and no watcher's. Also out of scope is the
  segment's run every five seconds, which herdr's own `tab_bar_right`
  interval makes, as `setup` shows. Flume's herdr plugin keeps no
  process once flume record 0051 as amended 2026-11-06 lands, which is
  flume's own change. Record 0093's host process is what each overview
  watch follows once it is released (129). This plugin needs no change
  for it.
