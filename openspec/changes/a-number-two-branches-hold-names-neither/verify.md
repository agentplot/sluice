# Verify: a-number-two-branches-hold-names-neither

Verified on 2026-11-12 at `b5666c6b` on
`change/a-number-two-branches-hold-names-neither`, against its base
`bolt/a-queue-held-on-github-is-decided-again` (`045cb7d0`), with sluice
0.91.4.

**Verdict: the whole checks pass, every task is done and every scenario
holds, but the work falls short. Once archived, the delta specs leave
`openspec/specs/branch-refusal/spec.md` and
`openspec/specs/number-lookup/spec.md` failing `openspec validate
--strict`. The change is not archived, and it goes back to its code
stage.**

## What falls short

`openspec archive a-number-two-branches-hold-names-neither --yes`, run on
a scratch copy of the committed tree (`git archive HEAD openspec`),
applies the deltas cleanly: `+ 8, ~ 2, - 0`. Afterwards, `openspec
validate --specs --strict --no-interactive` reports `74 passed, 2
failed`. On the tree as it stands it reports `75 passed, 0 failed`.

| Spec | Warning `--strict` fails | Cause |
|---|---|---|
| `branch-refusal` | `requirements[0]: Requirement text is very long (>500 characters)` | The modified requirement "A submission is found past a branch that cannot be read" grows from 499 characters on the base to 671. The added sentence, "Where more than one branch holds the number, the lookup is refused as the `number-lookup` capability says, naming the branch passed by beside them (record 0114).", takes it over. |
| `number-lookup` | `overview: Purpose section is still a placeholder` | The capability is new, and its delta has no `## Purpose`, so the archive writes its placeholder sentence. The warning says a delta's `## Purpose` is read when the capability is created, which this archive does. |

The gate cannot see this. `root:changes` runs `openspec validate
--changes --strict`, which passes for the change. `root:specs`, a
`check-fast` task, runs `openspec validate --specs --strict` over
`openspec/specs/`, which the change has not touched yet, so this run
skipped it. Once the archive lands on the bolt, `root:specs` would fail
there. CLAUDE.md asks
that `openspec/specs/` change only through a change validated strictly
and archived so the spec reconciles.

What the code stage does:

- In `specs/branch-refusal/spec.md`, bring the requirement's text under
  500 characters. For example, move the sentence on several holders into
  the scenario "Two branches that read hold the number", which states it
  already, or into a requirement of its own.
- In `specs/number-lookup/spec.md`, add a `## Purpose` saying what the
  capability is for, ahead of `## ADDED Requirements`.
- Then archive in a scratch copy and check that `openspec validate
  --specs --strict` passes before the change comes back here.

## The whole checks

`sluice exec bolt/a-queue-held-on-github-is-decided-again` ran the
bolt's gate, `#check-fast`, once. It exited 0: `passed: 18 tasks`,
`recorded 18 results under their keys for b5666c6`, in 6m 15s. It ran
the checks `b5666c6` affects against the bolt at `045cb7d`.

The tasks were cli:test, conformance:test, core:test, fakes:test,
nix:typecheck, plugin-board:test, plugin-board:typecheck,
plugin-board:validate, plugin-herdr:manifest, plugin-herdr:test-dash,
plugin-herdr:test-sh, plugin-sluice:validate, root:adrs, root:changes,
root:format, root:lint, site:prose and site:rules.

- `fakes:test` ran `numbers_from_gives_the_next_number`, which passed.
- `conformance:test` ran the seven tests of
  `tests/number_on_two_branches.rs`, which passed.
- `cli:test` ran the fourteen tests of `number_on_two_branches::`,
  which passed.
- `plugin-board:test` passed 157 with 0 failing, among them "a blame
  whose number another branch holds opens its own branch's tests".
- `plugin-herdr:test-sh` and `plugin-herdr:test-dash` each said `all
  138 passed, 3 skipped`, among them "jump: a number another branch
  holds" and "learn: a number another branch holds".
- `root:adrs` said `112 records, each with a number of its own`.
- `root:changes` said `10 passed, 0 failed`.
- `site:rules` said the page matches `docs/architecture/core.md`. The
  change edits core.11's details alone, which the page does not carry.
