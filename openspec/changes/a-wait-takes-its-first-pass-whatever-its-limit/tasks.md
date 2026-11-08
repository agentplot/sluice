# Tasks

One group, from this change's place on
`fix/main--handed-over-wait-holds-steady`, cut from main. Run suites
through sluice (`sluice exec main -- cargo test ...`); a quick `cargo
check` or one test over the fakes may run directly. Tests use scratch
repositories, a bare clone standing for GitHub's git and the forge
double only. Commit by pathspec, with Conventional Commits scoped to the
part of the tree, citing clauses by number.

## 1. A wait takes its first pass whatever its limit

- [x] 1.1 In `crates/sluice/src/lib.rs`, `wait_on` checks its deadline and pauses only from its second pass on, as `submit_pull`'s loop does, its doc comment saying so (1). Add `a_wait_out_of_time_at_its_start_takes_its_first_pass` to `crates/sluice/tests/cli.rs` (submission-wait, "Out of time as it starts"). True when `cargo test -p sluice --test cli a_wait_out_of_time_at_its_start_takes_its_first_pass` passes, and failed before the change with the record `queued` and nothing started.
- [x] 1.2 `handed_over_by_a_wait` waits with `--timeout 0s`, its doc comment saying its time is up as it starts (request-intake-host, "A wait on a submission handed over follows its pull request"). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests handed_over_by_a_wait` passes, and failed before the change as the gate did, `state` `testing` and no `carried_by`.
- [x] 1.3 The `sluice wait` row of `site/src/content/docs/reference/commands.md` says the wait runs one round, or reads once what the driver read, before its limit ends it. True when `sluice exec main -- moon run site:prose` passes.
- [ ] 1.4 The whole gate on the committed branch. True when `sluice exec main` passes.
