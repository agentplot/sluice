# Verify

The work on `change/the-herdr-plugin-keeps-one-watcher-per-host` at
34e2bd21, cut from `bolt/the-herdr-plugin-watches-once-per-host` at
ed8474b6, checked on 2026-11-07 against this change's proposal, design,
its four delta specs (`herdr-watch`, `herdr-sidebar`, `herdr-toasts` and
`herdr-status`) and its tasks.

**The work falls short, and the change is not archived.** Each
scenario of the delta specs has a test, and every test that ran passed.
All 20 tasks are ticked. Three things fail:

1. **The gate fails at `root:format`.** treefmt's shellcheck refuses
   `plugins/herdr/bin/watch.sh` and `plugins/herdr/lib/common.sh`, as
   shown under "Checks" below.
2. **Archived, the change fails `root:specs`.** `herdr-sidebar`'s
   requirement "The ahead token forecasts the merge" grows from 447 to
   594 characters. The pinned openspec 1.14.3 warns on more than 500,
   and `--strict` makes that warning a failure. The check is made only
   on the main specs, so `openspec validate --strict` on the change
   passes.
3. **One requirement's only test never runs in the gate.** Task 4.2's
   condition names `watch: three servers, two repositories, six
   processes` among the tests that pass under `sluice exec main -- moon
   run plugin-herdr:test-sh`. There it is skipped, because sluice's
   sandbox refuses `/bin/ps`. It is the only test of the requirement
   "The host runs one watcher and two processes for each source".

The four tests that skip without `ps` were not run outside the sandbox
for this report, since suites here run through sluice. Whether they
pass where `ps` runs is not known from this verify.

## What the code stage needs to do

- **`root:format`.**
  - `each_tokens` (`bin/watch.sh:353`) passes `"$@"` to `tokens_of`,
    but both of its callers (709, 842) pass it nothing (SC2120, SC2119).
    Drop the `"$@"`, or pass the arguments it is meant to carry.
  - `as` (`lib/common.sh:49`) exports `HERDR_SOCKET_PATH` inside its
    `( … )` group, on purpose, so the export stays with the call.
    shellcheck then reads `enlist`'s uses at 124 and 125 as possibly
    lost (SC2030, SC2031). A `# shellcheck disable=SC2030` at 49, with
    the reason, says it is meant.
  - Then run `sluice exec main` in full, not only `plugin-herdr`'s
    tasks.
- **`root:specs`.**
  - Bring "The ahead token forecasts the merge" back under 500
    characters. One way: keep its former text, which ends at "…the set
    of submissions waiting on the branch changes." Then add a
    requirement of its own, such as "A worktree is forecast once and
    follows its head", that states "A worktree SHALL be forecast once,
    whichever herdr servers' panes are in it, and its count SHALL
    follow its head within a tick of the head moving." Move the
    scenarios "Two servers, one worktree" and "A commit" under it.
  - Show the archive passing, as this report does in a scratch copy:
    `openspec archive … --yes`, then `openspec validate --specs
    --strict --no-interactive` with the pinned openspec.
- **Task 4.2.**
  - Either make `watch: three servers, two repositories, six processes`
    run where the gate runs, or have task 4.2's condition say why the
    gate cannot run it and where it is shown (rule core.21 allows that
    when the task says why).
  - The test needs every process whose parent is the watcher, and those
    processes' own children. The watcher's own records of its pids do
    not show that nothing else runs, so they cannot stand for `ps`.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 34e2bd21 | **exit 1**: 5 of 6 tasks passed in 6m 4s, recorded under their keys for 34e2bd2. `root:format` failed. `plugin-herdr:test-sh` and `plugin-herdr:test-dash` each reported "all 135 passed, 4 skipped". `plugin-herdr:manifest`, `site:prose` and `root:changes` (11 of 11) passed |
