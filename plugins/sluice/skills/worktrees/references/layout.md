# The layout, and the settings it needs

```
project/
├── .git            # file: "gitdir: ./project.git"
├── project.git/    # bare: core.bare = true
├── main/           # worktree on main
└── <branch>/       # one worktree per branch, its name sanitized (slashes to dashes)
```

## From a fresh clone

```sh
mkdir project && cd project
git clone --bare https://github.com/acme/project.git project.git
printf 'gitdir: ./project.git\n' > .git

# A bare clone maps no remote branches: give it the normal refspec, then fetch.
git config remote.origin.fetch '+refs/heads/*:refs/remotes/origin/*'
git fetch origin

# Worktrees record relative paths, so the whole folder can move (git 2.48+).
git config worktree.useRelativePaths true

git worktree add main main
git -C main branch --set-upstream-to=origin/main
```

## Settings and why

| setting | why |
|---|---|
| `core.bare = true` (set by `--bare`) | the git directory has no working copy of its own |
| `remote.origin.fetch = +refs/heads/*:refs/remotes/origin/*` | a bare clone fetches straight into `refs/heads/*` with no `origin/*` refs; without this, `git fetch` cannot show what moved on the remote, and upstream tracking does not work |
| `worktree.useRelativePaths = true` | each worktree's `.git` file and the git directory's back-pointer are relative, so renaming or moving `project/` keeps every worktree working; with it unset, they hold absolute paths and break on a move until `git worktree repair` |
| `project/.git` file | tools started in `project/` find the repository |

To switch an existing repository to relative paths: set the config, then
`git worktree repair` rewrites the existing links.

## worktrunk

Point worktrunk's `worktree-path` beside `main` when the repository is a
`.git` directory, and keep its default otherwise. In
`~/.config/worktrunk/config.toml`:

```toml
worktree-path = "{% if repo is endingwith '.git' %}{{ repo_path }}/../{{ branch | sanitize }}{% else %}{{ repo_path }}/../{{ repo }}.{{ branch | sanitize }}{% endif %}"
```

Check with `wt config show`. Before converting a repository, make sure this
template is live, or new worktrees land at `project.git.<branch>`.

A `[post-start]` hook such as `copy = "wt step copy-ignored"` in
`.config/wt.toml` copies the main worktree's ignored build outputs into a new
worktree, so it starts warm.

## Day to day

```sh
git -C project.git worktree list        # every worktree; nothing should read "prunable"
wt switch --create unit-x               # or: git worktree add unit-x -b unit-x main
git worktree remove unit-x              # when done (under sluice: sluice release first)
```

Branch-local state (a build directory, `node_modules`) belongs to each
worktree. Shared caches go outside every worktree.
