# Tasks

Before 1.1: this bolt must hold `bolt/sluice-lands-only-what-its-records-say`
(through main), so `FailedRelease`, `ReleaseFailure` and `sluice release
<name> --accept-failure` exist. Check with `git grep -n "pub struct
FailedRelease" crates/sluice-core/src/records.rs`.

## 1. The overview's document and text

- [x] 1.1 Give `BranchView` its `held_checkouts`, cloned from the
  branch's `failed_releases` in `branch_view`, always serialized, and
  update the core's own `view` test helper; verify with a unit test in
  `crates/sluice-core/src/overview.rs` that a branch with one stopped and
  one retrying failed release lists both, and one with none an empty
  list (`core:test`)
- [x] 1.2 Add a conformance test to
  `crates/sluice-conformance/tests/places.rs`, using `held_checkout`,
  that `branch_view_of(BRANCH)` lists the held checkout with `stopped`
  and its `failed`, and that reading the overview ten times leaves the
  branch's state, its log and the checkout's release count unchanged;
  verify with `conformance:test`
- [x] 1.3 In `crates/sluice/src/overview.rs`, print under each branch's
  line its held places as design D2 words them: stopped checkouts from
  `held_checkouts`, worktrees from `sessions[]` and previews from
  `previews[]` whose `release_failed` is stopped, each with the last
  line of what failed and its two commands, another host's named; then,
  after the branches, a `<branch> · no records` line for held places of
  a branch the document does not list. Verify with `cli:test` tests in
  `crates/sluice/tests/cli.rs`: a held checkout (text and
  `branches[].held_checkouts` in `--json`), a held worktree with
  `--worktree <path>`, a checkout still being retried printing nothing,
  and a branch with no records
- [ ] 1.4 Document it on the commands page: the `sluice overview`
  example and text gain a held checkout's two lines, the field table
  gains `branches[].held_checkouts` (as `sluice status --json` lists
  them), and the section on checkouts held because their release failed
  says the overview, herdr and the Claude Code board show them; verify
  with `site:prose`

## 2. herdr

- [ ] 2.1 In `plugins/herdr/bin/document.sh`, default
  `branches[].held_checkouts` to an empty list, keep only objects with a
  string `place`, and replace each `failed` (the branch's and each
  `release_failed.failed`) by its last non-empty line before control
  characters are stripped (design D4); verify with a `tests/run.sh` case
  feeding a document with a three-line `failed` and one with no
  `held_checkouts` (`plugin-herdr:test-sh`)
- [ ] 2.2 In `plugins/herdr/lib/words.jq`, add the held places of a
  branch (design D2), the RELEASE FAILED rows first under the branch's
  heading in `rows`, a branch with held places counted as shown, and
  the `N held: release failed` clause on `branch_line` (with a line for
  a branch with nothing queued); verify with `tests/run.sh` cases for
  each `herdr-status` scenario of this change: a held checkout, a held
  worktree, another host's checkout, an idle branch, one still being
  tried, an older document, and the segment's two lines
  (`plugin-herdr:test-sh`); update `tests/docs/board.json` and
  `board.frame` if the popup's fixture gains a held checkout
- [ ] 2.3 Make `queue_token` end with `N held: release failed`, set with
  nothing queued, a checkout two documents list counted once; verify
  with `tests/run.sh` cases for the three `herdr-sidebar` scenarios
  (`plugin-herdr:test-sh`), and `plugin-herdr:test-dash` still passing
- [ ] 2.4 Document the RELEASE FAILED rows and the `held: release
  failed` clause in `plugins/herdr/README.md` and
  `site/src/content/docs/guides/herdr.md`; verify with `site:prose`

## 3. The Claude Code board

- [ ] 3.1 In `plugins/sluice-board/types/index.d.ts`, add
  `OverviewBranch.held_checkouts?`, `OverviewSession.release_failed?`
  and a preview's `branch` and `release_failed?`; verify with
  `plugin-board:typecheck`
- [ ] 3.2 In `plugins/sluice-board/hooks/board.ts`, add the held places
  of a branch and their rows' words (design D2), count them in `shown`,
  `busy`, `branchLine` and `statusLine` (design D3, D5); verify with
  `tests/board.test.ts` cases for each `plugin-board` scenario of this
  change, the herdr popup's words matched line for line
  (`plugin-board:test`)
- [ ] 3.3 In `plugins/sluice-board/hooks/register.tsx`, draw RELEASE
  FAILED first under a branch's heading, each place in red with its
  command dimmed beneath, no row pressable; verify with a
  `tests/register.test.tsx` case over a fixture with a held checkout
  and a held worktree, and one over an older document
  (`plugin-board:test`)
- [ ] 3.4 Document it in `plugins/sluice-board/README.md` and
  `site/src/content/docs/guides/claude-code.md`; verify with
  `site:prose`

## 4. The whole

- [ ] 4.1 Run the gate's checks with `sluice exec main` once the work is
  committed, and confirm every task of `core`, `conformance`, `cli`,
  `plugin-herdr`, `plugin-board`, `site` and `root` passes
