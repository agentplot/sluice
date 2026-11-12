# Design

## Context

**The brief.** When a person asks their agent to set up sluice's herdr
plugin, sluice's setup skill has the agent add the plugin's lines to the
person's own herdr `config.toml`, on the person's word. The lines are
the keys for the queue and for the jump, picked free after reading the
keys the person already binds; the `$sluice` and `$sluice_q` rows added
to their sidebar rows; the tab bar's segment; and toast delivery on.
Then the agent runs `herdr config check` and `herdr server
reload-config`. The file stays the person's: the agent edits it at their
asking, and no tool writes it (principle 10). The plugin's setup pane
stays as the reference the skill reads from.

**Sources.**
- The user, 2026-11-11, declining proposal 134's print-and-compose
  design: "I'd rather they just be able to tell their agent to set up
  the plugin for me. I don't know why we have to go do extra shit." The
  change replaces the units `sluices-herdr-setup-prints-what-it-needs`
  and `boomdrives-setup-shows-herdrs-lines-for-the-family`
  (crew's run records, deneb, 2026-11-11: proposals 130 and 134 dropped).
- agentplot/blueprints `design/principles.md` 10 (origin/main d4db801):
  "A tool's file is written by its own module, by hand or by a control
  plane's local layer, and never by another tool." Principle 11: a step
  an agent now does for the person changes the instructions that have
  the person do it.
- deneb's herdr file as annotated for the group,
  `~/.local/state/crew/agentplot-main/reports/herdr-config-for-the-group.toml`.
  It binds the jump to `prefix+shift+s`, "sluice's own setup suggests
  prefix+shift+m, which Plannotator takes below" (`annotate.manage`).
  It writes the tab bar as `[[ui.tab_bar_right]]` tables, puts
  `$sluice_q` in the first spaces row beside another plugin's token, and
  delivers toasts by `"system"`.
- herdr 0.9.5, as installed on deneb, and its documentation for that
  version (`https://herdr.dev/llms.txt`):
  - `herdr --help` ends with `Config: <path>`, the file it reads, and
    says `HERDR_CONFIG_PATH` overrides it.
  - `herdr --default-config` prints herdr's defaults, its `[keys]` among
    them, each as a commented line such as `# detach = "prefix+q"`.
  - `herdr config check` validates the file. Read 2026-11-11 with two
    `[[keys.command]]` on `prefix+m`, it exited 1:
    `prefix+m: kept keys.command[0].key, disabled keys.command[1].key`.
    With one command on `prefix+x`, herdr's `close_pane`, it said
    `config: ok`.
  - `herdr plugin list --plugin <id> --json` gives
    `.result.plugins[0].plugin_root`. `herdr plugin config-dir <id>`
    prints a plugin's config directory, and nothing prints its state
    directory. herdr hands a plugin's commands `HERDR_PLUGIN_STATE_DIR`,
    and on deneb that is `~/.local/state/herdr/plugins/agentplot.sluice`,
    where the watcher keeps `segment/bin/segment.sh`.
  - `ui.toast.delivery` takes `"off"`, its default, `"herdr"`,
    `"terminal"` or `"system"`.
  - `[ui.sidebar.agents.rows_by_agent]` gives an agent's own rows, which
    replace `rows` for that agent.

**sluice today, read on this branch at f8c119c4 on 2026-11-11.**
- `plugins/herdr/bin/setup.sh` prints the two keys (`prefix+m`,
  `prefix+shift+m`), `[ui.sidebar.agents]` and `[ui.sidebar.spaces]`
  with herdr's own rows plus sluice's, a `[ui]` table with
  `tab_bar_right`, and `[ui.toast] delivery = "herdr"`. Its first lines
  say "add these to herdr's config.toml yourself (or to whatever
  generates it), then run `herdr server reload-config`". The segment's
  command names `$HOME` where the state directory is below it.
