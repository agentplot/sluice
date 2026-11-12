## Why

Clause 26, as amended on agentplot/blueprints's branch
`design/sluice-a-number-two-branches-hold-names-neither` (94fd86b), says
a number that entries on more than one branch hold names none of them
alone. A lookup by it is refused, naming each branch that holds it, and
the act that asked takes the branch to name one. Record 0114 decides it.

A branch whose records are on its remote takes its numbers from the
remote's count, and a branch in the host's own files from the host's.
One host can read two submissions with one number. On a host on
2026-11-12, `sluice withdraw 26` meant a remote-held bolt's #26 and was
refused, since another bolt held a landed #26 in the host's own files.
`sluice withdraw 25` failed the same way.

What sluice does at `045cb7d0`:

- `Sluice::find` in `crates/sluice-core/src/lib.rs` reads the branches
  in the order the records list them, by name, and stops at the first
  that holds the number.
- `lookup`, `state`, `landed_as`, `amend` and `withdraw` in
  `crates/sluice-core/src/ops.rs` all go through `find`, so each acts
  on, or answers for, that first branch's submission, saying nothing.
- `sluice wait`, `status`, `amend` and `withdraw` take a number alone.
  Nothing reaches the second branch's submission.
- A wait's follow, a waiting submit's own reads, the pull request
  driver's withdrawals, the board's and the herdr plugin's
  `sluice status <n> --json` all look up by number alone, though each
  knows the branch.

## What Changes

- **A number names a submission only where one branch holds it.** A
  lookup by number reads every branch it can. Where more than one
  holds the number, it is refused, naming each, and nothing is done.
  The branches that could not be read are named beside them (26; rule
  core.11). A number one branch holds is found as now.
- **The four commands take the branch.** `sluice wait`, `status`,
  `amend` and `withdraw` take `--branch <branch>` with a number, and
  then read that branch alone. A command given the branch names it in
  every command it suggests for that submission.
- **The refusal moves its reader on.** It names each branch, and under
  `--json` its `next` gives the command with `--branch` once for each.
  `status`, `amend` and `withdraw` exit `2`, as their refusals do. A
  wait exits `64`, since its `2` is a conflict.
- **Whatever knows the branch names it.** A wait's follow, a waiting
  submit's reads of its own submission, the pull request driver's
  withdrawals, the board's read of a blame's tests and the herdr
  plugin's read of a culprit's worktree each read the submission's own
  branch.

## Capabilities

### New Capabilities

- `number-lookup`: a lookup of a submission by its number refuses a
  number more than one branch holds, naming each, and the commands that
  look one up take its branch.

### Modified Capabilities

- `branch-refusal`: a submission is found past a branch that cannot be
  read where one branch that reads holds its number.
- `plugin-board`: a press reads a blamed submission's tests by its
  number and its branch.
- `herdr-toasts`: the culprit's worktree is read by its number and its
  branch.

## Impact

- `crates/sluice-core`: `Found`, `Lookup`, `AmendRefusal` and
  `WithdrawRefusal` gain the number held on several branches. `find`
  reads every branch. `lookup_on`, `state_on`, `landed_as_on`,
  `amend_on` and `withdraw_on` read one branch.
- `crates/sluice-fakes`: `World::numbers_from`, so a test can make two
  branches hold one number.
- `crates/sluice`: `--branch` on the four commands, the refusal, the
  wait's follow, the waiting submit's reads, and `pulls.rs`'s
  withdrawals.
- `plugins/sluice-board` and `plugins/herdr`: their `sluice status`
  calls name the branch.
- Records: nothing changes. Numbers stay as each count gives them.
- Docs: record 0114 gains the act's form. Rule core.11's details rule
  out acting on one of two. The site's commands reference and the
  using-sluice skill say how to name the branch.
- Out of scope:
  - Numbering every branch from one count, which record 0114 rejects.
  - The notices' words, `sluice inbox` and `sluice confirm`'s keys,
    which record 0114 does not name. A hint a notice gives keeps the
    bare number; where two branches hold it, the refusal names both.
  - The herdr plugin's cache of worktrees keyed by number alone.
  - Boomdrive's `entry` (`sluice status <id> --json`) and crew's
    conductor naming the branch. Each is its own repository's change,
    named in the design.
