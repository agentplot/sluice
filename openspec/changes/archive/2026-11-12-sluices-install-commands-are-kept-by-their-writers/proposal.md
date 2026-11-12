# sluice's install commands are kept by their writers

## Why

Rule core.56 has every install command sluice's pages give name a
release (record 0112). `root:installs`, `check-installs.sh`, held the
pages to it by reading them. Main's rule core.21 now says no task reads
a Markdown file, a README, a skill, a site page, a spec or a record, for
its prose, its commands, its length or its form (agentplot/blueprints
principle 16). agentplot-design ruled, agreed with agentplot-ops, that
`root:installs` and its script go, and that core.56 is a rule its
writers keep, enforced by no task. The design agent amends record 0112
to drop `root:installs` and to add the flake's pin of the plugins.

Taking main into the bolt also brought the setup skill's herdr
reference (record 0113), which offers
`herdr plugin install agentplot/sluice/plugins/herdr` with no `--ref`,
and plugin-setup's requirement that gives the same command. Both break
core.56.

## What Changes

- **`root:installs` and `check-installs.sh` go.** The root project's
  `installs` task and its comment leave `moon.yml`, and the script
  leaves the tree.
- **core.56 says it is kept by its writers.** Its details say no task
  reads the pages for it (core.21; principle 16). Its rule is unchanged.
- **install-from-release's scenarios read the pages, not the script.**
  Each requirement keeps what it asks of an install command. Its
  scenarios say what a person reading the page finds. The requirement
  that `root:installs` holds the commands is removed.
- **The herdr plugin is installed at a release's tag in setup too.**
  The setup skill's herdr reference installs the plugin with
  `--ref v<version>`, `<version>` the version `sluice --version`
  prints, as the herdr guide has it. plugin-setup's requirement says
  so.

## Capabilities

### Modified Capabilities

- `install-from-release`: its scenarios are what a reader finds on the
  pages; `root:installs` no longer holds them.
- `plugin-setup`: the herdr plugin the setup skill installs is at the
  tag of the sluice the person runs.

## Impact

- Removed: `check-installs.sh`; the `installs` task in `moon.yml`.
- `docs/architecture/core.md`: core.56's details.
- `plugins/sluice/skills/setting-up-sluice/references/herdr.md`: the
  plugin's install.
- Unchanged: sluice's code and its plugins' code; core.56's rule line,
  so `site/src/content/docs/architecture/rules.md` is unchanged.

### Out of scope

- **Record 0112's amendment.** The design agent amends it, dropping
  `root:installs` and adding the flake's pin of the plugins.
