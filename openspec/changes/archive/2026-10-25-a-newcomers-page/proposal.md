# Proposal

## Why

Someone new to sluice follows the README and still can't tell what sluice does for them. On the user's onboarding call of 2026-10-20 ('Sluce Work Tree Issues'), the newcomer thought sluice had made the worktrees their agents were working in, and asked whether sluice's job was to bring those worktrees back together. The user had to say it out loud: "Sluice on its own isn't trying to solve the agent thing", it is "like a CI server for merging your changes back", the worktrees come from Claude Code, and the batch and parallel settings are what keep a host from slowing to a crawl when several agents' long test suites run at once. Nothing on the site or in the README says those four things in one place, and the README sends a newcomer straight into setup.

## What Changes

- A new page on the documentation site, **New to sluice**, first among the guides. In a few short sections it says:
  - what sluice is: a CI server for merging agents' work, where each branch's checks are written in `.config/sluice.toml` much as a CI workflow names what runs on a branch, and where an agent submits its work and sluice lands only a combination that passed;
  - what sluice is not: an agent orchestrator. It starts no agents, hands out no work and makes none of the worktrees agents work in; telling an agent to submit through sluice is the person's or their tool's part;
  - that the worktrees agents work in come from Claude Code itself (`claude --worktree`, asking Claude to work in a worktree, or subagents in worktrees of their own), and that sluice makes only the checkouts it tests combinations in;
  - batch and parallel, in two lines, with a link to the full guide.
- The README links to the page before anything else: one line under its tagline, above the badges, so it is the first link a reader meets.
- The new page passes the site's prose check, `site:prose`, with nothing reported, suggestions included.

Out of scope:

- How setup treats a plain clone, which the same call raised (the newcomer's setup went on in an existing clone, with worktrees landing beside it). That is the statement's S61 (126), the setup's behaviour, and waits on its own change; this page neither describes nor works around it.
- A link from the site's start page. The sidebar lists the page first among the guides; a hero action or card on `index.mdx` waits on a change that redesigns the start page.
- Keeping the pull request workflow a team already has, which the call left for the user to confirm; the [pull requests guide](https://agentplot.github.io/sluice/guides/pull-requests/) stays the page for it.

## Capabilities

### New Capabilities

None. The change adds a documentation page and a README line; no behaviour of sluice changes, so the change carries no spec delta (`skip_specs: true`).

### Modified Capabilities

None.

## Impact

- `site/src/content/docs/guides/new-to-sluice.md`: new page.
- `README.md`: one line added under the tagline.
- No code, configuration, command, state file or plugin changes. `site:prose` already checks every page under `site/src/content/docs`, so the new page is under the gate with no new task (core.21).
- Nothing commercial appears on the page (core.13).

## Touches

```
site/src/content/docs/guides/new-to-sluice.md
README.md
```
