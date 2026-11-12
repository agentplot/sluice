# Verify: a-number-two-branches-hold-names-neither

Verified on 2026-11-12 at `81bdc695` on
`change/a-number-two-branches-hold-names-neither`, against its base
`bolt/a-queue-held-on-github-is-decided-again` (`045cb7d0`), with sluice
0.91.4 and openspec 1.14.3, the version `devenv.nix` pins.

**Verdict: the whole checks pass, every task is done, every requirement
and scenario holds, and the change archives clean. It is archived on
this branch.**

## The whole checks

`sluice exec bolt/a-queue-held-on-github-is-decided-again` ran the
bolt's gate, `#check-fast`, once. It exited 0 with `passed: 18 tasks`
and `recorded 18 results under their keys for 81bdc69`, in 6m 16s. It
ran the checks that `81bdc69` affects against the bolt at `045cb7d`.

The tasks were cli:test, conformance:test, core:test, fakes:test,
nix:typecheck, plugin-board:test, plugin-board:typecheck,
plugin-board:validate, plugin-herdr:manifest, plugin-herdr:test-dash,
plugin-herdr:test-sh, plugin-sluice:validate, root:adrs, root:changes,
root:format, root:lint, site:prose and site:rules.

- `fakes:test` passed 11, among them
  `numbers_from_gives_the_next_number`.
- `core:test` passed 154 with 0 failing.
- `conformance:test` ran the seven tests of
  `tests/number_on_two_branches.rs`, which passed.
- `cli:test` ran the fourteen tests of `number_on_two_branches::`,
  which passed.
- `plugin-board:test` passed 157 with 0 failing, among them "a blame
  whose number another branch holds opens its own branch's tests".
- `plugin-herdr:test-sh` and `plugin-herdr:test-dash` each said `all
  138 passed, 3 skipped`, among them "jump: a number another branch
  holds" and "learn: a number another branch holds".
- `root:format` formatted 387 files and changed none.
- `root:adrs` said `112 records, each with a number of its own`.
- `root:changes` said `10 passed, 0 failed`.
- `site:rules` said the page matches `docs/architecture/core.md`.
- `site:prose` found 0 errors and 0 warnings.
- `plugin-sluice:validate` and `plugin-board:validate` each said
  `Validation passed`.
- moon warned 18 times that it could not archive a task's outputs under
  the place's `.moon/cache/outputs`. Each warning is about caching
  only. Every task passed and had its result recorded, so the warnings
  do not change the outcome. They come from the host's `sluice exec`,
  not from this change.

Task 1.4's check is a test under `--features tools`. That feature
belongs to main's `#check-tools`, not the bolt's gate. It ran once, by
its own "True when":
`sluice exec bolt/a-queue-held-on-github-is-decided-again -- cargo test
-p sluice --features tools --test pull_requests -- closed_while_queued
one_of_a_group_on_a_new_head_is_withdrawn_alone` reported `2 passed; 0
failed`.

## The specs archive clean

The gate's `root:specs` validates `openspec/specs/`. This change does
not touch that directory until it is archived, so the gate did not run
it. The archive was checked separately, in a scratch copy of the
committed tree (`git archive HEAD openspec`), with the pinned openspec:

- `openspec validate a-number-two-branches-hold-names-neither --strict
  --no-interactive` reports the change valid.
- `openspec validate --specs --strict --no-interactive` reports `75
  passed, 0 failed` before the archive.
- `openspec archive a-number-two-branches-hold-names-neither --yes`
  applies `+ 9, ~ 2, - 0`. It creates `number-lookup` and updates
  `branch-refusal`, `herdr-toasts` and `plugin-board`.
- After the archive, `openspec validate --specs --strict
  --no-interactive` reports `76 passed, 0 failed`.

In the archived specs, `branch-refusal`'s requirement "A submission is
found past a branch that cannot be read" has 499 characters of text.
The longest of `number-lookup`'s requirements has 473. `number-lookup`
carries its own `## Purpose`.

## Requirements and scenarios