- `plugins/herdr/lib/common.sh` takes the state directory from
  `HERDR_PLUGIN_STATE_DIR`, else
  `${XDG_STATE_HOME:-$HOME/.local/state}/herdr-agentplot.sluice`, which
  is no directory herdr gives the plugin. `enlist` runs
  `watch.sh --ensure` where `HERDR_SOCKET_PATH` is set and the watcher's
  lease exists.
- `openspec/specs/herdr-status/spec.md`, "Setup shows, never edits", and
  `plugins/herdr/tests/run.sh`'s case `status: setup shows, never edits`
  hold that setup writes no file and calls no herdr.
- `plugins/sluice/skills/setting-up-sluice/SKILL.md` says nothing of
  herdr. Its "The host's file" checks whether a tool manages the host's
  sluice file (a link, a read-only file or folder) and writes nothing
  then. The `worktrees` skill's layout reference already has the agent
  set worktrunk's `worktree-path` in the person's own
  `~/.config/worktrunk/config.toml`.
- `plugins/herdr/README.md`, "Setting it up", and
  `site/src/content/docs/guides/herdr.md`, "Set up your keys and
  sidebar", tell the person to add the lines themselves. The site's
  segment entry says `timeout_seconds = 2`, where setup gives 30.

**The family.** Flume's herdr plugin needs lines in the same file: its
popup's key and the `$flume` rows. Today it reads herdr's file at
herdr's start, finding it by its own rule (`HERDR_CONFIG_PATH`, else
`$XDG_CONFIG_HOME/herdr/config.toml`, else `~/.config/herdr/config.toml`),
and `flume status` says what the file lacks (agentplot/flume
`plugins/herdr/README.md`, "Setting it up"). Boomdrive's unit would
have had the set's setup show every tool's lines. This design is the
family's one way to put a tool's herdr lines in the person's file:
- **The owner of each tool's lines is that tool's own setup.** The
  person's agent adds them at the person's asking, from the tool's own
  reference of its lines.
- **The file is the one herdr names** on `herdr --help`'s `Config:`
  line.
- **A key is free** where neither the file nor herdr's defaults bind
  it, so two tools' setups, run one after the other, never take one key.
- **The same rules for the rest:** the same merge, the same
  managed-file check, and the same `herdr config check` and reload.

Flume adopting it, and reading herdr's own `Config:` line in place of
its rule, is agentplot/flume's change, named here for the planner. No
set needs a composer of its own.

## Goals / Non-Goals

**Goals:**
- A person who tells their agent "set up sluice's herdr plugin" gets a
  working setup: keys that clash with nothing, sluice's rows beside
  their own, the tab bar's queue, toasts, a file herdr accepts, and a
  running server that has read it. The person does nothing else.
- The person's file reads as they wrote it, plus sluice's lines.
- One home for the lines: the plugin's setup.

**Non-Goals:**
- Flume's lines, or any other tool's (the family, above).
- Changing setup's suggested keys, or what the plugin shows.
- A file a tool generates: the skill shows the lines, and the person
  adds them where the file is generated.
- Testing the skill's conduct by a harness. The skill is prose an agent
  follows, held by `plugin-sluice:validate`. The plugin's part is held
  by `plugin-herdr:test-sh`.

## Decisions

### D1. The person's asking is the word, and the agent edits the file

Asked to set up sluice's herdr plugin, the agent edits herdr's file
without asking again. It then says what it added, the keys it bound and
why, and where its copy of the file as it was is, so undoing is one
copy. The skill's opening rule, ask before changing host config, holds
for everything else. The skill still asks before installing the plugin,
and before changing a line of sluice's the file already holds (D5).

Not taken:
- **Print and compose** (proposal 134): the user declined it.
- **A script, hook or plugin action that writes the file:** principle
  10, and the herdr-status requirement "Setup shows, never edits". herdr
  gives a plugin no way to declare its keys, and a tool writing herdr's
  file would be another tool writing it.
- **Showing the planned edit and waiting for a yes:** a second step the
  user's words rule out. The report after the write, with the copy,
  serves the same end.

