# Design

## Context

See proposal.md for why. The plan behind this change ranked the ideas and
chose this scope; the shared contract settles what sluice gives a reader:
`sluice overview`, its document (items 3, 4) and `from.path` (item 5),
read through the records seam only (item 6), the plugin at `plugins/herdr`
(item 9). ADR 0023 puts `plugins/` outside core.1's reach.

herdr 0.9.3 gives a plugin a manifest of actions, event hooks, panes and
one-shot `[[startup]]` commands, and the whole herdr CLI as its API.
Commands run with the plugin directory as working directory and get
`HERDR_BIN_PATH`, `HERDR_PLUGIN_STATE_DIR`, `HERDR_PLUGIN_CONFIG_DIR` and
`HERDR_PLUGIN_CONTEXT_JSON` (a flat object: `focused_pane_cwd`,
`workspace_cwd`, the ids). Startup commands
are not supervised and run again only on a server handoff. A popup
receives every key, Escape included, and closes when its command exits.
Key bindings, sidebar rows and tab-bar segments live only in the user's
herdr config. Pane and workspace tokens are set with `report-metadata
--token`, capped at 80 characters, and expire with `--ttl-ms`. Toasts
(`notification show`) take a title and a body and no action.

What the probe of sluice found, which shapes D7:
- `sluice overview --json` (without `--watch`) prints the document
  pretty-printed over many lines; `--watch` prints one line per document.
- `sluice submit <branch> --check --json` takes no lock, writes no record
  and moves no branch, but it is not free: it runs `git status
  --untracked-files=all` in the worktree, one `git merge-tree --write-tree`
  per submission ahead plus one for the head (writing loose objects), and,
  on a branch with a `remote`, fetches the branch from the remote on every
  call (`pushed_refs.rs`, `read` fetches). Its JSON is `{"conflict":null}`,
  `{"conflict":{"paths":[…],"with":[…]}}`, or a refusal.
- `sessions[]` lists only worktrees sluice holds a place for, so most
  panes' worktrees are found through `from.path`, not through it.
- A finished submission leaves `submissions[]`; `recent[]` lines name ids,
  not paths, so a blamed id's path is remembered from an earlier document
  or read with `sluice status <id> --json`, as the board does.

## Goals / Non-Goals

**Goals:**
- One `sluice overview --watch` per repository and source, whatever number
  of popups, segments and panes read it.
- Nothing the plugin shows outlives the watcher by more than two minutes:
  tokens carry a time to live.
- Every behavior testable with stand-in `herdr` and `sluice` and a scratch
  git repository, without herdr running.

**Non-Goals:**
- Writing herdr's config. The `setup` action shows snippets; the user, or
  whatever generates their config, adds them.
- Windows: `platforms = ["linux", "macos"]`.
- Reading sluice's record files. Everything comes through the commands in
  the proposal's Impact (core.11, ADR 0023).

## Decisions

### D1. POSIX sh and jq

The plan's choice holds. Each hard part has a plain answer:
- **Keys in the popup.** `stty -icanon -echo min 0 time 10` and a
  one-byte `dd` give a read that returns within a second; the loop redraws
  when the state file's `cksum` changes or a second has passed (for the
  clocks). A lone Escape is told from an arrow key by a second read with a
  tenth-of-a-second timeout. A trap restores the terminal on every exit.
- **Times.** jq does all date arithmetic (`fromdateiso8601`, `now`, with
  any fraction of a second stripped first), so nothing depends on GNU or
  BSD `date`. `SLUICE_HERDR_NOW` overrides `now` for the tests.
- **The watcher.** One subshell per source, `command | while read -r
  line`, each line handed to the document handler (D4). A main loop ticks
  every 15 s: renews the lease, finds new repositories, restarts a source
  that exited, refreshes tokens. Documents arrive at most once a second per
  repository, so a few jq calls per document cost nothing that matters.
