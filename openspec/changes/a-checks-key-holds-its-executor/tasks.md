# Tasks

Each group is one session's work for an implementer who reads this
change (proposal, design, specs) and its own group. The groups land in
order, as one landing. Work in a worktree of this change's name on its
branch, cut from main. Run suites through sluice (`sluice exec main --
cargo test ...`); a quick `cargo check` or one unit test may run
directly. No test reaches a credential or the host's own sluice state:
set `XDG_STATE_HOME` to the test's scratch directory. Tests of the
sandbox run on macOS only, as `crates/sluice/tests/checks.rs` already
gates them. Commit by pathspec (`git add <paths>; git commit -m
"<subject>" -- <paths>`), Conventional Commits, scoped to the part of
the tree, citing clauses by number and rules by id. Every group ends
with its check task passing.

## 1. A check's key holds its executor's grant

- [ ] 1.1 In `crates/sluice-seams/src/lib.rs`, add `Enforced::grant` as the design's "The grant of each executor" says, read as `None` from a line without it. In `crates/sluice/src/profiles/checks.rs`, render the sandbox profile by role beside `seatbelt` from the same inputs and digest it as `sb1:<hex>`; give the open executor `open:1`; set the grant on the run's `Enforced` for a candidate's run, a session run and a preview's alike. True when unit tests in `checks.rs` show two places and two cache tables giving one grant, a profile with `/dev/ptmx` and one without giving two, and the roles substituted back giving the written profile's rules.
- [ ] 1.2 In `crates/sluice-core/src/verdict.rs`, compare `grant` in `same_limits`, and require `same_limits` on the signed path of `counts` as on the unsigned one, as the design's "The gate compares the grant wherever it compares limits" says. Teach the conformance suite's fake executor in `crates/sluice-fakes` to report a grant, so the suite holds the core to it. True when unit tests in `verdict.rs` pass for each scenario of the check-grant requirements "Results under two grants are never weighed together" and "A result with no grant is weighed only with others with none", and `sluice exec main -- cargo test -p sluice-conformance` passes.
- [ ] 1.3 In `crates/sluice/src/signing/statement.rs`, carry the grant in the predicate's executor, and in the records seam's `remote refs` profile read a signed line's grant from the statement alone (core.22). True when a test in `crates/sluice/tests/remote_refs.rs` for the scenario "A grant beside the statement" passes.
- [ ] 1.4 Tests in `crates/sluice/tests/checks.rs` and `crates/sluice/tests/exec.rs` for the scenarios of "Every run records its executor's grant", and one for S66's first half: a key holding a fail with no grant and a fail under another grant, then a passing run under this grant, on a branch with `flaky = "fail"`, reads passed. True when `sluice exec main -- cargo test -p sluice --test checks --test exec --test remote_refs` passes.
- [ ] 1.5 Docs (rule core.26): `site/src/content/docs/reference/configuration.md` says under `flaky` that results under two sandbox profiles are never weighed together, and that results recorded before the grant are weighed only with each other, so each test runs once more; under `[executor]`, that every result records what the executor let it reach. `plugins/sluice/skills/using-sluice/references/verdicts.md` says the same in a line. True when `sluice exec main -- moon run site:prose` passes with no Vale warning on the pages this group touched.
- [ ] 1.6 Check the group: commit, then run `sluice exec main -- moon run cli:test cli:tools root:changes site:prose`. True when every task passes.

## 2. A check that failed on what the sandbox refused has no outcome

- [ ] 2.1 Probe on macOS which mark tells one run's Seatbelt denials apart from another's in the unified log, in the design's order: a message on each deny rule, the sandboxed processes' ids, the span alone; whether `log show` reads the kernel's Sandbox reports as the host's user without `sudo`; and which denials a passing `sluice exec main` on sluice itself meets (moon's writes to `.moon/cache/outputs/` and to the shared cache's `blobs/temp/`, seen 2026-10-29), each granted in `sandbox_profile` in this task or named in the design as why not. Write what was found in the design's "The profile marks its denials as the run's"; if the span alone is left, amend record 0072 and say what a second run in the span costs. True when the design names the mark used, with the macOS version probed, a passing run of sluice's own gate meets no denial, and `openspec validate a-checks-key-holds-its-executor --strict` passes.
- [ ] 2.2 Add `crates/sluice/src/profiles/denials.rs`, reading the run's denials from `log show --style ndjson` over the run's span and mark into `Denial { process, operation, path }`, as the design's "Reading the denials after a run" says, the log run under `crate::runs::run_to_end` with a bound, its answer kept as `denials.json` or `denials-error` in the run's directory. True when unit tests turn a recorded ndjson sample, kept under `crates/sluice/tests/data/`, into the denials it holds, ignore another mark's, and read the kept file without running the log again.
- [ ] 2.3 In `checks.rs`, after `ready_after` and only when it moved nothing, read the denials for a sandbox run in which a check did not pass and that its time limit did not end; on a denial, move every failed and missing task to `not_run` with the design's reason; on an error, keep it on the run's report for the text views. Keep the mark in the profile the run writes. True when the tests of 2.4 pass.
- [ ] 2.4 Tests in `crates/sluice/tests/checks.rs` and `crates/sluice/tests/exec.rs`, on macOS, a sandbox run whose test writes outside its place and fails, for each scenario of the run-outcome requirements this change adds, and S66 whole: the refused test not run with no line under its key, then a run under a grant that allows it passing. The unreadable log is a test with `log` replaced on the run's `PATH` by a script that fails. True when `sluice exec main -- cargo test -p sluice --test checks --test exec` passes.
- [ ] 2.5 In `crates/sluice/src/lib.rs` and the text views, name a check the sandbox refused, and the run whose refusals could not be read, in the design's words, without the set-aside hint. True when a test in `crates/sluice/tests/exec.rs` checks the words `sluice exec` prints for each.
- [ ] 2.6 Docs (rule core.26): `site/src/content/docs/reference/configuration.md` says under `[executor]` that a test that fails because the sandbox refused it something has no result and runs again, naming what was refused, and that a test failing on its own is recorded as ever; `plugins/sluice/skills/using-sluice/references/verdicts.md` and `exec.md` show the words and say the cure is sluice's, never a set-aside or a change to the test's inputs. True when `sluice exec main -- moon run site:prose` passes with no Vale warning on the pages this group touched, and the plugin's own project tasks pass.
- [ ] 2.7 Check the group: commit, then run `sluice exec main -- moon run cli:test cli:tools root:changes site:prose`. True when every task passes.

## 3. The gate

- [ ] 3.1 Confirm no check's output is read for a refusal: `rg -n 'openpty|Operation not permitted|denied' crates/sluice/src` finds nothing outside `denials.rs` and its tests. True when it holds and the commit message says so.
- [ ] 3.2 Remove this change's item from `docs/architecture/_open.md`, and the file once it is empty. True when nothing there names this change.
- [ ] 3.3 Run the gate's checks on the committed work with `sluice exec main`, and `sluice exec main -- moon run root:changes`. True when every task passes and the change validates strictly.
