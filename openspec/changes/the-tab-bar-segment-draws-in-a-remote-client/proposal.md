# Proposal

## Why

Every herdr client, on the server's host or another, should see sluice's
line in the tab bar whenever a branch of the focused repository has a
queue, however loaded the host is. Today it goes blank exactly when the
queue is busiest.

The user saw it first from another machine: watching deneb's sessions
through a remote client, no sluice line, while deneb's dashboard showed a
busy queue (swancloud signal `2026-10-22-chuck-mac-studio-0d9657d0/01`,
handed to madswan as `2026-10-23-madswan-design-8330b5b3/01`). The client
being remote is not the cause. herdr's configuration reference says
command entries in the tab bar "resolve on the Herdr server, so `herdr
--remote` shows the remote machine's values", and slc-1-ops's reading on
deneb (2026-10-24, 04:28–04:30Z; signal
`2026-10-24-madswan-design-d2a76780/01`) found every failure in the
servers' own logs to be one warning, `tab bar status command failed …
error="timed out after 2s"`: over 1,400 of them, 721 in one session's log
alone, clustered in the hours deneb's load ran 60 to 126. Run by hand at
a load near 7, the same command took 0.04–0.06 s.

So the line is drawn on the server, where the state is, and every client
shows what the server drew. It goes blank because the plugin's own setup
gives the command two seconds, and on a loaded host a login shell, git
and two jq runs over every document the watcher holds don't finish in
two. herdr clears an entry whose command times out, so whoever looks,
near or far, sees nothing.

## What Changes

- The tab-bar entry `setup` shows, and the README's, gives the command
  30 seconds instead of 2. herdr runs one command at a time per entry and
  keeps the last line until the next run ends, so on a loaded host the
  line is late, never blank.
- The segment reads only the focused repository's documents, never every
  document the watcher holds, so its work doesn't grow with the other
  repositories on the server and a bad document elsewhere can't blank it.
- The README says where the line comes from: herdr runs the command on
  the server the panes run on, so every client, on that host or another,
  shows the server's queue; the client's own `config.toml` holds the
  entry, which is why the command names `$HOME`.
- A new requirement says the segment depends on nothing of the client's
  host: given the server's home, socket and the pane's directory, it
  prints the server's queue, so a client on another host reads the same
  as one on the server's.

Out of scope: the `config.toml` files on deneb and mac-studio, which
carry `timeout_seconds = 2` today. `setup` shows and never edits
(herdr-status, "Setup shows, never edits"); changing them is the
operator's, once this lands, and until then those hosts' lines still
blank under load. The segment's start-up cost on a loaded host (the
login shell herdr uses, one git call) is herdr's and git's and stays as
it is. No later change waits on this one.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `herdr-status`: the segment reads only its repository's documents; a
  new requirement that setup's entry gives the segment 30 seconds, so a
  loaded host's slow run leaves the line in place; a new requirement
  that the segment is the server's, whatever host the client is on.

## Impact

- `plugins/herdr/bin/segment.sh`: reads the focused repository's
  documents by their state key instead of slurping every document.
- `plugins/herdr/bin/setup.sh`: the entry's `timeout_seconds = 30`.
- `plugins/herdr/README.md`: the snippet's time limit, and a paragraph on
  remote clients.
- `plugins/herdr/tests/run.sh`: the segment beside a broken document of
  another repository, the segment with nothing but the server's
  environment, and setup's time limit.
- No core code; the plugin stays outside the core and reads only the
  overview's documents (ADR 0023, core.1).

## Touches

```
plugins/herdr/bin/segment.sh
plugins/herdr/bin/setup.sh
plugins/herdr/README.md
plugins/herdr/tests/run.sh
```
