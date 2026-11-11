# Verify: sluices-setup-skill-adds-its-herdr-lines

Verified 2026-11-11 on `change/sluices-setup-skill-adds-its-herdr-lines`
at 1838d1ce, cut from `bolt/sluices-setup-skill-adds-its-herdr-lines`
(f8c119c4), with sluice 0.78.1 and herdr 0.9.5.

## The gate

`sluice exec bolt/sluices-setup-skill-adds-its-herdr-lines`, the bolt's
gate `#check-fast`, run once on 1838d1ce: **passed, 10 tasks, exit 0**,
in 6m 16s.

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

sluice did not record the run as the session's: this report was being
written in the worktree while it ran (`not recorded: after the run of
1838d1c the worktree holds …/verify.md beyond its head`). The report is
no check's output, and the run's outcome is as above.

Seen beside the gate, and no part of this change: moon warned on every
task that it could not store blobs in the repository's shared cache,
`sluice.git/moon-home/cache/shared/blobs/temp/…` ("Failed to store
blobs, will skip caching the manifest"), and could not archive outputs
under the worktree's `.moon/cache/outputs`. That is the installed
sluice 0.78.1's sandbox leaving the runner's shared store unwritable to
a session's run, which core.9 and core.35 (record 0105) rule out; it is
reported here for whoever owns the sandbox profile, not as this
change's work.

## Tasks

All seven tasks are ticked, and each holds in the tree:

| Task | Where | Holds |
|---|---|---|
| 1.1 | `docs/adr/0113-sluices-setup-skill-has-the-persons-agent-add-the-herdr-plugins-lines-to-their-own-herdr-file.md` | MADR as record 0111, `number: 113`, `status: proposed`, the user's words quoted, the four options, what was not taken and the consequences, as D8 gives them. 0113 is free on the bolt. |
| 1.2 | `docs/architecture/core.md`, `site/src/content/docs/architecture/rules.md` | core.57's line in `### Setting up` after core.54, its details after core.55's, both word for word as D8; the site's rules page carries the line. |
| 2.1 | `plugins/sluice/skills/setting-up-sluice/references/herdr.md` | The seven steps of D7 in order, each command in a `sh` block, the three forms of `ui.tab_bar_right` as TOML, other plugins written `<plugin>.<action>`, no host or tool of ours (core.26). |
| 2.2 | `plugins/sluice/skills/setting-up-sluice/SKILL.md` | The description gains the herdr request after "take pull requests through it"; `## herdr's plugin` follows `## What does my policy do?`, says it needs no repository, that the asking is the word for herdr's file, and that the install is still asked. |
| 3.1 | `plugins/herdr/bin/setup.sh`, `plugins/herdr/tests/run.sh` | The pane's first lines are Task notes 3.1's, word for word; the case `status: setup shows, never edits` wants `set up sluice's herdr plugin` and `herdr config check`. |
| 3.2 | `site/src/content/docs/guides/herdr.md`, `plugins/herdr/README.md` | Both put the agent's way first and keep the way by hand; the site's segment entry gives `timeout_seconds = 30`. |

## Specs

`openspec validate sluices-setup-skill-adds-its-herdr-lines --strict`:
valid.

### herdr-status

- **Setup shows, never edits** (modified): the pane's first lines name
  both ways, the agent's at the person's asking and the person's own or
  a generator's, and name `herdr config check` and
  `herdr server reload-config` (`plugins/herdr/bin/setup.sh:24-28`). The
  delta keeps the requirement's two scenarios as they were and adds
  "The two ways to add the lines", which the test's case reads.

### plugin-setup

Each requirement is met by `references/herdr.md` and `SKILL.md`:

| Requirement | Met by |
|---|---|
| The setup adds the herdr plugin's lines at the person's asking | The reference's opening and `SKILL.md`'s section: ask nothing more, say what was added. Step 3 writes the file whole where none exists ("No file yet"). Step 5 adds the keys, `$sluice`, `$sluice_q`, the segment and `delivery = "herdr"` ("A file with none of sluice's lines"). |
| The herdr plugin is installed only on the person's word | Step 1: herdr missing or older than 0.9.3 writes nothing; an empty `root` offers `herdr plugin install agentplot/sluice/plugins/herdr`, run on the person's word, and nothing is written until it is installed. |
| The herdr lines are the plugin's setup's | Step 2 runs `$root/bin/setup.sh` with `HERDR_PLUGIN_STATE_DIR` at `${XDG_STATE_HOME:-$HOME/.local/state}/herdr/plugins/agentplot.sluice`, `env -u HERDR_SOCKET_PATH`, stdin from `/dev/null`; the page keeps no copy, its TOML marked placeholders. "No copy yet" adds the entry and says when the queue shows. |
| The herdr plugin's actions take free keys | Step 4: an action the file binds keeps its key and gets no other; setup's key where free; otherwise a free key, saying which, why and what holds setup's; the two taken in turn. |
| A key is free where neither the file nor herdr binds it | Step 4 reads the file's `[[keys.command]]` and `[keys]`, and `herdr --default-config`'s `[keys]` for actions the file does not rebind, ranges expanded, and says never to read a key free because `herdr config check` passes. |
| The person's herdr file is edited in place | Step 5: in place with the agent's edit tool, never a TOML library; keys inside the table the file holds; the tab bar in the file's own form, tables with no `tab_bar_right` key and no second `[ui]`; nothing the file holds added again; everything there, nothing written and no reload. |
| sluice's rows go beside the person's own | Step 5: `$sluice` appended last to `rows` and each `rows_by_agent` list, the person's rows staying; `$sluice_q` at the end of the first spaces row; setup's rows only where the file holds none. |
| Toasts are turned on, and the person's delivery kept | Step 5: `"herdr"` where unset or `"off"`; `"terminal"` and `"system"` left. |
| A line of sluice's the herdr file holds changes only on the person's word | Step 5's "What the file already holds": say how it differs and why, in setup's comment's words, change it on the person's word; `timeout_seconds = 2` against 30 named. |
| A herdr file a tool writes is shown its lines, never written | Step 3: a link (naming its target) or read-only file or folder, or the person's word that a tool generates it, writes nothing, not the target nor a file in the link's place, and shows the lines merged with the keys; step 7 says so. |
| herdr's check gates the write and herdr is reloaded | Step 5's copy by `mktemp` outside herdr's folder; step 6 puts it back on a failed check, shows the words and reloads nothing; on `config: ok` runs `herdr server reload-config`, says the next server reads it where none answers, and that other servers read it at their reload; step 7 reports the file, the lines, the keys and why, and the copy's path. |

## Read on this host

The reference's read-only commands, run here against herdr 0.9.5:

- `herdr --help | sed -n 's/^Config: *//p'` gives the file herdr reads.
- Step 4's `herdr --default-config | awk … | sed … | grep …` lists each
  default action with its key, `switch_tab prefix+1..9` among them, and
  neither `prefix+m` nor `prefix+shift+m`, as D4 says.
- `herdr plugin list --plugin agentplot.sluice --json | jq -r
  '.result.plugins[0].plugin_root // empty'` gives the installed root,
  and the segment's copy is under herdr's state directory for the
  plugin, as step 2 reads it.

Nothing wrote herdr's file, reloaded a server or installed a plugin.

## Rules

core.1 (herdr named only under `plugins/`, the docs and the
architecture), core.21 (no new file type; the new reference is Markdown
under the plugin), core.26 (the skill, the pane, the README and the
site name no host or tool of ours; another plugin is a placeholder) and
core.57 hold. The change departs from no rule, and needs no clause of
the statement (core.12).
