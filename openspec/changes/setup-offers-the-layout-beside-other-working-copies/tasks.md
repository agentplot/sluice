# Tasks

Read first: this change's proposal, design and specs; clause 126 and S61
of `agentplot/blueprints:design/sluice/requirements.md`; ADR 0057;
`plugins/sluice/scripts/lay-out.sh` and its tests. Work inside `devenv
shell`. The script's tests run in `plugin-sluice:lay-out`, with git
isolated from the user's config and a stand-in `lsof`; keep every new
test in that isolation. Run suites with `sluice exec main -- moon run
<task>`.

## 1. The script tells how a repository stands

- [x] 1.1 Add the `kind <path>` form to `plugins/sluice/scripts/lay-out.sh` (design D1): its usage line, its paragraph in the header comment, `only <folder>` / `several <folder>` / `apart` on stdout, exit 1 with `refused: <path> is not a working copy of a git repository`, nothing changed and no `lsof` run; bash 3.2; verify by 1.2
- [x] 1.2 Add tests to `plugins/sluice/tests/lay-out.sh`, one per scenario of the `plugin-worktrees` delta: a plain clone (`only`, its folder); `clone`'s clone with its `-side` worktree, asked from the clone and from `-side` (`several`, the clone's folder both times); a clone with a detached worktree under `.git/sluice/` and a worktree whose folder was then removed (`only`); a laid-out repository's `main/` (`apart`); a plain folder (exit 1); and one clone reached through a symbolic link to its scratch directory, which names the physical folder; each with the folder's snapshot unchanged; verify `sluice exec main -- moon run plugin-sluice:lay-out` passes

## 2. The setup makes the offer and goes on

- [x] 2.1 Rewrite step 2 of `plugins/sluice/skills/setting-up-sluice/SKILL.md` around `lay-out.sh kind .` (design D2): `apart` goes to step 3; `only` keeps today's offer and wait, word for word; `several` runs `plan` on the named folder and shows it as the plain clone's offer is shown, with any "go refuses now" lines as what stops the move today, says the setup goes on as the repository stands and lays the folder out whenever asked, asks no question, and goes on to step 3 in the working copy it was asked in (design D3); replace the four git commands and the sentence leaving a clone with worktrees to the `worktrees` skill; verify `sluice exec main -- moon run plugin-sluice:validate` passes, and that the step names no AskUserQuestion or other wait in the `several` path, by reading it, since the gate drives no Claude session
- [x] 2.2 In the same step, say what "lay it out" does when the operator says it later in the setup (design D4): `go` as on the `only` path, `/cd <folder>/main` only when the setup was asked in that folder, and carrying on with the step it was on; verify `plugin-sluice:validate` passes
- [x] 2.3 In step 5, bind "write nothing here until the user has answered" and the record line to the `only` path where the user kept the plain clone (design D5); verify `plugin-sluice:validate` passes
- [x] 2.4 In `plugins/sluice/skills/worktrees/SKILL.md`, "With sluice", say that beside other working copies the setup offers the same move with its cost and goes on as the repository stands; verify `plugin-sluice:validate` passes

## 3. The guides

- [x] 3.1 In `site/src/content/docs/guides/claude-code.md` (the paragraph "In a plain clone, ...") and `site/src/content/docs/guides/worktrees.md` (the paragraph "For a clone you already have, ..."), add that in a clone with other worktrees Claude makes the same offer, names the same costs, and goes on with the repository as it is without waiting for an answer, laying it out whenever you ask; verify `sluice exec main -- moon run site:prose` clears every Vale warning

## 4. Try it and the gate

- [ ] 4.1 Try it once by hand: in a scratch clone with a worktree beside it, start Claude Code with this worktree's `plugins/sluice` loaded and ask it to set sluice up, once from the clone and once from the worktree; it shows the clone folder's plan and goes on to the survey in the same turn, asking nothing about the layout; then in a scratch plain clone it waits for the choice as before. By hand, since the gate drives no Claude session; the script's answers it rests on are tested in 1.2
- [ ] 4.2 Commit, then run the gate's checks: `sluice exec main` passes

## Workflow follow-up

- Archive the change once it has landed, and land the archive through sluice.