### D2. The lines come from the plugin's setup

The skill reads the lines by running the installed plugin's setup as
herdr runs its pane, with no herdr socket and no terminal:

```sh
root=$(herdr plugin list --plugin agentplot.sluice --json | jq -r '.result.plugins[0].plugin_root // empty')
st=${XDG_STATE_HOME:-$HOME/.local/state}/herdr/plugins/agentplot.sluice
env -u HERDR_SOCKET_PATH HERDR_PLUGIN_STATE_DIR="$st" sh "$root/bin/setup.sh" </dev/null
```

- The socket is unset so `enlist` returns at once: the read serves no
  server and starts no watcher.
- The state directory is given, because `common.sh`'s fallback names a
  directory herdr never gave the plugin, and the segment's entry would
  name a copy that is not there.
- `[ -f "$st/segment/bin/segment.sh" ]` confirms the place. Where there
  is no copy, the plugin has not run on this host: the skill adds the
  entry setup prints and says the queue shows in the tab bar once a
  herdr server has started the plugin.
- Where `root` is empty, the plugin is not installed: the skill offers
  `herdr plugin install agentplot/sluice/plugins/herdr` and writes
  nothing until it is installed.

Not taken:
- **A copy of the lines in the skill.** The lines would have two homes.
  The site's guide already shows one drifting: `timeout_seconds = 2`
  against setup's 30.
- **Opening the setup pane and reading it through herdr's pane read.**
  The popup wraps the 170-column rules line at its width, and opens
  over the person's work.
- **A new plugin action that prints the lines.** herdr hands an action's
  output to no caller.
- **Changing `common.sh`'s fallback.** It serves the tests and a script
  run with no herdr, and is no part of this.

### D3. The file is the one herdr names, and the skill checks who writes it

The file is the path on the `Config:` line of `herdr --help`. herdr
resolves `HERDR_CONFIG_PATH` itself, so the skill keeps no rule of its
own for it. Before writing, the skill runs "The host's file"'s check
on that path and its folder:
- a link, named with its target;
- a read-only file or folder;
- the person saying a tool generates the file.

Where any holds, the skill writes nothing: not the file, not the link's
target, not a file in the link's place. It shows the lines as they
would merge (D5), with the keys it would bind (D4), for the person to
add where the file is generated. Where no file exists, the agent writes
one holding the plugin's lines alone, and herdr keeps its defaults for
the rest.

Not taken: Flume's rule from `HERDR_CONFIG_PATH` and XDG, a second
reading of what herdr already says (the family, above).

### D4. Each action gets a free key

The skill gathers the bound keys:
- **The file's:** every `key` of its `[[keys.command]]` entries, and
  every value of its `[keys]` table.
- **herdr's:** every `prefix…` or bare key in the `[keys]` part of
  `herdr --default-config`, for each action the file's `[keys]` does
  not rebind. `prefix+1..9` binds `prefix+1` to `prefix+9`.

For each of `agentplot.sluice.status` and `agentplot.sluice.jump`:
- Where the file already binds a key to the action, that entry stays,
  and nothing is added.
- Where setup's key is free, the action takes it.
- Otherwise the agent picks a free key, and says which, why, and what
  holds setup's key.

herdr 0.9.5's defaults leave both setup keys free. The skill gives no
fixed list of fallbacks, since any list can clash with some person's
file.

Not taken:
- **Moving setup's jump key off `prefix+shift+m`.** No default is free
  of every plugin, and reading the file is what makes a key free.
- **Relying on `herdr config check`.** On 0.9.5 it reports two commands
  on one key, but not a command on a key herdr binds by default.
- **Asking the person to pick.** A step D1 rules out. The person can
  change the key in their file at any time.

### D5. The lines merge into the file as the person keeps it

The agent edits the file in place with its own edit tool. It never
rewrites the file through a TOML library, which would drop the person's
comments and reorder their tables. TOML refuses a table opened twice, so
a key goes inside the table the file already holds.
- **Keys:** a `[[keys.command]]` entry per action that needs one, with
  setup's `type`, `command` and `description`, after the file's last
  `[[keys.command]]`, else at the end.
