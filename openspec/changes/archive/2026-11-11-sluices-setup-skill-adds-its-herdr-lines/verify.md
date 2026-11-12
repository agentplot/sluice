# Verify: sluices-setup-skill-adds-its-herdr-lines

Verified 2026-11-11 on `change/sluices-setup-skill-adds-its-herdr-lines`
at d1429571, cut from `bolt/sluices-setup-skill-adds-its-herdr-lines`
(f8c119c4), with sluice 0.78.1, herdr 0.9.5 and openspec 1.16.0.

## Verdict: archived

The bolt's gate passes, every task is ticked and holds in the tree, every
requirement and scenario of the two delta specs is met, and the specs the
archive writes pass `openspec validate --specs --strict`.

## The gate

`sluice exec bolt/sluices-setup-skill-adds-its-herdr-lines`, the bolt's
gate `#check-fast`, run once on d1429571: **passed, 10 tasks, exit 0**,
in 5m 52s, and sluice recorded the 10 results under their keys for
d142957.

| Task | Outcome |
|---|---|
| `root:format` | passed |
| `root:adrs` | passed |
| `root:changes` | passed |
| `site:rules` | passed |
| `site:prose` | passed |
| `plugin-sluice:validate` | passed |
| `plugin-herdr:manifest` | passed |
| `plugin-herdr:test-sh` | passed: 136 passed, 3 skipped; `status: setup shows, never edits` ok |
| `plugin-herdr:test-dash` | passed: 136 passed, 3 skipped; `status: setup shows, never edits` ok |
| `cli:test` | passed |

The three skips in each herdr suite are the watcher's cases that need
`ps`, which the sandbox refuses (`/bin/ps: Operation not permitted`);
they skip by design and none bears on this change.