- **`sources.toml`.** jq cannot read TOML and no TOML tool can be assumed.
  The plugin reads a documented subset with awk: `[[sources]]` tables,
  `key = "string"` (with `\"` and `\\`), one-line arrays of strings, `true`
  and `false`, `#` comments. Anything else is a logged error naming the
  line, and the default source is used.
- No `stat`, no `setsid`, no `timeout`: none is portable between macOS and
  Linux.

Node or a compiled binary would read TOML and keys more easily, but would
need a build step at install (`[[build]]`) or a runtime the user may not
have; `sh` and `jq` are the only requirements beyond sluice. Scripts are
named `*.sh` so the repository's `format` task runs shellcheck over them.

### D2. Files

```
plugins/herdr/
  herdr-plugin.toml
  README.md
  bin/watch.sh       the watcher: --ensure starts it if the lease is stale
  bin/document.sh    one document in: state file, toasts
  bin/tokens.sh      the sidebar tokens
  bin/forecast.sh    the ahead counts and forecasts (D7)
  bin/status.sh      the popup and the dashboard
  bin/segment.sh     the tab-bar segment
  bin/jump.sh        focus the culprit's pane
  bin/setup.sh       print the config snippets
  bin/open.sh        open one of the plugin's panes
  lib/common.sh      state, names of repositories, calling herdr
  lib/sources.sh     sources.toml, with lib/toml.awk
  lib/panes.sh       panes, their worktrees, the one in front, focusing
  lib/words.jq       every word shown, and the tokens' rules
  tests/run.sh       the harness
  tests/fake/        herdr and sluice stand-ins
  tests/docs/        recorded documents, a recorded herdr context, the
                     expected popup frame
```

The manifest declares: `[[startup]]` `sh bin/watch.sh --ensure`; events
`pane.created` and `worktree.created` running the same (cheap when the
lease is fresh, and it nudges the watcher to look for new repositories);
actions `status`, `dashboard`, `jump`, `setup`; panes `status` (popup,
80% by 70%), `dashboard` (`placement = "tab"`), `setup` (popup).

### D3. The watcher and its lease

`watch.sh --ensure` exits at once when `watch.lease` was touched in the
last 60 s; otherwise it starts `nohup sh bin/watch.sh </dev/null
>>"$STATE/watch.log" 2>&1 &` and exits, so the startup command ends and
herdr's log reader is not held open. The watcher writes its pid in the
lease and touches it every tick; a second watcher that finds a live lease
not its own exits. herdr 0.9.3 leaves the detached watcher running when
the startup command ends; every event and every popup also runs
`--ensure`, so a watcher that died is back at the next one. It stops,
with its sources, once its herdr server has not answered for two ticks.

Repositories: each tick, `herdr pane list` gives every pane's
`foreground_cwd` (else `cwd`); `git rev-parse --path-format=absolute
--git-common-dir` turns each into a repository. A repository not yet seen
is probed once with `sluice overview --json`; with branches or sessions in
it, the watcher starts each `each_repository` source there; without, it is
probed again after five minutes or a nudge. The popup nudges for its own
repository by writing `want/<key>`.

### D4. Sources and state

`$HERDR_PLUGIN_CONFIG_DIR/sources.toml`, absent by default:

```toml
[[sources]]
name = "local"
command = ["sluice", "overview", "--watch", "--json"]
each_repository = true   # run in every repository found (D3)

[[sources]]
name = "build machine"
command = ["my-overview-stream", "app"]   # any command printing overview lines
```

A source without `each_repository` runs once, from the config directory.
Each document is labelled by its own `host` and `repository`, and keyed by
`<host>--<name>-<cksum of repository>`, where `name` is the repository's
directory name without `.git`. State, all under `HERDR_PLUGIN_STATE_DIR`:
- `docs/<key>.json`: the latest document, written to a temporary file and
  renamed, so a reader never sees half of one.
- `paths.json`: submission id to `from.path`, per repository key, learned
  from every document's `submissions[]` and kept for ids still in
  `recent[]`.
