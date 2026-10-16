# Tasks

Read first: this change's proposal, specs and design; ADR 0023;
`docs/architecture/core.md`. Work inside `devenv shell`. Fast tier:
`moon run ':#check-fast'`; real tools: `moon run ':#check-tools'`.

## 1. The seams and the drivers

- [x] 1.1 Add `Submitter.path` (optional, serde default, skipped when absent); set it in the worktrunk driver from `git rev-parse --show-toplevel`, leave it out in the pull-request driver and the fakes; verify `cargo check --workspace --all-targets --all-features`.

## 2. The core

- [x] 2.1 Add `overview::branch_view`, `recent` and `Sluice::overview`, reading each branch at `base`; verify the unit tests in `sluice-core/src/overview.rs` (the last ten landed, blamed and conflict lines, verbatim; an empty log).

## 3. The CLI

- [x] 3.1 Add `sluice overview [--json] [--watch]`, `Host::name` and `Host::repository`, and share `preview_json` and `worktree_json`; give it a one-second pause in main; verify in cli.rs over the fakes: the document's fields for a landed, a blamed and a testing submission, the text form, a repository with no records, and a watch that prints once at start and once per change, never for the time alone.
- [x] 3.2 Add the real-tool test in `tests/profiles/worktrunk.rs`: `wt submit --no-wait` records `from.path` as the worktree's canonical path, and `sluice overview --json` lists the branch, the submission with its path and the repository's git directory; verify `cargo test -p sluice --features tools --test worktrunk`.

## 4. Decisions, rules and docs

- [x] 4.1 Write ADR 0023; amend core.1's details in `docs/architecture/core.md` and `site/src/content/docs/architecture/rules.md` to name `plugins/`; verify `bun run build` renders the ADR.
- [x] 4.2 Document `sluice overview` on `reference/commands.md`, `from.path` on `reference/state.md`, and the README's command table; verify Vale reports no warning.
- [x] 4.3 Run `:#check-fast`, `:#check-tools`, `openspec validate overview --strict` and `bun run build`; verify all pass.
