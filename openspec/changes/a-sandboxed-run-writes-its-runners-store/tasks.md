# Tasks

Work in this change's place on its branch, cut from
`bolt/sluices-sandbox-keeps-its-proof`, which is `<base>` below. Run
suites through sluice (`sluice exec <base> -- cargo test ...`); a quick
`cargo check`, `cargo clippy` or one unit test may run directly. A test
that starts a sandbox runs outside any sandbox, since macOS refuses a
sandbox inside another and this repository's own gate runs under the
sandbox: run it with `devenv shell --` directly, as the `cli:sandbox`
task's comment says. No test reaches a credential or the host's own
sluice state. Commit by pathspec, Conventional Commits, scoped to the
part of the tree, citing clauses by number and rules by id.

## 1. The decision recorded

- [x] 1.1 Amend record 0105 point 1's sentence on a cold run: where the repository's own moon workspace turns sharing on, the store resolved from the cold run's environment is granted as any run's is, as the design's "A cold run is opened the store its own environment resolves" says, the amendment named in the record's status as the construct stage's own choice. True when `sluice exec <base> -- moon run root:adrs` passes.

## 2. A sandboxed moon run writes its store

- [x] 2.1 In `crates/sluice/src/profiles/checks.rs`, make `given_and_resolved` give, beside the path given, the canonical path of its nearest ancestor that exists joined with the rest of the path, where that differs, as the design's "A path is opened where it resolves through its nearest directory that exists" says; give `sandbox_profile` a `store: Option<&Path>` it opens by `given_and_resolved` only where its resolved path lies outside the resolved checkout and scratch, as "A store in the checkout or its scratch adds nothing" says; pass it from `Checks::start` as the `store` the run's `Meta` records, and `None` from `release_request`; rewrite the module's doc comment on the `"sandbox"` executor to name the store moon keeps beside the host's cache directories. Add the unit test `the_profile_opens_the_runners_store_outside_the_place` to the module's `tests`, as the host-caches spec's scenarios "Only the store is opened", "A store moon has not made yet" and "A store in the checkout" say, citing the C.2 sandbox row as amended at agentplot/blueprints 0c38f24, record 0105 and rules core.9 and core.35. True when `cargo test -p sluice --lib profiles::checks::tests::the_profile_opens_the_runners_store_outside_the_place` passes and `cargo clippy -p sluice --all-targets -- -D warnings` is clean.
- [x] 2.2 In `crates/sluice/tests/profiles/moon.rs`, add `#[cfg(target_os = "macos")]` `a_key_run_in_the_sandbox_is_proven_under_the_home_the_toolchain_gives_moon`, as the design's "The sandbox scenario runs the family's layout" and the spec's scenario "moon's home moved by the toolchain, on a host that shares moon's cache" say. True when `devenv shell -- cargo test -p sluice --features tools --test moon a_key_run_in_the_sandbox_is_proven_under_the_home_the_toolchain_gives_moon` passes, run outside any sandbox.

## 3. Say it to the operator

- [x] 3.1 In `crates/sluice/src/instance.rs`, make the doc comment of `[[caches]]` say a sandboxed run may write each `dir` and reach each `socket`, and may write where moon keeps what it shares for the run whatever `dir` says. True when `cargo clippy -p sluice --all-targets -- -D warnings` is clean.
- [x] 3.2 In `site/src/content/docs/reference/configuration.md`, make the `"sandbox"` row of the executor's table name, beside the host's cache directories, where the task runner keeps what it shares for the run, and make the paragraph on moon's shared worktree setting under `[[caches]]` say a sandboxed run may write where moon keeps it, wherever the repository moves moon's home and whatever `dir` says, in place of asking the operator to declare it as `dir`. Name no host, tool or team of ours (rule core.26). True when `sluice exec <base> -- moon run site:prose` passes with no warning on the page.
- [x] 3.3 In `plugins/sluice/skills/setting-up-sluice/references/profiles/checkout.md`, say after the `[[caches]]` example that the sandbox also opens where moon keeps what it shares for a run, whatever `dir` says. Name no host, tool or team of ours (rule core.26). True when `sluice exec <base> -- moon run plugin-sluice:validate` passes.
