---
number: 113
title: sluice's setup skill has the person's agent add the herdr plugin's lines to their own herdr file
status: proposed
date: 2026-11-11
decision-makers:
- the user, 2026-11-11
- the construct of sluices-setup-skill-adds-its-herdr-lines
---

# sluice's setup skill has the person's agent add the herdr plugin's lines to their own herdr file

Status: proposed. The decision is the user's words of 2026-11-11, quoted below; the record is proposed until the user has seen it as a record, and accepted on their yes. The form, how the agent reads the lines, finds the file, picks keys, merges, checks and reloads, is the construct's own, each part cheap to change.

Deciders: the user, 2026-11-11, declining proposal 134's print-and-compose design: "I'd rather they just be able to tell their agent to set up the plugin for me. I don't know why we have to go do extra shit."; and the construct of the change `sluices-setup-skill-adds-its-herdr-lines`, which chose the form below.
Sources: the user's words above, which replace the units `sluices-herdr-setup-prints-what-it-needs` and `boomdrives-setup-shows-herdrs-lines-for-the-family` (crew's run records, deneb, 2026-11-11: proposals 130 and 134 dropped); agentplot/blueprints `design/principles.md` 10 (origin/main d4db801): "A tool's file is written by its own module, by hand or by a control plane's local layer, and never by another tool.", and principle 11, a step an agent now does for the person changing the instructions that have the person do it; deneb's herdr file as annotated for the group, `~/.local/state/crew/agentplot-main/reports/herdr-config-for-the-group.toml`, which binds the jump to `prefix+shift+s` since the annotation plugin's `annotate.manage` holds setup's `prefix+shift+m`, writes the tab bar as `[[ui.tab_bar_right]]` tables, puts `$sluice_q` in the first spaces row beside another plugin's token, and delivers toasts by `"system"`; herdr 0.9.5 as installed on deneb and its documentation for that version (`https://herdr.dev/llms.txt`): `herdr --help`'s `Config:` line, `herdr --default-config`, `herdr config check` (exit 1 on two `[[keys.command]]` on `prefix+m`, `config: ok` with one command on herdr's own `prefix+x`), `herdr plugin list --plugin <id> --json`, `HERDR_PLUGIN_STATE_DIR`, `ui.toast.delivery` and `[ui.sidebar.agents.rows_by_agent]`; sluice at f8c119c4: `plugins/herdr/bin/setup.sh`, `plugins/herdr/lib/common.sh`, `plugins/herdr/tests/run.sh`, `plugins/herdr/README.md`, `plugins/sluice/skills/setting-up-sluice/SKILL.md`, `site/src/content/docs/guides/herdr.md` and the herdr-status spec's requirement "Setup shows, never edits".

## Context and problem statement

sluice's herdr plugin shows nothing of the queue until herdr's own `config.toml` holds its lines: a key for the status popup and one for the jump, the `$sluice` row under each agent, `$sluice_q` on each workspace's row, the tab bar's segment, and toasts delivered. herdr gives a plugin no way to declare them, so a person adds them. The plugin's setup pane prints them for the person to copy in by hand.

Pasted as printed, they break a file that already holds a `[ui]` or `[ui.toast]` table, since TOML refuses a table opened twice. The printed jump key, `prefix+shift+m`, may already be another plugin's, as it is on deneb, and `herdr config check` reports two commands on one key but not a command on a key herdr binds by default. Proposal 134 had a setup print what each tool needs and the person compose it; the user declined it, asking that a person tell their agent to set the plugin up and do nothing else.

The file stays the person's. Principle 10 has a tool's file written by its own module, by hand or by a control plane's local layer, never by another tool, so no script, hook or plugin of sluice's may write herdr's file.

## Considered options