- `told`: the keys of events already toasted (branch, event, id or ids,
  `at`), so a restart toasts nothing twice. On a repository's first
  document every event in `recent[]` is marked told without a toast.
- `checks/<cksum of path>`: the last forecast per worktree (D7).

The popup groups documents by repository name, so the same repository
read from two hosts shows as two blocks one under the other. Once peer
records land (ADR 0008) the two blocks agree; merging them into one is
left for then.

### D5. Matching a submission to a pane

A pane matches a submission when the pane's `foreground_cwd` (else `cwd`)
is `from.path` or inside it, compared after both are made absolute without
symbolic links. Documents from another host match no pane unless the paths
happen to agree; nothing in the plugin assumes they do. Focusing a pane is
`herdr agent focus <pane_id>`, which brings its tab and workspace forward
for a pane hosting an agent; for any other pane, `herdr workspace focus`
then `herdr tab focus` of its tab. "In front of the user" is the pane,
its tab and its workspace all `focused` in `pane list`, `tab list` and
`workspace list`.

### D6. Wording

The popup and segment reuse `plugins/sluice-board`'s words line for line:
durations `2m10s`, limits `15m`, combinations `main+#12+#13`, `▶ testing`,
`formed, not running:`, `✓ 19:41 #11 landed as 9ecc34a`, `✗ … blamed on`,
`⚠ … conflict ·`, `N worktrees held · N previews`; the segment is the
board's band line. The popup groups them under TESTING, QUEUED and
RECENT, in the order the board's pane draws them, and the board's pane
carries the same three headings. Sidebar tokens round to
whole minutes (`3m/15m`), since they are refreshed every tick, not every
second. The words are in one file, `lib/words.jq`, and the tests hold them
to the board's fixture.

### D7. The ahead tokens

For a pane whose worktree has no live submission and no blame or conflict
to show, the watcher finds the branch it would land on: the branch of the
worktree's latest submission, else its `sessions[]` entry's, else the only
branch in the document; with none of those, no token. It counts commits
ahead with `git rev-list --count <branch's at>..HEAD` in the worktree. When
the count is above zero and the pair (worktree head, branch `version`) is
not the one in `checks/`, it runs `sluice submit <branch> --check --json`
in the worktree, one forecast at a time across the watcher, in the tick,
never per document. A refusal, a failure or a forecast older than its pair
shows the count alone (`3 ahead`). `ahead_check = false` at the top of
`sources.toml` turns forecasts off, for repositories whose branch fetches
from a slow remote.

### D8. Tokens and their lifetime

Every token is reported with `--source agentplot.sluice`, an increasing
`--seq`, and `--ttl-ms 120000`, and refreshed each tick, so a dead watcher's
tokens fade within two minutes. A landing token is refreshed for 30 minutes
after the landing and then cleared. A pane with nothing to say gets
`--clear-token sluice`. Precedence and wording are in the herdr-sidebar
spec.

## Risks / Trade-offs

- [herdr may end the startup command's children] → D3's fallback: every
  event and popup runs `--ensure`; task 1.2 settles which holds.
- [`--check` fetches a remote branch and writes loose objects] → only on a
  change of head or version, one at a time, and `ahead_check = false`. The
  objects are unreferenced and git's own gc removes them.
- [A blamed submission whose path was never seen] → `sluice status <id>
  --json` reads it; when that fails too, the toast names no pane and the
  jump skips it.
- [The tab-bar segment and the sidebar need config the plugin cannot
  write] → `setup` shows snippets with the state directory's path filled
  in; the startup command copies `segment.sh` and its library into the
  state directory, whose path survives a reinstall (a reinstall makes a new
  plugin directory).
- [The awk TOML subset rejects valid TOML] → the README states the subset;
  the error names the line.
- [herdr changes `HERDR_PLUGIN_CONTEXT_JSON`'s shape] → the field names are
  read in one place, and a recorded context is one of the test documents.

## Migration Plan

New plugin; nothing to migrate. Rollback is `herdr plugin uninstall
agentplot.sluice`; its tokens expire on their own.
