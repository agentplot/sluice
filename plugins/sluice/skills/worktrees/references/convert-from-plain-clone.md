# Converting a normal clone to the layout

From `project/` with `project/.git` a directory and the working copy in
`project/` itself, to `project/project.git` plus `project/main/`.

The working copy's files move into `project/main/`, so do this only when
nothing is working in `project/`: no editor, agent or shell with its cwd
there (`lsof -d cwd -Fn | grep -F "/abs/path/to/project/"`, the absolute path
with a trailing slash, and the exact path too). Existing worktrees outside
`project/` are kept and repaired.

## Plan, then wait for "go"

1. `git -C project status --porcelain` is empty, or the user agrees to carry
   the changes as they are (they move with the files).
2. Note the branch checked out (`git -C project branch --show-current`) and
   every worktree (`git -C project worktree list`).
3. `project/project.git` and `project/main` do not exist.
4. No registered worktree lives inside `project/` (for example
   `project/.worktrees/x` or `project/.claude/worktrees/…`): the move below
   would carry it into `main/`. If one does, add `! -name <its top-level dir>`
   to the `find` below and repair it afterwards, or stop and report.

## Execute, from the parent of `project`

```sh
cd project
mv .git project.git
git --git-dir=project.git config core.bare true
printf 'gitdir: ./project.git\n' > .git

# The old working copy's files become the main worktree.
mkdir main
# move every entry except .git, project.git and main into main/ (include dotfiles)
find . -mindepth 1 -maxdepth 1 ! -name .git ! -name project.git ! -name main -exec mv {} main/ \;

# Register main/ as a worktree of the bare repository without checking out again.
git --git-dir=project.git worktree add --no-checkout main-tmp <branch>
mv main-tmp/.git main/.git && rmdir main-tmp
git --git-dir=project.git worktree repair main
git -C main reset --mixed      # index matches HEAD; files on disk are untouched

git --git-dir=project.git config remote.origin.fetch '+refs/heads/*:refs/remotes/origin/*'
git --git-dir=project.git config worktree.useRelativePaths true
git worktree repair
```

The `worktree add --no-checkout` step creates the git directory's record for
the worktree; moving its `.git` file into `main/` and repairing points that
record at `main/`. If the worktree record ends up named `main-tmp`, that is
only its internal name; `git worktree list` shows the path.

## Verify

- `git -C main status` shows the same changes as before the move, on the same
  branch.
- `git worktree list` lists `project.git (bare)`, `main`, and every earlier
  worktree, none `prunable`.
- `git fetch origin` populates `origin/*`.

If anything looks wrong, stop and show the user: every file is still on disk
and nothing was deleted.
