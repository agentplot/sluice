# Tasks

Read first: this change's proposal, specs and design;
`docs/architecture/core.md` (core.11, core.16, core.18); clause 97 of
`agentplot/blueprints:design/sluice/proposals/session-runs-1.md`. Work
inside `devenv shell`. Fast tier: `moon run ':#check-fast'`; real
tools: `moon run ':#check-tools'`.

## 1. The rule

- [x] 1.1 Make `flat_name` and `dir_name` in `crates/sluice/src/runs.rs` keep a name of at most 246 bytes as today and shorten a longer one to its first 160 bytes cut where a character ends, `%~` or `+~`, and 16 hex digits of the name's SHA-1 through `gix`; verify with the unit tests `a_name_that_fits_is_kept` and `a_long_name_is_shortened_and_distinct` and the existing `a_places_directory_holds_no_percent`.

## 2. A worktree at a long path

- [x] 2.1 Add `exec_runs_from_a_worktree_at_a_long_path` to `crates/sluice/tests/exec.rs`: a scratch repository, a worktree whose path is longer than 255 bytes, `sluice exec` from it passes, `sluice status --json` lists it by its whole path, and no file name under the repository's `sluice/` or the host's state is longer than 255 bytes; verify `cargo test -p sluice --features tools --test exec`.

## 3. Docs and gates

- [x] 3.1 Say on `site/src/content/docs/reference/state.md` how a long checkout name is shortened and that each record carries its whole path; verify Vale reports no warning on it.
- [ ] 3.2 Run `cargo test --workspace`, `moon run ':#check-fast'`, `moon run ':#check-tools'` and `openspec validate long-place-names --strict`; verify all pass.
