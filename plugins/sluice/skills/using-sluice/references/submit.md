# Submitting

Full reference: <https://agentplot.github.io/sluice/reference/commands/>.

## Flags

| flag | effect |
|---|---|
| `--check` | Only forecast: would the head conflict with the branch and everything queued? Writes nothing. Prints `no conflict with <branch> or anything queued` or `would conflict: <paths> with #<id> (<agent>, <worktree>)`. |
| `--no-wait` | Return once queued (or once the pull request is open). |
| `--status` | Where this worktree's last submission, or its pull request, stands. |
| `--after 2,5` | Land only after those submissions. |
| `--priority N` | Higher goes first, ahead of lower-priority submissions not yet under test; the default is 0. |
| `--worktree <path>` | Submit from another worktree than the current directory. |
| `--json` | One JSON document: the submission record when it finishes; `{id,branch,ahead}` with `--no-wait`; `{"conflict":null}` or `{"conflict":{paths,with}}` with `--check`; `{pull_request,branch,state}` for a pull request. |

## What a waiting submit prints

```
submission 12 to bolt/x · 1 queued ahead (#11)
testing bolt/x + #11 + #12 · 4 tasks to run, 29 replayed
passed · landed as b7d2e94
```

`N tasks to run, M replayed`: with moon, only affected tasks run, and a task
whose inputs did not change may replay its earlier result, when the branch
trusts `"cache"`.

## Who moves the queue

A round is one pass of sluice's work on a branch: read the queue, start
tests if there is room, read finished tests, blame a failure, land what
passed. Any process may run one; a lock lets one run at a time.

- A waiting `sluice submit` / `wt submit` runs rounds while it waits.
- `sluice step <branch>`: one round, then exit.
- `sluice run`: rounds for every branch until stopped. Needed for pull
  requests opened by hand, which nobody waits on.

## Branches on GitHub

With `remote = "origin"` on the branch, every round fetches the branch
first, and a landing is a push that only succeeds if the branch has not
moved (always a fast-forward of what it tested on). "landed" is reported only once GitHub accepted the push. A refused
push is logged as `refused` and the next round tests again on the new tip.

## Pull requests

With `[branches.main.pull_requests]`:

```
$ wt submit main
pushed unit/batch-review · opened #41: Batch review cards
waiting: needs 1 review, qa/approved
  forecast: merges without conflict behind #38, #40
queued behind #38, #40
testing main + #38 + #40 + #41
passed · landed as b7d2e94
```

- The pull request waits while it is a draft or misses a review or a check
  the policy names, then joins the queue.
- New commits send it back to waiting.
- sluice posts a `sluice` check on its latest commit with the state, and a
  comment when it sends one back.
- A submit following a pull request exits `1` when it is closed without
  landing.
- Never press GitHub's merge button on such a branch.

## Landing commits

One merge commit per submission: first parent the branch, second parent your
head. Trailers record what was tested:

```
Sluice-Submission: 12
Sluice-From: crw-1-unit-1 (unit/touches)
Sluice-Tested: 9ab3d07c11
Sluice-Result: passed (2 run, 31 replayed)
```

The plain `sluice submit` leaves your worktree and branch in place;
`wt submit` removes them on landing (`--no-remove` keeps them).
