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
These nine are all the states there are: `sluice wait --json` and
`sluice status <id> --json` give one of them as `state`, and none is
renamed, removed or added under `sluice/submission@1`. Decided means
`landed`, `failed`, `conflict` or `withdrawn`.

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

A repository can tell sluice what its tests need, in `[tests]`:

- `ready`: a command that shows the host can run the tests, such as
  `["docker", "info"]`. sluice runs it before each run of the tests and
  refuses the run while it fails. It runs it again after a run in which a
  test did not pass, and if it fails then, those tests could not run.
- `not_run`: exits by which a test says it could not run here, such as
  `[77]`.

A test that could not run has no result. Nothing is recorded under its key,
the combination is tested again, and nobody is blamed:

```
could not run: cli:tools: the host failed [tests] ready `docker info` after the run: ended exit status: 1: Cannot connect to the Docker daemon
nothing was recorded for a test that could not run: once the host can run it, run again
```

When a combination's tests could not run three runs in a row, its newest
submission is blamed, and each test is named with why:

```
not run: cli:tools
  cli:tools could not run: exited 77, which [tests] not_run names: it could not run here
the host could not run cli:tools: once it can, submit again
```

A host failure that neither `ready` nor `not_run` catches is recorded like
any fail, under the test's key. A later pass on the same key then makes the
test read as flaky. On a branch with `flaky = "fail"`, submitting the same
head again cannot land it:

```
failed: checks:macbook-pro (flaky)
  on main + #63 (a557c99)
  checks:macbook-pro passed in this run and failed on the same inputs at 2026-10-17T15:28:04Z on 88de9be
  log of this run: .git/sluice/.logs/main/c64/checks:macbook-pro.log
  log of the run that failed: .git/sluice/.logs/main/c51/checks:macbook-pro.log
  if the host caused that fail, the operator can set it aside: sluice set-aside checks:macbook-pro --at 2026-10-17T15:28:04Z --reason "…"
  main reads a test that passed and failed on the same inputs as failed (flaky = "fail")
the same inputs read failed whenever they are tested: submit again with a change to what checks:macbook-pro reads
```

`sluice set-aside <test> --at <time> --reason <why>` keeps the fail's line
and records that no gate reads it again, so the same head can land. It is
the operator's to do, never an agent's: an agent that sets aside its own
fails could land past any test. The plugin's guard refuses it. Tell the
user what you found on the host, and the command.

In JSON, such a test's `failed[].flaky` holds `here`, this run's outcome,
and `other`, the result that contradicts it: its `outcome`, `at`,
`branch`, `combination` and, when kept, `log`.

A pull request sent back is tested again only on new commits, or once it
is marked a draft and then ready for review. A submit with no new commits
says it ran nothing: `#63's head bd85bd4 was tested and blamed at …:
nothing was queued or run again`, then the failure it was sent back for.

On GitHub, the pull request carries the same verdict as a report: one
comment in its conversation, the `sluice` check's details, and a check per
required test on each system (`sluice / app:test (linux-x64)`). Each test's
row says whether it ran or was replayed, where, how long it took and how
many tries; a failed test's output is posted only when the branch's policy
sets `log_lines`. A pull request tested together with a blamed one shows
none of that failure: its report says its combination is formed again.
The report decides nothing; read the failure from `sluice submit` or
`sluice status --json` as above.

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
