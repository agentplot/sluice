# Verify

The work on `change/a-landing-beside-the-queue-names-the-verdict-that-covers-it`
at 63bcb041, which holds main at 3e0ae764, checked on 2026-11-05 against
this change's proposal, design, its delta spec `landing-beside`, and its
tasks, with main still at 3e0ae764.

**The work holds.** Every requirement and scenario of the spec is met by
the code and by the test the design's table names for it, and each of
those tests passed. All 21 tasks are ticked, each one's condition holds,
and the checks the tasks name pass. Task 5.2's notes are in the place's
notes, which are not committed; this report carries them under "After
the landing" below.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 63bcb041, against main at 3e0ae764 | 15 of 15 tasks passed in 12m 28s, recorded under their keys for 63bcb04: core:test 131 tests, seams:test 2, fakes:test 7, conformance:test 270, cli:test 569 and cli:tools 527, none failed; cli:e2e 44 checks ok, none not ok; root:format, root:lint, root:adrs, root:changes, root:screenshots-scene, site:prose, site:rules and plugin-sluice:validate. Every test the tables below name passed among them |
| `openspec validate a-landing-beside-the-queue-names-the-verdict-that-covers-it --strict` | valid |
| `git merge-base --is-ancestor c7ee329d HEAD` (1.4) | succeeds; main's 3e0ae764 is also in the branch, by 042d327c |
| `git -C ~/Code/github_agentplot/blueprints/blueprints.git log -1 --format=%s 1b21406` (1.2) | names 23, 140 and S68 |
| `git -C ~/Code/github_agentplot/blueprints/main log origin/main --format=%s \| grep -c 'record 0083'` (1.3) | 1: eb0d3eb, `design(sluice): a landing beside the chain names as its verdict its own, … (23, 140, S68; sluice record 0083, accepted on the user's word 2026-11-06)` |
| `git log --format=%s main..HEAD` | two `feat` commits, eaa5968a `feat(seams)` and 6f8e54b8 `feat(core)`, so the release is a minor one; the trailer's change is 0358d486 `fix(core)`, the forge's f5eed6bc `fix(cli)` |

The report was written outside the worktree while the gate ran, so the
run was recorded under its keys (rule core.18).

moon warned 33 times across the run. 28 warnings say it could not store
a task's outputs or a blob under its caches, four that it was asked to
hash a directory as an input, and one that it skipped an install for
want of a network. Every task still passed. None of these warnings comes
from this change; the code stage's notes name the same two cache
warnings.

`git grep "landed beside the queue"` outside the archive, the records
and this change finds no reader of the old trailer's words: only doc
comments, the words of submit, wait and the notice (out of scope), the
main spec's requirement title, and the pages this change wrote. Nothing
in the tree parses `Sluice-Result` but the tests.

The covering verdict is decided in the records alone (rule core.39,
core.11, core.14): `BranchDoc::covered_by` is written only by
`perform_landing`, from the intent's `covering` where the gate is the
same and to `None` otherwise, and cleared to `None` beside `verified`
where a round finds the branch moved by another hand
(`crates/sluice-core/src/step.rs`). No code reads a commit's message for
it.

## Requirements and scenarios

### landing-beside (modified)

