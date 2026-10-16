# Repairing worktree links

A worktree is linked both ways: its `.git` file names
`<git-dir>/worktrees/<name>`, and `<git-dir>/worktrees/<name>/gitdir` names the
worktree's `.git` file. Moving either end breaks the link.

## Symptoms

- `git worktree list` marks a worktree `prunable` ("gitdir file points to
  non-existent location").
- In a worktree, `git status` says "not a git repository" and its `.git` file
  points at a path that no longer exists.
- worktrunk or an editor cannot find the repository from `project/`.

## Fix

```sh
cat <worktree>/.git                          # where it thinks the git directory is
git -C project.git worktree list --porcelain # where the git directory thinks each worktree is

# From the container, after moving the git directory:
git worktree repair
# After moving one worktree, name its new path:
git worktree repair <new-path>
```

`git worktree repair` rewrites both ends and deletes nothing. Run it, then
`git worktree list` again.

Only when a worktree is truly gone (its directory deleted, its work landed)
remove its record: `git worktree prune --dry-run` first, show the user what it
would remove, and prune only on their word. Under sluice, a removed worktree
that held ports is released by the next `sluice exec` or round.

## Absolute or relative

With `worktree.useRelativePaths` unset, both ends hold absolute paths, so
renaming the container breaks every worktree until a repair. Set it once and
repair:

```sh
git config worktree.useRelativePaths true
git worktree repair
```
