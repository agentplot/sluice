## Why

`sluice overview --json` names a branch that `sluice status <branch>`
then calls absent. In swancloud's person's copy on deneb
(`~/Code/github_afterthought/swancloud/main`, sluice 0.68.1) these rows
stand:

```
{"branch": "bolt/a-rift-step-finds-its-pane", "at": null, "submissions": 0, "recent": 0, "candidates": 0, ...}
{"branch": "bolt/the-herdr-sign-in-step-says-which-command-and-which-machine", "at": null, "submissions": 0, ...}
```

and `sluice status bolt/a-rift-step-finds-its-pane` answers `no branch
bolt/a-rift-step-finds-its-pane; branches with records: ...,
bolt/a-rift-step-finds-its-pane, ...`: absent, and listed as having
records, in one line.

Each bolt was made in that copy by `sluice branch create`, which records
the revision it made the branch at (121), and removed once it landed.
Every submission to it was made in another copy, so this copy's records
hold nothing else. The overview lists every branch the records hold
(`State::branches`). A status read no revision, no live submission and
nothing finished as no branch (`Status::is_unknown`), so it refused a
branch whose records exist.

Flume reads `sluice status <branch> --all --json` for every branch the
overview names, to find where each place's work went, so every Flume
place in swancloud and in sluice printed five to ten `cannot read: the
merge queue's bolt/...` lines.

A removed branch's records stay (105), every entry's state is readable
(26), the records are read given a branch (61), and a status of a branch
reads the live records and the index (131; rules core.11, core.41). A
status that refuses a branch whose records exist breaks them.

## What Changes

- **A removed branch answers a status from its records.** A branch is
  no branch to `sluice status <branch>` only when it has neither records
  nor a revision. A branch whose records exist answers, plain and with
  `--all`, with what they hold: its queue, its history when asked, and
  that the branch itself is gone. Its first line is `<branch> is gone`,
  with `, last at <revision>` where its records saw one; `--json` gives
  `gone` beside `at`.
- **"Gone" is read with no new fetch.** Where the records have seen no
  revision of the branch, the status's own read of the branch says it;
  otherwise the revision the host holds of it, as the overview reads
  `revision`.
- **The overview names a removed branch gone.** `sluice overview --json`
  keeps every branch with records in `branches[]` and gives a removed
  one `"gone": true`. The text names a gone branch with nothing to watch
  (nothing queued, under test or recent, no hold, held checkout, refused
  write or refused read) apart, on one line after the branches:
  `gone, nothing queued or recent: <branch>, ...`. A gone branch with
  something to watch is shown as any branch, its first line saying it
  is gone.

## Out of scope

- A removed branch whose records are on its remote, under a policy
  naming signers: it is routed by the revisions of it the host holds
  (124; rule core.27), and with none its records are read from the
  host's own files. A status and the overview agree on it, but neither
  reaches its records on the remote. Routing a branch to a remote no
  revision names is what core.27 rules out; it waits on the statement's
  word.
- Flume needs no change: its read-submission asks a status of every
  branch the overview names, and that status now answers.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `branch-lifecycle`: a removed branch's records answer its status,
  which says it is gone.
- `overview`: a removed branch is named gone, and the text shows one
  with nothing to watch apart.

## Impact

- `crates/sluice-core`: `Status::gone`, `Status::recorded` and
  `Status::is_unknown` in `ops.rs`; `BranchView::gone` in `overview.rs`.
- `crates/sluice`: `sluice status`'s text and `--json`; the text
  overview's `gone_line` and the line of gone branches.
- Records: none change. The overview document gains `gone`, left out
  while false, and `sluice status --json` gains `gone`.
- Statement: 105 amended to say a removed branch's records are read as
  any branch's and the branch named gone, on `agentplot/blueprints`
  branch `design/sluice-a-removed-branch-answers-its-status`, unpushed.
  Record 0091; the details of rule core.11.
