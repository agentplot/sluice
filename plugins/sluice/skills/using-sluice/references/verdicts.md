# Verdicts, culprits and conflicts

## Submission states

| state | meaning |
|---|---|
| `queued` | in the queue, not under test |
| `waiting` | an `--after` submission is neither landed nor ahead of it |
| `testing` | in a combination under test |
| `passed` | its combination passed; it lands next |
| `landing` | its landing is written and the branch is being moved |
| `landed` | on the branch; `landed_as` is the landing commit |
| `failed` | blamed for a failure |
| `conflict` | does not merge with what is ahead of it |
| `withdrawn` | taken out, by its agent, a planner, the operator, or a resubmission |

`failed` and `conflict` are out of the queue until you submit again.

## Reading a failure

```
failed: tests:t-batch-cards
  on bolt/x + #12 + #13 (e20a6f9)
  #12 passed on its own, so the failure is from this submission
  log of tests:t-batch-cards: .git/sluice/.logs/bolt/x/c7/tests:t-batch-cards.log
fix it in your worktree and submit again
```

- sluice blames one submission, and only after every combination ahead of it
  has a result. If the combination ahead passed, the failure is yours.
- `on <combination>`: the exact revision tested, the branch plus every
  submission ahead plus yours. A failure may come from an interaction with
  work ahead of you that passes alone.
- The log path is under `$(git rev-parse --git-common-dir)/sluice/.logs/`
  (`.git/sluice/` in a normal clone, `project.git/sluice/` in a bare one). The last 20 failed combinations keep
  their logs.
- `timed out`: the run hit the branch's `timeout`. Check for a test or a
  server that never exits.
- A test with no result (skipped, never reached) counts as failed and has
  no `hash` or log.

From JSON:

```sh
sluice status 13 --json | jq '{state, result, tested, failed}'
```

`tested.combination` is the revision, `tested.submissions` who was in it,
`failed[].task` and `failed[].log` the tests and their logs.

## A failure that is not your code

A test can fail because of the host: docker down, a full disk, a slow
program start, a port held by a stray process. Check the host before you
touch code or resubmit.

Every result stays on record under its key, the test and its inputs' hash.
A later pass on the same key makes that test read as flaky. On a branch
with `flaky = "fail"`, then, once a host failure is recorded, submitting the
same head again cannot land it. Fix the host, then submit
with a change to the failing test's inputs, not a retry of the same commits.

## Reading a conflict

```
conflict: plugin/lib/plan.py
  conflicts with #12 (crw-1-unit-1, unit/touches)
merge or rebase onto bolt/x in your worktree and submit again
```

Bring the branch (and, if it has not landed yet, wait for #12, or rebase on
it once it lands) into your worktree, resolve, commit, submit again. Run
`sluice submit --check <branch>` first to see whether it now merges.

## Recent landings and blames for the whole branch

```sh
sluice overview --json | jq '.branches[] | {branch, recent}'
```

`recent` holds the branch's last ten `landed`, `blamed` and `conflict` log
lines exactly as written.