1. The person's agent adds the lines at their asking, following sluice's setup skill.
2. The setup prints what each tool needs and the person composes it (proposal 134).
3. The plugin writes the file, from a startup hook or an action.
4. The set's setup shows the family's lines (Boomdrive's unit).

## Decision outcome

Option 1.

- **The person's asking is the word.** Asked to set up sluice's herdr plugin, the agent edits herdr's file without asking again, then says what it added, the keys it bound and why, and where its copy of the file as it was is. The skill still asks before installing the plugin, and before changing a line of sluice's the file already holds.
- **The lines come from the plugin's setup.** The skill runs the installed plugin's `bin/setup.sh`, from the root `herdr plugin list --plugin agentplot.sluice --json` names, with `HERDR_PLUGIN_STATE_DIR` set to herdr's state directory for the plugin, `${XDG_STATE_HOME:-$HOME/.local/state}/herdr/plugins/agentplot.sluice`, no herdr socket and no terminal, so the segment's entry names this host's copy and no watcher starts. The skill keeps no copy of the lines. Where the plugin is not installed, it offers `herdr plugin install agentplot/sluice/plugins/herdr` and writes nothing until it is.
- **The file is the one herdr names**, on the `Config:` line of `herdr --help`. Where it or its folder is a link or read-only, or the person says a tool generates it, the skill writes nothing and shows the lines merged, with the keys it would bind, for the person to add where the file is generated, as the skill already treats the host's own sluice file. Where no file exists, the agent writes one with the plugin's lines alone.
- **Each action gets a free key.** A key is bound where the file's `[[keys.command]]` or `[keys]` binds it, or `herdr --default-config` gives it to an action the file does not rebind. An action the file already binds keeps its key; otherwise it takes setup's key where that is free, and a free key of the agent's choosing where it is not, the agent saying which, why, and what holds setup's key.
- **The lines merge into the file as the person keeps it.** The agent edits in place, never through a TOML library, and never opens a table the file holds a second time. It appends `$sluice` to the person's agent rows, in `rows` and in each list of `rows_by_agent`, and `$sluice_q` to the end of their first workspace row; it adds the segment in the form the file already writes `ui.tab_bar_right` in; and it sets `[ui.toast] delivery = "herdr"` where delivery is unset or `"off"`, keeping `"terminal"` and `"system"`. Nothing the file already holds is added again, and a line of sluice's that differs from setup's is changed only on the person's word.
- **`herdr config check` gates the write.** The agent copies the file to a fresh path outside herdr's folder before editing; where the check fails, it puts the file back and shows the check's words; where it passes, it runs `herdr server reload-config`, and says that every other running server reads the file at its own reload.

Not taken:

- **Option 2,** by the user's words.
- **Option 3,** by principle 10, and the herdr-status requirement "Setup shows, never edits": a tool writing herdr's file would be another tool writing it.
- **Option 4,** since each tool's setup owns its own lines; run one after another, they take no one key, as each reads the file first.
- **Showing the planned edit and waiting for a yes,** a second step the user's words rule out; the report after the write, with the copy, serves the same end.
- **A copy of the lines in the skill,** which would give them two homes; the site's guide already showed one drifting, `timeout_seconds = 2` against setup's 30.
- **Opening the setup pane and reading it through herdr's pane read,** which wraps the long rules line at the popup's width and opens over the person's work; and **a new plugin action that prints the lines,** whose output herdr hands to no caller.
- **Relying on `herdr config check` for a free key,** which on 0.9.5 reports no command on a key herdr binds by default; **moving setup's jump key,** since no default is free of every plugin; and **asking the person to pick,** a step the user's words rule out.
- **A rule of the skill's own for the file's path,** from `HERDR_CONFIG_PATH` and XDG, a second reading of what herdr already says.

### Consequences

- Good: a person's setup of sluice's herdr plugin is one sentence to their agent, and their file reads as they wrote it, plus sluice's lines, on keys that clash with nothing.
- Good: the lines have one home, the plugin's setup, which the skill, the README and the site's guide read from.
- Neutral: Flume's herdr lines wait on Flume's own setup adopting the same way, agentplot/flume's change.
- Neutral: the plugin's setup pane still shows the lines for a person who adds them by hand.
- Bad: a herdr server other than the agent's own keeps the old file until its reload, herdr's `reload_config` key or its next start.
- Bad: herdr's state directory for a plugin is read by convention, since no herdr command prints it; where herdr moves it, the entry names a missing copy until the person adds the line the setup pane, run by herdr, prints.
- Rule core.57 says it.