| Requirement | Scenario | Where it is met | Test, and the task it passes in |
|---|---|---|---|
| A landing beside's result reads as passed with nothing run (added) | A reader of the head reads it as passed | `beside_message` in `crates/sluice-core/src/text.rs` writes `Sluice-Submission`, `Sluice-From` and `Sluice-Result: passed (0 run, 0 replayed) beside the queue: no required check reads its changes; …`, no `Sluice-Tested` | `a_landing_beside_reads_as_passed_to_git`, against real git through the local refs profile: both trailer readings give one line, first word `passed`, whole as written, and `Sluice-Tested` empty; cli:tools |
| | A combination lands after a landing beside | `landing_message` unchanged in form, its result through `verdict_words` | `docs_land_beside_two_code_submissions_under_test_and_they_land_carrying_them`, extended: both of the combination's landings carry `Sluice-Tested` and their own `passed (`, no `beside the queue`; conformance:test. The through-queue message whole: `shared.rs`'s S1 and `text.rs`'s tests, conformance:test and core:test |
| A landing beside names the result that covers it (added) | Docs landed beside after a tested landing | `choose_beside` passes `line.doc.covered_by` to `beside_message`, which writes `covered by the result at <at>, <verdict_words>`; the chain's landing writes the intent's `covering` at its last commit with the deepest candidate's words, which that commit's own `Sluice-Result` names (`step.rs`) | `a_landing_beside_names_its_result_and_the_result_that_covers_it`, conformance:test: no `Sluice-Tested`, and the result `<BESIDE>; covered by the result at <L in full>, <L's own words>`; `a_landing_beside_names_passed_and_what_covers_it`, the whole message for a 40-digit revision, `passed`, 141 run, 0 replayed, core:test |
| | Two landings beside in a row | `perform_landing` for a landing beside sets `covered_by` to the intent's `covering`, which is the branch's `covered_by` as it stood | `two_landings_beside_in_a_row_both_land_with_the_combination`, extended: both covered by the result at `L`, the second naming no revision of the first; conformance:test |
| | Covered by a combination that landed after a landing beside | the chain's landing's `covering` | `docs_land_beside_two_code_submissions_…`, extended: a further docs submission is covered by the result at the combination's last landing, in its words; conformance:test |
| What never lands beside (modified) | The policy; A runner that reads every check; The reading fails | unchanged | `the_policy_never_lands_beside` and the tests beside it in `beside.rs`, conformance:test |
| | A revision no result covers | `choose_beside` returns where `verified` does not hold; a moved branch clears `covered_by` with `verified` | `nothing_lands_beside_on_a_revision_no_verdict_covers`, asserting `covered_by` unset on the branch and the submission; conformance:test |
| | State that names no covering result | `choose_beside` returns where `covered_by` is `None`, `verified` or not | `state_that_names_no_covering_result_takes_the_queue_once`: `covered_by` removed from `branch.json` under the branch's lock, the docs land through the queue with `Sluice-Tested`, the branch then covered at that landing, and the next docs land beside covered by it; conformance:test |
| The outcome says it landed beside the queue (modified) | The waiting submit's words; JSON; The notice | unchanged: `words.rs`'s `landed_beside` and `notice` pass `covered_by` by with `..`, the only change to them, so the words and `beside: true` are as record 0075's change built and tested them | `beside_the_queue_in_words` and the notice test in `words.rs`, the notice's built with `covered_by: None`; cli:test |
| | The request's check names the covering result | `outcome_shown` in `crates/sluice/src/profiles/github/pulls.rs` appends ` Covered by the result at \`<short>\`, <words>.` where `covered_by` is set, today's summary where not; `render::has_report` is true for a landing beside with `covered_by`, and `draw` writes the head, then the line with the revision linked to its commit, and no row | `a_landing_beside_the_queue_says_no_check_ran`, extended with both summaries, and `landed_testing_and_formed_again`, extended: a landing beside with a covering verdict has the head and the line, one with none no report; cli:test |
| | A request's earlier report is brought up to date | `show_report` edits the comment found by the host's id or its marker, and returns before the checks per required check for any landing beside | `a_landing_beside_edits_the_requests_report`, over a `FileForge`: the blamed report posted, then the same comment edited once to the landing beside's report, no second comment, the `sluice` check's text that report, and only the `sluice` check on the new head; cli:test |

The records written before the change read and write back unchanged:
`a_branch_and_an_outcome_from_before_covered_by_still_read`, core:test,
over a `branch.json` with `verified: true` and no `covered_by`, an
intent with no `covering`, an outbox item and a submission landed beside
with none, and the same with them named.

The scenario "A combination lands after a landing beside" asks that
`M`'s `Sluice-Tested` name its combination. The extended test asserts a
`Sluice-Tested` line on each of the combination's landings and not its
value; the value is `landing_message`'s `verdict.revision`, which this
change did not touch, and `shared.rs`'s S1 asserts it whole.

## Tasks

21 of 21 are ticked, and each one's condition holds:

- 1.1, 1.3: record 0083 is accepted on the user's words of 2026-11-06,
  "Approve: push the statement change and start its code", citing
  agentplot/blueprints main at eb0d3eb. Rule core.39 and its details
  name the landing commit's verdict and cite eb0d3eb and record 0083;
  the site's rules page carries the rule as core.md words it.
  `root:adrs`, `root:changes` and `site:rules` pass in the gate.
- 1.2: as the checks above.
- 1.4: as the checks above; the merge is 37b69928, and main was taken in
  again at 3e0ae764 by 042d327c.
