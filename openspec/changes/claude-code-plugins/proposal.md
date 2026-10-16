# Proposal

## Why

sluice's main users are agents, and nothing taught an agent, or the person
setting sluice up, to use it from Claude Code. A person had to install the
binary, read the configuration reference and write a policy by hand; an
agent had to be told not to merge and how to read a blame. The queue was
also invisible inside a session: an agent that submitted with `--no-wait`
polled, and nobody saw what was under test. ADR 0023 settled that the
plugins live in this repository under `plugins/`, outside the core, and
read the queue through `sluice overview`.

## What Changes

- A marketplace at the repository's root, `.claude-plugin/marketplace.json`,
  installing with `/plugin install sluice --marketplace agentplot/sluice`.
- `plugins/sluice`, a Claude Code plugin of skills and hooks:
  - `setting-up-sluice`: installs or upgrades the `sluice` binary, asking
    first; surveys the repository's tools; writes `.config/sluice.toml`
    from a reference per runner, executor, checkout, driver, remote,
    forge identity, pull requests and previews, with complete examples;
    protects the branch and sets the host up.
  - `using-sluice`: submit, wait, read blames and conflicts, `sluice exec`,
    previews, amend and withdraw.
  - `worktrees`: the bare repository with worktrees beside it, and
    conversions to it that move nothing in use and delete nothing.
  - A guard on Bash that refuses a push, merge or ref update onto a branch
    sluice protects and names `sluice submit` instead (core.4), and a
    session-start note naming the protected branches.
- `plugins/sluice-board`, a mod: the queue in a band above the prompt, a
  pane opened by `/sluice`, a status line, and a wake that tells an idle
  session its submission's outcome.
- The README and the site's landing page open with installing the plugin,
  with ways in for Claude Code and herdr; a site guide on the plugin.
- A core test parses every example policy the plugin ships.

Out of scope: the herdr plugin (`plugins/herdr`), its own change; a doctor
skill, a triage agent and `sluice config check`, which wait on experience
with v1; adding the marketplace to agentplot-kit's Home Manager modules,
a change in that repository.

## Capabilities

### New Capabilities
- `plugin-setup`: installing sluice and writing a repository's policy.
- `plugin-usage`: an agent landing work through sluice.
- `plugin-worktrees`: the bare repository and worktrees layout.
- `plugin-guard`: keeping a session off protected branches.
- `plugin-board`: the queue shown inside a session.

### Modified Capabilities
<!-- none -->

## Impact

- New: `.claude-plugin/`, `plugins/sluice/`, `plugins/sluice-board/`; site
  guides `claude-code.md` and `herdr.md`.
- Changed: the README, the site's landing page, a test in
  `crates/sluice-core/src/policy.rs` and its moon inputs.
- No crate imports `plugins/`; nothing under `crates/` names the plugins'
  tools (core.1, ADR 0023).
