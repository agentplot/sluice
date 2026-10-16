# Tasks

Read first: this change's proposal, design and specs; ADR 0023. Work inside
`devenv shell`.

## 1. Skills and the marketplace (landed as #71)

- [x] 1.1 Add `.claude-plugin/marketplace.json` and `plugins/sluice` with `setting-up-sluice`, `using-sluice` and `worktrees`; verify `claude plugin validate plugins/sluice` and `claude plugin validate .` pass.
- [x] 1.2 Add `the_plugins_example_policies_read` in `crates/sluice-core/src/policy.rs`, with the examples in its moon inputs; verify `cargo test -p sluice-core`.
- [x] 1.3 Try the worktrees procedures on scratch repositories: a fresh bare clone moved to another folder, and a plain clone with a modified file, an untracked file and an outside worktree converted; verify each `git worktree list` shows nothing prunable and the changes survive.
- [x] 1.4 Open the README and the site's landing page with the plugin's install, ways in for Claude Code and herdr, and the guides `claude-code.md` and `herdr.md`; verify Vale reports no warning and `bun run build` finds every link valid.

## 2. The guard and the session note

- [x] 2.1 Add `plugins/sluice/hooks/hooks.json` with a PreToolUse hook on Bash and a SessionStart hook, and their scripts; verify `claude plugin validate plugins/sluice`.
- [x] 2.2 Test the guard on a scratch repository with isolated git and no credentials: a push, merge, `update-ref` and `gh pr merge` aimed at `main` are refused naming `sluice submit main`; a push of `unit/x` and any command in a repository with no policy go through; verify with `plugins/sluice/tests/hooks.sh`, run by the moon task `root:plugin-hooks` (check-fast).
- [x] 2.3 Document the guard and the note in `guides/claude-code.md`; verify Vale.
- [x] 2.4 Teach when to use each form of `sluice exec` in `using-sluice` (SKILL.md and references/exec.md: the `--` form while iterating, the forced affected run before submitting, the branch's `trust` deciding whether it is reused) and name both in the session note; verify `plugins/sluice/tests/hooks.sh` and `crates/sluice/tests/plugin_docs.rs` pass.

## 3. The board

- [x] 3.1 Add `plugins/sluice-board` (manifest, hooks module, state contract) and list it in the marketplace; verify `claude plugin validate plugins/sluice-board`, `claude plugin validate .` and `tsc` over the module and its tests with the types' tsconfig.
- [x] 3.2 Feed it from `sluice overview --json --watch`, falling back to polling; draw the band, the `/sluice` pane and the status line; verify `claude plugin test plugins/sluice-board`: the band and the pane drawn from a fixture document on `terminal` and `desktop`, and the status line.
- [x] 3.3 Add the wake on `from.path`, once per outcome across reloads, behind a `wake` option; verify with tests that one blame starts one turn naming the test and its log, and none with `wake` off.
- [x] 3.4 Try it live: submissions to `main` tracked from testing to landed in a session, and the wake telling the session its own landing (#81, 2026-10-16); the band and pane reworded to say submissions and combinations, with pull requests linked to GitHub; documented in `guides/claude-code.md`.

## 4. Drift and close

- [x] 4.1 Add drift tests: every key the policy parser accepts appears in the setup references, and every subcommand in the skills (`crates/sluice/tests/plugin_docs.rs`, with the skills in `cli:test`'s inputs); verify they fail when a key is removed from a reference.
- [x] 4.2 Run `openspec validate claude-code-plugins --strict`, then archive the change once 2 and 3 have landed.
