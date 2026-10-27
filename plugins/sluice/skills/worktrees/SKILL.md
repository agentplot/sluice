---
name: worktrees
description: Lay out a git repository as a bare repo with worktrees beside it (project/project.git, project/main, project/<branch>), and work in it safely. Use when the user asks about git worktrees or a bare repo layout, wants a clone or a `.bare` directory converted to it, or has worktrees landing in the wrong place, "prunable" worktrees or a broken `.git` file.
---

# Bare repo and worktrees

sluice expects each agent to work in a git worktree of its own and submit
from there. The layout that suits that best keeps the repository's git
directory bare and every working copy, `main` included, a plain worktree
beside it:

```
project/
├── .git            # a file: gitdir: ./project.git
├── project.git/    # the bare repository
├── main/           # a worktree on main
└── unit-x/         # a worktree per branch, named for it
```

Why this one:

- No working copy is special. `main` is a worktree like the others, so no
  agent ever works in "the" checkout other agents depend on.
- `project/.git` lets tools that look for a repository from `project/`
  (worktrunk, editors) find it.
- Worktrees sit beside each other, not inside one another, so no tool walks
  into another worktree's files.

Details and the reasons behind each setting: [references/layout.md](references/layout.md).

## Pick the procedure

| the repository is | do |
|---|---|
| not cloned yet | [references/layout.md](references/layout.md), "From a fresh clone" |
| a normal clone (`project/.git` is a directory) | [references/convert-from-plain-clone.md](references/convert-from-plain-clone.md) |
| a `.bare` directory with `.bare.<branch>` worktrees | [references/convert-from-bare-dir.md](references/convert-from-bare-dir.md) |
| in the layout, but `git worktree list` shows `prunable`, or a `.git` file points nowhere | [references/repair.md](references/repair.md) |

## Rules that hold everywhere

- **Never move a worktree that something is working in.** A running agent or
  editor keeps its old path; its session breaks. Check first:
  `lsof -d cwd -Fn | grep -F "<path>"` and `git -C <path> status --porcelain`.
- **A plain clone's conversion is one script**,
  `${CLAUDE_PLUGIN_ROOT}/scripts/lay-out.sh`, never commands typed out by
  hand. It refuses while anything but this session holds the folder, and
  carries Claude Code's history: the folder's other sessions to `main/`'s
  key, its memory to the git directory's. This session, `${CLAUDE_SESSION_ID}`,
  it leaves for Claude Code's `/cd <folder>/main`, which the user types.
- **Never `git worktree prune`, `git worktree remove` or `rm -rf` a worktree
  during a conversion.** Moves and `git worktree repair` are enough, and
  nothing is lost if a step fails.
- After any move of the git directory or a worktree, run `git worktree
  repair` (from the container, and with the moved path as an argument).
- Use `git worktree remove <path>` to remove a worktree you are done with,
  never `rm -rf`. Under sluice, run `sluice release` in it first, or let
  `wt submit` do both.
- Show the plan and wait for the user's go before moving anything.

## With sluice

- sluice's own test checkouts are detached worktrees it adds and removes
  itself under `<git-dir>/sluice/`. They show in `git worktree list`, never
  beside your worktrees. A conversion marks them keep; one that shows as
  `prunable` after a crash is sluice's to clean up on its next round, not
  yours to prune.
- Submit from any worktree: `sluice submit <branch>` reads that worktree's
  head. Its `from.path` in the record is the worktree's path.
- sluice is set up in this layout. Asked to set sluice up in a
  repository's only working copy, the setting-up-sluice skill offers the
  layout first, by the same script, and goes on in the plain clone only on
  the user's word; the commit that adds `.config/sluice.toml` then says
  the user chose to keep the plain clone (126). Beside other working
  copies, where every session can already work from one of its own, it
  offers the same move with its cost and goes on as the repository
  stands, laying the folder out whenever the user asks. Give each agent a
  worktree.
