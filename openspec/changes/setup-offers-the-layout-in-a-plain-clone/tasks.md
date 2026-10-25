# Tasks

## 1. The conversion as a script, under the gate

- [x] 1.1 Write `plugins/sluice/scripts/lay-out.sh` with `plan <folder>` and `go <folder> [--session <id>]` as the design's "One script does the plan, the move and the carry" says: the layout steps of `references/convert-from-plain-clone.md`, its preconditions as refusals, exit 0, 1 (refused, nothing moved) or 2 (failed after the move began); verify `shellcheck` and `shfmt` pass on it through `moon run root:format` and the test in 1.4
- [x] 1.2 In the script, find what holds the folder as the design's "What holds the folder means" says: `lsof` working directories and open files at or under the folder, symbolic links resolved, the script's own process and its ancestors left out, and a refusal when `lsof` is missing or fails; verify by the refusal cases in 1.4
- [x] 1.3 In the script, carry the history as the design's "Carrying the history" says: the folder's transcripts and their folders to `main/`'s key except the `--session` one, `memory/` files to the git directory's key, under `CLAUDE_CONFIG_DIR` or `~/.claude`, never replacing or deleting, a key over 200 characters carried not at all, and a report of what moved and what stayed; verify by the carry cases in 1.4
- [x] 1.4 Write `plugins/sluice/tests/lay-out.sh` in the manner of `tests/hooks.sh` (scratch under `SLUICE_TMP`, git isolated from the user's config and credentials, a stand-in `lsof` on `PATH`, a scratch `CLAUDE_CONFIG_DIR`), covering every scenario of the `plugin-worktrees` delta: the plan changes nothing and names a holder; a holder refuses `go` and leaves the folder as it was; a failing `lsof` refuses; with no holder, `main/` keeps the branch, a modified and an untracked file, a worktree outside the folder still works and nothing is prunable; three transcripts with one named by `--session`, and `memory/`, carried as the scenario says; a memory file already at the new key left and reported; a long path carried not at all. Verify it passes with `sluice exec main -- moon run plugin-sluice:lay-out`
- [ ] 1.5 Add the `lay-out` task to `plugins/sluice/moon.yml`, `bash tests/lay-out.sh`, inputs `scripts/**/*`, `tests/**/*`, `/devenv.nix`, `/devenv.lock`, tagged `check-fast`; verify `moon query tasks --affected` lists it and `sluice exec main` runs it

## 2. The worktrees skill runs the script

- [ ] 2.1 Rewrite `plugins/sluice/skills/worktrees/references/convert-from-plain-clone.md` around `${CLAUDE_PLUGIN_ROOT}/scripts/lay-out.sh`: show `plan`, wait for the user's go, run `go` (with `--session ${CLAUDE_SESSION_ID}` when run from Claude Code), then `/cd` into `main/`; keep its Verify section; verify `moon run plugin-sluice:validate` passes
- [ ] 2.2 In `plugins/sluice/skills/worktrees/SKILL.md`, say under "Rules that hold everywhere" that a conversion's history is carried by the script and the running session by `/cd`, and under "With sluice" replace "A repository without worktrees can still use sluice" with the setup's offer and its record of a kept plain clone; verify `moon run plugin-sluice:validate` passes

## 3. The setup offers the layout first

- [ ] 3.1 Add step 2, "A working copy per branch", to `plugins/sluice/skills/setting-up-sluice/SKILL.md` before the survey, as the design's "When the setup offers the layout" and "Where the offer sits in the skill" say: the check, `plan` shown with its costs first, one question, `go` on the word with a refusal named and waited on, then `/cd <folder>/main` and the survey there; renumber the steps after it and drop the survey row that sends "none yet" to the `worktrees` skill; verify `moon run plugin-sluice:validate` passes and the skill reads through against each `plugin-setup` delta scenario
- [ ] 3.2 In the policy step of the same skill, say that when the operator kept the plain clone the commit adding `.config/sluice.toml` carries the line the design's "The setup's record of a kept plain clone" gives, and that without an answer to the offer no policy is written; verify against the "The operator keeps the plain clone" and "No answer yet" scenarios

## 4. The newcomer's way in

- [ ] 4.1 Rewrite Get started in `README.md`: the Claude Code path says Claude offers the layout first in a plain clone and goes on there only if told to; the by-hand path starts from the layout via the worktrees guide's new section, then the install and the walkthrough; no sentence sends a newcomer into a plain clone. Verify by reading, since no gate reads the README (core.21): no path in Get started sets sluice up in a plain clone unasked, and each site link names a page or section 4.2 and 4.3 make, as 5.2's build confirms
- [ ] 4.2 Add a first section to `site/src/content/docs/guides/worktrees.md`, one working copy per branch: the layout, the fresh-clone commands of `references/layout.md`, and what the setup offers for an existing clone, what it costs and what it carries; verify `moon run site:prose` passes with no Vale warning
- [ ] 4.3 In `site/src/content/docs/guides/claude-code.md`, "Set up a repository", say that in a plain clone Claude first offers the layout, names what holds the folder, and carries the sessions' history, and that the plain clone is kept only on the user's word; verify `moon run site:prose` passes with no Vale warning

## 5. Integration

- [ ] 5.1 Run the gate on the committed work, `sluice exec main`, and verify every task passes, `plugin-sluice:lay-out` among them
- [ ] 5.2 Check the site's links resolve: `sluice exec main -- moon run site:build` passes with no broken link reported on `guides/worktrees` or `guides/claude-code`, and the README's anchors exist in `dist/`. Run by hand because the site's build is the Pages workflow's, not a gate task (core.21)
