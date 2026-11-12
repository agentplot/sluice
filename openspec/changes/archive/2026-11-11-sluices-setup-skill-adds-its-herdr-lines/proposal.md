# sluice's setup skill adds its herdr lines

## Why

sluice's herdr plugin shows nothing of the queue until herdr's own
`config.toml` holds its lines: a key for the status popup and one for
the jump, the `$sluice` row under each agent, `$sluice_q` on each
workspace's row, the tab bar's segment, and toasts delivered. herdr
gives a plugin no way to declare them, so a person adds them. Today the
plugin's setup pane prints them and the person copies them in by hand
(`plugins/herdr/bin/setup.sh`). Pasted as printed, they break a file
that already holds a `[ui]` or `[ui.toast]` table, since TOML refuses a
table opened twice. The printed jump key, `prefix+shift+m`, may already
be another plugin's: on deneb it is the annotation plugin's
`annotate.manage`, and deneb's own file binds the jump to
`prefix+shift+s` instead
(`~/.local/state/crew/agentplot-main/reports/herdr-config-for-the-group.toml`).
`herdr config check` reports two commands on one key, but not a command
on a key herdr binds by default.

Proposal 134 had a setup print what each tool needs and the person
compose it. The user declined it on 2026-11-11: "I'd rather they just be
able to tell their agent to set up the plugin for me. I don't know why
we have to go do extra shit." This change replaces the units
`sluices-herdr-setup-prints-what-it-needs` and
`boomdrives-setup-shows-herdrs-lines-for-the-family`.

The file stays the person's. agentplot/blueprints `design/principles.md`
10 has a tool's file written "by its own module, by hand or by a
control plane's local layer, and never by another tool". The person's
agent edits herdr's file at the person's asking, as their hand would,
and no script, hook or plugin of sluice's writes it.

## What Changes

- **Asked to set up sluice's herdr plugin, the setup skill has the agent
  add the plugin's lines to the person's own herdr file.** Asking is the
  person's word to edit it, so the skill asks nothing more before
  writing. The skill finds the file herdr names on the `Config:` line of
  `herdr --help`. It reads the keys the file binds and the keys herdr
  binds by default (`herdr --default-config`). It binds the popup and
  the jump to setup's keys where those are free, and to free keys of
  its own choosing where they are not, saying which and why.
- **The lines come from the plugin's setup.** The skill runs the
  installed plugin's `bin/setup.sh`, from the root `herdr plugin list`
  names, with herdr's state directory for the plugin, so the tab bar's
  entry names this host's copy of the segment. The skill keeps no copy
  of the lines.
- **The lines are merged into the file as the person keeps it.** The
  skill adds keys inside tables the file already holds and never opens a
  table twice. It adds the `$sluice` row to the person's own agent rows
  and `$sluice_q` to the first of their workspace rows. It adds the
  segment in the form the file already writes `tab_bar_right` in, and
  turns toasts on where delivery is off or unset. The person's lines,
  comments and order stay as they are, and nothing the file already
  holds is added twice.
- **A file a tool writes is shown its lines, never written.** Where the
  file is a link, is read-only, or the person says a tool generates it,
  the skill writes nothing and shows the lines merged, for the person to
  add where the file is generated. This is how the skill already treats
  the host's own sluice file.
- **`herdr config check` gates the write.** Before editing, the agent
  copies the file outside herdr's folder. Where the check fails, it puts
  the file back as it was and shows the check's words. Where the check
  passes, it runs `herdr server reload-config` and says what it added.
- **The plugin's setup pane, its README and the site's herdr guide name
  the agent's way first.** Adding the lines by hand from the pane stays
  the other way. The site's guide shows setup's own segment entry, with
  its 30-second limit.
- The decision is recorded as record 0113 and rule core.57, in the
  section *Setting up*.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plugin-setup`: the setup skill adds the herdr plugin's lines to the
  person's herdr file at their asking. It takes them from the plugin's
  setup, binds free keys, merges them into the file as the person keeps
  it, shows them instead where a tool writes the file, and checks and
  reloads herdr.
- `herdr-status`: the setup pane's first lines name both ways to add its
  lines, the agent's at the person's asking and the person's own, and
  herdr's check and reload after either.

## Impact

- `plugins/sluice/skills/setting-up-sluice/SKILL.md`: the description
  and a section on herdr's plugin. New:
  `plugins/sluice/skills/setting-up-sluice/references/herdr.md`, the
  procedure.
- `plugins/herdr/bin/setup.sh`: its first lines. `plugins/herdr/tests/run.sh`:
  the case "status: setup shows, never edits" reads them.
  `plugins/herdr/README.md`, "Setting it up".
- `site/src/content/docs/guides/herdr.md`, "Set up your keys and
  sidebar".
- New: `docs/adr/0113-…md`. `docs/architecture/core.md` and
  `site/src/content/docs/architecture/rules.md` (core.57).
- Unchanged: what the plugin shows and reads. The lines the setup pane
  prints, its keys among them, are unchanged, and so is what sluice
  does. No binary changes.

### Out of scope

- **Flume's herdr lines.** Flume's herdr plugin finds herdr's file by a
  rule of its own and reports what the file lacks in `flume status`
  (agentplot/flume `plugins/herdr/README.md`). Having Flume's own setup
  add Flume's lines in the same way belongs to agentplot/flume. The
  design names it for the planner.
- **A setup for the whole family.** No skill of sluice's adds another
  tool's lines. Each tool's setup adds its own, so a set needs no
  composer of its own.
- **A key the setup pane suggests.** `prefix+shift+m` stays setup's
  suggestion for the jump. It is free of herdr's own keys. Reading the
  person's file is what keeps it from clashing with another plugin's
  key, and no default can be free of every plugin.
