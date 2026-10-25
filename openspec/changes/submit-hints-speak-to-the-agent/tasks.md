# Tasks

Read first: this change's proposal, specs and design;
`docs/architecture/core.md`; the statement's preamble and §2 Actors
(`agentplot/blueprints:design/sluice/requirements.md`). Work inside
`devenv shell`. Fast tier: `sluice exec main -- moon run ':#check-fast'`.
Commit by pathspec after each group, Conventional Commits, scope `cli`,
`plugin` or `site`.

## 1. sluice's way-on lines

- [x] 1.1 Add `words::agent_in(branch)` ("the agent working in {branch}") and rewrite `timed_out_way_on` and `kept_out_refusal` to take the worktree's branch and read as the design's table; verify with unit tests in `words.rs` for one and several tasks.
- [x] 1.2 In `lib.rs` `report()`, rewrite the blamed, conflict and could-not-run lines and `same_commits` ("or marks #n …") as the design's table, naming `sub.from.worktree`; verify by updating the assertions in `crates/sluice/tests/cli.rs` (the blamed, timed-out and conflict reports) and `crates/sluice/tests/profiles/pull_requests.rs` (the pull request sent back on its head), with `sluice exec main -- moon run cli:test`.
- [x] 1.3 In `lib.rs` `refusal()`, rewrite the uncommitted-changes and detached lines, naming the branch the driver reads for the worktree, or the worktree as given when there is none; the group refusal follows through `refusal()`; leave `refusal_next` as it is; verify by updating `a_dirty_place_is_refused_with_exit_two` and `a_group_is_queued_whole_or_not_at_all` in `cli.rs`, and adding a test that the `--json` refusal's `error.next` is unchanged and its `error.message` names the agent working in the worktree, with `moon run cli:test`.
- [x] 1.4 In `home.rs`, make the outside-a-worktree line `not in a worktree sluice can submit from: the agent working in a worktree submits its work from there`, `next` unchanged; verify by updating the assertion at `cli.rs` that reads it, with `moon run cli:test`.
- [x] 1.5 Add a test in `cli.rs` that a blamed submission from `unit/a`, waited on from another worktree with `sluice wait <id>`, prints a report naming the agent working in `unit/a` and containing neither `your worktree` nor `inside`, and that a flaky fail still names the operator's `sluice set-aside`; verify with `moon run cli:test`.
- [x] 1.6 Bring every quoted copy of the old lines to the new words: `README.md`, `docs/examples/README.md`, `site/src/content/docs/index.mdx`, `guides/walkthrough.md`, `reference/commands.md`, `plugins/sluice/skills/using-sluice/references/verdicts.md` (found with `grep -rn -e 'in your worktree and submit again' -e 'commit or remove them' -e 'in this worktree, or drop it' -e 'or mark #'`); verify that grep finds nothing outside `openspec/` and `crates/sluice/src/profiles/github/` (the forge's comments, out of scope), and `moon run site:prose` is clean.

## 2. The plugin tells Claude who submits

- [x] 2.1 Add a section "Agents you start in worktrees" to `plugins/sluice/skills/using-sluice/SKILL.md` as the design says: the prompt for a background agent in a worktree tells it to commit and submit from there with `sluice submit <branch>` (with `--no-wait` and `sluice wait` when its calls are capped) and report the outcome; work left in a worktree whose agent ended is submitted by an agent Claude starts in that worktree; Claude never asks the user to run `sluice submit`; a way-on line sluice prints is for the agent it names. Verify with `moon run plugin-sluice:validate` and `moon run cli:test` (`plugin_docs.rs`, every command still named).
- [x] 2.2 Add one sentence to the note in `plugins/sluice/scripts/session-context.sh`: an agent this session starts to work in a worktree submits its own work through sluice from there, told so in its prompt, and the user is never asked to run `sluice submit`; extend the session-note check in `plugins/sluice/tests/hooks.sh` to require it; verify with `moon run plugin-sluice:hooks`.
- [ ] 2.3 In `site/src/content/docs/guides/claude-code.md`, add to "What your agents do with it" that Claude tells each background agent it starts in a worktree to submit its own work through sluice, starts an agent to submit work whose agent has ended, and never hands you the submit; and to "Kept off protected branches" that the session note says so; verify `moon run site:prose` is clean.

## 3. The gate

- [ ] 3.1 Commit, then run the gate's checks with `sluice exec main`; verify it passes.