| Spec | Requirement | Scenario | Where it holds | Met |
|---|---|---|---|---|
| number-lookup | A number more than one branch holds names none of them alone | Withdraw refused naming both branches | `Sluice::find` reads every branch and answers `Found::Several`; `withdraw_in` refuses with `WithdrawRefusal::Several`. `cli.rs` `a_number_two_branches_hold_is_refused_naming_both` checks the line word for word, exit 2, both branches' record versions unchanged, `bolt/a` queued and `bolt/b` landed. Conformance `a_number_two_branches_hold_is_refused_naming_both`. | yes |
| number-lookup | A number more than one branch holds names none of them alone | Status and amend refused the same way | `status_and_amend_are_refused_the_same_way`: each exits 2 with the refusal, versions unchanged, both priorities 0. | yes |
| number-lookup | A number more than one branch holds names none of them alone | A branch that could not be read is named beside them | `Several.unread`; `several_lines` adds `<branch> could not be read: <words>`. `the_refusal_names_a_branch_that_could_not_be_read` checks both lines and exit 2. Conformance `a_branch_that_cannot_be_read_is_named_beside_them` checks the core. | yes |
| number-lookup | A number more than one branch holds names none of them alone | A number one branch holds | Conformance `a_number_one_branch_holds_is_found_as_before`: the other number is found and withdrawn on its one branch, and the shared number stays queued. | yes |
| number-lookup | A number more than one branch holds names none of them alone | (a moved submission) | `moved` reads every branch and returns each holder; `lookup_in` refuses where there is more than one. Conformance `a_number_two_branches_left_behind_is_refused`. | yes |
| number-lookup | A wait on a number more than one branch holds exits 64 | Never read as a conflict | `Command::Wait` calls `refuse_several(.., USAGE, ..)` before any round. `a_wait_refused_exits_64_and_runs_no_round` checks kind `refused`, exit 64, versions unchanged and nothing started. `WAIT_AFTER_HELP` gives 64 for the refusal; `wait_help_says_64_for_a_number_two_branches_hold`. | yes |
| number-lookup | The refusal's document names a command per branch | Under --json | `refuse_several` joins the lines with `; ` under `--json`, and gives `next` once per branch, in order. `under_json_the_refusal_names_a_command_per_branch` checks the message, `next` and exit 2. | yes |
| number-lookup | The commands take the branch | Each command acts on the branch named | `--branch` on `Wait`, `Status`, `Amend` and `Withdraw`, read through the `_on` calls. `each_command_given_the_branch_acts_on_that_branchs_submission`. Conformance `the_branch_named_is_read_alone`. | yes |
| number-lookup | The commands take the branch | A wait follows the branch named to its landing | `a_wait_given_the_branch_follows_it_to_its_landing`: `bolt/b`'s 26 landed, exit `LANDED`, and `bolt/a`'s 26 still queued. | yes |
| number-lookup | The commands take the branch | Another branch that cannot be read is not named | `find` with `on` reads that branch alone. `another_branch_that_cannot_be_read_is_not_named`, in text and under `--json`. | yes |
| number-lookup | The commands take the branch | The branch named holds no such number | `no_such` says `no submission 27 on bolt/a`, exit 1, next `sluice status bolt/a`. `a_branch_named_holding_no_such_number_is_no_submission_there` checks it for `status` and `wait`. Amend and withdraw refuse with exit 2, as their `no submission` does. Conformance `a_branch_named_that_holds_no_such_number_holds_none`. | yes |
| number-lookup | The commands take the branch | The branch named cannot be read | `a_branch_named_that_cannot_be_read_is_named`: the two lines, exit 3. Conformance `a_branch_named_that_cannot_be_read_is_named_alone`. | yes |
| number-lookup | --branch goes with a number | A branch target | `branch_goes_with_a_number`, exit `USAGE`. `branch_with_a_branch_target_is_a_usage_error` checks `wait main --branch`, `status <branch> --branch` and `status --branch`. | yes |
| number-lookup | A command given the branch names it in what it suggests | Under test | `asked` adds `--branch` to every hint; `Waiting.named` carries it through a wait. `a_hint_names_the_branch_the_command_was_given` checks the under-test text and `next`, and the owner's `next` with `--as`. | yes |
| number-lookup | What follows a submission reads its own branch | A waiting submit lands its own | `submit`, `submit_group` and `submit_pull` read with `state_on` on the target. `record` and `landed` read with `lookup_on`, `landed_as_on` and `state_on`. `a_waiting_submit_follows_its_own_branch_to_its_landing`. The pull request driver's three withdrawals call `withdraw_on(&sub.branch, ..)` and `withdraw_on(&e.branch, ..)`; task 1.4's tests passed. | yes |
| branch-refusal | A submission is found past a branch that cannot be read | A wait on another branch follows its submission to its verdict | One holder behaves as before; the existing tests ran in `cli:test` and passed. | yes |
| branch-refusal | A submission is found past a branch that cannot be read | Status, amend and withdraw find it the same way | As above. | yes |
| branch-refusal | A submission is found past a branch that cannot be read | A branch whose routing is refused stops no wait on another branch | As above. | yes |
| branch-refusal | A number two readable branches hold names the branch passed by | Two branches that read hold the number | `several_lines` names each holder, then each unread branch with the words it was refused with. `the_refusal_names_a_branch_that_could_not_be_read`, with `bolt/a` and `bolt/b` holding and `bolt/c` unread, checks both lines word for word. | yes |
| plugin-board | A press only reads | Every row pressed | "a press only reads" expects only `sluice status 9 --branch main --json` and `tail`. | yes |
| plugin-board | A press only reads | A blame whose number another branch holds | `blamedTests($, id, b.branch)`. The new test checks that `main`'s argv runs, never `feature/a`'s, and that `main`'s log opens. | yes |
| herdr-toasts | A culprit's worktree is asked of sluice with its branch | Jump to a culprit whose number another branch holds | `jump.sh` reads `.branch` from the event. "jump: a number another branch holds" checks the argv and that `p2` is focused, never `p1`. | yes |
| herdr-toasts | A culprit's worktree is asked of sluice with its branch | A blame's toast learns its tests | `learn` is given `$f3`. "learn: a number another branch holds" checks the argv, the toast's `tests:t-cards`, and the absence of `other:test`. | yes |