| `root:format`'s shellcheck | `bin/watch.sh:353` SC2120, `each_tokens` references arguments but none are ever passed; `bin/watch.sh:709` and `:842` SC2119, its callers; `lib/common.sh:49` SC2030, `HERDR_SOCKET_PATH` exported in `as`'s subshell; `lib/common.sh:124` and `:125` SC2031, `enlist`'s reads of it. The other 297 files were formatted with no change |
| The four skips, under both shells | `watch: a watcher stops only its own sources`, `watch: a killed watcher's sources are ended`, `watch: a source is known without ps's start times`, `watch: three servers, two repositories, six processes`: `/bin/ps: Operation not permitted` |
| `openspec validate the-herdr-plugin-keeps-one-watcher-per-host --strict` | valid |
| `openspec archive the-herdr-plugin-keeps-one-watcher-per-host --yes` in a scratch copy of `openspec/` at 34e2bd21, then `openspec validate --specs --strict --no-interactive` (`root:specs`), with the pinned openspec 1.14.3 from `devenv shell` and with 1.16.0 | the archive applies + 5, ~ 8, → 1 across the four specs; **64 of 65 specs pass**, and `herdr-sidebar` fails on `requirements[2]: Requirement text is very long (>500 characters)` |
| "The ahead token forecasts the merge", its text up to its first scenario, in characters | 447 at ed8474b6; 594 archived |
| `git merge-tree --write-tree HEAD origin/main`, main at 5db8b483 | merges cleanly |

The gate ran with this report outside the worktree, so the run was
recorded under its keys (rule core.18). moon warned that it could not
package the plugin tasks' outputs under its cache, and those tasks still
passed. That warning does not come from this change.

## Requirements and scenarios

Every scenario has a test that sets up its WHEN and asserts its THEN.
Every test named below passed under both `test-sh` and `test-dash` in
the gate, except the one marked skipped.

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
| The host runs one watcher and two processes for each source (added) | Three servers, two repositories | `watch: three servers, two repositories, six processes`: **skipped in the gate** (`/bin/ps: Operation not permitted`) |

### herdr-sidebar

| Requirement | Scenario | Test |
|---|---|---|
| The ahead token forecasts the merge (modified; **594 characters archived**) | Clean | `sidebar: clean` |
| | Conflicts | `sidebar: conflicts` |
| | Conflicts with what landed | `sidebar: conflicts with what landed` |
| | Nothing changed | `sidebar: nothing changed` |
| | Forecasts off | `sidebar: forecasts off` |
| | Two servers, one worktree | `sidebar: two servers forecast once` |
| | A commit | `sidebar: a commit is forecast within a tick` |
| A named pane gets no forecast (modified) | No forecast | `sidebar: no forecast for a named pane` |
| | Named in one server, not in another | `sidebar: named in one server, not in another` |
| Every server's panes get their tokens (added) | Two servers, one worktree | `sidebar: two servers, one worktree` |
| | A server stops | `sidebar: a server stops` |
| A pane's folder and named branch are read at herdr's events (added) | Named just after it opens | `sidebar: named just after it opens` |
| | Named later | `sidebar: named later` |

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

All 20 tasks are ticked. Every test a task names is in `tests/run.sh`
under its name. The tests that tasks 2.1 and 4.3 say leave the suite are
gone: `watch: a long tick keeps the lease`, `watch: a renewer killed
holding the lock`, `watch: one watcher per herdr server` and `toasts:
two watchers tell an event once`.

- Every task's condition is `sluice exec main -- moon run
  plugin-herdr:test-sh` (3.1's `test-dash`) passing with the tests it
  names. Those tasks pass in the gate, and so do the tests, apart from
  4.2's.
- Task 4.2's condition is not met as written. Of the tests it names,
  four skip under sluice's sandbox: `watch: a watcher stops only its
  own sources`, `watch: a killed watcher's sources are ended`, `watch:
  a source is known without ps's start times`, and `watch: three
  servers, two repositories, six processes`. The first three test
  behaviour that predates this change. The fourth is new, and it is
  the only test of a new requirement.
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
