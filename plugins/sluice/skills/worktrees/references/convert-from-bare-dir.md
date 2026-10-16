# Converting a `.bare` directory to the layout

From:

```
project/
├── .git              # gitdir: ./.bare
├── .bare/            # the bare repository
├── .bare.main/ …     # or main/
└── .bare.<branch>/   # hidden worktrees
```

to `project/project.git` with plain `project/<branch>` worktrees, without
disturbing anything in flight. Work in `CONTAINER` = the folder holding
`.bare`, `PROJECT` = its name.

## Phase 0: preconditions. Stop and report if any fails.

1. `git -C CONTAINER/.bare rev-parse --is-bare-repository` prints `true`.
2. `CONTAINER/PROJECT.git` does not exist.
3. If worktrunk is used, its `worktree-path` is the conditional template in
   [layout.md](layout.md) (`wt config show`). Otherwise new worktrees would
   land at `PROJECT.git.<branch>`. This is the user's config: tell them,
   don't change it unasked.
4. No process has its working directory inside `.bare` itself:
   `lsof -d cwd -Fn 2>/dev/null | grep -F "CONTAINER/.bare/"` is empty.

## Phase 1: inventory and plan. Print it, then wait for "go".

For each entry of `git -C CONTAINER/.bare worktree list --porcelain`:

- path, branch, and whether it is inside `CONTAINER`;
- **in flight** if any of these hold:
  - a process has its cwd at or under the path (`lsof -d cwd -Fn`);
  - a coding agent (Claude Code, codex, opencode) runs there
    (`pgrep -fl 'claude|codex|opencode'`, cross-checked with
    `lsof -p <pid> -d cwd`);
  - `git -C <path> status --porcelain` is not empty;
- the proposed path: `CONTAINER/<name>`, `<name>` being what follows
  `.bare.`; worktrees not named `.bare.*` keep their path;
- **skip** if in flight, if the proposed path exists, or if it is outside
  `CONTAINER`.

List `.bare.*` directories that are not registered worktrees as stale; leave
them alone.

Print a table: current path, branch, action (move / keep / skip and why).

## Phase 2: execute, in this order, from `CONTAINER`

```sh
mv .bare PROJECT.git
printf 'gitdir: ./PROJECT.git\n' > .git
git worktree repair                   # fixes every worktree's link to the renamed git dir
# for each worktree marked move:
mv .bare.<name> <name> && git worktree repair <name>
git worktree repair
```

Never `rm`, `git worktree remove` or `git worktree prune`. Touch nothing
marked keep or skip beyond the first repair.

Renaming `.bare` is safe with live sessions in worktrees: their `.git` files
point at a wrong path only until the repair a moment later. Moving a worktree
directory is not safe for a live session, which is why in-flight ones are
skipped and moved on a later run.

## Phase 3: verify and report

- `git worktree list` shows every worktree at its expected path, none
  `prunable`.
- Each worktree: `git -C <path> status --short --branch` works and shows the
  same branch as before; a moved one's `.git` points at
  `PROJECT.git/worktrees/…`.
- `cat .git` reads `gitdir: ./PROJECT.git`; `wt list` works from `main`.
- List, without editing, other tools' state that still names the old paths:
  `grep -rl "CONTAINER/.bare" ~/.claude*/projects 2>/dev/null | head` and any
  multiplexer or session manager's config. Claude Code's session history for
  a moved worktree stays keyed to the old path.

Finish with: what moved, what was kept and why, what to rerun once in-flight
worktrees are idle, and which apps (a git GUI, an editor) need the repository
reopened from the new path.

Then consider `git config worktree.useRelativePaths true` and a final
`git worktree repair`, so the next move needs no repair.
