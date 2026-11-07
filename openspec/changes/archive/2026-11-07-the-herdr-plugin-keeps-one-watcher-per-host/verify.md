# Verify

The work on `change/the-herdr-plugin-keeps-one-watcher-per-host` at
4127fb7c, cut from `bolt/the-herdr-plugin-watches-once-per-host` at
ed8474b6, checked on 2026-11-07 against this change's proposal, design,
its four delta specs (`herdr-watch`, `herdr-sidebar`, `herdr-toasts` and
`herdr-status`) and its tasks.

**The work holds, and the change is archived.** Every scenario of the
delta specs has a test that passes in the gate under both shells. All 21
tasks are ticked and their conditions are met. The gate passes, and the
change, archived, passes `root:specs` under the pinned openspec.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 4127fb7c, against main at 5db8b48 | **exit 0**: `passed: 6 tasks` in 6m 15s, `recorded 1 result under its key for 4127fb7` |
| `root:format` | treefmt formatted 340 files, 0 changed; shellcheck passes `bin/watch.sh` and `lib/common.sh` |
| `plugin-herdr:test-sh`, `plugin-herdr:test-dash` | each `all 136 passed, 3 skipped` |
| Their three skips, under both shells | `watch: a watcher stops only its own sources`, `watch: a killed watcher's sources are ended`, `watch: a source is known without ps's start times`: `/bin/ps: Operation not permitted`, the three task 6.1 names |
| `plugin-herdr:manifest`, `site:prose` | pass; `site:prose` 0 errors, 0 warnings |
| `root:changes` | 11 of 11 changes valid, this one among them |
| `openspec archive the-herdr-plugin-keeps-one-watcher-per-host --yes` in a scratch copy of `openspec/` at 4127fb7c, then `openspec validate --specs --strict --no-interactive` (`root:specs`), with openspec 1.14.3 from `devenv shell` | **65 of 65 specs pass** |
| "The ahead token forecasts the merge", archived | the same as on main, 447 characters to its first scenario |
| "A worktree is forecast once and follows its head", archived | 146 characters to its first scenario |
| `git merge-tree --write-tree HEAD origin/main`, main at 5db8b483 | merges cleanly |

The gate ran with this report outside the worktree, so the run was
recorded under its key (rule core.18). moon warned that it could not
store the tasks' outputs in its caches, and every task still passed.
That warning does not come from this change.

## Requirements and scenarios

Every scenario has a test that sets up its WHEN and asserts its THEN.
Every test named below passed under both `test-sh` and `test-dash` in
the gate.

### herdr-watch

| Requirement | Scenario | Test |
|---|---|---|
| One watcher per user on a host (renamed from One watcher per herdr server, modified) | Started twice | `watch: started twice` |
| | Two servers | `watch: two servers, one watcher` |
| | Watcher gone | `watch: watcher gone` |
| | herdr stops | `watch: herdr stops`, `watch: herdr gone` |
| | One of two servers stops | `watch: one of two servers stops` |
| | Another herdr server in its place | `watch: another herdr server in its place` |
| | A former version's watchers | `watch: a former version's watchers give way`; its part for a killed former watcher's source runs only where `ps` runs |
| Sources come from the user's configuration (modified) | No configuration | `watch: no configuration` |
| | Panes of two servers in one repository | `watch: panes of two servers in one repository` |
| | The last pane leaves | `watch: the last pane leaves` |
| | A source of the user's | `watch: a source of the user's` |
| | Configuration it cannot read | `watch: configuration it cannot read` |
| Each repository's latest document is kept (modified) | Two documents | `watch: two documents` |
| | Read by every server | `watch: read by every server` |
| | Source exits | `watch: source exits` |
| Each document is read whole in one pass (added) | A large document | `watch: a large document costs what a small one does` |
| Nothing is forked while nothing changes (added) | Idle, no token shown | `watch: idle ticks run nothing` |
| | A token shown | `sidebar: a shown token is set again once a minute` |
| | A change wakes it | `watch: a change wakes it` |
| The host runs one watcher and two processes for each source (added) | Three servers, two repositories | `watch: three servers, two repositories, six processes`, listing the watcher's children and its readers' by `pgrep` in the gate, where the sandbox refuses `ps` |