- `site:prose` found 0 errors and 0 warnings.
- `plugin-sluice:validate` said `Validation passed`.
- moon warned 18 times that it could not archive a task's outputs under
  the place's `.moon/cache/outputs`. Each warning is about caching only.
  Every task passed and its result was recorded, so the warnings do not
  affect the outcome. They come from the host's `sluice exec`, not from
  this change.

Task 1.4's check is a test under `--features tools`, which is main's
`#check-tools` and outside the bolt's gate. It ran once, by its own
"True when": `sluice exec bolt/a-queue-held-on-github-is-decided-again
-- cargo test -p sluice --features tools --test pull_requests --
closed_while_queued one_of_a_group_on_a_new_head_is_withdrawn_alone`
exited 0, `2 passed`.

`openspec validate a-number-two-branches-hold-names-neither --strict`
reports the change valid.

## Requirements and scenarios

| Spec | Requirement | Scenario | Where it holds | Met |
|---|---|---|---|---|
| number-lookup | A number more than one branch holds names none of them alone | Withdraw refused naming both branches | `Sluice::find` reads every branch and answers `Found::Several`. `withdraw_in` refuses with `WithdrawRefusal::Several`. `cli.rs` `a_number_two_branches_hold_is_refused_naming_both` checks the line word for word, exit 2, both branches' record versions unchanged, `bolt/a` queued and `bolt/b` landed. | yes |
| number-lookup | A number more than one branch holds names none of them alone | Status and amend refused the same way | `status_and_amend_are_refused_the_same_way`: each exits 2 with the refusal, versions unchanged, both priorities 0. | yes |
| number-lookup | A number more than one branch holds names none of them alone | A branch that could not be read is named beside them | `Several.unread`; `several_lines` adds `<branch> could not be read: <words>`. `the_refusal_names_a_branch_that_could_not_be_read` checks both lines and exit 2. Conformance `a_branch_that_cannot_be_read_is_named_beside_them` checks the core. | yes |
| number-lookup | A number more than one branch holds names none of them alone | A number one branch holds | Conformance `a_number_one_branch_holds_is_found_as_before`: the other number is found and withdrawn on its one branch, and the shared number stays queued. | yes |
| number-lookup | A number more than one branch holds names none of them alone | (a moved submission) | `moved` reads every branch and returns each; `lookup_in` refuses where more than one. Conformance `a_number_two_branches_left_behind_is_refused`. | yes |
| number-lookup | A wait on a number more than one branch holds exits 64 | Never read as a conflict | `Command::Wait` calls `refuse_several(.., USAGE, ..)` before any round. `a_wait_refused_exits_64_and_runs_no_round` checks kind `refused`, exit 64, versions unchanged, nothing started. `WAIT_AFTER_HELP` says 64 for it; `wait_help_says_64_for_a_number_two_branches_hold`. | yes |
| number-lookup | The refusal's document names a command per branch | Under --json | `refuse_several` joins the lines with `; ` under `--json`, `next` once per branch in order. `under_json_the_refusal_names_a_command_per_branch` checks message, `next` and exit 2. | yes |
| number-lookup | The commands take the branch | Each command acts on the branch named | `--branch` on `Wait`, `Status`, `Amend`, `Withdraw`, read through the `_on` calls. `each_command_given_the_branch_acts_on_that_branchs_submission`. | yes |
| number-lookup | The commands take the branch | A wait follows the branch named to its landing | `a_wait_given_the_branch_follows_it_to_its_landing`: `bolt/b`'s 26 landed, exit `LANDED`, `bolt/a`'s queued. | yes |
| number-lookup | The commands take the branch | Another branch that cannot be read is not named | `find` with `on` reads that branch alone. `another_branch_that_cannot_be_read_is_not_named`, in text and `--json`. | yes |
| number-lookup | The commands take the branch | The branch named holds no such number | `no_such` says `no submission 27 on bolt/a`, exit 1, next `sluice status bolt/a`. `a_branch_named_holding_no_such_number_is_no_submission_there`, for `status` and `wait`. Amend and withdraw refuse with exit 2, as their `no submission` does. | yes |
| number-lookup | The commands take the branch | The branch named cannot be read | `a_branch_named_that_cannot_be_read_is_named`: the two lines, exit 3. | yes |
| number-lookup | --branch goes with a number | A branch target | `branch_goes_with_a_number`, exit `USAGE`. `branch_with_a_branch_target_is_a_usage_error` checks `wait main --branch`, `status <branch> --branch` and `status --branch`. | yes |
| number-lookup | A command given the branch names it in what it suggests | Under test | `asked` adds `--branch` to every hint; `Waiting.named` carries it through a wait. `a_hint_names_the_branch_the_command_was_given` checks the under-test text and `next`, and the owner's `next` with `--as`. | yes |
| number-lookup | What follows a submission reads its own branch | A waiting submit lands its own | `submit`, `submit_group` and `submit_pull` read with `state_on` on the target; `record` and `landed` read with `lookup_on`, `landed_as_on` and `state_on`. `a_waiting_submit_follows_its_own_branch_to_its_landing`. The pull request driver's three withdrawals call `withdraw_on(&sub.branch, ..)`; task 1.4's tests passed. | yes |
| branch-refusal | A submission is found past a branch that cannot be read | A wait on another branch follows its submission to its verdict | Unchanged behaviour for one holder; its existing tests ran in `cli:test` and passed. | yes |
| branch-refusal | A submission is found past a branch that cannot be read | Status, amend and withdraw find it the same way | As above. | yes |
| branch-refusal | A submission is found past a branch that cannot be read | A branch whose routing is refused stops no wait on another branch | As above. | yes |
| branch-refusal | A submission is found past a branch that cannot be read | Two branches that read hold the number | `the_refusal_names_a_branch_that_could_not_be_read`, with `bolt/a`, `bolt/b` holding and `bolt/c` unread. | yes |
| plugin-board | A press only reads | Every row pressed | "a press only reads" expects only `sluice status 9 --branch main --json` and `tail`. | yes |
| plugin-board | A press only reads | A blame whose number another branch holds | `blamedTests($, id, b.branch)`. The new test checks `main`'s argv is run, never `feature/a`'s, and `main`'s log opens. | yes |
| herdr-toasts | A culprit's worktree is asked of sluice with its branch | Jump to a culprit whose number another branch holds | `jump.sh` reads `.branch` from the event. "jump: a number another branch holds" checks the argv and that `p2` is focused, never `p1`. | yes |
| herdr-toasts | A culprit's worktree is asked of sluice with its branch | A blame's toast learns its tests | `learn` is given `$f3`. "learn: a number another branch holds" checks the argv, the toast's `tests:t-cards`, and no `other:test`. | yes |

