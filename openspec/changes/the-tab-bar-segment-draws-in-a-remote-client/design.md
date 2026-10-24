# Design

## Context

See proposal.md for why. What the design rests on:

- **herdr runs the command on the server.** herdr's configuration
  reference (herdr.dev/docs/configuration, the `tab_bar_right` section,
  for 0.9.5, the version on deneb and mac-studio): "hostname, datetime,
  and command resolve on the Herdr server, so `herdr --remote` shows the
  remote machine's values … Command entries run immediately and then at
  `interval_seconds` without blocking rendering or overlapping a previous
  run … clears the entry after failure, empty output, or
  `timeout_seconds`, and provides the same active workspace, tab, pane,
  socket, binary, and working-directory context as custom command
  keybindings", through `/bin/sh -lc`. The same page: presentation
  settings come from the client's `config.toml`, custom commands run on
  the server. So the client's file holds the entry and the server runs
  it, with the server's socket and the focused pane's directory. That is
  why the segment's command already names `$HOME` (herdr-status, "Setup
  shows, never edits").
- **What ops found on deneb.** Every failure in the servers' logs is
  `timed out after 2s`; there are no non-zero exits. Run as the server
  runs it at load ~7 the segment took 0.04–0.06 s. The failures cluster
  where load ran 60–126. A blank at a quiet moment is right: no branch
  had a queue.
- **What the segment does today** (`plugins/herdr/bin/segment.sh`):
  sources `lib/common.sh` (which makes the state's folders and hashes the
  socket path), finds the repository with `git rev-parse`, then
  `all_docs` slurps every document under `servers/<socket>/docs` with one
  `jq -s` and pipes the array to a second `jq` that keeps the focused
  repository's and words the line. A document's file is named by
  `key_of`: `<host>--<name>-<cksum of the repository's path>.json`.

## Goals / Non-Goals

**Goals:**

- The line is not cleared by a slow run on a loaded host.
- The segment's work is its repository's, not every repository's on the
  server.
- A test holds that the segment needs only the server's home, socket and
  `PATH`, so it stays drawable from any client.

**Non-Goals:**

- Drawing the line on the client or carrying it over herdr's bridge.
  herdr already draws it on the server; nothing to carry.
- The watcher keeping a ready-drawn line for the segment to print. See
  Decisions.
- Editing anyone's `config.toml`; `setup` shows, never edits.
- The state folder being keyed on the literal `HERDR_SOCKET_PATH`
  string. herdr hands the absolute path; only a hand-run with `~/…` in
  the variable lands in another folder, which is a trap for people
  timing it, not a fault the tab bar meets.

## Decisions

**Give the command 30 seconds.** herdr never overlaps runs of one entry
and keeps the entry's last line until a run ends, clearing it only on
failure, empty output or the time limit. With the limit above the
interval, a run slowed by load ends late and replaces the line then;
the tab bar shows the previous line meanwhile instead of nothing. 30 s
is fifteen times the limit that failed and six intervals: far past any
run seen, while a run that truly hangs (git waiting on a lock, say)
still clears within half a minute. *Alternatives:* 10 s, which the
02Z hour at load 126 may well exceed, for no gain; a minute or more,
which would hold a frozen clock on the line long after it stopped
meaning anything.

**Read only the focused repository's documents.** The segment globs
`docs/*--<name>-<cksum>.json` for the focused repository (the same
`name_of` and `sum` that `key_of` uses) and, if none match, prints
nothing without running jq. Otherwise one `jq -s` with the plugin's
words reads those files and words the line, still keeping only
documents whose `repository` is the focused one, since two paths may
share a checksum. That drops a jq process and the read of every other
repository's document, and an unreadable document elsewhere no longer
fails the slurp that today blanks every repository's line.
*Alternative:* keep `all_docs` and filter in jq, as now; its cost grows
with every repository the server's panes touch, which is the cost that
fails under load.

**No pre-drawn line from the watcher.** The watcher could word the
line whenever a document arrives and the segment could print a file.
But the line carries a clock (`2m10s/15m`) that moves each second, so
the watcher would have to redraw while anything is under test, on every
server on the host (deneb runs eight), adding load in the hours that are
already loaded. The segment would still start a login shell and git.
The longer time limit gets the same outcome, a line that is never
blank, with none of that.

## Risks / Trade-offs

- [herdr might clear the entry while a run is in flight, not only when
  it ends badly] → The reference says it clears "after failure, empty
  output, or `timeout_seconds`". Verify on a live server that a run
  slower than the interval keeps the last line (a test entry running
  `sleep 8; date` at `interval_seconds = 5, timeout_seconds = 30`
  should show the date, never a gap). If it does not, the time limit
  alone does not hold the line and the change's premise needs another
  look.
- [A line late by up to 30 s under load shows an old clock] → It is the
  queue's last known standing and is replaced as soon as the run ends;
  the alternative is no line at all.
- [The hosts' `config.toml` keep `timeout_seconds = 2` until the
  operator changes them] → Nothing in this change helps until then; the
  README and `setup` say the new limit, and the merge note says the
  files to change.

## Migration Plan

Once the plugin is reinstalled on a host, the watcher copies the new
segment into the state directory on its next start, as it does today.
On each host whose `config.toml` holds the segment's entry (deneb and
mac-studio, line 130), change `timeout_seconds = 2` to `30`, then run
`herdr server reload-config` for each running session. Rolling back is
the old line; the segment itself reads the same documents either way.