- **Agent rows:**
  - Where the file holds `[ui.sidebar.agents] rows`, append setup's
    `$sluice` row, with its rules, as their last row.
  - Do the same in each list of `[ui.sidebar.agents.rows_by_agent]`,
    since those replace `rows` for their agents.
  - Where the file holds none, add setup's `rows`, herdr's own plus
    sluice's.
- **Workspace rows:** where the file holds `[ui.sidebar.spaces] rows`,
  append `"$sluice_q"` to the end of their first row. Otherwise add
  setup's `rows`.
- **The tab bar:** setup's one entry, written in the form the file
  already uses.
  - `[[ui.tab_bar_right]]` tables: one more such table, with `type`,
    `command`, `interval_seconds` and `timeout_seconds`.
  - `tab_bar_right = [ … ]` in `[ui]`: one more element.
  - Neither: the key `tab_bar_right = [ … ]` in the file's `[ui]`, or
    a new `[ui]` table where the file has none.
- **Toasts:** `delivery = "herdr"` in `[ui.toast]` (made where absent)
  where delivery is unset or `"off"`. `"terminal"` and `"system"` stay.
- **Already there:**
  - A key is matched by its `command`, the action's id.
  - A row by its token, `$sluice` or `$sluice_q`, anywhere in the rows.
  - The segment by a `command` containing
    `agentplot.sluice/segment/bin/segment.sh`.

  Each is left as it is. Where one differs from setup's (an older path,
  `timeout_seconds = 2`), the agent says how and why setup gives what
  it does, and changes it only on the person's word. Where everything
  is there, the agent writes nothing, reloads nothing, and says the
  plugin is set up and on which keys.

### D6. herdr's check gates the write, and herdr reloads

1. Before editing, the agent copies the file to a fresh temporary
   path outside herdr's folder (`mktemp`), and names it in its report.
   Nothing is written beside the file.
2. After editing, it runs `herdr config check`.
3. Where the check exits non-zero, the agent copies the file back,
   shows the check's words, and stops.
4. Where the check passes, it runs `herdr server reload-config`. Where
   no server answers, it says the next server to start reads the file.
   It says that every other running server reads the file at its own
   reload: herdr's `reload_config` key, `prefix+shift+r` by default, or
   its next start.

### D7. Where it is written

- **`SKILL.md`.**
  - The description gains the request: "set up sluice's herdr plugin:
    its keys, sidebar rows, tab bar and toasts in herdr's
    `config.toml`".
  - A section "herdr's plugin", after "What does my policy do?", says
    that this needs no repository, and that asking is the person's word
    for herdr's file. It follows `references/herdr.md`.
- **`references/herdr.md`**, the procedure, in order:
  1. herdr and the plugin (D2).
  2. Read the lines (D2).
  3. The file, and who writes it (D3).
  4. The keys (D4).
  5. Merge (D5).
  6. Check and reload (D6).
  7. What to say.

  Its examples name other plugins' actions only by placeholders, and no
  host or tool of ours (core.26).
- **The plugin's setup pane's first lines**, in `setup.sh`, name both
  ways (Task notes 3.1). The README's "Setting it up" and the site's
  guide put the agent's way first and keep the snippets for the
  person's own. The site shows setup's own entry, with
  `timeout_seconds = 30`.

### D8. The decision is recorded

**Record 0113**:
- **File:** `docs/adr/0113-sluices-setup-skill-has-the-persons-agent-add-the-herdr-plugins-lines-to-their-own-herdr-file.md`,
  MADR as record 0111 is laid out, `number: 113`.
- **Status:** proposed. The decision is the user's words. The record is
  proposed until the user has seen it as a record, and accepted on
  their yes.
- **Deciders:** the user, 2026-11-11, and the construct of this change
  for the form, D2 to D6, each cheap to change.
