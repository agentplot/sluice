# Tasks

Work in this change's place on
`change/sluices-setup-skill-adds-its-herdr-lines`, cut from
`bolt/sluices-setup-skill-adds-its-herdr-lines`. That bolt is the base
every `sluice exec` below names. Each moon task runs through `sluice exec
bolt/sluices-setup-skill-adds-its-herdr-lines -- moon run <task>`, as
CLAUDE.md's Running checks has it. No task reads or writes the host's
own herdr file, runs `herdr server reload-config` or installs a plugin:
the skill is prose an agent follows, and the plugin's tests run against
a stand-in herdr.

Commit by pathspec, with Conventional Commits subjects scoped to the
part of the tree touched, citing record 0113 and rule core.57.

## 1. The decision recorded

- [x] 1.1 Write `docs/adr/0113-sluices-setup-skill-has-the-persons-agent-add-the-herdr-plugins-lines-to-their-own-herdr-file.md`, status proposed, as design D8 and Task notes 1.1 give it. Take the next free number where 0113 is taken on main or this bolt. True when `sluice exec bolt/sluices-setup-skill-adds-its-herdr-lines -- moon run root:adrs` passes.
- [x] 1.2 In `docs/architecture/core.md`, add core.57's line to `### Setting up` after core.54, and its details after the last rule's, as design D8 gives them. Rewrite `site/src/content/docs/architecture/rules.md` with `node site/scripts/rules.mjs`. True when `sluice exec bolt/sluices-setup-skill-adds-its-herdr-lines -- moon run site:rules` passes.

## 2. The setup skill adds the herdr plugin's lines

- [x] 2.1 Write `plugins/sluice/skills/setting-up-sluice/references/herdr.md`, the procedure, as design D2 to D7 and Task notes 2.1 give it:
  - herdr and the plugin;
  - reading the lines from the plugin's setup;
  - the file `herdr --help` names, and who writes it;
  - free keys from the file and `herdr --default-config`;
  - the merge, `rows_by_agent` and the three forms of `ui.tab_bar_right` among it;
  - the copy, `herdr config check` and the reload;
  - what the agent says.

  It covers every requirement the change adds to the plugin-setup spec, from "The setup adds the herdr plugin's lines at the person's asking" to "herdr's check gates the write and herdr is reloaded". It names no host, plugin or tool of ours (core.26). True when `sluice exec bolt/sluices-setup-skill-adds-its-herdr-lines -- moon run plugin-sluice:validate` passes.
- [x] 2.2 In `plugins/sluice/skills/setting-up-sluice/SKILL.md`, add the herdr plugin's setup to the description and the section `## herdr's plugin` after `## What does my policy do?`, as design D1 and D7 and Task notes 2.2 give them. The section says the asking is the person's word for herdr's file, and the plugin's install is still asked first. True when `sluice exec bolt/sluices-setup-skill-adds-its-herdr-lines -- moon run plugin-sluice:validate` passes.

## 3. The plugin and its pages name the agent's way

- [x] 3.1 In `plugins/herdr/bin/setup.sh`, the setup pane's first lines name both ways and herdr's check and reload. In `plugins/herdr/tests/run.sh`, the case `status: setup shows, never edits` wants `set up sluice's herdr plugin` and `herdr config check` in the output. Both are as Task notes 3.1 gives them, and as the herdr-status spec's "The two ways to add the lines" says. True when `sluice exec bolt/sluices-setup-skill-adds-its-herdr-lines -- moon run plugin-herdr:test-sh` passes.
- [x] 3.2 In `site/src/content/docs/guides/herdr.md`, "Set up your keys and sidebar", put the agent's way first and keep the way by hand, as design D7 and Task notes 3.2 give it. The segment entry gives `timeout_seconds = 30`. In `plugins/herdr/README.md`, "Setting it up", do the same. True when `sluice exec bolt/sluices-setup-skill-adds-its-herdr-lines -- moon run site:prose` passes with no warning on the page. `plugins/herdr/README.md` has no check of its own: no gate reads `*.md` outside the site (core.21's details).
- [x] 3.3 In the delta `specs/herdr-status/spec.md`, leave "Setup shows, never edits" as the bolt has it, within the 500 characters `--strict` allows, and add the two ways and `herdr config check` and `herdr server reload-config` as a requirement of their own (ADDED), "Setup names both ways to add its lines", carrying the scenario "The two ways to add the lines". True when, in a copy of `openspec/` outside the place, `openspec archive sluices-setup-skill-adds-its-herdr-lines -y` and then `openspec validate --specs --strict --no-interactive` pass. The gate cannot run this while the change is open: `root:specs` reads only `openspec/specs/**`, which the change leaves untouched until it is archived.
