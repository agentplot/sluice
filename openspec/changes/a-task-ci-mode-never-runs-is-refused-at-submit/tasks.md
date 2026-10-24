# Tasks

Read first: this change's proposal, design and spec; ADR 0044;
`docs/architecture/core.md`. Run suites through
`sluice exec main -- <cmd>`, the gate's checks with `sluice exec main`.

## 1. The policy's lines and the seam

- [x] 1.1 `crates/sluice-core/src/policy.rs`: `RulesForm::required` as
  `Vec<toml::Spanned<String>>`; `Rules` keeps each required entry's line
  in the policy text beside `required`, which keeps its form for every
  other reader. Add unit tests for an entry on one line and for one of a
  `required` array written across lines (switchboard-kit's form). Verify
  `cargo test -p sluice-core --lib policy`.
- [x] 1.2 `crates/sluice-seams/src/lib.rs`: `Tests::kept_out(worktree,
  tests, required)` returning each kept-out task with the required entry
  that names it, defaulting to none, with its doc comment citing ADR
  0047. Verify `cargo check --workspace --all-targets`.

## 2. The refusal in the core

- [x] 2.1 `crates/sluice-core/src/ops.rs`: `SubmitRefusal::KeptOut`, each
  with task, required entry and line; `Sluice::submit` asks the seam in
  the submitting worktree after its existing refusals and before
  `publish_head`, with the gate read at the branch's tip. An error from
  the seam takes the submission and returns a warning on `Accepted`.
  Verify `cargo check --workspace --all-targets`.
- [x] 2.2 `crates/sluice-fakes`: `keeps_out` also answers `kept_out` for
  the required entries it names, and a switch makes `kept_out` fail.
  Add to `crates/sluice-conformance/tests/verdicts.rs`:
  `a_submission_whose_gate_requires_a_check_kept_out_of_ci_is_refused`
  (refused naming the task and its line, nothing queued, nothing
  published), `a_submission_that_lets_the_check_run_is_taken`, and
  `a_runner_that_cannot_be_asked_takes_the_submission_with_a_warning`.
  Change `a_required_check_kept_out_of_ci_is_missing_and_named` to keep
  the task out only after the submission is taken, as an entry ahead
  changing `moon.yml` would, so the run's missing and the status warning
  stay covered. Verify
  `sluice exec main -- cargo test -p sluice-conformance`.

## 3. The moon profile

- [x] 3.1 `crates/sluice/src/profiles/moon.rs` and `checks.rs`: one
  function from moon's tasks and the required entries to the kept-out
  list, used by `Moon::plan` and by `Checks::kept_out`, which sets up the
  run's environment and toolchain in the worktree (into a scratch
  directory of sluice's state), installs and checks moon, and runs
  `moon query tasks`; a command runner answers none. Verify
  `cargo test -p sluice --lib` and the existing
  `a_task_kept_out_of_ci_is_not_run` still pass.
- [x] 3.2 Add to `crates/sluice/tests/profiles/moon.rs`, on a real moon:
  `a_required_task_kept_out_of_ci_is_named_before_any_run` (a task
  required by name and one through a tag, each named with its entry; a
  task in CI is not named) and
  `asking_moon_leaves_the_worktree_clean` (`git status --porcelain`
  prints nothing after `kept_out`). Verify
  `sluice exec main -- cargo test -p sluice --features tools --test moon kept_out`.

## 4. What the submit says

- [x] 4.1 `crates/sluice/src/lib.rs` (and `words.rs` if the words live
  there): the refusal's text, naming each task, the tag it is required
  through, the policy file and line, and the way on; `refusal_next`
  gives `sluice status <branch>`; `--json` gives the usual refusal
  document; the warning when the runner could not be asked is printed,
  and carried as `warnings` under `--json --no-wait`. Add tests to
  `crates/sluice/tests/cli.rs` for the refusal's text and exit 2, its
  JSON, and the warning. Verify
  `sluice exec main -- cargo test -p sluice --test cli`.
- [x] 4.2 `crates/sluice/src/profiles/github/pulls.rs`: `open` refuses
  with `KeptOut` before `push_branch`; `refusal_word` gains it. Add a
  test to `crates/sluice/tests/profiles/pull_requests.rs` that nothing
  is pushed and no request is opened or marked ready. Verify
  `sluice exec main -- cargo test -p sluice --test pull_requests`.

## 5. Records of the decision and the docs

- [x] 5.1 `docs/adr/0047-a-submission-whose-gate-requires-a-check-its-runner-never-runs-is-refused.md`
  (MADR, as 0044): the options of the design's first decision, and why
  the worktree; renumber if another ADR takes 0047 first. Add to
  core.15's details in `docs/architecture/core.md`, under "Not ruled
  out", the runner's CI setting read from the submitting worktree at
  submit, which no gate counts (ADR 0047). Verify `sluice exec main --
  moon run root:format`.
- [x] 5.2 `site/src/content/docs/reference/configuration.md`: the
  `runInCI: false` bullet says a submission to a branch requiring such a
  task is refused, naming it and the policy line, and how the repair
  lands. `reference/commands.md`: the refusal among `sluice submit`'s.
  `plugins/sluice/skills/using-sluice/SKILL.md` rules and
  `references/submit.md`: what the refusal means and the way on. Verify
  `sluice exec main -- moon run site:prose` with no warnings, and
  `sluice exec main -- cargo test -p sluice --test plugin_docs`.

## 6. The gate

- [ ] 6.1 Commit, then run the gate's checks with `sluice exec main`;
  every check passes.
