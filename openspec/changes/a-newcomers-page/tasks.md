# Tasks

## 1. The page

- [ ] 1.1 Write `site/src/content/docs/guides/new-to-sluice.md` with title `New to sluice`, a one-line `description`, and `sidebar.order: -3`, holding the four sections design.md sets out: what sluice is (a CI server for merging agents' work, each branch's checks in `.config/sluice.toml`), what it leaves to other tools (no agent orchestrator: it starts no agents, gives no work, makes no agent's worktree), where worktrees come from (Claude Code itself: `claude --worktree`, asking Claude to work in one, subagents in their own; sluice's own checkouts only for combinations and previews), and batch and parallel in exactly two lines, each section linking its guide and the worktrees section linking https://code.claude.com/docs/en/worktrees. Verify by reading the page against design.md's four sections: every claim matches the guide it links, and the batch and parallel section is two lines.
- [ ] 1.2 Make `site:prose` report nothing on the site: `sluice exec main -- moon run site:prose` exits 0 and prints no warning or suggestion for `new-to-sluice.md` (the gate's fast tier runs the same task).
- [ ] 1.3 Check the page's links resolve: `sluice exec main -- moon run site:build` passes, its links validator reporting no broken link on the page. This is run by hand because the site's build is the Pages workflow's, not a gate task (core.21), and it needs the devenv shell's Chromium for the diagrams.

## 2. The README

- [ ] 2.1 Add to `README.md`, inside the centred block, after the "Agents submit…" paragraph and above the badges: `**New to sluice?** [What it is, and what it leaves to other tools](https://agentplot.github.io/sluice/guides/new-to-sluice/).` Verify by reading: the line is the README's first link after the heading and tagline, and its URL is the page's URL from 1.3's build (`dist/guides/new-to-sluice/index.html` exists). No gate reads the README (core.21), so this check is by hand.
