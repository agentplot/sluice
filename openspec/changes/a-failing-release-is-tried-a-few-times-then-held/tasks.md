# Tasks

Work in this change's place on
`change/a-failing-release-is-tried-a-few-times-then-held`, cut from
`bolt/what-a-place-starts-ends-with-it`. That bolt is the base every
`sluice exec` below names. Where the bolt has moved since this branch
was cut, merge it in first: sibling changes there touch
`crates/sluice/src/profiles/places.rs` too (design, Risks). A quick
test runs directly. A test of the check-tools tier and a moon task run
through `sluice exec bolt/what-a-place-starts-ends-with-it --`, as
CLAUDE.md's Running checks has it.

A task that changes what an existing test expects updates that test in
the same task. The tests named are in design's Task notes 2.x.

Commit by pathspec, with Conventional Commits subjects scoped to the
part of the tree touched, citing the new record and rule core.16 or
core.30.

## 1. The decision recorded

- [x] 1.1 Write the record of design D8 in `docs/adr/`, as Task notes 1.1 gives it, numbered 0116 or the next free number where 0116 is taken on main or this bolt. Add its "Amended in part" line to record 0112. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run root:adrs` passes.
- [x] 1.2 In `docs/architecture/core.md`, amend core.16's and core.30's details as Task notes 1.2 words them, each citing the record, and rewrite `site/src/content/docs/architecture/rules.md` with `node site/scripts/rules.mjs`. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run site:rules` passes.

## 2. One bound, whoever asks

- [x] 2.1 `Checkout::release_session` in `crates/sluice-seams/src/lib.rs` takes `retries`, documented as `release_preview`'s are. `Sluice::release_retries` in `crates/sluice-core/src/release.rs` is public. `exec::release` in `crates/sluice/src/exec.rs` passes it, read for the record's branch, or for the default branch where sluice holds nothing (design D1). The place profile's `release_held` (`crates/sluice/src/profiles/places.rs`) and the fake place seam (`crates/sluice-fakes/src/lib.rs`) keep a failed try held only once `tries > retries`. S27 in `crates/sluice-conformance/src/shared.rs`, `a_sessions_place_is_released_under_the_policy_read_now` and every other caller pass retries. `a_failed_session_release_keeps_its_place_with_what_failed` becomes `a_failed_session_release_is_held_once_its_retries_are_spent` (Task notes 2.x). `status_marks_a_worktree_whose_release_failed` expects `stopped` false. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test places a_failed_session_release_is_held_once_its_retries_are_spent` passes.
- [x] 2.2 An unheld worktree's failed release keeps the record it wrote, with the failure counted against the retries and its scratch, in the place profile's `release_unheld` and in the fake place seam. A passing release still lets go of it (design D3). `release_session`'s doc and `Released::Nothing`'s say so. `exec::release`'s failure words no longer say sluice holds nothing for the worktree, and its doc says what is kept. Add `an_unheld_worktrees_failed_release_keeps_its_place` to `crates/sluice/tests/profiles/places.rs` (Task notes 2.x). True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test places an_unheld_worktrees_failed_release_keeps_its_place` passes.
- [ ] 2.3 `an_unheld_worktrees_release_runs_its_release_command` in `crates/sluice/tests/exec.rs` expects that, after the failing release, `sluice status` lists the worktree with one failed try, and that `sluice release --accept-failure` there frees it (spec "An unheld worktree's release answers as a held one's and keeps only its failure"). True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test exec an_unheld_worktrees_release_runs_its_release_command` passes.
- [ ] 2.4 The place profile's `adopt` gives a session's record with no ports block a block where the settings name ports, written as for a record it makes, and keeps the record's `release_failed`. `Checkout::adopt`'s doc says so (design D3, D6). Add `a_worktrees_place_without_ports_is_given_them_at_its_next_run` to `crates/sluice/tests/profiles/places.rs`. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test places a_worktrees_place_without_ports_is_given_them_at_its_next_run` passes.

## 3. Every try has its scratch

- [ ] 3.1 The place profile's `release_held`, `release` and `release_preview` make the scratch the place's record names with `make_scratch` before any release command runs, as `adopt` does (design D4). The module's doc says every release does. Add `every_release_runs_with_the_scratch_its_record_names` to `crates/sluice/tests/profiles/places.rs` (Task notes 2.x). True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test places every_release_runs_with_the_scratch_its_record_names` passes.

