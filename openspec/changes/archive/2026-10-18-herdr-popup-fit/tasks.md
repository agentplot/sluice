# Tasks

Read first: this change's proposal and specs; core.md. Commit by
pathspec; land with `sluice submit main`.

## 1. The popup

- [x] 1.1 Fit the frame to the terminal's height in `lib/words.jq` (`fit`) and `bin/status.sh` (`height`, `draw`); verify the `plugin-herdr` tasks `test-sh` and `test-dash` pass with the test `status: a tall frame fits the terminal`, which fails before the change.
- [x] 1.2 From a pane in no git repository, name the folder and show every repository held; verify the test `status: a pane outside git shows every repository`, which fails before the change, and `status: a plain directory` pass in the same tasks.

## 2. Integration

- [x] 2.1 Verify `devenv shell -- moon run 'plugin-herdr:#check-fast' root:format --force` passes, and `tests/run.sh` passes under the sandbox executor's profile (`crates/sluice/src/profiles/checks.rs`) run by `sandbox-exec`.