### herdr-sidebar

| Requirement | Scenario | Test |
|---|---|---|
| A named pane gets no forecast (modified) | No forecast | `sidebar: no forecast for a named pane` |
| | Named in one server, not in another | `sidebar: named in one server, not in another` |
| A worktree is forecast once and follows its head (added) | Two servers, one worktree | `sidebar: two servers forecast once` |
| | A commit | `sidebar: a commit is forecast within a tick` |
| Every server's panes get their tokens (added) | Two servers, one worktree | `sidebar: two servers, one worktree` |
| | A server stops | `sidebar: a server stops` |
| A pane's folder and named branch are read at herdr's events (added) | Named just after it opens | `sidebar: named just after it opens` |
| | Named later | `sidebar: named later` |

"The ahead token forecasts the merge" is left as main has it, and its
scenarios keep their tests (`sidebar: clean`, `sidebar: conflicts`,
`sidebar: conflicts with what landed`, `sidebar: nothing changed`,
`sidebar: forecasts off`), which pass.

### herdr-toasts

| Requirement | Scenario | Test |
|---|---|---|
| A blame or a conflict raises a toast (modified) | Blamed | `toasts: blamed` |
| | Watcher restarted | `toasts: watcher restarted` |
| | First sight of a repository | `toasts: first sight of a repository` |
| | Three servers | `toasts: three servers` |
| | A repository no pane is in | `toasts: a repository no pane is in` |
| A landing raises a toast only away from its pane (modified) | Looking elsewhere | `toasts: landed, looking elsewhere` |
| | Looking at it | `toasts: landed, looking at it` |
| | Two servers | `toasts: landed in two servers` |

### herdr-status

| Requirement | Scenario | Test |
|---|---|---|
| The segment is the server's (modified) | Only the server's environment | `status: the segment is the server's`, run with `env -i HOME PATH` through `/bin/sh -lc` |
| | Any server on the host | `status: the segment is the same in every server` |

## Tasks

All 21 tasks are ticked. Every test a task names is in `tests/run.sh`
under its name. The tests that tasks 2.1 and 4.3 say leave the suite are
gone: `watch: a long tick keeps the lease`, `watch: a renewer killed
holding the lock`, `watch: one watcher per herdr server` and `toasts:
two watchers tell an event once`.

- Every task's condition but 6.1's is `sluice exec main -- moon run
  plugin-herdr:test-sh` (3.1's `test-dash`) passing with the tests it
  names. Those tasks pass in the gate, and so do the tests, apart from
  the three of 4.2's that 6.1 says the gate skips and why.
- Task 6.1's condition holds. `sluice exec main` passes. `test-sh` and
  `test-dash` each skip only `watch: a watcher stops only its own
  sources`, `watch: a killed watcher's sources are ended` and `watch: a
  source is known without ps's start times`, and each says why. Those
  three test the plugin's reading of a start time through `ps`
  (`lib/procs.sh`) and predate this change; rule core.21 allows a check
  the gate cannot run when its task says why. The change, archived in a
  scratch copy, passes `openspec validate --specs --strict
  --no-interactive` under openspec 1.14.3.
  - In `bin/watch.sh`, `each_tokens` hands `tokens_of` its server folder
    alone.
  - In `lib/common.sh`, `as` says its export is meant to stay in its
    subshell (SC2030), and `enlist` says it reads the environment's own
    socket (SC2031).
  - `watch: three servers, two repositories, six processes` lists
    processes by `ps` where it runs, else by `pgrep -lf -P`. It is
    skipped only where neither can run. The test still asserts every
    child of the watcher and every child of each reader, as
    `command command reader reader sleep`, so nothing beside the six
    passes unseen.
- Tasks 2.5 and 5.6 were read against the specs by hand.
  `plugins/herdr/README.md` and `site/src/content/docs/guides/herdr.md`
  say:
  - one watcher serves every herdr server on a host;
  - one `sluice overview --watch --json` runs in each repository a pane
    of any server is in, and stops once no pane is there;
  - a pane's named branch is read at the server's pane and worktree
    events, about a tick later, and once a minute while the server
    shows a row of the plugin's;
  - every row is set again within a minute and expires two minutes
    after it was last set.

  `site:prose` passes in the gate.