## 4. A held place runs again only when asked again

- [ ] 4.1 `sluice release` takes `--again`, refused beside `--accept-failure`, with help that says it runs a held place's release once more (`crates/sluice/src/lib.rs`). `exec::release` answers a held worktree asked without it as design D2 and D5 give it: nothing run, no policy read, the held words, `--json` with `held` and `tries`, exit 1. With it, the release runs once more. A failed answer's `--json` carries `held` and `tries`. `sluice exec --release` asks without it. Add `a_worktrees_release_is_tried_its_branchs_retries_then_held` to `crates/sluice/tests/cli.rs` (Task notes 2.x). True when `cargo test -p sluice --test cli a_worktrees_release_is_tried_its_branchs_retries_then_held` passes.
- [ ] 4.2 `Sluice::release_held` in `crates/sluice-core/src/release.rs` takes `again`. A stopped checkout asked without it answers `Releasing::StillHeld { failed, tries }`, with nothing run and nothing written (design D2, D6). `release_checkout` in `crates/sluice/src/lib.rs` passes `--again` and gives D5's words and document. `tell_releases` and `releases_json` in `crates/sluice/src/exec.rs` name `--again` for a hold and give `StillHeld` as `held`. `a_held_checkout_is_released_by_its_name` in `crates/sluice-conformance/tests/places.rs`, `release_by_name_releases_a_held_checkout` and `a_waiting_submit_says_a_held_checkout_once` in `crates/sluice/tests/cli.rs` expect it. True when `cargo test -p sluice --test cli release_by_name_releases_a_held_checkout` passes.
- [ ] 4.3 `preview::release` in `crates/sluice/src/preview.rs` takes `--again`. A held preview asked without it is answered as design D2 and D5 give it, read from the preview the seam lists, with nothing run. `tell_dropped` and the `sluice previews` line name `--again` for a held preview. Add `a_held_previews_release_runs_again_only_when_asked_again` to `crates/sluice/tests/previews.rs` (Task notes 2.x). True when `cargo test -p sluice --test previews a_held_previews_release_runs_again_only_when_asked_again` passes.
- [ ] 4.4 `sluice status` gives a worktree's failed release as design D5 words it: `<n> of <m> tries` while tries remain, `<m>` read with `Sluice::release_retries` for the worktree's branch, and held with `--again` once held. The footer under a branch's checkouts names `--again` for a held one (`crates/sluice/src/lib.rs`, `write_worktrees` and the checkouts' footer). `status_marks_a_worktree_whose_release_failed` expects the new words. True when `cargo test -p sluice --test cli status_marks_a_worktree_whose_release_failed` passes.
- [ ] 4.5 The overview's held lines give `sluice release <it> --again runs it again; sluice release <it> --accept-failure frees it without` (`write_held_places` in `crates/sluice/src/overview.rs`). `overview_names_a_held_worktree_under_its_branch` and `overview_names_a_held_worktree_of_a_branch_with_no_records` hold their worktree by failing its release on every try, and these two and `overview_names_a_held_checkout_under_its_branch` expect `--again`. True when `cargo test -p sluice --test cli overview_names_a_held_worktree_under_its_branch` passes.

## 5. The plugins

- [ ] 5.1 The board's held line gives `sluice release <it> --again · --accept-failure frees it without` (`plugins/sluice-board/hooks/board.ts`, `heldLines`), and its tests and README expect it. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run plugin-board:test` passes.
- [ ] 5.2 The herdr plugin's held line gives the same (`plugins/herdr/lib/words.jq`, `held_lines`), and `plugins/herdr/tests/run.sh` and its README expect it. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run plugin-herdr:test-sh` passes.

## 6. The docs

- [ ] 6.1 Say it on the site, as Task notes 3.x gives it: `site/src/content/docs/reference/commands.md`, `site/src/content/docs/reference/configuration.md`, `site/src/content/docs/guides/herdr.md` and `site/src/content/docs/guides/claude-code.md`. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run site:prose` passes.
- [ ] 6.2 The setup skill's `release_retries` bullet in `plugins/sluice/skills/setting-up-sluice/SKILL.md` says a worktree's and a preview's failing release is held the same way, and that `sluice release … --again` runs a held one again. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run plugin-sluice:validate` passes.
