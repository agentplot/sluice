# Proposal

## Why

Someone watching a host's herdr sessions from another machine saw no
sluice line in the tab bar while the same host's dashboard showed a busy
queue. The first reading (madswan signal
`2026-10-23-madswan-design-8330b5b3/01`, from swancloud's
`2026-10-22-chuck-mac-studio-0d9657d0/01`) blamed the client's host for
running the command without the server's state. herdr does not work that
way: its configuration reference says command entries in the tab bar
"resolve on the Herdr server, so `herdr --remote` shows the remote
machine's values", and slc-1-ops's reading on deneb (2026-10-24,
04:28–04:30Z) found every failure in the servers' own logs, all one
warning: `tab bar status command failed … error="timed out after 2s"`,
721 of them in one session's log, clustered in the hours deneb's load
ran 60 to 126. Run by hand at a load near 7, the same command took
0.04–0.06 s.

So the line is already drawn on the server, where the state is. It goes
blank because the plugin's own setup gives it two seconds, and on a
loaded host a login shell, git and two jq runs over every document the
watcher holds don't finish in two seconds. herdr clears an entry whose
command times out, so whoever looks, near or far, sees nothing exactly
when the queue is busiest.

## What Changes

- The segment reads only the focused repository's documents, never every
  document the watcher holds, so its work doesn't grow with the other
  repositories on the server and a bad document elsewhere can't blank it.
- The tab-bar entry `setup` shows, and the README's, gives the command
  30 seconds instead of 2. herdr runs one command at a time per entry and
  keeps the last line until the next run ends, so on a loaded host the
  line is late, never blank.
- The README says where the line comes from: herdr runs the command on
  the server the panes run on, so every client, on that host or another,
  shows the server's queue; the client's own `config.toml` holds the
  entry, which is why the command names `$HOME`.
- A new requirement says the segment depends on nothing of the client's
  host: given the server's home, socket and the pane's directory, it
  prints the server's queue.

Out of scope: the `config.toml` files on deneb and mac-studio, which
carry `timeout_seconds = 2` today. `setup` shows and never edits
(herdr-status, "Setup shows, never edits"); changing them is the
operator's, once this lands. The segment's start-up cost on a loaded
host (the login shell herdr uses, one git call) is herdr's and git's and
stays as it is.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `herdr-status`: the tab-bar segment reads only its repository's
  documents; setup's entry gives it 30 seconds; a new requirement that
  the segment is the server's, whatever host the client is on.

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