## Tasks

All thirteen tasks in `tasks.md` are ticked, and each one's "True when"
holds:

- 1.1: `World::numbers_from` and its test are in `sluice-fakes`;
  `fakes:test` passed.
- 1.2: `Several`, `Found::Several` and the three refusals' `Several`
  are in `sluice-core`. `find` reads every branch. `state` and
  `landed_as` answer `None`. `moved` reads every branch. The doc
  comments cite 26 and record 0114. The four conformance tests passed.
- 1.3: the five `_on` calls share one private body each with their
  number-alone call. The three conformance tests passed.
- 1.4: `pulls.rs`'s three withdrawals call `withdraw_on`. Its two tests
  passed (above).
- 2.1, 2.2, 2.3: `--branch`, the refusal, the hints and the reads that
  know the branch are in `crates/sluice/src/lib.rs`. The fourteen
  `number_on_two_branches::` tests passed.
- 2.4: record 0114 has "The act's form" under its outcome, with D4's
  and D5's paths not taken. `root:adrs` passed.
- 2.5: core.11's details carry the "Also rules out" and "Not ruled
  out" of record 0114, citing 94fd86b. `site:rules` passed.
- 2.6: the commands reference says the refusal, its exits and
  `--branch`, in `sluice wait`'s section, the `amend` and `withdraw`
  rows and the exit codes paragraph, naming no host or tool of ours.
  `site:prose` passed with 0 warnings.
- 2.7: the using-sluice skill has the sentence. `plugin-sluice:validate`
  passed.
- 3.1: `plugin-board:test` passed.
- 3.2: `plugin-herdr:test-sh` passed.

## Rules kept

- core.11: no lookup by a number two branches hold answers for, or
  acts on, one of them. The refusal names every holder and every
  unread branch. A command given `--branch` reads that branch alone.
- core.4: the refused wait runs no round, as
  `a_wait_refused_exits_64_and_runs_no_round` checks.
- core.26: the site, the skill and the output name no host or tool of
  ours. Record 0114 keeps the host it was met on, as records may.
- core.12: no clause is reinterpreted. Clause 26 was amended in the
  statement first, at agentplot/blueprints 94fd86b.
