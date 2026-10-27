# Tasks

Read first: this change's proposal, specs and design; clauses 23 and 26
of `agentplot/blueprints:design/sluice/requirements.md`; record 0064;
`docs/architecture/core.md`. Work inside `devenv shell`. Run suites
through sluice: `sluice exec main -- cargo test -p sluice --test cli
<name>` while iterating, `sluice exec main` once a group is committed.
Commit by pathspec after each group, Conventional Commits, scope the
crate, `herdr`, or `site`.

## 1. The name a submission is shown by

- [x] 1.1 If `Submission` in `crates/sluice-core/src/records.rs` has no `subject`, add it exactly as `git show origin/bolt/a-pull-request-is-titled-by-what-lands:crates/sluice-core/src/records.rs` defines it (doc comment, serde attributes, after `held`), and `subject: None` at each struct literal; verify `cargo check --workspace --all-targets` passes and `git diff` of the field against that branch's shows no difference.
- [x] 1.2 Add `Submission::name()` (the subject, else `from.worktree`), `subject: Option<String>` with `name()` on `FinishedLine` (serde default, skipped when none), and write the subject into the index line in `Line::changes`; verify with unit tests in `records.rs` and `line.rs`: a submission with and without a subject, an index line without the field read back with none, a submission moved to the finished records keeping its subject in the index.
- [x] 1.3 Add `shown()` to `crates/sluice-core/src/text.rs`, each run of control characters one space, trimmed; verify with unit tests: a line break, a tab, an escape sequence, and a subject left as it was.

## 2. `sluice status` and `sluice overview` as text

- [x] 2.1 Add the per-line naming (`Names` and `named`, design: "A line names each submission once") in `crates/sluice/src/lib.rs`; make `combination()` and `hashes()` take it, and `progress::testing_line`/`progress::lines` take `Option<&Names>`, `sluice wait` and the waiting submit passing `None`; verify unit tests for a first and a repeated number, and that the existing wait tests in `crates/sluice/tests/cli.rs` pass unchanged.
- [x] 2.2 Add `named` to `Status` in `crates/sluice-core/src/ops.rs` from the finished index; make `write_status` begin each row with the padded name after its number, starting each row's set with its own number, and name the detail's and the testing lines' submissions; verify with CLI tests over fakes for the scenarios of `submission-names`' status requirement: a subject, none, a row's own number in its detail, `after` a finished submission under `--all`, a combination under test, rows lining up, and a subject with a line break printed on one line with `--json` keeping it as given.
- [x] 2.3 Name the submissions of `sluice overview`'s testing lines and recent lines (`recent_line` in `crates/sluice/src/overview.rs`), from the branch's in-chain submissions, `recent_subjects` and `recent_from`; verify with CLI tests: a landing of two, a blame, a conflict, a combination under test.
- [x] 2.4 Update `site/src/content/docs/reference/commands.md`'s `sluice status` and `sluice overview` examples and `site/src/content/docs/guides/walkthrough.md`'s status row to the new lines, leaving the wait's and submit's examples as they are; verify `moon run 'site:prose'` is clean.

## 3. The overview's JSON and the home view

- [x] 3.1 Add `recent_subjects` to `BranchView` in `crates/sluice-core/src/overview.rs`, built beside `recent_from` from the live records and the finished index, only for submissions with a subject; verify with the overview's unit tests and a CLI test for the four scenarios of the `overview` delta, and that `recent` and `recent_from` are unchanged.
- [x] 3.2 Name the home view's submission `#<id> <name> to <branch> · <state>` and add `subject` to its JSON in `crates/sluice/src/home.rs`; verify with CLI tests for both scenarios of the home view requirement, and that the JSON's other fields and `next` are unchanged.
- [ ] 3.3 Update the home view's example in `site/src/content/docs/reference/commands.md` and the overview's `--json` fields where the page lists them; verify `moon run 'site:prose'` is clean.

## 4. The herdr popup and segment

- [ ] 4.1 Add `name_of($b)` and `named($b)` to `plugins/herdr/lib/words.jq` (design: "Where each view finds its names"), and use them in `combination`, `waiting`, `recent_words`, the QUEUED rows (names padded so states line up), the TESTING and formed rows, and `branch_line`; say in the file's header that the popup and segment name subjects where the board does not yet; verify `moon run 'plugin-herdr:test-sh'` and `moon run 'plugin-herdr:test-dash'` pass.
- [ ] 4.2 Give the fixtures in `plugins/herdr/tests/` subjects and update `tests/docs/board.frame` and `tests/run.sh`'s expected lines; add cases for each scenario of the `herdr-status` delta: testing with and without tasks, a landing of two, what is waiting and the last event in the segment, a document with no `recent_subjects` or `recent_from`, and a subject with a line break and an escape sequence drawn on one line with the colors unchanged; verify `plugin-herdr:test-sh` and `plugin-herdr:test-dash` pass.
- [ ] 4.3 Update the popup and segment examples in `plugins/herdr/README.md` and `site/src/content/docs/guides/herdr.md`; verify `moon run 'site:prose'` is clean.

## 5. The whole

- [ ] 5.1 Run the gate's checks with `sluice exec main` on the committed tree; verify every task passes.