## Tasks

All fourteen tasks in `tasks.md` are ticked, and each one's "True when"
holds:

- 1.1: `World::numbers_from` and its test are in `sluice-fakes`;
  `fakes:test` passed.
- 1.2: `Several`, `Found::Several` and the three refusals' `Several`
  are in `sluice-core`. `find` reads every branch. `state` and
  `landed_as` answer `None` for a number two branches hold. `moved`
  reads every branch. The doc comments cite 26 and record 0114. The
  four conformance tests passed.
- 1.3: each of the five `_on` calls shares one private body with its
  number-alone call. The three conformance tests passed.
- 1.4: `pulls.rs`'s three withdrawals call `withdraw_on`. Its two tests
  passed (above).
- 2.1, 2.2, 2.3: `--branch`, the refusal, the hints and the reads that
  know the branch are in `crates/sluice/src/lib.rs`. The fourteen
  `number_on_two_branches::` tests passed.
- 2.4: record 0114 has "The act's form" under its outcome, with D4's
  and D5's paths not taken. `root:adrs` passed.
- 2.5: core.11's details carry record 0114's "Also rules out" and "Not
  ruled out", citing 94fd86b. `site:rules` passed.
- 2.6: the commands reference states the refusal, its exits and
  `--branch` in `sluice wait`'s section, the `amend` and `withdraw`
  rows and the exit codes paragraph. It names no host or tool of ours.
  `site:prose` passed with 0 warnings.
- 2.7: the using-sluice skill has the sentence.
  `plugin-sluice:validate` passed.
- 3.1: `plugin-board:test` passed.
- 3.2: `plugin-herdr:test-sh` passed.
- 4.1: `branch-refusal`'s case of several holders is a requirement of
  its own, and `number-lookup` has a `## Purpose`. The change validates
  strictly, and once archived the specs validate strictly (above).

## Rules kept

- core.11: no lookup by a number that two branches hold answers for,
  or acts on, either of them. The refusal names every holder and every
  branch it could not read. A command given `--branch` reads that
  branch alone.
- core.4: the refused wait runs no round, as
  `a_wait_refused_exits_64_and_runs_no_round` checks.
- core.26: the site, the skill and the output name no host or tool of
  ours. Record 0114 keeps the host where the problem was first seen, as
  records may.
- core.12: no clause is reinterpreted. Clause 26 was amended first in
  the statement, at agentplot/blueprints 94fd86b.