Seen beside the gate, and no part of this change: moon warned on each of
the ten tasks that it could not store blobs in the repository's shared
cache (`sluice.git/moon-home/cache/shared/blobs/temp/…`, "Failed to
store blobs, will skip caching the manifest") and could not archive
outputs under the worktree's `.moon/cache/outputs`. That is the installed
sluice 0.78.1's sandbox leaving the runner's shared store unwritable to a
session's run, which core.9 and core.35 (record 0105) rule out. It is
reported here for whoever owns the sandbox profile, not as this change's
work.

## Specs after the archive

`root:specs` reads only `openspec/specs/**`, which the open change leaves
untouched, so the gate cannot weigh the specs the archive writes. Task
3.3's check was run in a copy of the tree's `openspec/` at d1429571,
outside the place:

```
$ openspec validate sluices-setup-skill-adds-its-herdr-lines --strict --no-interactive
Change 'sluices-setup-skill-adds-its-herdr-lines' is valid
$ openspec archive sluices-setup-skill-adds-its-herdr-lines -y
herdr-status: + 1 added; plugin-setup: + 11 added
Totals: + 12, ~ 0, - 0, → 0
$ openspec validate --specs --strict --no-interactive
Totals: 75 passed, 0 failed (75 items)
```

The archive warns, without blocking, that the proposal's Why passes 1000
characters and that the change has more than ten deltas; neither is a
check of the gate's.

"Setup shows, never edits" stays in `herdr-status` as the bolt has it,
with its scenarios "Setup" and "One line for every host"; "Setup names
both ways to add its lines" follows the spec's last requirement.

## Tasks

All eight tasks are ticked, and each holds in the tree:

| Task | Where | Holds |
|---|---|---|
| 1.1 | `docs/adr/0113-sluices-setup-skill-has-the-persons-agent-add-the-herdr-plugins-lines-to-their-own-herdr-file.md` | MADR, `number: 113`, `status: proposed`, the user named as decision-maker of 2026-11-11; 0113 is free on the bolt. `root:adrs` passes. |
| 1.2 | `docs/architecture/core.md`, `site/src/content/docs/architecture/rules.md` | core.57's line in `### Setting up` after core.54 and its details after core.55's; the site's rules page carries the line. `site:rules` passes. |
| 2.1 | `plugins/sluice/skills/setting-up-sluice/references/herdr.md` | The seven steps in order: herdr and the plugin, the setup's lines, the file and who writes it, free keys, the merge, the check and reload, what to say. Each command in a `sh` block; the three forms of `ui.tab_bar_right` as TOML; another plugin written `<plugin>.<action>`. `plugin-sluice:validate` passes. |
| 2.2 | `plugins/sluice/skills/setting-up-sluice/SKILL.md` | The description names the herdr plugin's setup; `## herdr's plugin` follows `## What does my policy do?`, says it needs no repository, that the asking is the person's word for herdr's file, and that the install is still asked. `plugin-sluice:validate` passes. |
| 3.1 | `plugins/herdr/bin/setup.sh`, `plugins/herdr/tests/run.sh` | The pane's first lines name the agent's way and the person's own, and `herdr config check` and `herdr server reload-config`; the case `status: setup shows, never edits` wants `set up sluice's herdr plugin` and `herdr config check`. `plugin-herdr:test-sh` passes. |
| 3.2 | `site/src/content/docs/guides/herdr.md`, `plugins/herdr/README.md` | Both put the agent's way first and keep the way by hand; the site's segment entry gives `timeout_seconds = 30`. `site:prose` passes. |
| 3.3 | `specs/herdr-status/spec.md` (the delta) | One ADDED requirement, "Setup names both ways to add its lines", with the scenario "The two ways to add the lines"; "Setup shows, never edits" is left as the bolt has it. The archive check above passes. |

## Requirements

### herdr-status

| Requirement | Met by |
|---|---|
| Setup names both ways to add its lines | `plugins/herdr/bin/setup.sh`'s first lines: ask your agent to set up sluice's herdr plugin, or add them yourself or to whatever generates the file, then `herdr config check` and `herdr server reload-config`. The test's case reads the agent's way and `herdr config check`; `herdr server reload-config` is printed beside it and read by no case. |

### plugin-setup

Each requirement is met by `references/herdr.md` and `SKILL.md`:

| Requirement | Met by |
|---|---|
| The setup adds the herdr plugin's lines at the person's asking | The reference's opening and `SKILL.md`'s section: ask nothing more, say what was added. Step 3 writes the file with the plugin's lines alone where none exists ("No file yet"); step 5 adds the keys, `$sluice`, `$sluice_q`, the segment and `delivery = "herdr"` ("A file with none of sluice's lines"); step 7 says what and where. |
| The herdr plugin is installed only on the person's word | Step 1: herdr missing or older than 0.9.3 writes nothing; an empty `root` offers `herdr plugin install agentplot/sluice/plugins/herdr`, run on the person's word, nothing written until it is installed. |
| The herdr lines are the plugin's setup's | Step 2 runs `$root/bin/setup.sh` with `HERDR_PLUGIN_STATE_DIR` at `${XDG_STATE_HOME:-$HOME/.local/state}/herdr/plugins/agentplot.sluice`, `env -u HERDR_SOCKET_PATH`, stdin from `/dev/null`; the page's TOML is marked a placeholder. "No copy yet" adds the entry and says when the queue shows. |
| The herdr plugin's actions take free keys | Step 4: an action the file binds keeps its key and gets no other; setup's key where free; otherwise a free key, saying which, why and what holds setup's; the two taken in turn. |
| A key is free where neither the file nor herdr binds it | Step 4 reads the file's `[[keys.command]]`, `[keys]` and `[keys.indexed]`, and `herdr --default-config`'s `[keys]` for actions the file does not rebind, ranges expanded; never free because `herdr config check` passes. |
| The person's herdr file is edited in place | Step 5: in place with the agent's edit tool, never a TOML library; keys inside the table the file holds; the tab bar in the file's own form, as tables with no `tab_bar_right` key and no second `[ui]`; nothing the file holds added again; everything there, nothing written and no reload. |
| sluice's rows go beside the person's own | Step 5: `$sluice` appended last to `rows` and each `rows_by_agent` list, the person's rows staying; `$sluice_q` at the end of the first spaces row; setup's rows only where the file holds none. |
| Toasts are turned on, and the person's delivery kept | Step 5: `"herdr"` where unset or `"off"`; `"terminal"` and `"system"` left. |
| A line of sluice's the herdr file holds changes only on the person's word | Step 5's "What the file already holds": say how it differs and why, in setup's comment's words, change it on the person's word; `timeout_seconds = 2` against 30 named. |
| A herdr file a tool writes is shown its lines, never written | Step 3: the file or its folder a link (naming its target) or read-only, or the person's word that a tool generates it: nothing written, not the target nor a file in the link's place, the lines shown merged with the keys; step 7 says so. |
| herdr's check gates the write and herdr is reloaded | Step 5's copy by `mktemp` outside herdr's folder; step 6 puts it back on a failed check, shows the words and reloads nothing; on `config: ok` runs `herdr server reload-config`, says the next server reads it where none answers and that other servers read it at their reload; step 7 names the copy's path. |

## Rules

core.1 (herdr named only under `plugins/`, the docs and the
architecture), core.21 (no new file type; the new reference is Markdown
under the plugin), core.26 (the change adds no host, team or tool of
ours to the skill, the pane, the README or the site; another plugin is a
placeholder) and core.57 hold. The change departs from no rule and needs
no clause of the statement (core.12).

Nothing in this verification wrote herdr's file, reloaded a server or
installed a plugin.
