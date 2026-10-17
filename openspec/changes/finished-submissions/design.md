# Design

## The records' layout

```text
<branch>/
  branch.json           the queue's document, as now
  submissions/<id>.json submissions not yet finished: in a chain, landing,
                        blamed or conflicting
  .finished/<id>.json   submissions landed or withdrawn, moved there as written
  finished.jsonl        one line per finished submission: id, state, worktree
  log.jsonl             the current log segment
  log/<n>.jsonl         closed segments, oldest first, each at most 1,000 lines
  version               as now
```

A finished submission's file is the same document it was as the last
write to `submissions/` left it (53). A reader of an older sluice's
records sees every submission under `submissions/` and none under
`.finished/`, so it reads as before; a newer sluice reading an older
branch moves its finished files on its next write.

## The records seam

- `read(branch)` returns the live set: everything but `.finished/`;
  after part 2, `log.jsonl` and the newest closed part only, so the
  overview's ten recent lines are always in it.
- `read_finished(branch, name)` reads one finished submission and
  `read_all_finished(branch)` every one; after part 2, a `read_log`
  reads every part in order, for a reader that asks for the whole log.
- `.finished` starts with a dot, which no git branch's component can,
  so it never meets a nested branch's records, and the local files
  profile's walk already passes over it.
- `Changes` gains `remove: BTreeSet<String>`, applied in the same all
  or nothing write as `put` and `append` (63): moving a submission is a
  `put` of `.finished/<id>.json`, a `remove` of `submissions/<id>.json`
  and an append to `finished.jsonl`.
- Every profile of the seam implements the three; the local files
  profile, the one there is, by directory.

## The core

- `Line::changes` moves each submission landed or withdrawn that no
  outbox item, landing or combination names, in any write that changes
  something; a submission named by one moves on a later write.
- Ordering's landed set, `after`'s known ids and `attempt` read the
  index as well as the live submissions.
- `find(id)` reads each branch's live records and index; ids are unique
  across a repository, so the first answer is the only one.
- `state(id)` reads one branch's live records, or one finished file:
  never every submission of every branch.
- `amend` and `withdraw` of a finished submission are refused as before
  it moved: not the asker's, else past changing, read from its file.
- Part 2: the log's part is closed by the write that appends its 1,001st
  line, under the branch's lock, as every write is (rule core.14).

## What a read costs after it

| Read | Now | After |
|---|---|---|
| a round, `overview`, the home view, `status <branch>` | every submission and the whole log | the queue and the index; after part 2, at most 2,000 log lines |
| `status <id>` of a finished submission | every submission of every branch | the live records and one file |
| `status <branch> --all`, `--json --all` | every submission | the same, from `.finished/` |

## Risks

- A write that moves a file is two operations on disk; the local files
  profile already makes a write whole through its pending file (51) and
  carries the removal in it.
- A reader that globbed `submissions/*.json` for history, such as a
  script, sees only the queue. The state page names `.finished/`.
- `status <branch> --json` changing to the queue alone breaks a reader
  that read history from it; `e2e/e2e.sh` does, and passes `--all`.
