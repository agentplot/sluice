# Design

## The records' layout

```text
<branch>/
  branch.json           the queue's document, as now
  submissions/<id>.json submissions still in a chain: queued, under test, landing
  finished/<id>.json    submissions out of the queue, moved there as written
  log.jsonl             the current log segment
  log/<n>.jsonl         closed segments, oldest first, each at most 1,000 lines
  version               as now
```

A finished submission's file is the same document it was as the last
write to `submissions/` left it (53). A reader of an older sluice's
records sees every submission under `submissions/` and none under
`finished/`, so it reads as before; a newer sluice reading an older
branch moves its finished files on its next write.

## The records seam

- `read(branch)` returns the live set: `branch.json`, `submissions/`,
  `log.jsonl` and the newest closed segment, so the overview's ten
  recent lines are always in it.
- `read_finished(branch, id) -> Option<Vec<u8>>` reads one finished
  submission; `read_log(branch) -> Vec<u8>` reads every segment in
  order, for a reader that asks for the whole log.
- `Changes` gains `remove: BTreeSet<String>`, applied in the same all
  or nothing write as `put` and `append` (63): moving a submission is a
  `put` of `finished/<id>.json` and a `remove` of `submissions/<id>.json`.
- Every profile of the seam implements the three; the local files
  profile, the one there is, by directory.

## The core

- `Line::write` moves each submission whose state is out of the queue
  in the write that puts it out.
- `find(id)` reads `branch.json` and `submissions/` from each branch's
  live read, then `read_finished`; ids are unique across a repository,
  so the first answer is the only one.
- `state(id)` reads one branch's live records, or one finished file:
  never every submission of every branch.
- The log's segment is closed by the write that appends its 1,001st
  line, under the branch's lock, as every write is (rule core.14).

## What a read costs after it

| Read | Now | After |
|---|---|---|
| a round, `overview`, the home view, `status <branch>` | every submission and the whole log | the queue and at most 2,000 log lines |
| `status <id>` of a finished submission | every submission of every branch | the live records and one file |
| `status <branch> --all`, `--json --all` | every submission | the same, from `finished/` |

## Risks

- A write that moves a file is two operations on disk; the local files
  profile already makes a write whole through its pending file (51) and
  carries the removal in it.
- A reader that globbed `submissions/*.json` for history, such as a
  script, sees only the queue. The state page names `finished/`.
- `status <branch> --json` changing to the queue alone breaks a reader
  that read history from it; `e2e/e2e.sh` does, and passes `--all`.