- **Sources:** this design's sources.
- **Considered options:**
  1. The person's agent adds the lines at their asking.
  2. The setup prints what each tool needs and the person composes it
     (proposal 134).
  3. The plugin writes the file, from a startup hook or an action.
  4. The set's setup shows the family's lines (Boomdrive's unit).
- **Decision:** it takes 1, D1 to D6 condensed.
- **Not taken:** 2 by the user's words, 3 by principle 10, 4 since each
  tool's setup owns its own lines.
- **Consequences:** good, a person's setup is one sentence to their
  agent. Neutral, Flume's lines wait on Flume's own setup. Bad, a
  server other than the agent's own keeps the old file until its
  reload.
- **Number:** where 0113 is taken on main or the bolt first, take the
  next free number. 0112 is held by the change
  `a-previews-containers-end-with-it`.

**Rule core.57**, in `### Setting up` after core.54. Its line:

> **[core.57]** herdr's configuration file is the person's, and nothing
> of sluice's writes it but the person's agent at their asking: the
> herdr plugin's setup shows its lines, and the setup skill merges them
> into the file as the person keeps it, on keys neither the file nor
> herdr binds, shows them instead where a tool writes the file, and
> puts back a write that fails `herdr config check`.

Its details, after the last rule's details:
- **Rules out:**
  - a script, hook, watcher or action of the herdr plugin, or any other
    part of sluice, writing herdr's `config.toml` (principle 10;
    herdr-status's "Setup shows, never edits");
  - the setup skill keeping a copy of the plugin's lines of its own, so
    the lines have two homes (the site's guide giving
    `timeout_seconds = 2` while setup gave 30);
  - a sluice action bound to a key the file binds, or herdr binds by
    default (deneb, 2026-11-11: setup's `prefix+shift+m` for the jump,
    which the annotation plugin's `annotate.manage` held there; and
    `herdr config check`, which reports no command on herdr's own
    keys);
  - a table the file holds opened a second time;
  - a line of the person's rewritten or reordered, or its comment
    dropped;
  - a line the file holds added again;
  - the file left failing `herdr config check`;
  - writing a link, a read-only file, or a file the person says a tool
    generates;
  - the person asked again before the write, once they asked for the
    plugin's setup (the user, 2026-11-11);
  - the skill adding another tool's herdr lines.
- **Not ruled out:**
  - the person's own key for a sluice action, kept;
  - toasts the person delivers by `terminal` or `system`, kept;
  - the agent's copy of the file as it was, outside herdr's folder;
  - `herdr plugin install` run on the person's word;
  - the person adding the lines by hand from the setup pane;
  - a line of sluice's the file holds, changed on the person's word.
- **Source:** the user, 2026-11-11, declining proposal 134 ("I'd rather
  they just be able to tell their agent to set up the plugin for me. I
  don't know why we have to go do extra shit."); agentplot/blueprints
  `design/principles.md` 10; record 0113.

The line names no host or tool of ours, since the site shows it
(core.26). `site/src/content/docs/architecture/rules.md` is rewritten by
`node site/scripts/rules.mjs`. Where core.57 is taken on main or the
bolt first, take the next free id. core.56 is held by the change
`sluices-plugins-install-from-its-release`.

**The statement** needs no clause. The herdr plugin and the setup skill
are no clause's, and who writes a tool's file is the family's principle
10 (core.12).

### Rules relied on

- **core.1:** herdr is named only under `plugins/`.
- **core.12:** no clause is reinterpreted.
- **core.21:** no new file type. The new reference is Markdown under the
  plugin, as its siblings are, and every check is a task the gate
  already runs.
- **core.26:** the skill, the README, the site and the pane name no
  host, plugin or tool of ours. Other plugins' actions are
  placeholders.
- **core.51:** the module writes no other tool's file, unchanged.

This change departs from no rule.

## Risks / Trade-offs

- **herdr's state directory is read by convention.** No herdr command
  prints it. The convention is herdr 0.9.5's on deneb, and the
  README's segment line already names it. The segment copy's presence
  confirms it. Where herdr moves it, the entry names a missing copy,
  and the setup pane, run by herdr, prints the right one.
- **An agent's edit can break the file.** D6's check and copy put it
  back.
- **Other running servers keep the old file until their reload.** D6
  says so.
- **herdr's defaults change in a later release.** The skill reads them
  from the installed herdr each time, never from a list of its own.
- **The person may dislike a key the agent picked.** The report names
  it, and it is one line in their file.

## Migration Plan

Nothing on a host changes until a person asks. A file set up by hand
keeps its lines, and asking again finds them there (D5).

## Task notes

**1.1** Write record 0113 as D8 gives it, laid out as record 0111 is:
- frontmatter `number`, `title`, `status: proposed`,
  `date: 2026-11-11` and `decision-makers`;
- the status line, saying it is proposed until the user has seen it as
  a record;
- the deciders and sources lines;
- "Context and problem statement", "Considered options", "Decision
  outcome", with what was not taken, and "Consequences".

**1.2** Add core.57's line to `### Setting up` after core.54, and its
details after the last rule's, as D8 gives them. Run
`node site/scripts/rules.mjs` and commit the page it writes.

**2.1** Write `plugins/sluice/skills/setting-up-sluice/references/herdr.md`
from D2 to D6 and D7's outline. Give each step's commands in a `sh`
block, as the skill's other references do:
- D2's three lines and the segment copy's check;
- D3's check, written as "The host's file"'s is, on the path from
  `herdr --help | sed -n 's/^Config: //p'`;
- for D4, `herdr --default-config` with its `[keys]` part read.

Show the merge's three forms of `ui.tab_bar_right` as short TOML
examples, the other plugin's action written `<plugin>.<action>`. End
with what the agent says:
- the file;
- what it added;
- the keys, why, and what holds any setup key it did not take;
- the copy's path;
- whether herdr reloaded, and that other servers read it at their
  reload.

**2.2** In `SKILL.md`:
- The description: add, after "or take pull requests through it", "or
  set up sluice's herdr plugin: its keys, sidebar rows, tab bar and
  toasts in herdr's `config.toml`".
- The new section `## herdr's plugin`, after `## What does my policy
  do?`, says:
  - when the person asks to set up sluice's herdr plugin, follow
    [herdr](references/herdr.md);
  - it needs no repository;
  - the asking is the person's word to edit herdr's file, so the
    opening rule's ask before changing host config is met for that
    file, and still holds for installing the plugin.

**3.1** In `plugins/herdr/bin/setup.sh`, replace the two lines after
`cat <<EOF` with:

```
sluice for herdr: the lines herdr's config.toml needs. Ask your agent
to set up sluice's herdr plugin (with sluice's Claude Code plugin
installed), and it adds them to your file on keys you have not bound.
Or add them yourself, or to whatever generates the file. Then run
`herdr config check` and `herdr server reload-config`.
```

In `plugins/herdr/tests/run.sh`, the case `status: setup shows, never
edits` adds two strings to its `want` list:
`set up sluice's herdr plugin` and `herdr config check`.

**3.2** In `plugins/herdr/README.md`, "Setting it up", and
`site/src/content/docs/guides/herdr.md`, "Set up your keys and
sidebar", start with the agent's way:
- Ask your agent to set up sluice's herdr plugin. With sluice's Claude
  Code plugin installed, its setup skill reads your `config.toml`.
- It binds the popup and the jump to keys you have not bound, and adds
  the sidebar rows, the tab bar's entry and toasts.
- It runs `herdr config check` and `herdr server reload-config`, and
  says what it added.
- Nothing of sluice's writes the file otherwise.

Then keep the way by hand: the setup action and its snippets, the
README's TOML block, and the site's segment entry. The site's entry
reads `timeout_seconds = 30`, as setup prints it. Name no host, plugin
or tool of ours (core.26).
