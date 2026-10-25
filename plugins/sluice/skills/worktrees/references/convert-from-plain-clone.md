# Converting a normal clone to the layout

From `project/` with `project/.git` a directory and the working copy in
`project/` itself, to `project/project.git` plus `project/main/`.

One script does it, `${CLAUDE_PLUGIN_ROOT}/scripts/lay-out.sh`, told the
session's id, `${CLAUDE_SESSION_ID}`. Where this page shows either as
written, take it from SKILL.md, which Claude Code fills in. It moves
nothing while any process but the session running it has its working
directory or a file open in `project/`, and it overwrites and deletes
nothing. Existing worktrees outside `project/` are kept and repaired.

## Plan, then wait for "go"

1. When worktrunk is used, its `worktree-path` template is the one in
   [layout.md](layout.md), "worktrunk" (`wt config show`), or new
   worktrees land at `project.git.<branch>`.
2. Run the plan, which changes nothing:

   ```sh
   bash "${CLAUDE_PLUGIN_ROOT}/scripts/lay-out.sh" plan /abs/path/to/project
   ```

   It prints the git directory's new name, `main/` on the branch checked
   out with the changes it carries as they are, every other worktree and
   what becomes of it, each process holding the folder (its id, command,
   and whether by working directory or an open file), and the Claude Code
   history it will carry, from which key to which. It exits 1, with the
   reasons, when `go` would refuse now.
3. Show the user the plan, costs first: each process that must be stopped,
   or must close the folder, and be started again from `project/main`;
   every editor or app to reopen at `main/`, since an editor that only
   watches the folder shows in no listing; the history to be carried and
   where it goes; and what stays under the old path: prompt recall, the
   folder's trust and permission answers (Claude asks again once for
   `main/`), and the sessions started in a subfolder, which
   `claude --resume <session-id>` still finds.
4. Wait for the user's go.

## Execute

From Claude Code, name this session so its own transcript, still being
written, is left for `/cd`:

```sh
bash "${CLAUDE_PLUGIN_ROOT}/scripts/lay-out.sh" go /abs/path/to/project --session "${CLAUDE_SESSION_ID}"
```

Outside Claude Code, leave out `--session`.

- Exit 1: refused, nothing moved. Tell the user what holds the folder,
  wait for their word that it has let go, and run `go` again.
- Exit 2: a step failed after the move began. Stop and show the user its
  output: every file is still on disk and nothing was deleted.
- Exit 0: it prints what it carried and what it left. Ask the user to
  type `/cd /abs/path/to/project/main`, which moves this session there;
  then work from `main/`.

What `go` does, in order: renames `.git` to `project.git` and makes it
bare; writes `project/.git` as `gitdir: ./project.git`; moves every other
entry into `main/`; registers `main/` as a worktree on the branch with
`worktree add --no-checkout` and `worktree repair`, and resets its index
to the branch, the files on disk untouched; maps `origin`'s branches to
`origin/*`; sets `worktree.useRelativePaths`; repairs every worktree.
Then it moves the folder's transcripts, each `<id>.jsonl` with its
`<id>/` folder, from the folder's key under `projects/` to `main/`'s,
and the files of its `memory/` to the git directory's key.

## Verify

- `git -C main status` shows the same changes as before the move, on the same
  branch.
- `git worktree list` lists `project.git (bare)`, `main`, and every earlier
  worktree, none `prunable`.
- `git fetch origin` populates `origin/*`.

If anything looks wrong, stop and show the user: every file is still on disk
and nothing was deleted.