- 2.1, 2.2: `CoveringVerdict` in `crates/sluice-seams/src/lib.rs` as D3
  gives it, and `covered_by` on `Decided::Landed` and
  `Standing::Landed`; `BranchDoc::covered_by`, `LandingIntent::covering`,
  `Submission::covered_by` and `DecidedRecord::Landed`'s `covered_by` in
  `records.rs`, each `serde(default, skip_serializing_if =
  "Option::is_none")`, and `to_decided` and `Submission::decided`
  carrying it.
- 3.1, 3.2: `verdict_words`, `landing_message` through it, and
  `beside_message` with the covering verdict, its doc comment citing 15,
  23, 70, 140 and record 0083. In `step.rs`: the chain's intent's
  `covering` at its last commit; `choose_beside` returning where
  `covered_by` is unset, passing it to the message, setting each
  submission's `covered_by` and the intent's `covering`;
  `perform_landing` setting the branch's `covered_by` under the same
  gate, carrying each beside submission's into its outbox item, and
  clearing a refused submission's; the moved branch clearing it.
- 3.3, 3.4: the conformance and git tests named in the table above.
- 3.5, 4.5: `reference/state.md` shows the trailer, says its first word
  is the landing's own result, `passed` on every landing, with what it
  ran, and describes `covered_by` on `branch.json` and a submission;
  `guides/walkthrough.md` shows the landing beside's trailer;
  `references/submit.md` shows it and says a tool reads the first word;
  `guides/pull-requests.md`'s row names the covering result. No team or
  tool of ours is named on the pages or in the plugin (rule core.26):
  `git diff main...HEAD -- site plugins` adds no such word.
  `site:prose` and `plugin-sluice:validate` pass in the gate.
- 4.1 to 4.4: as the table above.
- 2.3, 3.6, 4.6, 5.1: the group checks, which the gate's run above
  covers, every task they name among its 15.
- 5.2: the notes are in the place's notes, `code.md`, under "Once the
  change lands (for the orchestrator)". They are not committed, as the
  code stage is told; this report carries them below. No task of the
  change waits on the landing.

## After the landing

The orchestrator's, once the submission lands on main, after the code,
verify and submission stages:

1. **Confirm the release.** CI's `version` job reads git-cliff's next
   version from the landing and its `release` job tags and publishes it:
   a minor one, v0.69.0 where no other release comes first, since
   eaa5968a and 6f8e54b8 are `feat` commits and `cliff.toml` sets
   `features_always_bump_minor`. `gh release list -R agentplot/sluice -L
   3` shows it at the landing commit.
2. **Tell crew's owner** (afterthought/crew): a landing beside's
   `Sluice-Result` now opens with `passed`, e.g. `passed (0 run, 0
   replayed) beside the queue: no required check reads its changes;
   covered by the result at <revision>, passed (141 run, 0 replayed)`,
   with no `Sluice-Tested`, so `kit.verification` (`plugin/lib/kit.py`,
   `result.split()[0] == "passed"`) reads it as passed with no change of
   crew's. A bolt whose head is a landing beside written before the
   release, switchboard-kit's at b01e966 among them, reads "verification
   unknown" until its next landing; reading the old words as passed,
   walking first parents to the covering landing, would be crew's own
   change, which this change does not ask.
3. **Tell swancloud's owner** (afterthought/swancloud): the same
   trailer's first line matches the releaser's `passed*`
   (`packages/swancloud-releaser/releaser.sh`), so a docs-only landing
   on swancloud's main builds a release once its hosts run the release,
   with no change of swancloud's. A main whose head is an older landing
   beside builds nothing until its next landing.
4. **The statement's markers.** agentplot/blueprints main still marks
   23, 140 and S68 "(record 0083, proposed 2026-11-05)", though record
   0083 is accepted. Updating the markers is a blueprints commit for the
   orchestrator, not this change.
5. **A flaky test on main, outside this change.** The code stage's
   first gate at 699b9c37 failed cli:test on
   `reader::tests::a_free_lock_is_left_to_the_lease_holder`
   (`crates/sluice/src/reader.rs`, `assert!(is_lead(&other))`), a file
   this branch never touched; alone it passed 15 times in 15. The code
   stage's notes give the likely cause, another test thread's started
   process holding a copy of the reader's lock between its fork and its
   exec, as cae8bc26 fixed for the lease probe. It wants its own fix on
   main.
